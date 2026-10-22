//! Break-glass access (CRM-T44): a time-limited, reasoned, reviewed elevation
//! for an authenticated caller who needs to read what their role masks (an
//! emergency, an out-of-hours case).
//!
//! Defaults, all stated and changeable in code: a written reason of at least
//! 20 characters; 30 minutes by default, 60 at most; read elevation only
//! (never write or destructive); every grant audited and listed for review;
//! a grant is reviewed by someone other than the person who took it.
//!
//! How it takes effect: while a grant is live the caller's authorization
//! context carries the environment attribute `env.break_glass=true`, which a
//! policy rule can match (the reference policy lifts the mask for reads). The
//! live set is cached per process. The database is the record and is re-read
//! every 10 seconds; with `CRM_REDIS_URL` set, grants are also published to
//! Redis and polled every second, so they reach other replicas in about a
//! second. The live set is the union of both.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Duration, Utc};

/// Shortest acceptable written reason.
pub const MIN_REASON_CHARS: usize = 20;
/// Longest accepted reason.
pub const MAX_REASON_CHARS: usize = 500;
/// Grant length when none is asked for, in minutes.
pub const DEFAULT_MINUTES: i64 = 30;
/// Longest grant, in minutes.
pub const MAX_MINUTES: i64 = 60;
/// Shortest review note.
pub const MIN_NOTE_CHARS: usize = 10;

/// Why a grant request is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GrantError {
    /// The reason is shorter than [`MIN_REASON_CHARS`].
    #[error("reason must be at least {MIN_REASON_CHARS} characters; say why the access is needed")]
    ReasonTooShort,
    /// The reason is longer than [`MAX_REASON_CHARS`].
    #[error("reason must be at most {MAX_REASON_CHARS} characters")]
    ReasonTooLong,
    /// The duration is outside `1..=`[`MAX_MINUTES`].
    #[error("minutes must be between 1 and {MAX_MINUTES}")]
    MinutesOutOfRange,
}

impl GrantError {
    /// The request member the refusal is about.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        match self {
            Self::ReasonTooShort | Self::ReasonTooLong => "reason",
            Self::MinutesOutOfRange => "minutes",
        }
    }
}

/// Why a review is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReviewError {
    /// The reviewer is the grantee: that is a `403`, not a bad request.
    #[error("a break-glass grant must be reviewed by someone else")]
    SelfReview,
    /// The note is shorter than [`MIN_NOTE_CHARS`].
    #[error("review note must be at least {MIN_NOTE_CHARS} characters")]
    NoteTooShort,
}

/// Validate a grant request; returns the trimmed reason and the minutes.
///
/// # Errors
///
/// [`GrantError`] naming what is wrong.
pub fn validate_request(reason: &str, minutes: Option<i64>) -> Result<(String, i64), GrantError> {
    let reason = reason.trim();
    let length = reason.chars().count();
    if length < MIN_REASON_CHARS {
        return Err(GrantError::ReasonTooShort);
    }
    if length > MAX_REASON_CHARS {
        return Err(GrantError::ReasonTooLong);
    }
    let minutes = minutes.unwrap_or(DEFAULT_MINUTES);
    if !(1..=MAX_MINUTES).contains(&minutes) {
        return Err(GrantError::MinutesOutOfRange);
    }
    Ok((reason.to_string(), minutes))
}

/// Validate a review: a real note, and not the grantee reviewing themself.
///
/// # Errors
///
/// [`ReviewError`] naming what is wrong.
pub fn validate_review(grantee: &str, reviewer: &str, note: &str) -> Result<String, ReviewError> {
    if grantee == reviewer {
        return Err(ReviewError::SelfReview);
    }
    let note = note.trim();
    if note.chars().count() < MIN_NOTE_CHARS {
        return Err(ReviewError::NoteTooShort);
    }
    Ok(note.to_string())
}

/// Expiry of a grant taken at `now`.
#[must_use]
pub fn expiry(now: DateTime<Utc>, minutes: i64) -> DateTime<Utc> {
    now + Duration::minutes(minutes)
}

/// The live grants this process knows about: subject to expiry.
#[derive(Debug, Default)]
pub struct Registry {
    live: Mutex<HashMap<String, DateTime<Utc>>>,
}

impl Registry {
    /// Record a grant (keeping the later expiry if one exists).
    pub fn grant(&self, subject: &str, expires_at: DateTime<Utc>) {
        let mut live = self
            .live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = live.entry(subject.to_string()).or_insert(expires_at);
        if expires_at > *entry {
            *entry = expires_at;
        }
    }

    /// Replace the whole set (the database refresh).
    pub fn replace(&self, grants: HashMap<String, DateTime<Utc>>) {
        *self
            .live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = grants;
    }

    /// Whether `subject` holds a live grant at `now`.
    #[must_use]
    pub fn active(&self, subject: &str, now: DateTime<Utc>) -> bool {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(subject)
            .is_some_and(|expires_at| *expires_at > now)
    }
}

/// The process-wide registry.
#[must_use]
pub fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reason_is_required_and_bounded() {
        assert_eq!(
            validate_request("too short", None),
            Err(GrantError::ReasonTooShort)
        );
        assert_eq!(
            validate_request(&"x".repeat(MAX_REASON_CHARS + 1), None),
            Err(GrantError::ReasonTooLong)
        );
        let (reason, minutes) =
            validate_request("  Patient collapsed, covering colleague absent  ", None).unwrap();
        assert_eq!(reason, "Patient collapsed, covering colleague absent");
        assert_eq!(minutes, DEFAULT_MINUTES);
    }

    #[test]
    fn duration_is_capped() {
        let reason = "Urgent case needing the unmasked record";
        assert_eq!(
            validate_request(reason, Some(0)),
            Err(GrantError::MinutesOutOfRange)
        );
        assert_eq!(
            validate_request(reason, Some(MAX_MINUTES + 1)),
            Err(GrantError::MinutesOutOfRange)
        );
        assert_eq!(
            validate_request(reason, Some(MAX_MINUTES)).unwrap().1,
            MAX_MINUTES
        );
    }

    #[test]
    fn reviewer_must_differ_and_leave_a_note() {
        assert_eq!(
            validate_review("a", "a", "looked fine to me"),
            Err(ReviewError::SelfReview)
        );
        assert_eq!(
            validate_review("a", "b", "ok"),
            Err(ReviewError::NoteTooShort)
        );
        // The text clients read is unchanged.
        assert_eq!(
            ReviewError::SelfReview.to_string(),
            "a break-glass grant must be reviewed by someone else"
        );
        assert_eq!(
            GrantError::MinutesOutOfRange.to_string(),
            format!("minutes must be between 1 and {MAX_MINUTES}")
        );
        assert_eq!(
            validate_review("a", "b", " justified by incident 42 ").unwrap(),
            "justified by incident 42"
        );
    }

    #[test]
    fn grants_expire() {
        let registry = Registry::default();
        let now = Utc::now();
        registry.grant("u", expiry(now, 30));
        assert!(registry.active("u", now));
        assert!(registry.active("u", now + Duration::minutes(29)));
        assert!(!registry.active("u", now + Duration::minutes(30)));
        assert!(!registry.active("other", now));
    }

    #[test]
    fn replace_drops_revoked_grants_and_grant_keeps_the_later_expiry() {
        let registry = Registry::default();
        let now = Utc::now();
        registry.grant("u", expiry(now, 10));
        registry.grant("u", expiry(now, 5));
        assert!(registry.active("u", now + Duration::minutes(9)));
        registry.replace(HashMap::new());
        assert!(!registry.active("u", now));
    }
}
