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
//! live set is cached per process and refreshed from the database every few
//! seconds, so a grant reaches other replicas within one refresh interval.

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

/// Validate a grant request; returns the trimmed reason and the minutes.
///
/// # Errors
///
/// A message naming what is wrong.
pub fn validate_request(reason: &str, minutes: Option<i64>) -> Result<(String, i64), String> {
    let reason = reason.trim();
    let length = reason.chars().count();
    if length < MIN_REASON_CHARS {
        return Err(format!(
            "reason must be at least {MIN_REASON_CHARS} characters; say why the access is needed"
        ));
    }
    if length > MAX_REASON_CHARS {
        return Err(format!(
            "reason must be at most {MAX_REASON_CHARS} characters"
        ));
    }
    let minutes = minutes.unwrap_or(DEFAULT_MINUTES);
    if !(1..=MAX_MINUTES).contains(&minutes) {
        return Err(format!("minutes must be between 1 and {MAX_MINUTES}"));
    }
    Ok((reason.to_string(), minutes))
}

/// Validate a review: a real note, and not the grantee reviewing themself.
///
/// # Errors
///
/// A message naming what is wrong.
pub fn validate_review(grantee: &str, reviewer: &str, note: &str) -> Result<String, String> {
    if grantee == reviewer {
        return Err("a break-glass grant must be reviewed by someone else".to_string());
    }
    let note = note.trim();
    if note.chars().count() < MIN_NOTE_CHARS {
        return Err(format!(
            "review note must be at least {MIN_NOTE_CHARS} characters"
        ));
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
        assert!(validate_request("too short", None).is_err());
        assert!(validate_request(&"x".repeat(MAX_REASON_CHARS + 1), None).is_err());
        let (reason, minutes) =
            validate_request("  Patient collapsed, covering colleague absent  ", None).unwrap();
        assert_eq!(reason, "Patient collapsed, covering colleague absent");
        assert_eq!(minutes, DEFAULT_MINUTES);
    }

    #[test]
    fn duration_is_capped() {
        let reason = "Urgent case needing the unmasked record";
        assert!(validate_request(reason, Some(0)).is_err());
        assert!(validate_request(reason, Some(MAX_MINUTES + 1)).is_err());
        assert_eq!(
            validate_request(reason, Some(MAX_MINUTES)).unwrap().1,
            MAX_MINUTES
        );
    }

    #[test]
    fn reviewer_must_differ_and_leave_a_note() {
        assert!(validate_review("a", "a", "looked fine to me").is_err());
        assert!(validate_review("a", "b", "ok").is_err());
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
