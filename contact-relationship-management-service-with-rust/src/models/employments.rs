//! Contacts the worker service says are employees (CRM-R92), and the events it sent.
//!
//! Raw SQL over the two tables of migration 000052, like [`super::deliveries`]: an employment row
//! pauses marketing to the contact (segments leave them out, a campaign or nurture step skips
//! them) and its removal lifts the pause. Consent is never touched here. Migration 000053 adds
//! `employment_last_events`, the last hire or leave applied per person and organization, which
//! orders the events (CRM-R93).

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Serialize;

fn stmt(sql: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

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
pub async fn is_employee<C: ConnectionTrait>(db: &C, contact_pid: Uuid) -> Result<bool> {
    let row = db
        .query_one_raw(stmt(
            "SELECT 1 AS one FROM contact_employments WHERE contact_pid = $1 LIMIT 1",
            [contact_pid.into()],
        ))
        .await?;
    Ok(row.is_some())
}

/// The contact's employments, by organization.
///
/// # Errors
///
/// When the query fails.
pub async fn of_contact<C: ConnectionTrait>(db: &C, contact_pid: Uuid) -> Result<Vec<Employment>> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT organization_ref, since FROM contact_employments \
             WHERE contact_pid = $1 ORDER BY organization_ref",
            [contact_pid.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Employment {
                organization_ref: row.try_get("", "organization_ref")?,
                since: row.try_get("", "since")?,
            })
        })
        .collect()
}

/// Record an event id. `false` when it was already recorded: the event was applied before.
///
/// # Errors
///
/// When the statement fails.
pub async fn claim_event<C: ConnectionTrait>(db: &C, event_id: &str, kind: &str) -> Result<bool> {
    let done = db
        .execute_raw(stmt(
            "INSERT INTO wpm_events (event_id, kind, outcome) VALUES ($1, $2, 'received') \
             ON CONFLICT (event_id) DO NOTHING",
            [event_id.into(), kind.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Note what an event did.
///
/// # Errors
///
/// When the statement fails.
pub async fn settle_event<C: ConnectionTrait>(db: &C, event_id: &str, outcome: &str) -> Result<()> {
    db.execute_raw(stmt(
        "UPDATE wpm_events SET outcome = $2 WHERE event_id = $1",
        [event_id.into(), outcome.into()],
    ))
    .await?;
    Ok(())
}

/// The live contacts that carry `person_ref`.
///
/// # Errors
///
/// When the query fails.
pub async fn contacts_of<C: ConnectionTrait>(db: &C, person_ref: &str) -> Result<Vec<Uuid>> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT pid FROM contacts WHERE person_ref = $1 AND deleted_at IS NULL ORDER BY id",
            [person_ref.into()],
        ))
        .await?;
    rows.iter().map(|row| Ok(row.try_get("", "pid")?)).collect()
}

/// Mark the contact an employee of `organization_ref`. `false` when they already were (the first
/// record is kept).
///
/// # Errors
///
/// When the statement fails.
pub async fn record<C: ConnectionTrait>(
    db: &C,
    contact_pid: Uuid,
    person_ref: &str,
    organization_ref: &str,
    since: chrono::NaiveDate,
    event_id: &str,
) -> Result<bool> {
    let done = db
        .execute_raw(stmt(
            "INSERT INTO contact_employments \
               (contact_pid, person_ref, organization_ref, since, source_event_id) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (contact_pid, organization_ref) DO NOTHING",
            [
                contact_pid.into(),
                person_ref.into(),
                organization_ref.into(),
                since.into(),
                event_id.into(),
            ],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// End the contact's employment by `organization_ref`. `false` when there was none.
///
/// # Errors
///
/// When the statement fails.
pub async fn end<C: ConnectionTrait>(
    db: &C,
    contact_pid: Uuid,
    organization_ref: &str,
) -> Result<bool> {
    let done = db
        .execute_raw(stmt(
            "DELETE FROM contact_employments WHERE contact_pid = $1 AND organization_ref = $2",
            [contact_pid.into(), organization_ref.into()],
        ))
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
pub async fn lock_order<C: ConnectionTrait>(
    db: &C,
    person_ref: &str,
    organization_ref: &str,
) -> Result<()> {
    db.query_one_raw(stmt(
        "SELECT pg_advisory_xact_lock(hashtextextended($1 || ' ' || $2, 92)) IS NULL AS locked",
        [person_ref.into(), organization_ref.into()],
    ))
    .await?;
    Ok(())
}

/// The stamp of the last hire or leave applied for `person_ref` by `organization_ref`, if any.
///
/// # Errors
///
/// When the query fails.
pub async fn last_applied<C: ConnectionTrait>(
    db: &C,
    person_ref: &str,
    organization_ref: &str,
) -> Result<Option<crate::rules::wpm_events::Stamp>> {
    let row = db
        .query_one_raw(stmt(
            "SELECT occurred_at, effective_on FROM employment_last_events \
             WHERE person_ref = $1 AND organization_ref = $2",
            [person_ref.into(), organization_ref.into()],
        ))
        .await?;
    row.map(|row| {
        Ok(crate::rules::wpm_events::Stamp {
            occurred_at: row
                .try_get::<chrono::DateTime<chrono::FixedOffset>>("", "occurred_at")?
                .with_timezone(&chrono::Utc),
            effective_on: row.try_get("", "effective_on")?,
        })
    })
    .transpose()
}

/// Note that a hire (`hired`) or leave (`left`) stamped `stamp` was applied for `person_ref` by
/// `organization_ref`, replacing the one before.
///
/// # Errors
///
/// When the statement fails.
pub async fn note_applied<C: ConnectionTrait>(
    db: &C,
    person_ref: &str,
    organization_ref: &str,
    kind: &str,
    stamp: crate::rules::wpm_events::Stamp,
    event_id: &str,
) -> Result<()> {
    db.execute_raw(stmt(
        "INSERT INTO employment_last_events \
           (person_ref, organization_ref, kind, occurred_at, effective_on, event_id) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (person_ref, organization_ref) DO UPDATE SET kind = EXCLUDED.kind, \
           occurred_at = EXCLUDED.occurred_at, effective_on = EXCLUDED.effective_on, \
           event_id = EXCLUDED.event_id, applied_at = CURRENT_TIMESTAMP",
        [
            person_ref.into(),
            organization_ref.into(),
            kind.into(),
            stamp.occurred_at.into(),
            stamp.effective_on.into(),
            event_id.into(),
        ],
    ))
    .await?;
    Ok(())
}

/// The last hire or leave applied for `person_ref`, by organization.
///
/// # Errors
///
/// When the query fails.
pub async fn last_events_of<C: ConnectionTrait>(
    db: &C,
    person_ref: &str,
) -> Result<Vec<LastEvent>> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT organization_ref, kind, occurred_at, effective_on FROM employment_last_events \
             WHERE person_ref = $1 ORDER BY organization_ref",
            [person_ref.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(LastEvent {
                organization_ref: row.try_get("", "organization_ref")?,
                kind: row.try_get("", "kind")?,
                occurred_at: row
                    .try_get::<chrono::DateTime<chrono::FixedOffset>>("", "occurred_at")?
                    .with_timezone(&chrono::Utc),
                effective_on: row.try_get("", "effective_on")?,
            })
        })
        .collect()
}

/// Delete what is kept of the order of `person_refs`' employments (erasure). The number deleted.
///
/// # Errors
///
/// When the statement fails.
pub async fn forget_last_events<C: ConnectionTrait>(db: &C, person_refs: &[String]) -> Result<u64> {
    let mut deleted = 0;
    for person_ref in person_refs {
        let done = db
            .execute_raw(stmt(
                "DELETE FROM employment_last_events WHERE person_ref = $1",
                [person_ref.as_str().into()],
            ))
            .await?;
        deleted += done.rows_affected();
    }
    Ok(deleted)
}

/// The kind (`hired` or `left`) of the last hire or leave applied for `person_ref` by
/// `organization_ref`, if one is kept (CRM-R93).
///
/// # Errors
///
/// When the query fails.
pub async fn last_kind<C: ConnectionTrait>(
    db: &C,
    person_ref: &str,
    organization_ref: &str,
) -> Result<Option<String>> {
    let row = db
        .query_one_raw(stmt(
            "SELECT kind FROM employment_last_events \
             WHERE person_ref = $1 AND organization_ref = $2",
            [person_ref.into(), organization_ref.into()],
        ))
        .await?;
    Ok(row.map(|row| row.try_get("", "kind")).transpose()?)
}

/// The distinct person references of the live contacts, sorted (for the employment check,
/// CRM-R94).
///
/// # Errors
///
/// When the query fails.
pub async fn live_person_refs<C: ConnectionTrait>(db: &C) -> Result<Vec<String>> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT DISTINCT person_ref FROM contacts WHERE deleted_at IS NULL \
             ORDER BY person_ref",
            [],
        ))
        .await?;
    rows.iter()
        .map(|row| Ok(row.try_get("", "person_ref")?))
        .collect()
}
