//! Contacts the worker service says are employees (CRM-R92), and the events it sent.
//!
//! Raw SQL over the two tables of migration 000052, like [`super::deliveries`]: an employment row
//! pauses marketing to the contact (segments leave them out, a campaign or nurture step skips
//! them) and its removal lifts the pause. Consent is never touched here.

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
