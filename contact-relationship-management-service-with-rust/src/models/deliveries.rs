//! Per-recipient delivery state for campaign email (CRM-T39).
//!
//! A row per `(campaign, contact)` is the idempotency key of a send: a run
//! claims rows with one atomic statement, so two runs at once, or a retry
//! after a crash, never mail the same contact twice (a crash between the
//! provider accepting a message and the row being marked `sent` can still
//! produce one duplicate, which is the usual at-least-once trade-off).
//!
//! States: `pending` (to do), `sending` (claimed), `sent` (the provider
//! accepted it), `failed` (retried while the error is transient and attempts
//! remain), `no_address` and `suppressed` (the contact could not be mailed this
//! time; both are tried again on the next run and do not block completion).

use std::collections::BTreeMap;

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use uuid::Uuid;

/// A transient failure is retried at most this many times.
pub const MAX_ATTEMPTS: i32 = 5;

/// A `sending` row not touched for this long is taken to be abandoned.
const ABANDONED_MINUTES: i32 = 10;

/// A claimed delivery.
#[derive(Debug, Clone, Copy)]
pub struct Claimed {
    /// The `email_deliveries.id`.
    pub id: i32,
    /// The contact to mail.
    pub contact_pid: Uuid,
}

/// Make sure a pending row exists for each contact. Existing rows are kept.
///
/// # Errors
///
/// When a statement fails.
pub async fn ensure_rows<C: ConnectionTrait>(
    db: &C,
    campaign_pid: Uuid,
    contacts: &[Uuid],
) -> Result<()> {
    for contact_pid in contacts {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO email_deliveries (pid, campaign_pid, contact_pid) \
             VALUES ($1, $2, $3) ON CONFLICT (campaign_pid, contact_pid) DO NOTHING",
            [
                Uuid::new_v4().into(),
                campaign_pid.into(),
                (*contact_pid).into(),
            ],
        ))
        .await?;
    }
    Ok(())
}

/// Atomically claim up to `limit` rows that still need a send, pending ones
/// first. Concurrent callers get disjoint rows.
///
/// # Errors
///
/// When the statement fails.
pub async fn claim<C: ConnectionTrait>(
    db: &C,
    campaign_pid: Uuid,
    limit: i64,
) -> Result<Vec<Claimed>> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!(
                "UPDATE email_deliveries SET state = 'sending', attempts = attempts + 1, \
                   updated_at = now() \
                 WHERE id IN ( \
                   SELECT id FROM email_deliveries WHERE campaign_pid = $1 AND ( \
                     state IN ('pending', 'no_address', 'suppressed') \
                     OR (state = 'failed' AND attempts < {MAX_ATTEMPTS} \
                         AND (error IS NULL OR error NOT LIKE 'permanent:%')) \
                     OR (state = 'sending' \
                         AND updated_at < now() - interval '{ABANDONED_MINUTES} minutes')) \
                   ORDER BY (state = 'pending') DESC, id LIMIT $2 \
                   FOR UPDATE SKIP LOCKED) \
                 RETURNING id, contact_pid"
            ),
            [campaign_pid.into(), limit.into()],
        ))
        .await?;
    let mut claimed = Vec::with_capacity(rows.len());
    for row in &rows {
        claimed.push(Claimed {
            id: row.try_get("", "id")?,
            contact_pid: row.try_get("", "contact_pid")?,
        });
    }
    Ok(claimed)
}

/// Record the outcome of one claimed row.
///
/// # Errors
///
/// When the statement fails.
pub async fn settle<C: ConnectionTrait>(
    db: &C,
    id: i32,
    state: &str,
    message_id: Option<&str>,
    error: Option<&str>,
) -> Result<()> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE email_deliveries SET state = $2, message_id = $3, error = $4, \
           sent_at = CASE WHEN $2 = 'sent' THEN now() ELSE sent_at END, updated_at = now() \
         WHERE id = $1",
        [
            id.into(),
            state.into(),
            message_id.map(ToString::to_string).into(),
            error.map(ToString::to_string).into(),
        ],
    ))
    .await?;
    Ok(())
}

/// Rows per state for a campaign.
///
/// # Errors
///
/// When the query fails.
pub async fn summary<C: ConnectionTrait>(
    db: &C,
    campaign_pid: Uuid,
) -> Result<BTreeMap<String, i64>> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT state, COUNT(*) AS n FROM email_deliveries \
             WHERE campaign_pid = $1 GROUP BY state",
            [campaign_pid.into()],
        ))
        .await?;
    let mut out = BTreeMap::new();
    for row in &rows {
        out.insert(row.try_get("", "state")?, row.try_get("", "n")?);
    }
    Ok(out)
}

/// How many rows a further run could still send: pending, abandoned, or failed
/// with a transient error and attempts left.
///
/// # Errors
///
/// When the query fails.
pub async fn retriable<C: ConnectionTrait>(db: &C, campaign_pid: Uuid) -> Result<i64> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) AS n FROM email_deliveries WHERE campaign_pid = $1 AND ( \
                   state IN ('pending', 'sending') \
                   OR (state = 'failed' AND attempts < {MAX_ATTEMPTS} \
                       AND (error IS NULL OR error NOT LIKE 'permanent:%')))"
            ),
            [campaign_pid.into()],
        ))
        .await?;
    Ok(match row {
        Some(row) => row.try_get("", "n")?,
        None => 0,
    })
}

/// Whether the contact's mailbox is on the suppression list (a hard bounce or
/// a complaint was reported).
///
/// # Errors
///
/// When the query fails.
pub async fn is_suppressed<C: ConnectionTrait>(db: &C, contact_pid: Uuid) -> Result<bool> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT 1 AS one FROM email_suppressions WHERE contact_pid = $1",
            [contact_pid.into()],
        ))
        .await?;
    Ok(row.is_some())
}

/// What a provider event did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventEffect {
    /// No delivery carries that message id.
    UnknownMessage,
    /// The same event for the same message was already recorded.
    Duplicate,
    /// Recorded; carries the contact when it must now be suppressed.
    Recorded {
        /// The contact the message was sent to.
        contact_pid: Uuid,
        /// The campaign it was sent for.
        campaign_pid: Uuid,
    },
}

/// Record one provider event once. Returns what it matched; the caller applies
/// the consequences in the same transaction.
///
/// # Errors
///
/// When a statement fails.
pub async fn record_event<C: ConnectionTrait>(
    db: &C,
    message_id: &str,
    event: &str,
) -> Result<EventEffect> {
    let found = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT contact_pid, campaign_pid FROM email_deliveries WHERE message_id = $1",
            [message_id.into()],
        ))
        .await?;
    let Some(found) = found else {
        return Ok(EventEffect::UnknownMessage);
    };
    let inserted = db
        .execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO email_events (message_id, event) VALUES ($1, $2) \
             ON CONFLICT (message_id, event) DO NOTHING",
            [message_id.into(), event.into()],
        ))
        .await?;
    if inserted.rows_affected() == 0 {
        return Ok(EventEffect::Duplicate);
    }
    Ok(EventEffect::Recorded {
        contact_pid: found.try_get("", "contact_pid")?,
        campaign_pid: found.try_get("", "campaign_pid")?,
    })
}

/// Put a contact on the suppression list (idempotent).
///
/// # Errors
///
/// When the statement fails.
pub async fn suppress<C: ConnectionTrait>(db: &C, contact_pid: Uuid, reason: &str) -> Result<()> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO email_suppressions (contact_pid, reason) VALUES ($1, $2) \
         ON CONFLICT (contact_pid) DO NOTHING",
        [contact_pid.into(), reason.into()],
    ))
    .await?;
    Ok(())
}

/// Count one confirmed delivery on the campaign.
///
/// # Errors
///
/// When the statement fails.
pub async fn count_delivered<C: ConnectionTrait>(db: &C, campaign_pid: Uuid) -> Result<()> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE campaigns SET delivered = delivered + 1 WHERE pid = $1",
        [campaign_pid.into()],
    ))
    .await?;
    Ok(())
}
