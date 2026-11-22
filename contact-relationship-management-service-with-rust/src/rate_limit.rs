//! Abuse resistance (CRM-T43): a fixed-window request limit and a failed-authentication
//! lockout, counted by `axa::ratelimit::Limiter` in each process (CRM-T219), or in Redis when
//! `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` is set ([`crate::redis_store`]).
//!
//! What stays here is the service's own: the settings, and [`client_key`], which counts a
//! signed-in caller by its verified subject rather than by address (CRM-T217). axa's
//! `by_client` and `lock_out_failures` count by address only, so the middleware calls
//! [`Limiter::check`] and [`Limiter::peek`] with this key instead.

use std::net::{IpAddr, Ipv6Addr};
use std::num::NonZeroU32;

use axa::ratelimit::Limiter;

/// The two limits the middleware uses, and the numbers the shared counters need.
#[derive(Debug, Clone)]
pub struct Guard {
    /// Requests per client per minute; `None` when switched off.
    pub requests: Option<Limiter>,
    /// Failed authentications per client per window; at the limit the client is locked out
    /// until the window ends. `None` when switched off.
    pub failures: Option<Limiter>,
    requests_per_minute: u32,
    failure_limit: u32,
    failure_window_secs: u64,
}

impl Guard {
    /// Build from explicit limits (0 switches a limit off).
    #[must_use]
    pub fn new(requests_per_minute: u32, failure_limit: u32, failure_window_secs: u64) -> Self {
        let failure_window_secs = failure_window_secs.max(1);
        let window = NonZeroU32::new(u32::try_from(failure_window_secs).unwrap_or(u32::MAX))
            .unwrap_or(NonZeroU32::MIN);
        Self {
            requests: NonZeroU32::new(requests_per_minute)
                .map(|limit| Limiter::new(limit, NonZeroU32::new(60).unwrap_or(NonZeroU32::MIN))),
            failures: NonZeroU32::new(failure_limit).map(|limit| Limiter::new(limit, window)),
            requests_per_minute,
            failure_limit,
            failure_window_secs,
        }
    }

    /// Requests allowed per minute, `0` when off.
    #[must_use]
    pub fn requests_per_minute(&self) -> u32 {
        self.requests_per_minute
    }

    /// Failed authentications allowed per window, `0` when off.
    #[must_use]
    pub fn failure_limit(&self) -> u32 {
        self.failure_limit
    }

    /// Seconds in the failed-authentication window.
    #[must_use]
    pub fn failure_window_secs(&self) -> u64 {
        self.failure_window_secs
    }

    /// Defaults: on in production (600 requests a minute, lockout after 10
    /// failures in 5 minutes), off elsewhere so local runs and test suites
    /// are not throttled. `CONTACT_RELATIONSHIP_MANAGEMENT_RATE_LIMIT_PER_MIN`, `CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_FAIL_LIMIT`,
    /// and `CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_FAIL_WINDOW_SECS` override; 0 disables.
    ///
    /// # Errors
    ///
    /// A setting that is set but blank, not a whole number or too large; it names the setting and
    /// never its value. The defaults apply only to a setting that is not set at all (RCA-IN-3).
    pub fn from_env(production: bool) -> Result<Self, String> {
        Self::from_lookup(production, &|name| std::env::var(name).ok())
    }

    /// [`Guard::from_env`] over any `name -> value` lookup.
    ///
    /// # Errors
    ///
    /// As [`Guard::from_env`].
    pub fn from_lookup(
        production: bool,
        get: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let number = |name: &str, default: u64| -> Result<u64, String> {
            get(name).map_or(Ok(default), |value| {
                value
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| format!("{name} must be a whole number"))
            })
        };
        let count = |name: &str, default: u64| -> Result<u32, String> {
            u32::try_from(number(name, default)?)
                .map_err(|_| format!("{name} must be at most {}", u32::MAX))
        };
        Ok(Self::new(
            count(
                "CONTACT_RELATIONSHIP_MANAGEMENT_RATE_LIMIT_PER_MIN",
                if production { 600 } else { 0 },
            )?,
            count(
                "CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_FAIL_LIMIT",
                if production { 10 } else { 0 },
            )?,
            number("CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_FAIL_WINDOW_SECS", 300)?,
        ))
    }
}

/// The setting for axa's lockout by client address (CRM-T224).
pub const ADDRESS_LOCKOUT_VAR: &str = "LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT";

/// axa's `lock_out_failures`, from [`ADDRESS_LOCKOUT_VAR`]: after `count` `401`s from one client
/// address within `window_secs`, every request from that address is refused with `429` until the
/// window ends, signed in or not. It counts by address alone and in this process alone, unlike
/// [`Guard::failures`], which counts a verified caller by subject and can share counts through
/// Redis. Unset, empty or `0` is off.
///
/// # Errors
///
/// When the value is not a whole number, so a mistyped setting stops the start rather than
/// leaving the lockout off unseen.
pub fn address_lockout(value: Option<&str>, window_secs: u64) -> Result<Option<Limiter>, String> {
    let value = value.map_or_default(str::trim);
    if value.is_empty() {
        return Ok(None);
    }
    let count: u32 = value.parse().map_err(|_| {
        format!("{ADDRESS_LOCKOUT_VAR} must be a whole number of failures, or 0 for off")
    })?;
    let window = NonZeroU32::new(u32::try_from(window_secs.max(1)).unwrap_or(u32::MAX))
        .unwrap_or(NonZeroU32::MIN);
    Ok(NonZeroU32::new(count).map(|count| Limiter::new(count, window)))
}

/// A limit whose number is a setting that can change while the service runs (the web forms'
/// and the public knowledge base's, CRM-R31). An axa [`Limiter`] has a fixed number, so a new
/// one is made when the setting changes, and the counts start again then.
#[derive(Debug)]
pub struct Settable {
    window_seconds: NonZeroU32,
    current: std::sync::Mutex<Option<(u32, Limiter)>>,
}

impl Settable {
    /// A limit counted over `window_seconds`.
    #[must_use]
    pub const fn new(window_seconds: NonZeroU32) -> Self {
        Self {
            window_seconds,
            current: std::sync::Mutex::new(None),
        }
    }

    /// Count one request by `key` against `limit`, where `0` is off.
    ///
    /// # Errors
    ///
    /// The seconds to wait, at least 1, when `key` is already at `limit`. A refused request
    /// is not counted.
    pub fn check(&self, limit: u32, key: impl std::hash::Hash) -> Result<(), u64> {
        let Some(number) = NonZeroU32::new(limit) else {
            return Ok(());
        };
        let limiter = {
            let mut current = self
                .current
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match &*current {
                Some((held, limiter)) if *held == limit => limiter.clone(),
                _ => {
                    let limiter = Limiter::new(number, self.window_seconds);
                    *current = Some((limit, limiter.clone()));
                    limiter
                }
            }
        };
        let verdict = limiter.check(key);
        if verdict.allowed {
            Ok(())
        } else {
            Err(verdict.reset_seconds.max(1))
        }
    }
}

/// The key a request is counted under. `subject` is the `sub` of a token that verified, so a
/// signed-in caller has one budget wherever it calls from, and however many tokens it holds.
/// Anything else counts by `address`, the one `axa` resolved from the connection and
/// `TRUSTED_PROXIES`: the caller writes neither a bearer value nor the left end of
/// `X-Forwarded-For`, so neither may name a bucket, or each new value would buy a fresh limit.
/// With neither, every such caller shares one bucket, the safe direction for a lockout.
///
/// A subject is hashed so the counters hold no one's identifier. IPv6 counts by its `/64`, the
/// block one customer is routinely given, as one address there would be free to replace.
#[must_use]
pub fn client_key(subject: Option<&str>, address: Option<IpAddr>) -> String {
    use sha2::{Digest, Sha256};
    if let Some(subject) = subject {
        let digest: [u8; 32] = Sha256::digest(subject.as_bytes()).into();
        let [b0, b1, b2, b3, b4, b5, b6, b7, ..] = digest;
        return format!(
            "sub:{:016x}",
            u64::from_be_bytes([b0, b1, b2, b3, b4, b5, b6, b7])
        );
    }
    match address.map(|address| address.to_canonical()) {
        Some(IpAddr::V4(address)) => format!("ip:{address}"),
        Some(IpAddr::V6(address)) => {
            let [a, b, c, d, ..] = address.segments();
            format!("ip:{}/64", Ipv6Addr::new(a, b, c, d, 0, 0, 0, 0))
        }
        None => "anon".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn client_key_is_the_verified_subject_wherever_it_calls_from() {
        let key = client_key(Some("person-1"), Some(address("203.0.113.9")));
        assert!(key.starts_with("sub:"), "{key}");
        assert!(!key.contains("person-1"));
        assert_eq!(
            key,
            client_key(Some("person-1"), Some(address("198.51.100.1")))
        );
        assert_ne!(
            key,
            client_key(Some("person-2"), Some(address("203.0.113.9")))
        );
    }

    #[test]
    fn client_key_without_a_subject_is_the_address() {
        assert_eq!(
            client_key(None, Some(address("203.0.113.9"))),
            "ip:203.0.113.9"
        );
        assert_eq!(
            client_key(None, Some(address("::ffff:203.0.113.9"))),
            "ip:203.0.113.9"
        );
    }

    #[test]
    fn client_key_counts_ipv6_by_its_block() {
        let one = client_key(None, Some(address("2001:db8:1:2::1")));
        assert_eq!(one, "ip:2001:db8:1:2::/64");
        assert_eq!(one, client_key(None, Some(address("2001:db8:1:2:aaaa::9"))));
        assert_ne!(one, client_key(None, Some(address("2001:db8:1:3::1"))));
    }

    #[test]
    fn client_key_with_neither_is_one_shared_bucket() {
        assert_eq!(client_key(None, None), "anon");
    }

    #[test]
    fn a_settable_limit_counts_per_key_and_starts_again_when_the_setting_changes() {
        let limit = Settable::new(NonZeroU32::new(600).unwrap());
        for _ in 0..3 {
            assert!(limit.check(3, "a").is_ok());
        }
        let wait = limit.check(3, "a").unwrap_err();
        assert!((599..=600).contains(&wait), "{wait}");
        assert!(limit.check(3, "b").is_ok(), "another key");
        assert!(
            limit.check(4, "a").is_ok(),
            "a new setting starts the count again"
        );
        assert!(limit.check(0, "a").is_ok(), "zero is off");
    }

    #[test]
    fn the_address_lockout_is_off_unless_given_a_count() {
        for off in [None, Some(""), Some("  "), Some("0")] {
            assert!(address_lockout(off, 300).unwrap().is_none(), "{off:?}");
        }
        assert!(address_lockout(Some(" 5 "), 300).unwrap().is_some());
        for wrong in ["five", "-1", "2.5", "99999999999"] {
            let why = address_lockout(Some(wrong), 300).unwrap_err();
            assert!(why.contains(ADDRESS_LOCKOUT_VAR), "{why}");
            assert!(!why.contains(wrong), "the value is not quoted: {why}");
        }
    }

    #[test]
    fn the_address_lockout_refuses_after_the_count() {
        let limiter = address_lockout(Some("2"), 300).unwrap().unwrap();
        assert!(limiter.check("ip:203.0.113.9").allowed);
        assert!(limiter.check("ip:203.0.113.9").allowed);
        assert!(!limiter.check("ip:203.0.113.9").allowed);
    }

    #[test]
    fn production_defaults_on_elsewhere_off() {
        assert!(Guard::from_env(true).unwrap().requests.is_some());
        assert!(Guard::from_env(false).unwrap().requests.is_none());
    }

    /// A rate-limit number that is set but not a whole number, or too large, stops start-up and
    /// names the setting; it is not the default and not the largest value.
    #[test]
    fn a_rate_limit_setting_that_is_set_and_invalid_is_refused() {
        let with = |name: &'static str, value: &'static str| {
            Guard::from_lookup(true, &move |n| (n == name).then(|| value.to_string()))
        };
        let per_min = "CONTACT_RELATIONSHIP_MANAGEMENT_RATE_LIMIT_PER_MIN";
        let limit = "CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_FAIL_LIMIT";
        let window = "CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_FAIL_WINDOW_SECS";
        assert!(with(per_min, " 25 ").unwrap().requests.is_some());
        assert!(with(per_min, "0").unwrap().requests.is_none());
        assert!(with(window, "60").is_ok());
        for (name, bad) in [
            (per_min, "lots"),
            (per_min, ""),
            (per_min, "-1"),
            (per_min, "4294967296"),
            (limit, "4294967296"),
            (limit, "x"),
            (window, "soon"),
        ] {
            let refused = with(name, bad).unwrap_err();
            assert!(refused.contains(name), "{refused}");
            assert!(!refused.contains(&format!("= {bad}")), "{refused}");
        }
    }
}
