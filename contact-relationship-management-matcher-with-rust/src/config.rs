//! Tuning parameters, and the check that they make sense.

use std::fmt;

use serde::{Deserialize, Serialize};

/// How a field behaves, in Fellegi–Sunter terms.
///
/// - `m`: the chance the field agrees when the two contacts *are* the same
///   person (less than 1, because of typos, moves and job changes).
/// - `u`: the chance it agrees by coincidence when they are *not*.
///
/// An agreement is worth `ln(m / u)`; a disagreement `ln((1 - m) / (1 - u))`.
/// The rarer a coincidence, the more an agreement is worth.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldParameters {
    /// P(agree | same person).
    pub m: f64,
    /// P(agree | different people).
    pub u: f64,
}

/// All the tuning in one place. Load it from JSON with serde; every value
/// has a default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    /// Before looking at any field, the chance that two candidate contacts
    /// are the same person. It is small when you compare everything with
    /// everything, and larger when you only compare contacts that already
    /// share a block (the same surname, say).
    pub prior: f64,
    /// A probability at or above this is a [`crate::Verdict::Match`].
    pub match_threshold: f64,
    /// A probability at or above this, and below `match_threshold`, is a
    /// [`crate::Verdict::Review`]; below it, a non-match.
    pub review_threshold: f64,
    /// A similarity at or above this counts as full agreement.
    pub agree_at: f64,
    /// A similarity at or below this counts as full disagreement. Between
    /// the two, the weight is interpolated.
    pub disagree_at: f64,
    /// Given name.
    pub given_name: FieldParameters,
    /// Family name.
    pub family_name: FieldParameters,
    /// Email: agreement means a shared address.
    pub email: FieldParameters,
    /// Phone: agreement means a shared number.
    pub phone: FieldParameters,
    /// Organisation.
    pub organization: FieldParameters,
    /// Job title.
    pub job_title: FieldParameters,
    /// Treat `a.b+news@gmail.com` as `ab@gmail.com` (Gmail ignores dots and
    /// `+tags` in the local part).
    pub fold_gmail: bool,
    /// Calling code (`"44"`, `"1"`) for phone numbers written nationally.
    /// Without it, a national number only matches an international one that
    /// ends with the same digits.
    pub default_calling_code: Option<String>,
    /// Extra nickname classes, on top of the built-in English ones. Each
    /// class is a list of names that stand for the same person.
    pub nicknames: Vec<Vec<String>>,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            prior: 0.001,
            match_threshold: 0.9,
            review_threshold: 0.5,
            agree_at: 0.92,
            disagree_at: 0.7,
            given_name: FieldParameters { m: 0.95, u: 0.03 },
            family_name: FieldParameters { m: 0.95, u: 0.01 },
            email: FieldParameters { m: 0.7, u: 0.0005 },
            phone: FieldParameters { m: 0.7, u: 0.0005 },
            organization: FieldParameters { m: 0.85, u: 0.02 },
            job_title: FieldParameters { m: 0.5, u: 0.05 },
            fold_gmail: true,
            default_calling_code: None,
            nicknames: Vec::new(),
        }
    }
}

/// Why a [`MatchConfig`] was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid match configuration: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

impl MatchConfig {
    /// Check the values. A bad configuration would give meaningless
    /// probabilities, so [`crate::Matcher::new`] calls this and refuses it.
    ///
    /// # Errors
    ///
    /// Returns the first problem found: a probability outside `(0, 1)`, an
    /// `m` that is not above its `u`, thresholds out of order, or a
    /// calling code that is not digits.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let open_unit = |name: &str, v: f64| {
            if v.is_finite() && v > 0.0 && v < 1.0 {
                Ok(())
            } else {
                Err(ConfigError(format!(
                    "{name} must be between 0 and 1, exclusive"
                )))
            }
        };
        open_unit("prior", self.prior)?;
        open_unit("match_threshold", self.match_threshold)?;
        open_unit("review_threshold", self.review_threshold)?;
        open_unit("agree_at", self.agree_at)?;
        open_unit("disagree_at", self.disagree_at)?;
        if self.review_threshold > self.match_threshold {
            return Err(ConfigError(
                "review_threshold must not exceed match_threshold".into(),
            ));
        }
        if self.disagree_at >= self.agree_at {
            return Err(ConfigError("disagree_at must be below agree_at".into()));
        }
        for (name, p) in [
            ("given_name", self.given_name),
            ("family_name", self.family_name),
            ("email", self.email),
            ("phone", self.phone),
            ("organization", self.organization),
            ("job_title", self.job_title),
        ] {
            open_unit(&format!("{name}.m"), p.m)?;
            open_unit(&format!("{name}.u"), p.u)?;
            if p.m <= p.u {
                return Err(ConfigError(format!(
                    "{name}.m must exceed {name}.u, or agreement would count against a match"
                )));
            }
        }
        if let Some(code) = &self.default_calling_code
            && (code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(ConfigError(
                "default_calling_code must be digits, such as \"44\"".into(),
            ));
        }
        Ok(())
    }
}
