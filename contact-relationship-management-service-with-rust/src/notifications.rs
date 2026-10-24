//! Task reminders and SLA breaches turned into inbox rows (CRM-R32).
//!
//! The sweep finds what is due, asks the access policy whether the recipient may read the
//! record, and writes one row per recipient, kind and cause. A database uniqueness rule makes
//! each fire once, so a second sweep, a second replica or a retry makes nothing more. A
//! notification holds no record text.

use std::collections::BTreeMap;

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, Value};
use serde::Serialize;
use uuid::Uuid;

use crate::models::_entities::{contacts, deals};
use crate::models::live::FindLive;

/// The most candidates handled per kind in one sweep; the rest wait for the next.
const BATCH: i64 = 500;

/// What one sweep did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(rename = "NotificationSweep")]
pub struct Outcome {
    /// Notifications made for the recipient's inbox.
    pub created: u64,
    /// Due, but the recipient may not read the record (or it is gone), so none was shown.
    pub suppressed: u64,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// The attributes the policy matches on for the record a notification is about, or `None` when
/// the record is gone. Only contacts and deals carry record-level attributes; the others are
/// decided on the entity alone.
///
/// # Errors
///
/// A database error.
pub async fn subject_attrs<C: ConnectionTrait>(
    db: &C,
    kind: &str,
    pid: Uuid,
) -> Result<Option<BTreeMap<String, Vec<String>>>, DbErr> {
    use sea_orm::{ColumnTrait, QueryFilter};
    match kind {
        "contact" => Ok(contacts::Entity::find_live()
            .filter(contacts::Column::Pid.eq(pid))
            .one(db)
            .await?
            .map(|c| crate::auth::contact_resource_attrs(&c))),
        "deal" => Ok(deals::Entity::find_live()
            .filter(deals::Column::Pid.eq(pid))
            .one(db)
            .await?
            .map(|d| crate::auth::deal_resource_attrs(&d))),
        "account" | "lead" | "ticket" => {
            let table = match kind {
                "account" => "accounts",
                "lead" => "leads",
                _ => "tickets",
            };
            let found = db
                .query_one_raw(stmt(
                    &format!("SELECT 1 AS one FROM {table} WHERE pid = $1 AND deleted_at IS NULL"),
                    [pid.into()],
                ))
                .await?;
            Ok(found.map(|_| BTreeMap::new()))
        }
        _ => Ok(None),
    }
}

/// One thing that is due and would notify `recipient`.
struct Due {
    kind: &'static str,
    recipient: String,
    subject_kind: String,
    subject_pid: Uuid,
    source_pid: Uuid,
    source_key: String,
}

async fn due_reminders<C: ConnectionTrait>(db: &C) -> Result<Vec<Due>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT a.pid, a.subject_kind, a.subject_pid, a.assignee_ref, \
                    (extract(epoch FROM a.remind_at) * 1000000)::bigint AS micros \
             FROM activities a \
             WHERE a.kind = 'task' AND a.done = FALSE AND a.deleted_at IS NULL \
               AND a.assignee_ref IS NOT NULL AND a.remind_at <= now() \
               AND NOT EXISTS (SELECT 1 FROM notifications n \
                    WHERE n.kind = 'task_reminder' AND n.source_pid = a.pid \
                      AND n.recipient_ref = a.assignee_ref \
                      AND n.source_key = (extract(epoch FROM a.remind_at) * 1000000)::bigint::text) \
             ORDER BY a.remind_at LIMIT $1",
            [BATCH.into()],
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            Ok(Due {
                kind: "task_reminder",
                recipient: row.try_get("", "assignee_ref")?,
                subject_kind: row.try_get("", "subject_kind")?,
                subject_pid: row.try_get("", "subject_pid")?,
                source_pid: row.try_get("", "pid")?,
                source_key: row.try_get::<i64>("", "micros")?.to_string(),
            })
        })
        .collect()
}

async fn due_breaches<C: ConnectionTrait>(db: &C) -> Result<Vec<Due>, DbErr> {
    let mut out = Vec::new();
    for (flag, key) in [
        ("first_response_breached", "first_response"),
        ("resolution_breached", "resolution"),
    ] {
        let rows = db
            .query_all_raw(stmt(
                &format!(
                    "SELECT t.pid, t.assignee_ref FROM tickets t \
                     WHERE t.{flag} = TRUE AND t.deleted_at IS NULL \
                       AND t.status IN ('open', 'pending', 'resolved') \
                       AND t.assignee_ref IS NOT NULL \
                       AND NOT EXISTS (SELECT 1 FROM notifications n \
                            WHERE n.kind = 'sla_breach' AND n.source_pid = t.pid \
                              AND n.recipient_ref = t.assignee_ref AND n.source_key = $1) \
                     ORDER BY t.id LIMIT $2"
                ),
                [key.into(), BATCH.into()],
            ))
            .await?;
        for row in rows {
            let pid: Uuid = row.try_get("", "pid")?;
            out.push(Due {
                kind: "sla_breach",
                recipient: row.try_get("", "assignee_ref")?,
                subject_kind: "ticket".to_string(),
                subject_pid: pid,
                source_pid: pid,
                source_key: key.to_string(),
            });
        }
    }
    Ok(out)
}

/// Make the notifications that are due. Idempotent: what already has a row is not found again,
/// and a concurrent sweep that races to the same row loses on the uniqueness rule.
///
/// # Errors
///
/// A database error.
pub async fn sweep<C: ConnectionTrait>(db: &C) -> Result<Outcome, DbErr> {
    let mut due = due_reminders(db).await?;
    due.extend(due_breaches(db).await?);
    let mut outcome = Outcome::default();
    for item in due {
        let readable = match subject_attrs(db, &item.subject_kind, item.subject_pid).await? {
            Some(attrs) => {
                crate::auth::may_read(&crate::auth::claims_for_worker(&item.recipient), &attrs)
            }
            None => false,
        };
        let inserted = db
            .execute_raw(stmt(
                "INSERT INTO notifications \
                 (pid, recipient_ref, kind, subject_kind, subject_pid, source_pid, source_key, \
                  suppressed) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
                [
                    Uuid::new_v4().into(),
                    item.recipient.into(),
                    item.kind.into(),
                    item.subject_kind.into(),
                    item.subject_pid.into(),
                    item.source_pid.into(),
                    item.source_key.into(),
                    (!readable).into(),
                ],
            ))
            .await?
            .rows_affected();
        if inserted == 1 {
            if readable {
                outcome.created += 1;
            } else {
                outcome.suppressed += 1;
            }
        }
    }
    Ok(outcome)
}
