//! SQL rollups of the activities table for the insight views (CRM-T80).
//!
//! The views used to load every activity and fold them in memory, which cost
//! time and memory in step with the table, and decrypted every summary on the
//! way. Each needs only a per-subject digest (the latest touch, and whether a
//! dated follow-up is still open) or counts for a window, so the database
//! computes those and sends back one row per subject or per group.
//!
//! On `sqlx` (CRM-D24, CRM-T207).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

type Result<T> = std::result::Result<T, sqlx::Error>;

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
pub(super) async fn touches(pool: &PgPool) -> Result<Vec<Touch>> {
    sqlx::query_as!(
        Touch,
        "SELECT subject_kind, subject_pid, MAX(occurred_at) AS \"last_at!\", \
                COUNT(*) FILTER (WHERE NOT done AND due_on IS NOT NULL) AS \"open_next!\" \
         FROM activities \
         WHERE deleted_at IS NULL \
           AND subject_kind IN ('contact', 'account', 'deal', 'lead') \
         GROUP BY subject_kind, subject_pid"
    )
    .fetch_all(pool)
    .await
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
    pool: &PgPool,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Result<Vec<EngagementCount>> {
    sqlx::query_as!(
        EngagementCount,
        "SELECT actor_ref, to_char(occurred_at AT TIME ZONE 'UTC', 'YYYY-MM') AS \"month!\", \
                kind, sentiment, COUNT(*) AS \"count!\" \
         FROM activities \
         WHERE deleted_at IS NULL AND occurred_at >= $1 AND occurred_at <= $2 \
         GROUP BY 1, 2, 3, 4",
        since,
        until,
    )
    .fetch_all(pool)
    .await
}

/// How many consent events each `source` has recorded, ever.
pub(super) async fn consent_events_by_source(
    pool: &PgPool,
) -> Result<std::collections::BTreeMap<String, usize>> {
    let rows =
        sqlx::query!("SELECT source, COUNT(*) AS \"n!\" FROM consent_events GROUP BY source")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.source, usize::try_from(row.n).unwrap_or(0)))
        .collect())
}

/// How many withdrawals happened since `since`, estate-wide.
pub(super) async fn withdrawals_since(pool: &PgPool, since: DateTime<Utc>) -> Result<usize> {
    let n = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"n!\" FROM consent_events \
         WHERE action = 'withdrawn' AND occurred_at >= $1",
        since,
    )
    .fetch_one(pool)
    .await?;
    Ok(usize::try_from(n).unwrap_or(0))
}

/// Withdrawals since `since` by the account of the contact (live contacts that
/// have an account; a withdrawal for any other contact is not counted).
pub(super) async fn withdrawals_by_account(
    pool: &PgPool,
    since: DateTime<Utc>,
) -> Result<std::collections::BTreeMap<Uuid, usize>> {
    let rows = sqlx::query!(
        "SELECT c.account_pid AS \"account_pid!\", COUNT(*) AS \"n!\" \
         FROM consent_events e \
         JOIN contacts c ON c.pid = e.contact_pid \
          AND c.deleted_at IS NULL AND c.account_pid IS NOT NULL \
         WHERE e.action = 'withdrawn' AND e.occurred_at >= $1 \
         GROUP BY c.account_pid",
        since,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.account_pid, usize::try_from(row.n).unwrap_or(0)))
        .collect())
}
