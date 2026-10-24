//! Consent per purpose (CRM-R29): reading and writing the state of each purpose.
//!
//! The default purpose (`marketing`) lives in `contacts.marketing_consent`, as it always has, so
//! everything that read it still works. Every other purpose is a row in `contact_consents`; no
//! row means `never`. Only `granted` allows a send.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, Value};
use serde::Serialize;
use uuid::Uuid;

use crate::models::_entities::contacts;
use crate::rules::tokens::{DEFAULT_PURPOSE, PURPOSES};

/// One purpose and where the contact stands on it.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ConsentPurpose")]
pub struct PurposeState {
    /// `marketing`, `newsletter`, `events` or `product_updates`.
    pub purpose: String,
    /// `granted`, `withdrawn`, `never` or `pending`.
    pub state: String,
    /// When it last changed; absent for a purpose nobody has asked about.
    pub changed_at: Option<DateTime<Utc>>,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// The states held for the non-default purposes, with when they changed.
///
/// # Errors
///
/// A database error.
pub async fn others<C: ConnectionTrait>(
    db: &C,
    contact: Uuid,
) -> Result<BTreeMap<String, (String, DateTime<Utc>)>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT purpose, state, changed_at FROM contact_consents WHERE contact_pid = $1",
            [contact.into()],
        ))
        .await?;
    let mut out = BTreeMap::new();
    for row in rows {
        out.insert(
            row.try_get("", "purpose")?,
            (row.try_get("", "state")?, row.try_get("", "changed_at")?),
        );
    }
    Ok(out)
}

/// The contact's state for `purpose`. A purpose nobody has asked about is `never`.
///
/// # Errors
///
/// A database error.
pub async fn state_of<C: ConnectionTrait>(
    db: &C,
    contact: &contacts::Model,
    purpose: &str,
) -> Result<String, DbErr> {
    if purpose == DEFAULT_PURPOSE {
        return Ok(contact.marketing_consent.clone());
    }
    Ok(others(db, contact.pid)
        .await?
        .remove(purpose)
        .map_or_else(|| "never".to_string(), |(state, _)| state))
}

/// The state of every purpose, in the fixed order.
///
/// # Errors
///
/// A database error.
pub async fn all<C: ConnectionTrait>(
    db: &C,
    contact: &contacts::Model,
) -> Result<Vec<PurposeState>, DbErr> {
    let mut held = others(db, contact.pid).await?;
    Ok(PURPOSES
        .iter()
        .map(|purpose| {
            if *purpose == DEFAULT_PURPOSE {
                PurposeState {
                    purpose: (*purpose).to_string(),
                    state: contact.marketing_consent.clone(),
                    changed_at: contact.consent_changed_at.map(|t| t.to_utc()),
                }
            } else if let Some((state, at)) = held.remove(*purpose) {
                PurposeState {
                    purpose: (*purpose).to_string(),
                    state,
                    changed_at: Some(at),
                }
            } else {
                PurposeState {
                    purpose: (*purpose).to_string(),
                    state: "never".to_string(),
                    changed_at: None,
                }
            }
        })
        .collect())
}

/// Set the state of a non-default purpose.
///
/// # Errors
///
/// A database error.
pub async fn set_other<C: ConnectionTrait>(
    db: &C,
    contact: Uuid,
    purpose: &str,
    state: &str,
    at: DateTime<Utc>,
) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        "INSERT INTO contact_consents (contact_pid, purpose, state, changed_at) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (contact_pid, purpose) DO UPDATE SET state = $3, changed_at = $4",
        [contact.into(), purpose.into(), state.into(), at.into()],
    ))
    .await
    .map(|_| ())
}

/// Exit the contact's active nurture enrolments in sequences sent under `purpose`.
///
/// # Errors
///
/// A database error.
pub async fn exit_enrolments<C: ConnectionTrait>(
    db: &C,
    contact: Uuid,
    purpose: &str,
) -> Result<u64, DbErr> {
    db.execute_raw(stmt(
        "UPDATE nurture_enrollments SET status = 'exited', next_due_at = NULL \
         WHERE contact_pid = $1 AND status = 'active' AND deleted_at IS NULL \
         AND sequence_pid IN (SELECT pid FROM nurture_sequences WHERE purpose = $2)",
        [contact.into(), purpose.into()],
    ))
    .await
    .map(|r| r.rows_affected())
}

/// The purpose a message was sent under: its campaign's, or its nurture step's sequence's. A
/// message that cannot be found is `marketing`.
///
/// # Errors
///
/// A database error.
pub async fn purpose_of_message<C: ConnectionTrait>(
    db: &C,
    message: Uuid,
) -> Result<String, DbErr> {
    let row = db
        .query_one_raw(stmt(
            "SELECT purpose FROM campaigns WHERE pid = $1 \
             UNION ALL \
             SELECT q.purpose FROM nurture_steps s JOIN nurture_sequences q ON q.pid = s.sequence_pid \
             WHERE s.pid = $1 LIMIT 1",
            [message.into()],
        ))
        .await?;
    Ok(row
        .and_then(|r| r.try_get::<String>("", "purpose").ok())
        .unwrap_or_else(|| DEFAULT_PURPOSE.to_string()))
}

/// How one non-default purpose combined in a merge or an unmerge.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ConsentPurposeOutcome")]
pub struct PurposeOutcome {
    /// The purpose.
    pub purpose: String,
    /// The receiving contact's state before.
    pub before: String,
    /// The other contact's state.
    pub other: String,
    /// The state the receiving contact has now: the stricter of the two.
    pub result: String,
}

/// Make `target` at least as strict as `other` on every non-default purpose (CRM-R24,
/// CRM-R26: consent never widens). Where that changes `target`, the state is written, a
/// consent event records it with `source`, and a withdrawal exits the matching enrolments.
/// Returns every purpose either contact holds something for.
///
/// # Errors
///
/// A database error.
pub async fn tighten<C: ConnectionTrait>(
    db: &C,
    target: Uuid,
    other: Uuid,
    source: &str,
) -> Result<Vec<PurposeOutcome>, DbErr> {
    let mine = others(db, target).await?;
    let theirs = others(db, other).await?;
    let now = Utc::now();
    let mut out = Vec::new();
    for purpose in PURPOSES.iter().filter(|p| **p != DEFAULT_PURPOSE) {
        let held = |m: &BTreeMap<String, (String, DateTime<Utc>)>| {
            m.get(*purpose)
                .map_or_else(|| "never".to_string(), |(state, _)| state.clone())
        };
        let (before, other_state) = (held(&mine), held(&theirs));
        if !mine.contains_key(*purpose) && !theirs.contains_key(*purpose) {
            continue;
        }
        let result = crate::rules::merge::stricter_consent(&before, &other_state).to_string();
        if result != before {
            set_other(db, target, purpose, &result, now).await?;
            db.execute_raw(stmt(
                "INSERT INTO consent_events (pid, contact_pid, action, source, purpose, occurred_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
                [
                    Uuid::new_v4().into(),
                    target.into(),
                    result.clone().into(),
                    source.into(),
                    (*purpose).into(),
                    now.into(),
                ],
            ))
            .await?;
            if result == "withdrawn" {
                exit_enrolments(db, target, purpose).await?;
            }
        }
        out.push(PurposeOutcome {
            purpose: (*purpose).to_string(),
            before,
            other: other_state,
            result,
        });
    }
    Ok(out)
}
