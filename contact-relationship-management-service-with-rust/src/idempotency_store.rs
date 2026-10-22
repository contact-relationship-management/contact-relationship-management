//! The Postgres side of idempotency keys (CRM-T52): claim a key, look up what
//! an earlier attempt left, record its result, or forget it. The rules are in
//! [`crate::idempotency`]; the middleware that uses these is in `app.rs`.
//!
//! The row is guarded by the `(scope, key)` primary key, so the guarantee
//! holds across replicas.

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};

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
    db: &DatabaseConnection,
    scope: &str,
    key: &str,
    fingerprint: &str,
) -> Result<bool, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO idempotency_keys (scope, key, fingerprint, state) \
             VALUES ($1, $2, $3, 'in_progress') \
             ON CONFLICT (scope, key) DO UPDATE SET fingerprint = EXCLUDED.fingerprint, \
               state = 'in_progress', status = NULL, content_type = NULL, body = NULL, \
               created_at = now() \
             WHERE idempotency_keys.created_at < now() - interval '24 hours' \
                OR (idempotency_keys.state = 'in_progress' \
                    AND idempotency_keys.created_at < now() - interval '2 minutes') \
             RETURNING 1",
            [scope.into(), key.into(), fingerprint.into()],
        ))
        .await?;
    Ok(!rows.is_empty())
}

/// The row an earlier attempt left, or `None` if it was forgotten in between.
///
/// # Errors
///
/// When the store cannot be reached.
pub async fn lookup(
    db: &DatabaseConnection,
    scope: &str,
    key: &str,
) -> Result<Option<Stored>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT fingerprint, state, status, content_type, body \
             FROM idempotency_keys WHERE scope = $1 AND key = $2",
            [scope.into(), key.into()],
        ))
        .await?;
    Ok(row.map(|row| Stored {
        fingerprint: row.try_get("", "fingerprint").unwrap_or_default(),
        state: row.try_get("", "state").unwrap_or_default(),
        status: row.try_get("", "status").unwrap_or(200),
        content_type: row.try_get("", "content_type").ok(),
        body: row.try_get("", "body").unwrap_or_default(),
    }))
}

/// Store the response so a retry replays it.
///
/// # Errors
///
/// When the store cannot be reached.
pub async fn record(
    db: &DatabaseConnection,
    scope: &str,
    key: &str,
    status: u16,
    content_type: Option<String>,
    body: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE idempotency_keys SET state = 'done', status = $3, content_type = $4, \
         body = $5 WHERE scope = $1 AND key = $2",
        [
            scope.into(),
            key.into(),
            i32::from(status).into(),
            content_type.into(),
            body.into(),
        ],
    ))
    .await
    .map(|_| ())
}

/// Forget the key, so a retry runs again (the attempt failed, or its response
/// was oversized or not text).
///
/// # Errors
///
/// When the store cannot be reached.
pub async fn forget(db: &DatabaseConnection, scope: &str, key: &str) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM idempotency_keys WHERE scope = $1 AND key = $2",
        [scope.into(), key.into()],
    ))
    .await
    .map(|_| ())
}
