//! When an open deal is stalled (CRM-R4, CRM-D20): at least N whole days of elapsed time since its
//! last touch, the later of its last stage move and its last activity that has happened. A closed
//! deal never is. N is the organization's `stalled_after_days`.

use chrono::{DateTime, Duration, Utc};

/// N until an administrator sets one.
pub const DEFAULT_DAYS: u32 = 14;
/// The largest N.
pub const MAX_DAYS: u32 = 365;

/// A stored or offered N, checked: a whole number of days from 1 to [`MAX_DAYS`].
///
/// # Errors
///
/// The reason, for a value outside that range.
pub fn days(raw: i64) -> Result<u32, &'static str> {
    u32::try_from(raw)
        .ok()
        .filter(|d| (1..=MAX_DAYS).contains(d))
        .ok_or("is a whole number of days from 1 to 365")
}

/// The deal's last touch: its last stage move, or its last activity if that is later and has
/// happened by `now` (a planned activity is not a touch yet).
#[must_use]
pub fn last_touch(
    stage_moved_at: DateTime<Utc>,
    last_activity: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    match last_activity {
        Some(at) if at <= now && at > stage_moved_at => at,
        _ => stage_moved_at,
    }
}

/// Whether a deal last touched at `touched` is stalled at `now` after `days` days.
#[must_use]
pub fn is_stalled(closed: bool, touched: DateTime<Utc>, now: DateTime<Utc>, days: u32) -> bool {
    !closed && now - touched >= Duration::days(i64::from(days))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().to_utc()
    }

    #[test]
    fn n_is_a_whole_number_of_days_from_one_to_a_year() {
        assert_eq!(days(1), Ok(1));
        assert_eq!(days(14), Ok(14));
        assert_eq!(days(365), Ok(365));
        for bad in [0, -1, 366, i64::MAX, i64::MIN] {
            assert!(days(bad).is_err(), "{bad}");
        }
        assert_eq!(DEFAULT_DAYS, 14);
    }

    #[test]
    fn the_last_touch_is_the_later_of_the_move_and_an_activity_that_has_happened() {
        let now = at("2026-10-31T12:00:00Z");
        let moved = at("2026-10-01T09:00:00Z");
        assert_eq!(last_touch(moved, None, now), moved);
        let call = at("2026-10-20T10:00:00Z");
        assert_eq!(last_touch(moved, Some(call), now), call);
        // An activity before the move does not move the touch back.
        assert_eq!(
            last_touch(moved, Some(at("2026-09-01T00:00:00Z")), now),
            moved
        );
        // A planned call next week is not a touch yet.
        assert_eq!(
            last_touch(moved, Some(at("2026-11-05T10:00:00Z")), now),
            moved
        );
        // One at exactly now is.
        assert_eq!(last_touch(moved, Some(now), now), now);
    }

    #[test]
    fn stalled_after_n_whole_days_and_never_once_closed() {
        let touched = at("2026-10-17T12:00:00Z");
        // 14 days less a second: not yet; 14 days exactly: stalled.
        assert!(!is_stalled(false, touched, at("2026-10-31T11:59:59Z"), 14));
        assert!(is_stalled(false, touched, at("2026-10-31T12:00:00Z"), 14));
        assert!(is_stalled(false, touched, at("2027-01-01T00:00:00Z"), 14));
        // A closed deal is never stalled.
        assert!(!is_stalled(true, touched, at("2027-01-01T00:00:00Z"), 14));
        // N = 1.
        assert!(!is_stalled(false, touched, at("2026-10-18T11:00:00Z"), 1));
        assert!(is_stalled(false, touched, at("2026-10-18T12:00:00Z"), 1));
        // A touch in the future (a clock skew) is not stalled.
        assert!(!is_stalled(
            false,
            at("2026-11-01T00:00:00Z"),
            at("2026-10-31T00:00:00Z"),
            1
        ));
    }
}
