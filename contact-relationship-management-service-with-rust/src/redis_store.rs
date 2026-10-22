//! Shared counters in Redis (the same `CRM_REDIS_URL` Loco's cache uses), so
//! the rate limit and failed-authentication lockout count across replicas
//! rather than per process. Unset URL means no shared state: callers fall
//! back to the in-process counters in [`crate::rate_limit`].
//!
//! Failure policy: a Redis error never blocks a request. The caller logs it
//! and uses the in-process counter, so an outage degrades to per-process
//! limits instead of an outage of the API.

use std::collections::HashMap;
use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;

static GLOBAL: OnceLock<Shared> = OnceLock::new();

/// Install the process-wide handle (once, at start-up).
pub fn set_global(shared: Shared) {
    let _ = GLOBAL.set(shared);
}

/// The process-wide handle, if `CRM_REDIS_URL` is set.
#[must_use]
pub fn global() -> Option<&'static Shared> {
    GLOBAL.get()
}

const GRANT_PREFIX: &str = "crm:bg:";

/// A cloneable handle to the shared Redis connection.
#[derive(Clone)]
pub struct Shared {
    conn: ConnectionManager,
}

impl Shared {
    /// Connect using `CRM_REDIS_URL`; `None` when it is unset or empty.
    ///
    /// # Errors
    ///
    /// When the URL is set but invalid or unreachable.
    pub async fn from_env() -> Result<Option<Self>, redis::RedisError> {
        let Some(url) = crate::settings::current().redis_url.clone() else {
            return Ok(None);
        };
        let client = redis::Client::open(url)?;
        Ok(Some(Self {
            conn: ConnectionManager::new(client).await?,
        }))
    }

    /// Count one event under `key` in a fixed window of `span_secs`; returns
    /// the new count and the seconds left in the window. The expiry is set
    /// only when the key is created, so the window does not slide.
    ///
    /// # Errors
    ///
    /// On any Redis error.
    pub async fn hit(&self, key: &str, span_secs: u64) -> Result<(u64, u64), redis::RedisError> {
        let mut conn = self.conn.clone();
        let (count, ttl): (u64, i64) = redis::pipe()
            .atomic()
            .incr(key, 1)
            .cmd("EXPIRE")
            .arg(key)
            .arg(span_secs)
            .arg("NX")
            .ignore()
            .ttl(key)
            .query_async(&mut conn)
            .await?;
        Ok((count, u64::try_from(ttl).unwrap_or(span_secs).max(1)))
    }

    /// The current count and seconds left for `key`, without counting.
    ///
    /// # Errors
    ///
    /// On any Redis error.
    pub async fn peek(&self, key: &str) -> Result<(u64, u64), redis::RedisError> {
        let mut conn = self.conn.clone();
        let count: Option<u64> = conn.get(key).await?;
        let ttl: i64 = conn.ttl(key).await?;
        Ok((count.unwrap_or(0), u64::try_from(ttl).unwrap_or(1).max(1)))
    }
}

impl Shared {
    /// Publish a break-glass grant so every replica sees it within one poll.
    /// The key expires with the grant, and a later expiry is never shortened.
    ///
    /// # Errors
    ///
    /// On any Redis error.
    pub async fn put_grant(
        &self,
        subject: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), redis::RedisError> {
        let key = format!("{GRANT_PREFIX}{subject}");
        let secs = (expires_at - Utc::now()).num_seconds();
        if secs <= 0 {
            return Ok(());
        }
        let mut conn = self.conn.clone();
        let existing: Option<String> = conn.get(&key).await?;
        if existing
            .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
            .is_some_and(|e| e.with_timezone(&Utc) >= expires_at)
        {
            return Ok(());
        }
        conn.set_ex::<_, _, ()>(&key, expires_at.to_rfc3339(), secs.unsigned_abs())
            .await
    }

    /// Every live grant in Redis, by subject.
    ///
    /// # Errors
    ///
    /// On any Redis error.
    pub async fn grants(&self) -> Result<HashMap<String, DateTime<Utc>>, redis::RedisError> {
        let mut conn = self.conn.clone();
        let keys: Vec<String> = {
            let mut iter = conn
                .scan_match::<_, String>(format!("{GRANT_PREFIX}*"))
                .await?;
            let mut keys = Vec::new();
            while let Some(key) = iter.next_item().await {
                keys.push(key?);
            }
            keys
        };
        let mut live = HashMap::new();
        for key in keys {
            let value: Option<String> = conn.get(&key).await?;
            if let (Some(subject), Some(expires_at)) = (
                key.strip_prefix(GRANT_PREFIX),
                value.and_then(|v| DateTime::parse_from_rfc3339(&v).ok()),
            ) {
                live.insert(subject.to_string(), expires_at.with_timezone(&Utc));
            }
        }
        Ok(live)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a Redis on `CRM_REDIS_URL` (`compose.test.yaml` provides one).
    #[tokio::test]
    #[ignore = "requires Redis on CRM_REDIS_URL"]
    async fn counts_in_a_fixed_window_and_peek_does_not_count() {
        let Some(shared) = Shared::from_env().await.unwrap() else {
            eprintln!("CRM_REDIS_URL unset; skipping");
            return;
        };
        let key = format!("crm:test:{}", uuid::Uuid::new_v4());
        assert_eq!(shared.hit(&key, 30).await.unwrap().0, 1);
        let (count, ttl) = shared.hit(&key, 30).await.unwrap();
        assert_eq!(count, 2);
        assert!((1..=30).contains(&ttl));
        assert_eq!(shared.peek(&key).await.unwrap().0, 2);
        assert_eq!(shared.peek(&key).await.unwrap().0, 2);
    }

    /// A published grant is visible through `grants`, is never shortened by
    /// an earlier expiry, and an expired one is not stored.
    #[tokio::test]
    #[ignore = "requires Redis on CRM_REDIS_URL"]
    async fn grants_are_published_and_never_shortened() {
        let Some(shared) = Shared::from_env().await.unwrap() else {
            eprintln!("CRM_REDIS_URL unset; skipping");
            return;
        };
        let subject = format!("user-{}", uuid::Uuid::new_v4());
        let later = Utc::now() + chrono::Duration::minutes(30);
        let sooner = Utc::now() + chrono::Duration::minutes(5);
        shared.put_grant(&subject, later).await.unwrap();
        shared.put_grant(&subject, sooner).await.unwrap();
        let live = shared.grants().await.unwrap();
        let seen = live.get(&subject).expect("grant published");
        assert!((*seen - later).num_seconds().abs() <= 1, "kept the later");
        let expired = format!("user-{}", uuid::Uuid::new_v4());
        shared
            .put_grant(&expired, Utc::now() - chrono::Duration::minutes(1))
            .await
            .unwrap();
        assert!(!shared.grants().await.unwrap().contains_key(&expired));
    }
}
