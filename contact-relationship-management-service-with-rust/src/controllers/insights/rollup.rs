//! SQL rollups of the activities table for the insight views (CRM-T80).
//!
//! The views used to load every activity and fold them in memory, which cost
//! time and memory in step with the table, and decrypted every summary on the
//! way. Each needs only a per-subject digest (the latest touch, and whether a
//! dated follow-up is still open) or counts for a window, so the database
//! computes those and sends back one row per subject or per group.

use chrono::{DateTime, Utc};
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, Value};
use uuid::Uuid;

/// What the activities on one subject add up to.
#[derive(Debug, Clone)]
pub(super) struct Touch {
    /// `contact`, `account`, `deal` or `lead`.
    pub subject_kind: String,
    /// The subject.
    pub subject_pid: Uuid,
    /// When its latest activity happened.
    pub last_at: DateTime<Utc>,
    /// How many of its activities are not done and carry a due date.
    pub open_next: i64,
}

/// The digest of every live activity, one row per subject that has any.
pub(super) async fn touches(db: &DatabaseConnection) -> Result<Vec<Touch>> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT subject_kind, subject_pid, MAX(occurred_at) AS last_at, \
                    COUNT(*) FILTER (WHERE NOT done AND due_on IS NOT NULL) AS open_next \
             FROM activities \
             WHERE deleted_at IS NULL \
               AND subject_kind IN ('contact', 'account', 'deal', 'lead') \
             GROUP BY subject_kind, subject_pid",
        ))
        .await?;
    rows.iter()
        .map(|row| {
            let last_at: chrono::DateTime<chrono::FixedOffset> =
                row.try_get("", "last_at").map_err(model_error)?;
            Ok(Touch {
                subject_kind: row.try_get("", "subject_kind").map_err(model_error)?,
                subject_pid: row.try_get("", "subject_pid").map_err(model_error)?,
                last_at: last_at.to_utc(),
                open_next: row.try_get("", "open_next").map_err(model_error)?,
            })
        })
        .collect()
}

/// How many activities fall in one group of the engagement view.
#[derive(Debug, Clone)]
pub(super) struct EngagementCount {
    /// Who recorded them, if anyone.
    pub actor_ref: Option<String>,
    /// The month they happened in, `YYYY-MM` (UTC).
    pub month: String,
    /// `call`, `email`, ….
    pub kind: String,
    /// The recorded sentiment, if any.
    pub sentiment: Option<String>,
    /// How many.
    pub count: i64,
}

/// Live activities that happened between `since` and `until`, inclusive,
/// grouped by recorder, month, kind and sentiment.
pub(super) async fn engagement_counts(
    db: &DatabaseConnection,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Result<Vec<EngagementCount>> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT actor_ref, to_char(occurred_at AT TIME ZONE 'UTC', 'YYYY-MM') AS month, \
                    kind, sentiment, COUNT(*) AS n \
             FROM activities \
             WHERE deleted_at IS NULL AND occurred_at >= $1 AND occurred_at <= $2 \
             GROUP BY actor_ref, month, kind, sentiment",
            [Value::from(since), Value::from(until)],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(EngagementCount {
                actor_ref: row.try_get("", "actor_ref").map_err(model_error)?,
                month: row.try_get("", "month").map_err(model_error)?,
                kind: row.try_get("", "kind").map_err(model_error)?,
                sentiment: row.try_get("", "sentiment").map_err(model_error)?,
                count: row.try_get("", "n").map_err(model_error)?,
            })
        })
        .collect()
}

/// How many consent events each `source` has recorded, ever.
pub(super) async fn consent_events_by_source(
    db: &DatabaseConnection,
) -> Result<std::collections::BTreeMap<String, usize>> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT source, COUNT(*) AS n FROM consent_events GROUP BY source",
        ))
        .await?;
    rows.iter()
        .map(|row| {
            let n: i64 = row.try_get("", "n").map_err(model_error)?;
            Ok((
                row.try_get("", "source").map_err(model_error)?,
                usize::try_from(n).unwrap_or(0),
            ))
        })
        .collect()
}

/// How many withdrawals happened since `since`, estate-wide.
pub(super) async fn withdrawals_since(
    db: &DatabaseConnection,
    since: DateTime<Utc>,
) -> Result<usize> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS n FROM consent_events \
             WHERE action = 'withdrawn' AND occurred_at >= $1",
            [Value::from(since)],
        ))
        .await?;
    let n: i64 = match row {
        Some(row) => row.try_get("", "n").map_err(model_error)?,
        None => 0,
    };
    Ok(usize::try_from(n).unwrap_or(0))
}

/// Withdrawals since `since` by the account of the contact (live contacts that
/// have an account; a withdrawal for any other contact is not counted).
pub(super) async fn withdrawals_by_account(
    db: &DatabaseConnection,
    since: DateTime<Utc>,
) -> Result<std::collections::BTreeMap<Uuid, usize>> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT c.account_pid AS account_pid, COUNT(*) AS n \
             FROM consent_events e \
             JOIN contacts c ON c.pid = e.contact_pid \
              AND c.deleted_at IS NULL AND c.account_pid IS NOT NULL \
             WHERE e.action = 'withdrawn' AND e.occurred_at >= $1 \
             GROUP BY c.account_pid",
            [Value::from(since)],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            let n: i64 = row.try_get("", "n").map_err(model_error)?;
            Ok((
                row.try_get("", "account_pid").map_err(model_error)?,
                usize::try_from(n).unwrap_or(0),
            ))
        })
        .collect()
}

fn model_error(error: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(error))
}
