//! Web form capture (CRM-R31): what counts as an address and a name, and the submission counter.
//!
//! Pure and DB-free. The clock is passed in, so tests need not sleep.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The longest address accepted (RFC 5321).
pub const MAX_EMAIL_LEN: usize = 254;
/// The longest name accepted.
pub const MAX_NAME_LEN: usize = 200;
/// Entries kept before expired ones are dropped.
const PRUNE_AT: usize = 10_000;

/// Whether `email` is plausibly one mailbox: one `@`, a local part and a dotted domain, no
/// spaces, control characters or characters that would make it a list or a header. The
/// confirmation mail is the real test.
#[must_use]
pub fn valid_email(email: &str) -> bool {
    if email.is_empty() || email.len() > MAX_EMAIL_LEN {
        return false;
    }
    if email
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || "<>\"(),;:\\[]".contains(c))
    {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && !domain.contains('@')
        && domain.contains('.')
        && domain
            .split('.')
            .all(|label| !label.is_empty() && !label.starts_with('-') && !label.ends_with('-'))
}

/// The part of `email` after the `@`, lower-cased.
#[must_use]
pub fn domain_of(email: &str) -> Option<String> {
    email
        .rsplit_once('@')
        .map(|(_, domain)| domain.to_ascii_lowercase())
}

/// Whether `name` can be shown and put in a message: not blank, not too long, no control
/// characters (a newline in a name must not reach a mail header).
#[must_use]
pub fn valid_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name.chars().count() <= MAX_NAME_LEN && !name.chars().any(char::is_control)
}

/// Fixed-window counts per key.
#[derive(Debug, Default)]
pub struct Counter {
    windows: HashMap<String, (Instant, u32)>,
}

impl Counter {
    /// Count one event for `key`. `Ok` while the key has had no more than `limit` events in the
    /// last `span`; otherwise `Err` with the seconds until its window ends. A `limit` of `0`
    /// turns the check off.
    ///
    /// # Errors
    ///
    /// The seconds to wait (at least 1) when the key is over its limit.
    pub fn hit(&mut self, key: &str, limit: u32, span: Duration, now: Instant) -> Result<(), u64> {
        if limit == 0 {
            return Ok(());
        }
        if self.windows.len() >= PRUNE_AT {
            self.windows
                .retain(|_, (start, _)| now.saturating_duration_since(*start) < span);
        }
        let entry = self.windows.entry(key.to_string()).or_insert((now, 0));
        if now.saturating_duration_since(entry.0) >= span {
            *entry = (now, 0);
        }
        if entry.1 >= limit {
            let left = span.saturating_sub(now.saturating_duration_since(entry.0));
            return Err(left.as_secs().max(1));
        }
        entry.1 += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_one_mailbox_with_a_dotted_domain() {
        for good in ["a@b.co", "first.last+tag@sub.example.org", "ünï@exämple.de"] {
            assert!(valid_email(good), "{good}");
        }
        for bad in [
            "",
            "plain",
            "a@b",
            "@b.co",
            "a@@b.co",
            "a@b@c.co",
            "a b@c.co",
            "a@b.co\nBcc: x@y.z",
            "a,b@c.co",
            "<a@b.co>",
            "a@-b.co",
            "a@b..co",
        ] {
            assert!(!valid_email(bad), "{bad:?}");
        }
        assert!(!valid_email(&format!("{}@b.co", "a".repeat(65))));
        assert!(!valid_email(&format!("a@{}.co", "b".repeat(260))));
    }

    #[test]
    fn a_name_cannot_carry_a_line_break() {
        assert!(valid_name("Ada Lovelace"));
        assert!(!valid_name("  "));
        assert!(!valid_name("Ada\nBcc: x@y.z"));
        assert!(!valid_name(&"x".repeat(MAX_NAME_LEN + 1)));
    }

    #[test]
    fn the_domain_is_lower_cased() {
        assert_eq!(domain_of("Ada@Example.ORG").as_deref(), Some("example.org"));
        assert_eq!(domain_of("nope"), None);
    }

    #[test]
    fn the_counter_limits_per_key_and_resets_after_the_window() {
        let mut counter = Counter::default();
        let start = Instant::now();
        let span = Duration::from_mins(10);
        for _ in 0..3 {
            assert!(counter.hit("a", 3, span, start).is_ok());
        }
        let wait = counter
            .hit("a", 3, span, start + Duration::from_mins(4))
            .unwrap_err();
        assert!((350..=360).contains(&wait), "{wait}");
        assert!(counter.hit("b", 3, span, start).is_ok(), "another key");
        assert!(
            counter
                .hit("a", 3, span, start + Duration::from_mins(10))
                .is_ok(),
            "a new window"
        );
        assert!(counter.hit("a", 0, span, start).is_ok(), "zero is off");
    }
}
