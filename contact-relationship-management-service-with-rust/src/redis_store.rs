//! Shared counters in Redis (the same `CRM_REDIS_URL` Loco's cache uses), so
//! the rate limit and failed-authentication lockout count across replicas
//! rather than per process. Unset URL means no shared state: callers fall
//! back to the in-process counters in [`crate::rate_limit`].
//!
//! Failure policy: a Redis error never blocks a request. The caller logs it
//! and uses the in-process counter, so an outage degrades to per-process
//! limits instead of an outage of the API.

use redis::AsyncCommands;
use redis::aio::ConnectionManager;

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
        let Some(url) = std::env::var("CRM_REDIS_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
        else {
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
}
