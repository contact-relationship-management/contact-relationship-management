//! The Postgres side of idempotency keys (CRM-T52): claim a key, look up what
//! an earlier attempt left, record its result, or forget it. The rules are in
//! [`crate::idempotency`]; the middleware that uses these is in `app.rs`.
//!
//! The row is guarded by the `(scope, key)` primary key, so the guarantee
//! holds across replicas.

use sqlx::PgPool;

/// What an earlier attempt with the same key left behind.
#[derive(Debug)]
pub struct Stored {
    /// Fingerprint of the request that claimed the key.
    pub fingerprint: String,
    /// `in_progress` or `done`.
    pub state: String,
    /// The stored response status (when `done`).
    pub status: i32,
    /// The stored response `Content-Type`, if there was one.
    pub content_type: Option<String>,
    /// The stored response body.
    pub body: String,
}

/// Claim the key for this request. An expired row, or one whose first attempt
/// crashed (still `in_progress` after 2 minutes), is taken over; anything else
/// leaves the existing row alone. `true` when this request now owns the key.
///
/// # Errors
///
/// When the store cannot be reached.
pub async fn claim(
    db: &PgPool,
    scope: &str,
    key: &str,
    fingerprint: &str,
) -> Result<bool, sqlx::Error> {
    let claimed = sqlx::query!(
        r#"INSERT INTO idempotency_keys (scope, key, fingerprint, state)
           VALUES ($1, $2, $3, 'in_progress')
           ON CONFLICT (scope, key) DO UPDATE SET fingerprint = EXCLUDED.fingerprint,
             state = 'in_progress', status = NULL, content_type = NULL, body = NULL,
             created_at = now()
           WHERE idempotency_keys.created_at < now() - interval '24 hours'
              OR (idempotency_keys.state = 'in_progress'
                  AND idempotency_keys.created_at < now() - interval '2 minutes')
           RETURNING 1 AS claimed"#,
        scope,
        key,
        fingerprint,
    )
    .fetch_optional(db)
    .await?;
    Ok(claimed.is_some())
}

/// The row an earlier attempt left, or `None` if it was forgotten in between.
/// A first attempt still running has no response yet, so it reads as status 200 and an empty body.
///
/// # Errors
///
/// When the store cannot be reached.
pub async fn lookup(db: &PgPool, scope: &str, key: &str) -> Result<Option<Stored>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT fingerprint, state, status, content_type, body
         FROM idempotency_keys WHERE scope = $1 AND key = $2",
        scope,
        key,
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|row| Stored {
        fingerprint: row.fingerprint,
        state: row.state,
        status: row.status.unwrap_or(200),
        content_type: row.content_type,
        body: row.body.unwrap_or_default(),
    }))
}

/// Store the response so a retry replays it.
///
/// # Errors
///
/// When the store cannot be reached.
pub async fn record(
    db: &PgPool,
    scope: &str,
    key: &str,
    status: u16,
    content_type: Option<String>,
    body: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE idempotency_keys SET state = 'done', status = $3, content_type = $4,
         body = $5 WHERE scope = $1 AND key = $2",
        scope,
        key,
        i32::from(status),
        content_type,
        body,
    )
    .execute(db)
    .await
    .map(|_| ())
}

/// Forget the key, so a retry runs again (the attempt failed, or its response
/// was oversized or not text).
///
/// # Errors
///
/// When the store cannot be reached.
pub async fn forget(db: &PgPool, scope: &str, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM idempotency_keys WHERE scope = $1 AND key = $2",
        scope,
        key,
    )
    .execute(db)
    .await
    .map(|_| ())
}
