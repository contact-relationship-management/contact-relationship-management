//! Contacts the worker service says are employees (CRM-R92), and the events it sent.
//!
//! Raw SQL over the two tables of migration 000052, like [`super::deliveries`]: an employment row
//! pauses marketing to the contact (segments leave them out, a campaign or nurture step skips
//! them) and its removal lifts the pause. Consent is never touched here. Migration 000053 adds
//! `employment_last_events`, the last hire or leave applied per person and organization, which
//! orders the events (CRM-R93).
//!
//! On `sqlx` since CRM-T216 (CRM-D24): each function is one statement on any executor.

use axa::prelude::*;
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

/// One employment, as the contact's page and the subject-access export show it.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ContactEmployment")]
pub struct Employment {
    /// The employer, an `organization:` URN.
    pub organization_ref: String,
    /// The first day of employment WPM gave.
    pub since: chrono::NaiveDate,
}

/// Whether the contact is an employee of any organization (their marketing is paused).
///
/// # Errors
///
/// When the query fails.
pub async fn is_employee<'e>(db: impl PgExecutor<'e>, contact_pid: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM contact_employments WHERE contact_pid = $1) AS \"found!\"",
        contact_pid,
    )
    .fetch_one(db)
    .await?)
}

/// The contact's employments, by organization.
///
/// # Errors
///
/// When the query fails.
pub async fn of_contact<'e>(db: impl PgExecutor<'e>, contact_pid: Uuid) -> Result<Vec<Employment>> {
    Ok(sqlx::query_as!(
        Employment,
        "SELECT organization_ref, since FROM contact_employments \
         WHERE contact_pid = $1 ORDER BY organization_ref",
        contact_pid,
    )
    .fetch_all(db)
    .await?)
}

/// Record an event id. `false` when it was already recorded: the event was applied before.
///
/// # Errors
///
/// When the statement fails.
pub async fn claim_event<'e>(db: impl PgExecutor<'e>, event_id: &str, kind: &str) -> Result<bool> {
    let done = sqlx::query!(
        "INSERT INTO wpm_events (event_id, kind, outcome) VALUES ($1, $2, 'received') \
         ON CONFLICT (event_id) DO NOTHING",
        event_id,
        kind,
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Note what an event did.
///
/// # Errors
///
/// When the statement fails.
pub async fn settle_event<'e>(
    db: impl PgExecutor<'e>,
    event_id: &str,
    outcome: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE wpm_events SET outcome = $2 WHERE event_id = $1",
        event_id,
        outcome,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The live contacts that carry `person_ref`.
///
/// # Errors
///
/// When the query fails.
pub async fn contacts_of<'e>(db: impl PgExecutor<'e>, person_ref: &str) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT pid FROM contacts WHERE person_ref = $1 AND deleted_at IS NULL ORDER BY id",
        person_ref,
    )
    .fetch_all(db)
    .await?)
}

/// Mark the contact an employee of `organization_ref`. `false` when they already were (the first
/// record is kept).
///
/// # Errors
///
/// When the statement fails.
pub async fn record<'e>(
    db: impl PgExecutor<'e>,
    contact_pid: Uuid,
    person_ref: &str,
    organization_ref: &str,
    since: chrono::NaiveDate,
    event_id: &str,
) -> Result<bool> {
    let done = sqlx::query!(
        "INSERT INTO contact_employments \
           (contact_pid, person_ref, organization_ref, since, source_event_id) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (contact_pid, organization_ref) DO NOTHING",
        contact_pid,
        person_ref,
        organization_ref,
        since,
        event_id,
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// End the contact's employment by `organization_ref`. `false` when there was none.
///
/// # Errors
///
/// When the statement fails.
pub async fn end<'e>(
    db: impl PgExecutor<'e>,
    contact_pid: Uuid,
    organization_ref: &str,
) -> Result<bool> {
    let done = sqlx::query!(
        "DELETE FROM contact_employments WHERE contact_pid = $1 AND organization_ref = $2",
        contact_pid,
        organization_ref,
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// The last hire or leave applied for a person by an organization (CRM-R93), as the
/// subject-access export shows it.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "EmploymentLastEvent")]
pub struct LastEvent {
    /// The employer, an `organization:` URN.
    pub organization_ref: String,
    /// `hired` or `left`.
    pub kind: String,
    /// When WPM recorded it.
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    /// The day it took effect.
    pub effective_on: chrono::NaiveDate,
}

/// Hold the order of `person_ref`'s employment by `organization_ref` until the transaction ends,
/// so two events for it are judged one after the other (CRM-R93). A transaction-scoped advisory
/// lock: it works before the row exists and goes with the commit.
///
/// # Errors
///
/// When the statement fails.
pub async fn lock_order<'e>(
    db: impl PgExecutor<'e>,
    person_ref: &str,
    organization_ref: &str,
) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1 || ' ' || $2, 92)) IS NULL AS locked",
        person_ref,
        organization_ref,
    )
    .fetch_one(db)
    .await?;
    Ok(())
}

/// The stamp of the last hire or leave applied for `person_ref` by `organization_ref`, if any.
///
/// # Errors
///
/// When the query fails.
pub async fn last_applied<'e>(
    db: impl PgExecutor<'e>,
    person_ref: &str,
    organization_ref: &str,
) -> Result<Option<crate::rules::wpm_events::Stamp>> {
    Ok(sqlx::query!(
        "SELECT occurred_at, effective_on FROM employment_last_events \
         WHERE person_ref = $1 AND organization_ref = $2",
        person_ref,
        organization_ref,
    )
    .fetch_optional(db)
    .await?
    .map(|row| crate::rules::wpm_events::Stamp {
        occurred_at: row.occurred_at,
        effective_on: row.effective_on,
    }))
}

/// Note that a hire (`hired`) or leave (`left`) stamped `stamp` was applied for `person_ref` by
/// `organization_ref`, replacing the one before.
///
/// # Errors
///
/// When the statement fails.
pub async fn note_applied<'e>(
    db: impl PgExecutor<'e>,
    person_ref: &str,
    organization_ref: &str,
    kind: &str,
    stamp: crate::rules::wpm_events::Stamp,
    event_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO employment_last_events \
           (person_ref, organization_ref, kind, occurred_at, effective_on, event_id) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (person_ref, organization_ref) DO UPDATE SET kind = EXCLUDED.kind, \
           occurred_at = EXCLUDED.occurred_at, effective_on = EXCLUDED.effective_on, \
           event_id = EXCLUDED.event_id, applied_at = CURRENT_TIMESTAMP",
        person_ref,
        organization_ref,
        kind,
        stamp.occurred_at,
        stamp.effective_on,
        event_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The last hire or leave applied for `person_ref`, by organization.
///
/// # Errors
///
/// When the query fails.
pub async fn last_events_of<'e>(
    db: impl PgExecutor<'e>,
    person_ref: &str,
) -> Result<Vec<LastEvent>> {
    Ok(sqlx::query_as!(
        LastEvent,
        "SELECT organization_ref, kind, occurred_at, effective_on FROM employment_last_events \
         WHERE person_ref = $1 ORDER BY organization_ref",
        person_ref,
    )
    .fetch_all(db)
    .await?)
}

/// Delete what is kept of the order of `person_refs`' employments (erasure). The number deleted.
///
/// # Errors
///
/// When the statement fails.
pub async fn forget_last_events<'e>(
    db: impl PgExecutor<'e>,
    person_refs: &[String],
) -> Result<u64> {
    Ok(sqlx::query!(
        "DELETE FROM employment_last_events WHERE person_ref = ANY($1)",
        person_refs,
    )
    .execute(db)
    .await?
    .rows_affected())
}

/// The kind (`hired` or `left`) of the last hire or leave applied for `person_ref` by
/// `organization_ref`, if one is kept (CRM-R93).
///
/// # Errors
///
/// When the query fails.
pub async fn last_kind<'e>(
    db: impl PgExecutor<'e>,
    person_ref: &str,
    organization_ref: &str,
) -> Result<Option<String>> {
    Ok(sqlx::query_scalar!(
        "SELECT kind FROM employment_last_events \
         WHERE person_ref = $1 AND organization_ref = $2",
        person_ref,
        organization_ref,
    )
    .fetch_optional(db)
    .await?)
}

/// The distinct person references of the live contacts, sorted (for the employment check,
/// CRM-R94).
///
/// # Errors
///
/// When the query fails.
pub async fn live_person_refs<'e>(db: impl PgExecutor<'e>) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar!(
        "SELECT DISTINCT person_ref FROM contacts WHERE deleted_at IS NULL ORDER BY person_ref"
    )
    .fetch_all(db)
    .await?)
}
