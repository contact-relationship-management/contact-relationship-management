//! Merge two contacts into one (CRM-R24).
//!
//! `POST /api/contacts/{pid}/merge` folds the contact in the path into
//! `into_pid`, which survives. It is destructive-classified (the `/merge`
//! suffix), runs in one transaction, and never leaves the survivor more
//! permissive about consent than either contact was. With `dry_run` it does
//! the same work and rolls it back, so the counts it returns are exact.

use crate::controllers::PidPath;
use crate::json::Json;
use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveValue, ConnectionTrait, DatabaseBackend, DatabaseTransaction, QueryOrder, QuerySelect,
    Statement, TransactionTrait, Value,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, json};
use uuid::Uuid;

use super::privacy::TOMBSTONE_PERSON;
use super::{record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{consent_events, contact_merges, contacts};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::merge as rules;
use crate::streaming;
use crate::validation::Problems;

/// The merge request.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MergePayload {
    /// The contact that survives.
    pub into_pid: String,
    /// Why, in the operator's words. Recorded in the audit entry.
    pub reason: String,
    /// Report what would move, and change nothing.
    #[serde(default)]
    pub dry_run: bool,
}

/// Tables whose rows move to the survivor by repointing one column:
/// `(response key, table, contact column, extra condition, id column)`.
pub(super) const MOVES: [(&str, &str, &str, &str, &str); 7] = [
    (
        "activities",
        "activities",
        "subject_pid",
        "subject_kind = 'contact'",
        "pid",
    ),
    ("leads", "leads", "contact_pid", "TRUE", "pid"),
    ("deals", "deals", "primary_contact_pid", "TRUE", "pid"),
    ("tickets", "tickets", "contact_pid", "TRUE", "pid"),
    (
        "email_deliveries",
        "email_deliveries",
        "contact_pid",
        "TRUE",
        "pid",
    ),
    (
        "external_links",
        "external_links",
        "subject_pid",
        "subject_kind = 'contact'",
        "pid",
    ),
    (
        "consent_events",
        "consent_events",
        "contact_pid",
        "TRUE",
        "pid",
    ),
];

pub(super) fn conflict(message: &str) -> Error {
    Error::CustomError(StatusCode::CONFLICT, ErrorDetail::new("conflict", message))
}

/// A contact that can take part in a merge, or the reason it cannot.
fn usable(contact: &contacts::Model) -> Result<()> {
    if contact.person_ref == TOMBSTONE_PERSON {
        return Err(unprocessable("an erased contact cannot be merged"));
    }
    if contact.deleted_at.is_some() {
        return Err(if contact.merged_into_pid.is_some() {
            conflict("this contact has already been merged into another")
        } else {
            Error::NotFound
        });
    }
    Ok(())
}

/// Run `sql` with two contact pids and return the first column of each
/// affected row as a string (a `pid`).
async fn ids(
    txn: &DatabaseTransaction,
    sql: &str,
    survivor: Uuid,
    merged: Uuid,
) -> Result<Vec<String>> {
    let rows = txn
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [Value::from(survivor), Value::from(merged)],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            row.try_get_by_index::<Uuid>(0)
                .map(|id| id.to_string())
                .map_err(|e| Error::Model(ModelError::from(e)))
        })
        .collect()
}

/// Run `sql` with two contact pids and return the first column of each row, a
/// JSON document (`to_jsonb` of a row).
async fn rows_json(
    txn: &DatabaseTransaction,
    sql: &str,
    survivor: Uuid,
    merged: Uuid,
) -> Result<Vec<serde_json::Value>> {
    let rows = txn
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [Value::from(survivor), Value::from(merged)],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            row.try_get_by_index::<serde_json::Value>(0)
                .map_err(|e| Error::Model(ModelError::from(e)))
        })
        .collect()
}

/// What moved or was dropped, by table, as the ids and as the counts, and the
/// rows that were dropped, whole, so an unmerge can put them back.
#[derive(Default)]
struct Changes {
    moved: Map<String, serde_json::Value>,
    dropped: Map<String, serde_json::Value>,
    dropped_rows: Map<String, serde_json::Value>,
}

impl Changes {
    fn counts(map: &Map<String, serde_json::Value>) -> serde_json::Value {
        map.iter()
            .map(|(k, v)| (k.clone(), json!(v.as_array().map_or(0, Vec::len))))
            .collect::<Map<_, _>>()
            .into()
    }
}

/// Move every row keyed to `merged` over to `survivor`, dropping the ones the
/// survivor already has (the same nurture sequence, the same working group).
async fn move_rows(txn: &DatabaseTransaction, survivor: Uuid, merged: Uuid) -> Result<Changes> {
    let mut out = Changes::default();
    for (key, table, column, condition, id) in MOVES {
        let sql = format!(
            "UPDATE {table} SET {column} = $1 WHERE {column} = $2 AND {condition} RETURNING {id}"
        );
        out.moved.insert(
            key.to_string(),
            json!(ids(txn, &sql, survivor, merged).await?),
        );
    }
    // An enrolment in a sequence the survivor is already in is dropped. Keep
    // the rows first: an unmerge restores them as they were.
    out.dropped_rows.insert(
        "nurture_enrollments".into(),
        json!(
            rows_json(
                txn,
                "SELECT to_jsonb(e) FROM nurture_enrollments e \
                 WHERE e.contact_pid = $2 AND e.deleted_at IS NULL AND e.sequence_pid IN \
                 (SELECT sequence_pid FROM nurture_enrollments WHERE contact_pid = $1 AND deleted_at IS NULL)",
                survivor,
                merged,
            )
            .await?
        ),
    );
    let dropped = ids(
        txn,
        "UPDATE nurture_enrollments SET deleted_at = now(), status = 'exited', next_due_at = NULL \
         WHERE contact_pid = $2 AND deleted_at IS NULL AND sequence_pid IN \
         (SELECT sequence_pid FROM nurture_enrollments WHERE contact_pid = $1 AND deleted_at IS NULL) \
         RETURNING pid",
        survivor,
        merged,
    )
    .await?;
    out.dropped
        .insert("nurture_enrollments".into(), json!(dropped));
    out.moved.insert(
        "nurture_enrollments".into(),
        json!(
            ids(
                txn,
                "UPDATE nurture_enrollments SET contact_pid = $1 WHERE contact_pid = $2 \
                 AND deleted_at IS NULL RETURNING pid",
                survivor,
                merged,
            )
            .await?
        ),
    );
    // A working-group seat the survivor already holds is dropped.
    out.dropped_rows.insert(
        "working_group_members".into(),
        json!(
            rows_json(
                txn,
                "SELECT to_jsonb(m) FROM working_group_members m WHERE m.contact_pid = $2 AND m.group_pid IN \
                 (SELECT group_pid FROM working_group_members WHERE contact_pid = $1)",
                survivor,
                merged,
            )
            .await?
        ),
    );
    let dropped = ids(
        txn,
        "DELETE FROM working_group_members WHERE contact_pid = $2 AND group_pid IN \
         (SELECT group_pid FROM working_group_members WHERE contact_pid = $1) \
         RETURNING group_pid",
        survivor,
        merged,
    )
    .await?;
    out.dropped
        .insert("working_group_members".into(), json!(dropped));
    out.moved.insert(
        "working_group_members".into(),
        json!(
            ids(
                txn,
                "UPDATE working_group_members SET contact_pid = $1 WHERE contact_pid = $2 \
                 RETURNING group_pid",
                survivor,
                merged,
            )
            .await?
        ),
    );
    Ok(out)
}

/// Fill the survivor's empty fields from the merged contact, and set its
/// consent to the stricter of the two. Returns the names of the fields filled.
async fn combine(
    txn: &DatabaseTransaction,
    survivor: contacts::Model,
    merged: &contacts::Model,
) -> Result<(Vec<&'static str>, String)> {
    let mut filled = Vec::new();
    let mut note = |name: &'static str, was_empty: bool, has_value: bool| {
        if was_empty && has_value {
            filled.push(name);
        }
    };
    note(
        "account_pid",
        survivor.account_pid.is_none(),
        merged.account_pid.is_some(),
    );
    note(
        "owner_ref",
        survivor.owner_ref.is_none(),
        merged.owner_ref.is_some(),
    );
    note(
        "job_title",
        survivor.job_title.is_none(),
        merged.job_title.is_some(),
    );
    note(
        "stakeholder_role",
        survivor.stakeholder_role.is_none(),
        merged.stakeholder_role.is_some(),
    );
    note(
        "influence",
        survivor.influence.is_none(),
        merged.influence.is_some(),
    );
    note(
        "interest",
        survivor.interest.is_none(),
        merged.interest.is_some(),
    );

    let consent =
        rules::stricter_consent(&survivor.marketing_consent, &merged.marketing_consent).to_string();
    let consent_changed = consent != survivor.marketing_consent;
    let pid = survivor.pid;
    let mut active: contacts::ActiveModel = survivor.clone().into();
    active.account_pid = ActiveValue::set(rules::fill(survivor.account_pid, merged.account_pid));
    active.owner_ref = ActiveValue::set(rules::fill(survivor.owner_ref, merged.owner_ref.clone()));
    active.job_title = ActiveValue::set(rules::fill(survivor.job_title, merged.job_title.clone()));
    active.stakeholder_role = ActiveValue::set(rules::fill(
        survivor.stakeholder_role,
        merged.stakeholder_role.clone(),
    ));
    active.influence = ActiveValue::set(rules::fill(survivor.influence, merged.influence));
    active.interest = ActiveValue::set(rules::fill(survivor.interest, merged.interest));
    if consent_changed {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        active.marketing_consent = ActiveValue::set(consent.clone());
        active.consent_changed_at = ActiveValue::set(Some(now));
        consent_events::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            contact_pid: ActiveValue::set(pid),
            action: ActiveValue::set(consent.clone()),
            source: ActiveValue::set(format!("merge:{}", merged.pid)),
            occurred_at: ActiveValue::set(now),
            ..Default::default()
        }
        .insert(txn)
        .await?;
    }
    active.update(txn).await?;
    if consent == "withdrawn" {
        // A withdrawal exits every active enrolment, moved ones included.
        txn.execute_unprepared(&format!(
            "UPDATE nurture_enrollments SET status = 'exited', next_due_at = NULL \
             WHERE contact_pid = '{pid}' AND status = 'active' AND deleted_at IS NULL"
        ))
        .await?;
    }
    Ok((filled, consent))
}

/// Lock both contacts, in a fixed order so two merges (or a merge and an
/// unmerge) cannot cross.
pub(super) async fn lock_rows(
    txn: &DatabaseTransaction,
    merged_pid: Uuid,
    survivor_pid: Uuid,
) -> Result<Vec<contacts::Model>> {
    Ok(contacts::Entity::find()
        .filter(contacts::Column::Pid.is_in([merged_pid, survivor_pid]))
        .order_by_asc(contacts::Column::Id)
        .lock_exclusive()
        .all(txn)
        .await?)
}

/// `(merged, survivor)` out of locked rows, or `NotFound` if either is absent.
pub(super) fn pick_pair(
    rows: &[contacts::Model],
    merged_pid: Uuid,
    survivor_pid: Uuid,
) -> Result<(contacts::Model, contacts::Model)> {
    let pick = |wanted: Uuid| {
        rows.iter()
            .find(|c| c.pid == wanted)
            .cloned()
            .ok_or(Error::NotFound)
    };
    Ok((pick(merged_pid)?, pick(survivor_pid)?))
}

/// Both contacts must be writable by the caller at the destructive tier
/// (CRM-T24): a merge and an unmerge each change both records.
pub(super) fn authorize_destructive(
    caller: &MaybeAuthUser,
    contacts: [&contacts::Model; 2],
) -> Result<()> {
    for contact in contacts {
        auth::authorize_record(
            caller,
            authentication_verifier::Action::Destructive,
            &auth::contact_resource_attrs(contact),
        )
        .map_err(record_rejection)?;
    }
    Ok(())
}

/// What a merge did, for the ledger, the audit entry and the event.
struct MergeDone<'a> {
    survivor_pid: Uuid,
    merged: &'a contacts::Model,
    reason: &'a str,
    moved: &'a Changes,
    counts: &'a serde_json::Value,
    dropped: &'a serde_json::Value,
    filled: &'a [&'static str],
    /// `(survivor, merged)` consent before the merge.
    before: &'a (String, String),
    consent: &'a str,
}

/// Write the ledger row (what an unmerge needs), the audit entry and the event.
async fn record_merge(
    txn: &DatabaseTransaction,
    caller: &MaybeAuthUser,
    done: &MergeDone<'_>,
) -> Result<()> {
    contact_merges::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        survivor_pid: ActiveValue::set(done.survivor_pid),
        merged_pid: ActiveValue::set(done.merged.pid),
        merged_person_ref: ActiveValue::set(done.merged.person_ref.clone()),
        actor: ActiveValue::set(caller.actor().map(str::to_string)),
        moved: ActiveValue::set(
            json!({ "moved": done.moved.moved, "dropped": done.moved.dropped }),
        ),
        // What an unmerge needs. Ids, states and field names: no personal data.
        before: ActiveValue::set(Some(json!({
            "version": 1,
            "survivor_consent": done.before.0,
            "merged_consent": done.before.1,
            "filled": done.filled,
            "dropped_rows": done.moved.dropped_rows,
        }))),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Audit::record(
        txn,
        "contact",
        done.survivor_pid,
        "contacts_merged",
        caller.actor(),
        Some(json!({
            "merged_pid": done.merged.pid,
            "reason": done.reason,
            "moved": done.counts,
            "dropped": done.dropped,
            "filled": done.filled,
            "consent_result": done.consent,
        })),
    )
    .await?;
    streaming::emit_on(
        txn,
        "contact",
        "contacts_merged",
        &done.survivor_pid.to_string(),
        "",
        caller.actor(),
        Some(json!({ "merged_pid": done.merged.pid })),
    )
    .await
}

/// How the two contacts' marketing consent combined.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ContactMergeConsent")]
pub(crate) struct MergeConsent {
    survivor: String,
    merged: String,
    result: String,
}

/// `POST /api/contacts/{pid}/merge`: what moved, what was dropped, what was filled.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ContactMerge {
    survivor_pid: Uuid,
    merged_pid: Uuid,
    dry_run: bool,
    /// Rows moved to the survivor, by kind.
    #[schemars(with = "std::collections::BTreeMap<String, u64>")]
    moved: serde_json::Value,
    /// Rows dropped as duplicates of the survivor's, by kind.
    #[schemars(with = "std::collections::BTreeMap<String, u64>")]
    dropped: serde_json::Value,
    filled: Vec<&'static str>,
    consent: MergeConsent,
}

/// `POST /api/contacts/{pid}/merge` (CRM-R24).
#[debug_handler]
async fn merge(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(merged_pid): PidPath,
    Json(payload): Json<MergePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("reason", &payload.reason);
    problems.finish()?;
    let survivor_pid = records::parse_pid(&payload.into_pid)?;
    if merged_pid == survivor_pid {
        return Err(unprocessable("a contact cannot be merged into itself"));
    }

    let txn = ctx.db.begin().await?;
    let rows = lock_rows(&txn, merged_pid, survivor_pid).await?;
    let (merged, survivor) = pick_pair(&rows, merged_pid, survivor_pid)?;
    usable(&merged)?;
    usable(&survivor)?;
    authorize_destructive(&caller, [&merged, &survivor])?;

    let before = (
        survivor.marketing_consent.clone(),
        merged.marketing_consent.clone(),
    );
    let moved = move_rows(&txn, survivor_pid, merged_pid).await?;
    let (filled, consent) = combine(&txn, survivor, &merged).await?;

    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let mut gone: contacts::ActiveModel = merged.clone().into();
    gone.deleted_at = ActiveValue::set(Some(now));
    gone.merged_into_pid = ActiveValue::set(Some(survivor_pid));
    gone.update(&txn).await?;

    let counts = Changes::counts(&moved.moved);
    let dropped = Changes::counts(&moved.dropped);
    let body = ContactMerge {
        survivor_pid,
        merged_pid,
        dry_run: payload.dry_run,
        moved: counts.clone(),
        dropped: dropped.clone(),
        filled: filled.clone(),
        consent: MergeConsent {
            survivor: before.0.clone(),
            merged: before.1.clone(),
            result: consent.clone(),
        },
    };
    if payload.dry_run {
        txn.rollback().await?;
        return format::json(body);
    }

    let done = MergeDone {
        survivor_pid,
        merged: &merged,
        reason: &payload.reason,
        moved: &moved,
        counts: &counts,
        dropped: &dropped,
        filled: &filled,
        before: &before,
        consent: &consent,
    };
    record_merge(&txn, &caller, &done).await?;
    txn.commit().await?;
    format::json(body)
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/merge", post(merge))
}
