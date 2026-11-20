//! The scoring engine.

use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

use crate::config::{ConfigError, FieldParameters, MatchConfig};
use crate::contact::Contact;
use crate::nicknames::Nicknames;
use crate::normalize::{self, Name, Phone, PhoneMatch};

/// Log-odds are clamped here so a result always serialises (JSON has no
/// infinity) and the probability never rounds to exactly 0 or 1 by accident.
const LOG_ODDS_LIMIT: f64 = 40.0;
/// Similarity credited to a known nickname (`Bob` for `Robert`).
const NICKNAME_SIMILARITY: f64 = 0.97;
/// Similarity credited to an initial (`J` for `John`).
const INITIAL_SIMILARITY: f64 = 0.85;

/// A field the matcher compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Field {
    /// Given name.
    GivenName,
    /// Family name.
    FamilyName,
    /// Email addresses.
    Email,
    /// Phone numbers.
    Phone,
    /// Organisation.
    Organization,
    /// Job title.
    JobTitle,
}

/// What a field said about the pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Outcome {
    /// Full agreement: counts toward a match.
    Agree,
    /// Close but not equal: a share of the agreement weight.
    Partial,
    /// Disagreement: counts against a match.
    Disagree,
    /// One side has no value: no evidence either way.
    Missing,
}

/// One field's contribution, for explaining a result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FieldScore {
    /// Which field.
    pub field: Field,
    /// What it said.
    pub outcome: Outcome,
    /// The similarity, `0.0..=1.0` (`0.0` when missing).
    pub similarity: f64,
    /// Added to the log-odds of a match. Positive favours a match.
    pub weight: f64,
}

/// The decision, from the probability and the configured thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Verdict {
    /// Probably the same person.
    Match,
    /// A person should look.
    Review,
    /// Probably different people.
    NonMatch,
}

/// The outcome of comparing two contacts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MatchResult {
    /// The chance the two contacts are the same person, `0.0..=1.0`.
    pub probability: f64,
    /// The natural log of the odds behind it: the prior plus every weight.
    pub log_odds: f64,
    /// The decision.
    pub verdict: Verdict,
    /// True when a shared `person_ref` decided it, whatever the fields say.
    pub deterministic: bool,
    /// Whether the given and family names matched better when swapped
    /// (`Ruiz Ana` against `Ana Ruiz`).
    pub names_swapped: bool,
    /// Every field, in a fixed order, including missing ones.
    pub fields: Vec<FieldScore>,
}

/// A candidate's place in a ranking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ranked {
    /// Position of the candidate in the slice that was passed in.
    pub index: usize,
    /// The comparison with the query.
    pub result: MatchResult,
}

/// Compares contacts. Cheap to share: it holds only the configuration and
/// the nickname table.
#[derive(Clone, Debug)]
pub struct Matcher {
    config: MatchConfig,
    nicknames: Nicknames,
}

impl Default for Matcher {
    fn default() -> Self {
        let config = MatchConfig::default();
        Self {
            nicknames: Nicknames::new(&config.nicknames),
            config,
        }
    }
}

/// A contact with every field prepared once, so a one-to-many comparison
/// does not redo the work for the query.
struct Prepared {
    person_ref: Option<String>,
    name: Name,
    emails: Vec<String>,
    phones: Vec<Phone>,
    organization: String,
    job_title: String,
}

impl Matcher {
    /// A matcher with the given configuration.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] if the configuration does not validate.
    pub fn new(config: MatchConfig) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self {
            nicknames: Nicknames::new(&config.nicknames),
            config,
        })
    }

    /// The configuration in use.
    #[must_use]
    pub const fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Compare two contacts. The result is symmetric:
    /// `compare(a, b)` equals `compare(b, a)`.
    #[must_use]
    pub fn compare(&self, a: &Contact, b: &Contact) -> MatchResult {
        self.score(&self.prepare(a), &self.prepare(b))
    }

    /// Compare one contact with many. The result is parallel to `candidates`.
    #[must_use]
    pub fn compare_one_to_many(&self, query: &Contact, candidates: &[Contact]) -> Vec<MatchResult> {
        let query = self.prepare(query);
        candidates
            .iter()
            .map(|c| self.score(&query, &self.prepare(c)))
            .collect()
    }

    /// Compare one contact with many and sort the answer: highest
    /// probability first, ties broken by the candidate's position so the
    /// order is the same every time. Candidates whose verdict is
    /// [`Verdict::NonMatch`] are left out.
    #[must_use]
    pub fn rank_one_to_many(&self, query: &Contact, candidates: &[Contact]) -> Vec<Ranked> {
        let mut ranked: Vec<Ranked> = self
            .compare_one_to_many(query, candidates)
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.verdict != Verdict::NonMatch)
            .map(|(index, result)| Ranked { index, result })
            .collect();
        ranked.sort_by(|x, y| {
            y.result
                .probability
                .total_cmp(&x.result.probability)
                .then(x.index.cmp(&y.index))
        });
        ranked
    }

    fn prepare(&self, c: &Contact) -> Prepared {
        let mut emails: Vec<String> = c
            .emails
            .iter()
            .filter_map(|e| normalize::email(e, self.config.fold_gmail))
            .collect();
        emails.sort();
        emails.dedup();
        let code = self.config.default_calling_code.as_deref();
        Prepared {
            person_ref: c
                .person_ref
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            name: normalize::name_of(
                c.given_name.as_deref(),
                c.family_name.as_deref(),
                c.display_name.as_deref(),
            ),
            emails,
            phones: c
                .phones
                .iter()
                .filter_map(|p| normalize::phone(p, code))
                .collect(),
            organization: c
                .organization
                .as_deref()
                .map_or_default(normalize::organization),
            job_title: c.job_title.as_deref().map_or_default(normalize::job_title),
        }
    }

    fn score(&self, a: &Prepared, b: &Prepared) -> MatchResult {
        let cfg = &self.config;
        let (given, family, names_swapped) = self.name_similarities(&a.name, &b.name);

        let shared = |x: &[String], y: &[String]| x.iter().any(|e| y.contains(e));
        let email = (!a.emails.is_empty() && !b.emails.is_empty())
            .then(|| f64::from(u8::from(shared(&a.emails, &b.emails))));
        let phone = (!a.phones.is_empty() && !b.phones.is_empty()).then(|| {
            let best = a
                .phones
                .iter()
                .flat_map(|p| b.phones.iter().map(|q| normalize::phone_match(p, q)))
                .max()
                .unwrap_or(PhoneMatch::None);
            match best {
                PhoneMatch::Exact => 1.0,
                PhoneMatch::Suffix => f64::midpoint(cfg.agree_at, cfg.disagree_at),
                PhoneMatch::None => 0.0,
            }
        });
        let organization = (!a.organization.is_empty() && !b.organization.is_empty())
            .then(|| text_similarity(&a.organization, &b.organization));
        let job_title = (!a.job_title.is_empty() && !b.job_title.is_empty())
            .then(|| title_similarity(&a.job_title, &b.job_title));

        let fields = vec![
            self.field(Field::GivenName, given, cfg.given_name),
            self.field(Field::FamilyName, family, cfg.family_name),
            self.field(Field::Email, email, cfg.email),
            self.field(Field::Phone, phone, cfg.phone),
            self.field(Field::Organization, organization, cfg.organization),
            self.field(Field::JobTitle, job_title, cfg.job_title),
        ];

        let deterministic = matches!((&a.person_ref, &b.person_ref), (Some(x), Some(y)) if x == y);
        let log_odds = if deterministic {
            LOG_ODDS_LIMIT
        } else {
            let prior = (cfg.prior / (1.0 - cfg.prior)).ln();
            (prior + fields.iter().map(|f| f.weight).sum::<f64>())
                .clamp(-LOG_ODDS_LIMIT, LOG_ODDS_LIMIT)
        };
        let probability = if deterministic {
            1.0
        } else {
            1.0 / (1.0 + (-log_odds).exp())
        };
        let verdict = if probability >= cfg.match_threshold {
            Verdict::Match
        } else if probability >= cfg.review_threshold {
            Verdict::Review
        } else {
            Verdict::NonMatch
        };
        MatchResult {
            probability,
            log_odds,
            verdict,
            deterministic,
            names_swapped,
            fields,
        }
    }

    /// Similarity of given and family names, trying the pair as written and
    /// swapped, and keeping whichever agrees better.
    ///
    /// Swapped, each name is matched against the *other* side's opposite
    /// part, and the two cross comparisons are averaged and reported for
    /// both fields. Averaging keeps `compare(a, b)` equal to `compare(b, a)`.
    fn name_similarities(&self, a: &Name, b: &Name) -> (Option<f64>, Option<f64>, bool) {
        let straight = (
            self.name_similarity(&a.given, &b.given),
            text_option(&a.family, &b.family),
        );
        let all_present = [&a.given, &a.family, &b.given, &b.family]
            .iter()
            .all(|s| !s.is_empty());
        if all_present {
            let p = self.name_similarity(&a.given, &b.family).unwrap_or(0.0);
            let q = self.name_similarity(&a.family, &b.given).unwrap_or(0.0);
            let swapped = f64::midpoint(p, q);
            let total = straight.0.unwrap_or(0.0) + straight.1.unwrap_or(0.0);
            if 2.0 * swapped > total + 1e-9 {
                return (Some(swapped), Some(swapped), true);
            }
        }
        (straight.0, straight.1, false)
    }

    fn name_similarity(&self, a: &str, b: &str) -> Option<f64> {
        if a.is_empty() || b.is_empty() {
            return None;
        }
        if a == b {
            return Some(1.0);
        }
        if self.nicknames.related(a, b) {
            return Some(NICKNAME_SIMILARITY);
        }
        let initial =
            |short: &str, long: &str| short.chars().count() == 1 && long.starts_with(short);
        if initial(a, b) || initial(b, a) {
            return Some(INITIAL_SIMILARITY);
        }
        Some(jaro_winkler(a, b))
    }

    fn field(&self, field: Field, similarity: Option<f64>, p: FieldParameters) -> FieldScore {
        let Some(similarity) = similarity else {
            return FieldScore {
                field,
                outcome: Outcome::Missing,
                similarity: 0.0,
                weight: 0.0,
            };
        };
        let agree = (p.m / p.u).ln();
        let disagree = ((1.0 - p.m) / (1.0 - p.u)).ln();
        let (agree_at, disagree_at) = (self.config.agree_at, self.config.disagree_at);
        let (outcome, weight) = if similarity >= agree_at {
            (Outcome::Agree, agree)
        } else if similarity <= disagree_at {
            (Outcome::Disagree, disagree)
        } else {
            let share = (similarity - disagree_at) / (agree_at - disagree_at);
            (Outcome::Partial, disagree + (agree - disagree) * share)
        };
        FieldScore {
            field,
            outcome,
            similarity,
            weight,
        }
    }
}

fn text_option(a: &str, b: &str) -> Option<f64> {
    if a.is_empty() || b.is_empty() {
        None
    } else {
        Some(text_similarity(a, b))
    }
}

fn text_similarity(a: &str, b: &str) -> f64 {
    if a == b { 1.0 } else { jaro_winkler(a, b) }
}

/// Titles are compared as the better of character similarity and word
/// overlap, so `Senior Engineer` and `Engineer, Senior` agree.
fn title_similarity(a: &str, b: &str) -> f64 {
    let (wa, wb): (Vec<&str>, Vec<&str>) = (a.split(' ').collect(), b.split(' ').collect());
    let shared = wa.iter().filter(|w| wb.contains(w)).count();
    let union = wa.len() + wb.len() - shared;
    #[allow(clippy::cast_precision_loss)]
    let overlap = shared as f64 / union as f64;
    text_similarity(a, b).max(overlap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weight(result: &MatchResult, field: Field) -> &FieldScore {
        result.fields.iter().find(|f| f.field == field).unwrap()
    }

    // With every field missing, the probability is the prior itself; with some present, it is the
    // prior's log-odds plus each weight.
    #[test]
    fn the_probability_is_the_prior_plus_the_weights() {
        let matcher = Matcher::default();
        let prior = matcher.config.prior;
        let nothing = matcher.compare(&Contact::new(), &Contact::new());
        assert!(
            (nothing.probability - prior).abs() < 1e-12,
            "{}",
            nothing.probability
        );
        let a = Contact::new().given_name("Ana").email("ana@example.com");
        let b = Contact::new().given_name("Ana").email("bo@example.com");
        let result = matcher.compare(&a, &b);
        let sum: f64 = result.fields.iter().map(|f| f.weight).sum();
        let expected = (prior / (1.0 - prior)).ln() + sum;
        assert!(
            (result.log_odds - expected).abs() < 1e-12,
            "{} {expected}",
            result.log_odds
        );
        assert!((result.probability - 1.0 / (1.0 + (-expected).exp())).abs() < 1e-12);
    }

    // A field between the bands is weighted in proportion to where it falls between them.
    #[test]
    fn a_partial_field_is_weighted_by_its_place_between_the_bands() {
        let matcher = Matcher::default();
        let p = matcher.config.given_name;
        let (agree_at, disagree_at) = (matcher.config.agree_at, matcher.config.disagree_at);
        let agree = (p.m / p.u).ln();
        let disagree = ((1.0 - p.m) / (1.0 - p.u)).ln();
        let similarity = disagree_at + 0.25 * (agree_at - disagree_at);
        let score = matcher.field(Field::GivenName, Some(similarity), p);
        assert_eq!(score.outcome, Outcome::Partial);
        let expected = disagree + (agree - disagree) * 0.25;
        assert!(
            (score.weight - expected).abs() < 1e-12,
            "{} {expected}",
            score.weight
        );
    }

    // A name or title on one side only is missing, not a disagreement.
    #[test]
    fn a_field_on_one_side_only_is_missing() {
        let matcher = Matcher::default();
        let a = Contact::new()
            .given_name("Ana")
            .family_name("Ruiz")
            .job_title("VP Sales");
        let b = Contact::new().family_name("Ruiz");
        let result = matcher.compare(&a, &b);
        assert_eq!(weight(&result, Field::GivenName).outcome, Outcome::Missing);
        assert_eq!(weight(&result, Field::JobTitle).outcome, Outcome::Missing);
        let c = Contact::new().given_name("Ana");
        assert_eq!(
            weight(&matcher.compare(&a, &c), Field::FamilyName).outcome,
            Outcome::Missing
        );
    }

    // Only a single letter counts as an initial; a longer prefix is scored by its characters.
    #[test]
    fn only_one_letter_is_an_initial() {
        let matcher = Matcher::default();
        assert_eq!(
            matcher.name_similarity("a", "anabel"),
            Some(INITIAL_SIMILARITY)
        );
        assert_eq!(
            matcher.name_similarity("ana", "anabel"),
            Some(jaro_winkler("ana", "anabel"))
        );
    }

    // Names that are as good straight as swapped are not called swapped.
    #[test]
    fn a_tie_is_not_a_swap() {
        let matcher = Matcher::default();
        let lee = Contact::new().given_name("Lee").family_name("Lee");
        assert!(!matcher.compare(&lee, &lee).names_swapped);
        let a = Contact::new().given_name("Ana").family_name("Ruiz");
        let b = Contact::new().given_name("Ruiz").family_name("Ana");
        assert!(matcher.compare(&a, &b).names_swapped);
    }

    // The word overlap is the shared words over all the different words.
    #[test]
    fn a_title_scores_the_better_of_its_characters_and_its_words() {
        // Two of three different words are shared, and the characters alone score less.
        let (a, b) = ("sales head regional", "regional sales");
        assert!(jaro_winkler(a, b) < 0.5);
        assert!((title_similarity(a, b) - 2.0 / 3.0).abs() < 1e-12);
        assert!((title_similarity("senior engineer", "engineer senior") - 1.0).abs() < 1e-12);
    }
}
