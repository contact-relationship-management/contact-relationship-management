//! Undo a merge (CRM-R26).
//!
//! `POST /api/contacts/{pid}/unmerge` takes the merged-away contact in the path
//! and gives it back what its merge took from it, using the ledger the merge
//! wrote. It returns the rows that still point at the survivor, restores the
//! rows the merge dropped, clears the fields the merge filled (when the
//! survivor still holds the value it was given), and makes the contact live
//! again. It never widens consent. Work the survivor did after the merge stays
//! with the survivor: an unmerge undoes the merge, it does not split two
//! people's history by guesswork.
//!
//! On `sqlx` since CRM-T213 (CRM-D24). The moves back are built from [`MOVES`] at run time, as the
//! merge's are, and a filled field's clearing from [`FILLABLE`]: every name comes from those
//! lists, every value is bound, and they are reviewed by hand. Every other statement is checked.

use crate::controllers::PidPath;
use crate::json::Json;
use axa::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Map, json};
use sqlx::{AssertSqlSafe, PgConnection};
use uuid::Uuid;

use super::merge::{MOVES, authorize_destructive, conflict, ids, lock_rows, pick_pair};
use super::privacy::TOMBSTONE_PERSON;
use super::{record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::db;
use crate::models::_entities::{contact_merges, contacts};
use crate::models::DateTimeWithTimeZone;
use crate::models::audit_logs::Model as Audit;
use crate::models::rows;
use crate::rules::merge as rules;
use crate::rules::tokens;
use crate::streaming;
use crate::validation::Problems;

/// The unmerge request.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnmergePayload {
    /// Why, in the operator's words. Recorded in the audit entry.
    pub reason: String,
    /// Report what would be restored, and change nothing.
    #[serde(default)]
    pub dry_run: bool,
}

/// The fields a merge can fill, which an unmerge may clear again.
const FILLABLE: [&str; 8] = [
    "account_pid",
    "owner_ref",
    "job_title",
    "given_name",
    "family_name",
    "stakeholder_role",
    "influence",
    "interest",
];

/// The ids under `key` in a ledger document, checked to be UUIDs.
fn ledger_ids(doc: &serde_json::Value, key: &str) -> Vec<Uuid> {
    doc.get("moved")
        .and_then(|m| m.get(key))
        .and_then(serde_json::Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                .collect()
        })
        .unwrap_or_default()
}

/// Give back the rows that still point at the survivor. Returns `(restored,
/// left behind)` per table: a row is left behind when it has since been moved
/// on, deleted or re-pointed.
async fn restore_moved(
    txn: &mut PgConnection,
    ledger: &contact_merges::Model,
) -> Result<(
    Map<String, serde_json::Value>,
    Map<String, serde_json::Value>,
)> {
    let (survivor, merged) = (ledger.survivor_pid, ledger.merged_pid);
    let mut restored = Map::new();
    let mut left = Map::new();
    let mut put = |key: &str, ids: usize, back: usize| {
        restored.insert(key.to_string(), json!(back));
        left.insert(key.to_string(), json!(ids - back));
    };
    for (key, table, column, condition, id) in MOVES {
        let moved = ledger_ids(&ledger.moved, key);
        let back = if moved.is_empty() {
            0
        } else {
            let sql = format!(
                "UPDATE {table} SET {column} = $1 WHERE {id} = ANY($3) AND {column} = $2 \
                 AND {condition} RETURNING {id}"
            );
            sqlx::query_scalar::<_, Uuid>(AssertSqlSafe(sql))
                .bind(merged)
                .bind(survivor)
                .bind(&moved)
                .fetch_all(&mut *txn)
                .await?
                .len()
        };
        put(key, moved.len(), back);
    }
    let moved = ledger_ids(&ledger.moved, "nurture_enrollments");
    let back = sqlx::query_scalar!(
        "UPDATE nurture_enrollments SET contact_pid = $1 WHERE pid = ANY($3) \
         AND contact_pid = $2 AND deleted_at IS NULL RETURNING pid",
        merged,
        survivor,
        &moved,
    )
    .fetch_all(&mut *txn)
    .await?
    .len();
    put("nurture_enrollments", moved.len(), back);
    let moved = ledger_ids(&ledger.moved, "working_group_members");
    let back = sqlx::query_scalar!(
        "UPDATE working_group_members SET contact_pid = $1 WHERE group_pid = ANY($3) \
         AND contact_pid = $2 RETURNING group_pid",
        merged,
        survivor,
        &moved,
    )
    .fetch_all(&mut *txn)
    .await?
    .len();
    put("working_group_members", moved.len(), back);
    Ok((restored, left))
}

/// Put back the nurture enrolments and group seats the merge dropped because
/// the survivor already had them. Returns how many of each.
async fn restore_dropped(
    txn: &mut PgConnection,
    ledger: &contact_merges::Model,
    before: &serde_json::Value,
) -> Result<Map<String, serde_json::Value>> {
    let mut out = Map::new();
    let rows = |key: &str| -> Vec<serde_json::Value> {
        before
            .get("dropped_rows")
            .and_then(|d| d.get(key))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let mut enrolments = 0;
    for row in rows("nurture_enrollments") {
        // The row as the merge dropped it, deleted or not: its own `deleted_at` is put back.
        enrolments += sqlx::query_scalar!(
            "UPDATE nurture_enrollments t SET deleted_at = r.deleted_at, status = r.status, \
             next_due_at = r.next_due_at \
             FROM jsonb_populate_record(NULL::nurture_enrollments, $1::jsonb) r \
             WHERE t.pid = r.pid AND t.contact_pid = $2 RETURNING t.pid",
            row,
            ledger.merged_pid,
        )
        .fetch_all(&mut *txn)
        .await?
        .len();
    }
    out.insert("nurture_enrollments".into(), json!(enrolments));
    let mut seats = 0;
    for row in rows("working_group_members") {
        seats += sqlx::query_scalar!(
            "INSERT INTO working_group_members (created_at, updated_at, group_pid, contact_pid) \
             SELECT r.created_at, r.updated_at, r.group_pid, r.contact_pid \
             FROM jsonb_populate_record(NULL::working_group_members, $1::jsonb) r \
             WHERE r.contact_pid = $2 ON CONFLICT DO NOTHING RETURNING group_pid",
            row,
            ledger.merged_pid,
        )
        .fetch_all(&mut *txn)
        .await?
        .len();
    }
    out.insert("working_group_members".into(), json!(seats));
    Ok(out)
}

/// Clear the fields the merge filled on the survivor, where the survivor
/// still holds the value copied from the merged contact. Returns `(cleared,
/// kept)`: a field the survivor has changed since is kept.
async fn clear_filled(
    txn: &mut PgConnection,
    ledger: &contact_merges::Model,
    before: &serde_json::Value,
) -> Result<(Vec<String>, Vec<String>)> {
    let (mut cleared, mut kept) = (Vec::new(), Vec::new());
    let names = before
        .get("filled")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for name in names.iter().filter_map(serde_json::Value::as_str) {
        // The name goes into SQL, so it must be one the merge can fill.
        if !FILLABLE.contains(&name) {
            continue;
        }
        let sql = format!(
            "UPDATE contacts SET {name} = NULL WHERE pid = $1 AND {name} IS NOT NULL \
             AND {name} IS NOT DISTINCT FROM (SELECT {name} FROM contacts WHERE pid = $2) \
             RETURNING pid"
        );
        let changed = ids(txn, sql, ledger.survivor_pid, ledger.merged_pid)
            .await?
            .len();
        if changed > 0 {
            cleared.push(name.to_string());
        } else {
            kept.push(name.to_string());
        }
    }
    Ok((cleared, kept))
}

/// The newest merge of `merged_pid` that has not been undone, or the reason
/// there is none.
async fn open_ledger(txn: &mut PgConnection, merged_pid: Uuid) -> Result<contact_merges::Model> {
    let ledger = sqlx::query_as!(
        contact_merges::Model,
        "SELECT created_at, updated_at, id, pid, survivor_pid, merged_pid, merged_person_ref, actor, \
        moved, before, unmerged_at AS \"unmerged_at: DateTimeWithTimeZone\", unmerged_actor FROM \
        contact_merges WHERE merged_pid = $1 AND unmerged_at IS NULL ORDER BY id DESC LIMIT 1",
        merged_pid,
    )
    .fetch_optional(&mut *txn)
    .await?;
    if let Some(ledger) = ledger {
        return Ok(ledger);
    }
    // Deleted or not: a merged contact is deleted.
    let known = sqlx::query!("SELECT deleted_at FROM contacts WHERE pid = $1", merged_pid,)
        .fetch_optional(&mut *txn)
        .await?
        .is_some();
    Err(if known {
        conflict("this contact has not been merged into another, or was already unmerged")
    } else {
        Error::NotFound
    })
}

/// Lock both contacts in a fixed order, as merge does, then read the ledger
/// row again under lock: another unmerge may have finished first. Returns
/// `(ledger, merged, survivor)`.
async fn lock_for_unmerge(
    txn: &mut PgConnection,
    ledger: &contact_merges::Model,
    merged_pid: Uuid,
) -> Result<(contact_merges::Model, contacts::Model, contacts::Model)> {
    let survivor_pid = ledger.survivor_pid;
    let rows = lock_rows(txn, merged_pid, survivor_pid).await?;
    let ledger = sqlx::query_as!(
        contact_merges::Model,
        "SELECT created_at, updated_at, id, pid, survivor_pid, merged_pid, merged_person_ref, actor, \
        moved, before, unmerged_at AS \"unmerged_at: DateTimeWithTimeZone\", unmerged_actor FROM \
        contact_merges WHERE id = $1 FOR UPDATE",
        ledger.id,
    )
    .fetch_optional(&mut *txn)
    .await?
    .filter(|l| l.unmerged_at.is_none())
    .ok_or_else(|| conflict("this merge was already undone"))?;
    let (merged, survivor) = pick_pair(&rows, merged_pid, survivor_pid)?;
    Ok((ledger, merged, survivor))
}

/// The state the merge left must still be the state the ledger describes.
/// Returns the ledger's `before` record, which the undo works from.
fn check_unmergeable(
    ledger: &contact_merges::Model,
    merged: &contacts::Model,
    survivor: &contacts::Model,
) -> Result<serde_json::Value> {
    if merged.person_ref == TOMBSTONE_PERSON || survivor.person_ref == TOMBSTONE_PERSON {
        return Err(unprocessable("an erased contact cannot be unmerged"));
    }
    if merged.merged_into_pid != Some(ledger.survivor_pid) {
        return Err(conflict(
            "this contact is not merged into the recorded survivor",
        ));
    }
    if survivor.deleted_at.is_some() {
        return Err(conflict(if survivor.merged_into_pid.is_some() {
            "the survivor was itself merged into another contact: unmerge that first"
        } else {
            "the survivor has been deleted"
        }));
    }
    ledger.before.clone().ok_or_else(|| {
        unprocessable("this merge was made before the undo record existed and cannot be undone")
    })
}

/// Bring the merged contact back. Consent never widens: the survivor keeps
/// what it has now, and the returning contact gets the stricter of its own
/// state and the survivor's, so a withdrawal made since the merge applies to
/// both. Returns the returning contact's consent.
async fn bring_back(
    txn: &mut PgConnection,
    merged: &contacts::Model,
    survivor: &contacts::Model,
    now: DateTimeWithTimeZone,
) -> Result<(String, Vec<crate::consents::PurposeOutcome>)> {
    let result =
        rules::stricter_consent(&merged.marketing_consent, &survivor.marketing_consent).to_string();
    let changed = result != merged.marketing_consent;
    // The merged contact is deleted, and is made live again.
    sqlx::query!(
        "UPDATE contacts SET deleted_at = NULL, merged_into_pid = NULL, marketing_consent = $2, \
         consent_changed_at = CASE WHEN $3 THEN $4 ELSE consent_changed_at END WHERE id = $1",
        merged.id,
        result,
        changed,
        now,
    )
    .execute(&mut *txn)
    .await?;
    if changed {
        sqlx::query!(
            "INSERT INTO consent_events (pid, contact_pid, action, source, occurred_at) \
             VALUES ($1, $2, $3, $4, $5)",
            Uuid::new_v4(),
            merged.pid,
            result,
            format!("unmerge:{}", survivor.pid),
            now,
        )
        .execute(&mut *txn)
        .await?;
    }
    if result == "withdrawn" {
        crate::consents::exit_enrolments(&mut *txn, merged.pid, tokens::DEFAULT_PURPOSE).await?;
    }
    // The other purposes follow the same rule, each on its own (CRM-R29).
    let purposes = crate::consents::tighten(
        &mut *txn,
        merged.pid,
        survivor.pid,
        &format!("unmerge:{}", survivor.pid),
    )
    .await?;
    Ok((result, purposes))
}

/// What an unmerge did, for the audit entry and the event.
struct UnmergeDone<'a> {
    survivor_pid: Uuid,
    merged_pid: Uuid,
    reason: &'a str,
    restored: &'a Map<String, serde_json::Value>,
    left_behind: &'a Map<String, serde_json::Value>,
    dropped_restored: &'a Map<String, serde_json::Value>,
    cleared: &'a [String],
    consent: &'a str,
}

/// Close the ledger row, then write the audit entry and the event.
async fn record_unmerge(
    txn: &mut PgConnection,
    caller: &MaybeAuthUser,
    ledger: contact_merges::Model,
    now: DateTimeWithTimeZone,
    done: &UnmergeDone<'_>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE contact_merges SET unmerged_at = $2, unmerged_actor = $3 WHERE id = $1",
        ledger.id,
        now,
        caller.actor(),
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(
        txn,
        "contact",
        done.survivor_pid,
        "contacts_unmerged",
        caller.actor(),
        Some(json!({
            "merged_pid": done.merged_pid,
            "reason": done.reason,
            "restored": done.restored,
            "left_behind": done.left_behind,
            "dropped_restored": done.dropped_restored,
            "fields_cleared": done.cleared,
            "consent_result": done.consent,
        })),
    )
    .await?;
    streaming::emit_in(
        txn,
        "contact",
        "contacts_unmerged",
        &done.survivor_pid.to_string(),
        "",
        caller.actor(),
        Some(json!({ "merged_pid": done.merged_pid })),
    )
    .await
}

/// `POST /api/contacts/{pid}/unmerge` (CRM-R26).
#[debug_handler]
async fn unmerge(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(merged_pid): PidPath,
    Json(payload): Json<UnmergePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("reason", &payload.reason);
    problems.finish()?;

    let mut txn = db::pool(&ctx).begin().await?;
    let ledger = open_ledger(&mut txn, merged_pid).await?;
    let survivor_pid = ledger.survivor_pid;
    let (ledger, merged, survivor) = lock_for_unmerge(&mut txn, &ledger, merged_pid).await?;
    let before = check_unmergeable(&ledger, &merged, &survivor)?;
    authorize_destructive(&caller, [&merged, &survivor])?;

    let (restored, left_behind) = restore_moved(&mut txn, &ledger).await?;
    let dropped_restored = restore_dropped(&mut txn, &ledger, &before).await?;
    let (cleared, kept) = clear_filled(&mut txn, &ledger, &before).await?;
    let now: DateTimeWithTimeZone = crate::clock::now().into();
    let (result, purposes) = bring_back(&mut txn, &merged, &survivor, now).await?;

    let body = ContactUnmerge {
        survivor_pid,
        merged_pid,
        dry_run: payload.dry_run,
        restored: restored.clone(),
        left_behind: left_behind.clone(),
        dropped_restored: dropped_restored.clone(),
        fields_cleared: cleared.clone(),
        fields_kept: kept,
        consent: UnmergeConsent {
            survivor: survivor.marketing_consent.clone(),
            merged_before: merged.marketing_consent.clone(),
            merged_result: result.clone(),
            purposes: purposes.clone(),
        },
    };
    if payload.dry_run {
        txn.rollback().await?;
        return format::json(body);
    }

    let done = UnmergeDone {
        survivor_pid,
        merged_pid,
        reason: &payload.reason,
        restored: &restored,
        left_behind: &left_behind,
        dropped_restored: &dropped_restored,
        cleared: &cleared,
        consent: &result,
    };
    record_unmerge(&mut txn, &caller, ledger, now, &done).await?;
    txn.commit().await?;
    format::json(body)
}

/// How the merged contact's consent came back.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ContactUnmergeConsent")]
pub(crate) struct UnmergeConsent {
    survivor: String,
    merged_before: String,
    merged_result: String,
    /// The other purposes (`newsletter`, …) that either contact holds a state for.
    purposes: Vec<crate::consents::PurposeOutcome>,
}

/// `POST /api/contacts/{pid}/unmerge`: what came back and what could not.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ContactUnmerge {
    survivor_pid: Uuid,
    merged_pid: Uuid,
    dry_run: bool,
    #[schemars(with = "std::collections::BTreeMap<String, u64>")]
    restored: Map<String, serde_json::Value>,
    #[schemars(with = "std::collections::BTreeMap<String, u64>")]
    left_behind: Map<String, serde_json::Value>,
    #[schemars(with = "std::collections::BTreeMap<String, u64>")]
    dropped_restored: Map<String, serde_json::Value>,
    fields_cleared: Vec<String>,
    fields_kept: Vec<String>,
    consent: UnmergeConsent,
}

/// One entry of `GET /api/contacts/{pid}/merges`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ContactMergeEntry")]
pub(crate) struct MergeEntry {
    merged_pid: Uuid,
    merged_name: String,
    merged_at: chrono::DateTime<chrono::FixedOffset>,
    undoable: bool,
}

/// `GET /api/contacts/{pid}/merges`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ContactMerges {
    merges: Vec<MergeEntry>,
}

/// `GET /api/contacts/{pid}/merges`: the contacts merged into this one that
/// can still be looked at, newest first, and whether each can be undone.
#[debug_handler]
async fn merges(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let survivor = rows::contact(pool, pid).await?.ok_or(Error::NotFound)?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::contact_resource_attrs(&survivor),
    )
    .map_err(record_rejection)?;
    let ledger = sqlx::query_as!(
        contact_merges::Model,
        "SELECT created_at, updated_at, id, pid, survivor_pid, merged_pid, merged_person_ref, actor, \
        moved, before, unmerged_at AS \"unmerged_at: DateTimeWithTimeZone\", unmerged_actor FROM \
        contact_merges WHERE survivor_pid = $1 AND unmerged_at IS NULL ORDER BY id DESC",
        pid,
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for entry in ledger {
        // The merged row may be gone (the retention sweep), and then so is the
        // chance to undo it. It is deleted, as a merged contact is.
        let Some(merged) = sqlx::query!(
            "SELECT display_name, person_ref, deleted_at FROM contacts WHERE pid = $1",
            entry.merged_pid,
        )
        .fetch_optional(pool)
        .await?
        else {
            continue;
        };
        out.push(MergeEntry {
            merged_pid: entry.merged_pid,
            merged_name: merged.display_name,
            merged_at: entry.created_at,
            undoable: entry.before.is_some() && merged.person_ref != TOMBSTONE_PERSON,
        });
    }
    format::json(ContactMerges { merges: out })
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/merges", get(merges))
        .add("/contacts/{pid}/unmerge", post(unmerge))
}
