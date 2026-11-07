//! Abuse resistance (CRM-T43): a fixed-window request limiter and a
//! failed-authentication lockout, both in memory and per process.
//!
//! The counters are DB-free and take the clock as an argument so the rules
//! are unit-pinned. Limits: state is per process (run N replicas and each
//! counts alone, so enforce a global limit at the ingress as well), and the
//! client key is only as trustworthy as its source (see [`client_key`]).

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The verdict for one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Proceed.
    Allowed,
    /// Refuse with `429`; retry after this many seconds.
    Limited {
        /// Seconds until the window resets.
        retry_after_secs: u64,
    },
}

/// A fixed-window counter keyed by client.
#[derive(Debug)]
pub struct Window {
    limit: u32,
    span: Duration,
    state: Mutex<HashMap<String, (Instant, u32)>>,
}

/// Entries kept before stale ones are pruned.
const PRUNE_AT: usize = 10_000;

impl Window {
    /// A counter allowing `limit` events per `span`; a `limit` of 0 disables it.
    #[must_use]
    pub fn new(limit: u32, span: Duration) -> Self {
        Self {
            limit,
            span,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// The events allowed per window.
    #[must_use]
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Whether this counter is switched off.
    #[must_use]
    pub fn disabled(&self) -> bool {
        self.limit == 0
    }

    fn remaining(&self, start: Instant, now: Instant) -> u64 {
        self.span
            .saturating_sub(now.saturating_duration_since(start))
            .as_secs()
            .max(1)
    }

    /// Count one event for `key` and decide.
    #[must_use]
    pub fn hit(&self, key: &str, now: Instant) -> Decision {
        if self.disabled() {
            return Decision::Allowed;
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.len() >= PRUNE_AT {
            state.retain(|_, (start, _)| now.saturating_duration_since(*start) < self.span);
        }
        let entry = state.entry(key.to_string()).or_insert((now, 0));
        if now.saturating_duration_since(entry.0) >= self.span {
            *entry = (now, 0);
        }
        entry.1 = entry.1.saturating_add(1);
        if entry.1 > self.limit {
            Decision::Limited {
                retry_after_secs: self.remaining(entry.0, now),
            }
        } else {
            Decision::Allowed
        }
    }

    /// What is left for `key` in the current window: `(limit, remaining,
    /// seconds until reset)`, or `None` when the counter is off. Counts nothing.
    #[must_use]
    pub fn budget(&self, key: &str, now: Instant) -> Option<(u32, u32, u64)> {
        if self.disabled() {
            return None;
        }
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Some(match state.get(key) {
            Some((start, count)) if now.saturating_duration_since(*start) < self.span => (
                self.limit,
                self.limit.saturating_sub(*count),
                self.remaining(*start, now),
            ),
            _ => (self.limit, self.limit, self.span.as_secs()),
        })
    }

    /// Whether `key` is already over the limit, without counting an event.
    #[must_use]
    pub fn blocked(&self, key: &str, now: Instant) -> Decision {
        if self.disabled() {
            return Decision::Allowed;
        }
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match state.get(key) {
            Some((start, count))
                if *count >= self.limit && now.saturating_duration_since(*start) < self.span =>
            {
                Decision::Limited {
                    retry_after_secs: self.remaining(*start, now),
                }
            }
            _ => Decision::Allowed,
        }
    }
}

/// The two counters the middleware uses.
#[derive(Debug)]
pub struct Guard {
    /// Requests per client per window.
    pub requests: Window,
    /// Failed authentications per client per window; at the limit the client
    /// is locked out until the window ends.
    pub failures: Window,
}

impl Guard {
    /// Build from explicit limits (0 disables a counter).
    #[must_use]
    pub fn new(requests_per_minute: u32, failure_limit: u32, failure_window_secs: u64) -> Self {
        Self {
            requests: Window::new(requests_per_minute, Duration::from_mins(1)),
            failures: Window::new(
                failure_limit,
                Duration::from_secs(failure_window_secs.max(1)),
            ),
        }
    }

    /// Seconds in the failed-authentication window.
    #[must_use]
    pub fn failure_window_secs(&self) -> u64 {
        self.failures.span.as_secs()
    }

    /// Defaults: on in production (600 requests a minute, lockout after 10
    /// failures in 5 minutes), off elsewhere so local runs and test suites
    /// are not throttled. `CRM_RATE_LIMIT_PER_MIN`, `CRM_AUTH_FAIL_LIMIT`,
    /// and `CRM_AUTH_FAIL_WINDOW_SECS` override; 0 disables.
    #[must_use]
    pub fn from_env(production: bool) -> Self {
        let number = |name: &str, default: u64| {
            std::env::var(name)
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(default)
        };
        let on = u64::from(production);
        Self::new(
            u32::try_from(number("CRM_RATE_LIMIT_PER_MIN", 600 * on)).unwrap_or(u32::MAX),
            u32::try_from(number("CRM_AUTH_FAIL_LIMIT", 10 * on)).unwrap_or(u32::MAX),
            number("CRM_AUTH_FAIL_WINDOW_SECS", 300),
        )
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
        let digest = Sha256::digest(subject.as_bytes());
        let first: [u8; 8] = digest[..8].try_into().unwrap_or_default();
        return format!("sub:{:016x}", u64::from_be_bytes(first));
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

    fn at(base: Instant, secs: u64) -> Instant {
        base + Duration::from_secs(secs)
    }

    #[test]
    fn allows_up_to_the_limit_then_limits_with_retry_after() {
        let w = Window::new(3, Duration::from_mins(1));
        let t0 = Instant::now();
        for _ in 0..3 {
            assert_eq!(w.hit("a", t0), Decision::Allowed);
        }
        assert_eq!(
            w.hit("a", at(t0, 10)),
            Decision::Limited {
                retry_after_secs: 50
            }
        );
    }

    #[test]
    fn window_resets_after_the_span() {
        let w = Window::new(1, Duration::from_mins(1));
        let t0 = Instant::now();
        assert_eq!(w.hit("a", t0), Decision::Allowed);
        assert!(matches!(w.hit("a", at(t0, 1)), Decision::Limited { .. }));
        assert_eq!(w.hit("a", at(t0, 60)), Decision::Allowed);
    }

    #[test]
    fn clients_are_counted_separately() {
        let w = Window::new(1, Duration::from_mins(1));
        let t0 = Instant::now();
        assert_eq!(w.hit("a", t0), Decision::Allowed);
        assert_eq!(w.hit("b", t0), Decision::Allowed);
    }

    #[test]
    fn budget_counts_down_and_resets() {
        let base = Instant::now();
        let w = Window::new(3, Duration::from_secs(60));
        assert_eq!(w.budget("k", base), Some((3, 3, 60)));
        let _ = w.hit("k", base);
        let _ = w.hit("k", at(base, 10));
        assert_eq!(w.budget("k", at(base, 20)), Some((3, 1, 40)));
        assert_eq!(w.budget("k", at(base, 61)), Some((3, 3, 60)));
        assert_eq!(
            Window::new(0, Duration::from_secs(60)).budget("k", base),
            None
        );
    }

    #[test]
    fn zero_limit_disables() {
        let w = Window::new(0, Duration::from_mins(1));
        let t0 = Instant::now();
        for _ in 0..100 {
            assert_eq!(w.hit("a", t0), Decision::Allowed);
        }
        assert_eq!(w.blocked("a", t0), Decision::Allowed);
    }

    #[test]
    fn lockout_blocks_at_the_limit_without_counting() {
        let w = Window::new(2, Duration::from_mins(5));
        let t0 = Instant::now();
        assert_eq!(w.blocked("a", t0), Decision::Allowed);
        let _ = w.hit("a", t0);
        assert_eq!(w.blocked("a", t0), Decision::Allowed);
        let _ = w.hit("a", t0);
        assert_eq!(
            w.blocked("a", t0),
            Decision::Limited {
                retry_after_secs: 300
            }
        );
        // `blocked` does not count: still locked, then released after the window.
        assert!(matches!(
            w.blocked("a", at(t0, 100)),
            Decision::Limited { .. }
        ));
        assert_eq!(w.blocked("a", at(t0, 300)), Decision::Allowed);
    }

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
    fn production_defaults_on_elsewhere_off() {
        assert!(!Guard::from_env(true).requests.disabled());
        assert!(Guard::from_env(false).requests.disabled());
    }
}
