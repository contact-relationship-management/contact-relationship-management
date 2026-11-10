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
//!
//! On `sqlx` since CRM-T211 (CRM-D24): every statement is checked, and the two limits are bound,
//! not written into the SQL.

use std::collections::BTreeMap;

use axa::prelude::*;
use sqlx::{PgConnection, PgExecutor};
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
pub async fn ensure_rows(
    conn: &mut PgConnection,
    campaign_pid: Uuid,
    contacts: &[Uuid],
) -> Result<()> {
    for contact_pid in contacts {
        sqlx::query!(
            "INSERT INTO email_deliveries (pid, campaign_pid, contact_pid) \
             VALUES ($1, $2, $3) ON CONFLICT (campaign_pid, contact_pid) DO NOTHING",
            Uuid::new_v4(),
            campaign_pid,
            contact_pid,
        )
        .execute(&mut *conn)
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
pub async fn claim<'e>(
    db: impl PgExecutor<'e>,
    campaign_pid: Uuid,
    limit: i64,
) -> Result<Vec<Claimed>> {
    Ok(sqlx::query_as!(
        Claimed,
        "UPDATE email_deliveries SET state = 'sending', attempts = attempts + 1, \
           updated_at = now() \
         WHERE id IN ( \
           SELECT id FROM email_deliveries WHERE campaign_pid = $1 AND ( \
             state IN ('pending', 'no_address', 'suppressed') \
             OR (state = 'failed' AND attempts < $3 \
                 AND (error IS NULL OR error NOT LIKE 'permanent:%')) \
             OR (state = 'sending' \
                 AND updated_at < now() - make_interval(mins => $4))) \
           ORDER BY (state = 'pending') DESC, id LIMIT $2 \
           FOR UPDATE SKIP LOCKED) \
         RETURNING id, contact_pid",
        campaign_pid,
        limit,
        MAX_ATTEMPTS,
        ABANDONED_MINUTES,
    )
    .fetch_all(db)
    .await?)
}

/// Record the outcome of one claimed row.
///
/// # Errors
///
/// When the statement fails.
pub async fn settle<'e>(
    db: impl PgExecutor<'e>,
    id: i32,
    state: &str,
    message_id: Option<&str>,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE email_deliveries SET state = $2, message_id = $3, error = $4, \
           sent_at = CASE WHEN $5 THEN now() ELSE sent_at END, updated_at = now() \
         WHERE id = $1",
        id,
        state,
        message_id,
        error,
        state == "sent",
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Rows per state for a campaign.
///
/// # Errors
///
/// When the query fails.
pub async fn summary<'e>(
    db: impl PgExecutor<'e>,
    campaign_pid: Uuid,
) -> Result<BTreeMap<String, i64>> {
    Ok(sqlx::query!(
        "SELECT state, COUNT(*) AS \"n!\" FROM email_deliveries \
         WHERE campaign_pid = $1 GROUP BY state",
        campaign_pid,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| (row.state, row.n))
    .collect())
}

/// How many rows a further run could still send: pending, abandoned, or failed
/// with a transient error and attempts left.
///
/// # Errors
///
/// When the query fails.
pub async fn retriable<'e>(db: impl PgExecutor<'e>, campaign_pid: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"n!\" FROM email_deliveries WHERE campaign_pid = $1 AND ( \
           state IN ('pending', 'sending') \
           OR (state = 'failed' AND attempts < $2 \
               AND (error IS NULL OR error NOT LIKE 'permanent:%')))",
        campaign_pid,
        MAX_ATTEMPTS,
    )
    .fetch_one(db)
    .await?)
}

/// Whether the contact's mailbox is on the suppression list (a hard bounce or
/// a complaint was reported).
///
/// # Errors
///
/// When the query fails.
pub async fn is_suppressed<'e>(db: impl PgExecutor<'e>, contact_pid: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM email_suppressions WHERE contact_pid = $1) AS \"found!\"",
        contact_pid,
    )
    .fetch_one(db)
    .await?)
}

/// The suppression on a contact, as `(reason, created_at)`.
///
/// # Errors
///
/// When the query fails.
pub async fn suppression<'e>(
    db: impl PgExecutor<'e>,
    contact_pid: Uuid,
) -> Result<Option<(String, chrono::DateTime<chrono::FixedOffset>)>> {
    Ok(sqlx::query!(
        "SELECT reason, created_at FROM email_suppressions WHERE contact_pid = $1",
        contact_pid,
    )
    .fetch_optional(db)
    .await?
    .map(|row| (row.reason, row.created_at.into())))
}

/// Take a contact off the suppression list. Returns whether one was removed.
///
/// # Errors
///
/// When the statement fails.
pub async fn unsuppress<'e>(db: impl PgExecutor<'e>, contact_pid: Uuid) -> Result<bool> {
    let done = sqlx::query!(
        "DELETE FROM email_suppressions WHERE contact_pid = $1",
        contact_pid,
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() > 0)
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
pub async fn record_event(
    conn: &mut PgConnection,
    message_id: &str,
    event: &str,
) -> Result<EventEffect> {
    let Some(found) = sqlx::query!(
        "SELECT contact_pid, campaign_pid, message_id FROM email_deliveries \
         WHERE message_id = $1 OR provider_id = $1 LIMIT 1",
        message_id,
    )
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(EventEffect::UnknownMessage);
    };
    // An event may name the message by the provider's id; record it under ours.
    let inserted = sqlx::query!(
        "INSERT INTO email_events (message_id, event) VALUES ($1, $2) \
         ON CONFLICT (message_id, event) DO NOTHING",
        found.message_id,
        event,
    )
    .execute(&mut *conn)
    .await?;
    if inserted.rows_affected() == 0 {
        return Ok(EventEffect::Duplicate);
    }
    Ok(EventEffect::Recorded {
        contact_pid: found.contact_pid,
        campaign_pid: found.campaign_pid,
    })
}

/// Remember the provider's own id for a sent row, so an event that names the
/// message by that id can find it.
///
/// # Errors
///
/// When the statement fails.
pub async fn note_provider_id<'e>(
    db: impl PgExecutor<'e>,
    id: i32,
    provider_id: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE email_deliveries SET provider_id = $2 WHERE id = $1",
        id,
        provider_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Put a contact on the suppression list (idempotent).
///
/// # Errors
///
/// When the statement fails.
pub async fn suppress<'e>(db: impl PgExecutor<'e>, contact_pid: Uuid, reason: &str) -> Result<()> {
    sqlx::query!(
        "INSERT INTO email_suppressions (contact_pid, reason) VALUES ($1, $2) \
         ON CONFLICT (contact_pid) DO NOTHING",
        contact_pid,
        reason,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The contact and the campaign (or nurture step) a message was sent for, by our message id or the
/// provider's.
///
/// # Errors
///
/// When the statement fails.
pub async fn find_message<'e>(
    db: impl PgExecutor<'e>,
    message_id: &str,
) -> Result<Option<(Uuid, Uuid)>> {
    Ok(sqlx::query!(
        "SELECT contact_pid, campaign_pid FROM email_deliveries \
         WHERE message_id = $1 OR provider_id = $1 LIMIT 1",
        message_id,
    )
    .fetch_optional(db)
    .await?
    .map(|row| (row.contact_pid, row.campaign_pid)))
}

/// Count one first open or click on the campaign (a nurture step has no counter, so nothing
/// changes for one). A deleted campaign still counts what its mail did, as it always has.
///
/// # Errors
///
/// When the statement fails.
pub async fn count_engagement<'e>(
    db: impl PgExecutor<'e>,
    campaign_pid: Uuid,
    event: &str,
) -> Result<()> {
    // One checked statement per counter, in place of a column name written into the SQL.
    if event == "opened" {
        sqlx::query!(
            "UPDATE campaigns SET opened = opened + 1 WHERE pid = $1 \
             /* whatever deleted_at holds: the mail went out */",
            campaign_pid,
        )
        .execute(db)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE campaigns SET clicked = clicked + 1 WHERE pid = $1 \
             /* whatever deleted_at holds: the mail went out */",
            campaign_pid,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

/// Count one confirmed delivery on the campaign, deleted or not, as [`count_engagement`] does.
///
/// # Errors
///
/// When the statement fails.
pub async fn count_delivered<'e>(db: impl PgExecutor<'e>, campaign_pid: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE campaigns SET delivered = delivered + 1 WHERE pid = $1 \
         /* whatever deleted_at holds: the mail went out */",
        campaign_pid,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The row for one `(campaign or nurture step, contact)` pair, created if
/// missing: `(id, state, message_id)`.
///
/// # Errors
///
/// When a statement fails.
pub async fn row_for(
    conn: &mut PgConnection,
    owner_pid: Uuid,
    contact_pid: Uuid,
) -> Result<(i32, String, Option<String>)> {
    ensure_rows(conn, owner_pid, &[contact_pid]).await?;
    let row = sqlx::query!(
        "SELECT id, state, message_id FROM email_deliveries \
         WHERE campaign_pid = $1 AND contact_pid = $2",
        owner_pid,
        contact_pid,
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| Error::string("delivery row vanished"))?;
    Ok((row.id, row.state, row.message_id))
}
