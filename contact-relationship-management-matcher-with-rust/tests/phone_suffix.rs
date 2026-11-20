//! Measures the evidence supplied by a national number that is only the suffix of an
//! international number (CRM-T273).
//!
//! The fixture is deliberately balanced between same-line and different-line pairs. It proves
//! that an unconfigured suffix cannot distinguish the two cases; it does not estimate their
//! prevalence in production data. Run with `-- --nocapture` to print the per-pair report.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers fail the test on bad fixtures"
)]

use contact_relationship_management_matcher::{
    Contact, Field, MatchConfig, MatchResult, Matcher, Outcome, Verdict,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Pair {
    label: String,
    same: bool,
    national_calling_code: String,
    international_calling_code: String,
    national: String,
    international: String,
}

fn pairs() -> Vec<Pair> {
    serde_json::from_str(include_str!("fixtures/phone_suffix_pairs.json"))
        .expect("phone suffix fixture parses")
}

fn contacts(pair: &Pair, shared_name: bool) -> (Contact, Contact) {
    let mut national = Contact::new().phone(&pair.national);
    let mut international = Contact::new().phone(&pair.international);
    if shared_name {
        national = national.given_name("Alex").family_name("Kim");
        international = international.given_name("Alex").family_name("Kim");
    }
    (national, international)
}

fn phone(result: &MatchResult) -> &contact_relationship_management_matcher::FieldScore {
    result
        .fields
        .iter()
        .find(|score| score.field == Field::Phone)
        .expect("phone score is present")
}

/// The verdict the old full-agreement treatment would have produced, holding every other field
/// fixed. This is a counterfactual for the measured verdict effect, not a second matcher path.
fn verdict_with_full_phone(result: &MatchResult, config: &MatchConfig) -> Verdict {
    let full_weight = (config.phone.m / config.phone.u).ln();
    let log_odds = result.log_odds - phone(result).weight + full_weight;
    let probability = 1.0 / (1.0 + (-log_odds).exp());
    if probability >= config.match_threshold {
        Verdict::Match
    } else if probability >= config.review_threshold {
        Verdict::Review
    } else {
        Verdict::NonMatch
    }
}

#[test]
fn unconfigured_suffixes_are_partial_evidence_for_both_truth_labels() {
    let pairs = pairs();
    assert_eq!(pairs.iter().filter(|pair| pair.same).count(), 6);
    assert_eq!(pairs.iter().filter(|pair| !pair.same).count(), 6);

    let matcher = Matcher::default();
    let config = matcher.config();
    let expected_similarity = f64::midpoint(config.agree_at, config.disagree_at);
    let mut phone_only = [0_usize; 2];
    let mut phone_only_full = [0_usize; 2];
    let mut shared_name = [0_usize; 2];
    let mut shared_name_full = [0_usize; 2];

    println!("label truth plans phone-only partial/full shared-name partial/full");
    for pair in &pairs {
        assert_eq!(
            pair.same,
            pair.national_calling_code == pair.international_calling_code,
            "fixture label and plans disagree for {}",
            pair.label
        );
        let (national, international) = contacts(pair, false);
        let result = matcher.compare(&national, &international);
        let reverse = matcher.compare(&international, &national);
        assert_eq!(
            result, reverse,
            "comparison is not symmetric for {}",
            pair.label
        );
        assert_eq!(phone(&result).outcome, Outcome::Partial, "{}", pair.label);
        assert_eq!(
            phone(&result).similarity,
            expected_similarity,
            "{}",
            pair.label
        );
        assert_eq!(result.verdict, Verdict::NonMatch, "{}", pair.label);
        let full = verdict_with_full_phone(&result, config);
        assert_eq!(full, Verdict::Review, "{}", pair.label);

        let (national_named, international_named) = contacts(pair, true);
        let named = matcher.compare(&national_named, &international_named);
        assert_eq!(phone(&named).outcome, Outcome::Partial, "{}", pair.label);
        assert_eq!(named.verdict, Verdict::Review, "{}", pair.label);
        let named_full = verdict_with_full_phone(&named, config);
        assert_eq!(named_full, Verdict::Match, "{}", pair.label);

        let truth = usize::from(pair.same);
        phone_only[truth] += usize::from(result.verdict == Verdict::NonMatch);
        phone_only_full[truth] += usize::from(full == Verdict::Review);
        shared_name[truth] += usize::from(named.verdict == Verdict::Review);
        shared_name_full[truth] += usize::from(named_full == Verdict::Match);
        println!(
            "{} {} {}/{} {:?}/{full:?} {:?}/{named_full:?}",
            pair.label,
            if pair.same { "same" } else { "different" },
            pair.national_calling_code,
            pair.international_calling_code,
            result.verdict,
            named.verdict
        );
    }

    // Index 0 is different-line, index 1 same-line. Identical counts are the measured ambiguity:
    // suffix evidence alone cannot tell the labels apart.
    assert_eq!(phone_only, [6, 6]);
    assert_eq!(phone_only_full, [6, 6]);
    assert_eq!(shared_name, [6, 6]);
    assert_eq!(shared_name_full, [6, 6]);
}
