//! Stakeholder-engagement / innovation-partnership / confederation
//! sub-resources — all **declared** data (a role, a grid score, a
//! partnership stage, a membership) recorded by an operator; nothing
//! here is inferred. Lifecycles live in
//! [`crate::rules::engagement`]; the derived views live in
//! [`super::insights`].

//!
//! On `sqlx` since CRM-T212 (CRM-D24): each change and its audit row commit in one transaction.

use crate::controllers::PidPath;
use crate::db;
use crate::json::Json;
use crate::models::DateTimeWithTimeZone;
use axa::extract::Path;
use axa::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::unprocessable;
use crate::models::_entities::{
    accounts, activities, contacts, memberships, partnerships, working_groups,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::models::rows;
use crate::rules::engagement as rules;
use crate::validation::Problems;

/// The auth extractor (actor stamping only).
use crate::auth::MaybeAuthUser;

/// `PUT /api/contacts/{pid}/stakeholder` body — declared typing; an
/// omitted field clears the declaration.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContactStakeholderPayload {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    influence: Option<i32>,
    #[serde(default)]
    interest: Option<i32>,
}

/// `PUT /api/contacts/{pid}/stakeholder` — declare (or clear) a
/// contact's stakeholder role and power–interest scores (1–5).
#[debug_handler]
async fn declare_contact_stakeholder(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ContactStakeholderPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt("role", rules::STAKEHOLDER_ROLES, payload.role.as_deref());
    for (field, score) in [
        ("influence", payload.influence),
        ("interest", payload.interest),
    ] {
        if let Some(score) = score
            && !rules::valid_grid_score(score)
        {
            problems.push(format!("{field} must be between 1 and 5"));
        }
    }
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let row = sqlx::query_as!(
        contacts::Model,
        "UPDATE contacts SET stakeholder_role = $2, influence = $3, interest = $4 \
            WHERE pid = $1 AND deleted_at IS NULL RETURNING created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
            status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
            \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
            given_name, consent_jurisdiction",
        pid,
        payload.role,
        payload.influence,
        payload.interest,
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    Audit::record_on(
        &mut txn,
        "contact",
        row.pid,
        "stakeholder_declared",
        caller.actor(),
        Some(serde_json::json!({
            "role": payload.role, "influence": payload.influence, "interest": payload.interest,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `PUT /api/accounts/{pid}/stakeholder` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccountStakeholderPayload {
    #[serde(default)]
    role: Option<String>,
}

/// `PUT /api/accounts/{pid}/stakeholder` — declare (or clear) an
/// account's stakeholder role.
#[debug_handler]
async fn declare_account_stakeholder(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<AccountStakeholderPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt("role", rules::STAKEHOLDER_ROLES, payload.role.as_deref());
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let row = sqlx::query_as!(
        accounts::Model,
        "UPDATE accounts SET stakeholder_role = $2 WHERE pid = $1 AND deleted_at IS NULL \
            RETURNING created_at, updated_at, id, pid, organization_ref, owner_ref, display_name, tier, \
            industry, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stakeholder_role",
        pid,
        payload.role,
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    Audit::record_on(
        &mut txn,
        "account",
        row.pid,
        "stakeholder_declared",
        caller.actor(),
        Some(serde_json::json!({ "role": payload.role })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/accounts/{pid}/partnerships` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PartnershipPayload {
    kind: String,
    summary: String,
    #[serde(default)]
    started_on: Option<chrono::NaiveDate>,
}

/// `POST /api/accounts/{pid}/partnerships` — open a partnership record
/// (starts at `scouting`).
#[debug_handler]
async fn create_partnership(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PartnershipPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("kind", rules::PARTNERSHIP_KINDS, &payload.kind);
    problems.require_text("summary", &payload.summary);
    problems.cap_text("summary", &payload.summary);
    problems.finish()?;
    let pool = db::pool(&ctx);
    let account = find_account(pool, pid).await?;
    let mut txn = pool.begin().await?;
    let row = sqlx::query_as!(
        partnerships::Model,
        "INSERT INTO partnerships (pid, account_pid, kind, stage, summary, started_on) \
            VALUES ($1, $2, $3, 'scouting', $4, $5) RETURNING created_at, updated_at, id, pid, account_pid, kind, stage, summary, started_on, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\"",
        Uuid::new_v4(),
        account.pid,
        payload.kind,
        payload.summary,
        payload.started_on,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "partnership",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/accounts/{pid}/partnerships` — the account's partnership
/// records, newest first.
#[debug_handler]
async fn list_partnerships(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let account = find_account(pool, pid).await?;
    format::json(rows::partnerships_of_account(pool, account.pid).await?)
}

/// `POST /api/partnerships/{pid}/stage` — the lifecycle move (forward
/// one step, or retire; the pure machine refuses everything else).
#[debug_handler]
async fn partnership_stage(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<super::StatusPayload>,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let row = rows::partnership_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    rules::partnership_transition(&row.stage, &payload.to)
        .map_err(|reason| unprocessable(&reason.to_string()))?;
    let updated = sqlx::query_as!(
        partnerships::Model,
        "UPDATE partnerships SET stage = $2 WHERE id = $1 RETURNING created_at, updated_at, id, pid, account_pid, kind, stage, summary, started_on, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\"",
        row.id,
        payload.to,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "partnership",
        row.pid,
        "partnership_stage_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": row.stage, "to": payload.to })),
    )
    .await?;
    txn.commit().await?;
    format::json(updated)
}

/// `PUT /api/accounts/{pid}/membership` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MembershipPayload {
    joined_on: chrono::NaiveDate,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    renewal_on: Option<chrono::NaiveDate>,
}

/// `PUT /api/accounts/{pid}/membership` — declare or update the
/// account's confederation membership (one record per account).
#[debug_handler]
async fn upsert_membership(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<MembershipPayload>,
) -> Result<Response> {
    let status = payload
        .status
        .clone()
        .unwrap_or_else(|| "active".to_string());
    let mut problems = Problems::new();
    problems.require_token("status", rules::MEMBERSHIP_STATUSES, &status);
    problems.finish()?;
    let pool = db::pool(&ctx);
    let account = find_account(pool, pid).await?;
    let mut txn = pool.begin().await?;
    // One record per account: a deleted one is taken back into use, as before, so it is read
    // whatever its `deleted_at`.
    let existing = sqlx::query!(
        "SELECT id, deleted_at FROM memberships WHERE account_pid = $1 ORDER BY id LIMIT 1 \
         FOR UPDATE",
        account.pid,
    )
    .fetch_optional(&mut *txn)
    .await?;
    let row = match existing {
        Some(found) => {
            sqlx::query_as!(
                memberships::Model,
                "UPDATE memberships SET joined_on = $2, status = $3, renewal_on = $4, \
                    deleted_at = NULL WHERE id = $1 RETURNING created_at, updated_at, id, pid, account_pid, joined_on, status, renewal_on, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\"",
                found.id,
                payload.joined_on,
                status,
                payload.renewal_on,
            )
            .fetch_one(&mut *txn)
            .await?
        }
        None => {
            sqlx::query_as!(
                memberships::Model,
                "INSERT INTO memberships (pid, account_pid, joined_on, status, renewal_on) \
                    VALUES ($1, $2, $3, $4, $5) RETURNING created_at, updated_at, id, pid, account_pid, joined_on, status, renewal_on, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\"",
                Uuid::new_v4(),
                account.pid,
                payload.joined_on,
                status,
                payload.renewal_on,
            )
            .fetch_one(&mut *txn)
            .await?
        }
    };
    Audit::record_on(
        &mut txn,
        "membership",
        row.pid,
        "membership_declared",
        caller.actor(),
        Some(serde_json::json!({ "status": status, "renewal_on": payload.renewal_on })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/groups` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GroupPayload {
    name: String,
    #[serde(default)]
    purpose: Option<String>,
}

/// `POST /api/groups` — create a working group.
#[debug_handler]
async fn create_group(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<GroupPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.cap_text("name", &payload.name);
    problems.cap_opt("purpose", payload.purpose.as_deref());
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let row = sqlx::query_as!(
        working_groups::Model,
        "INSERT INTO working_groups (pid, name, purpose) VALUES ($1, $2, $3) RETURNING created_at, updated_at, id, pid, name, purpose, deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\"",
        Uuid::new_v4(),
        payload.name,
        payload.purpose,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "working_group",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/groups` — all live working groups with member counts.
#[debug_handler]
async fn list_groups(State(ctx): State<AppContext>) -> Result<Response> {
    let pool = db::pool(&ctx);
    let groups = rows::working_groups(pool).await?;
    let counts: std::collections::BTreeMap<Uuid, i64> = sqlx::query!(
        "SELECT group_pid, COUNT(*) AS \"n!\" FROM working_group_members GROUP BY group_pid"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| (row.group_pid, row.n))
    .collect();
    let view: Vec<serde_json::Value> = groups
        .iter()
        .map(|group| {
            serde_json::json!({
                "pid": group.pid,
                "name": group.name,
                "purpose": group.purpose,
                "members": counts.get(&group.pid).copied().unwrap_or(0),
            })
        })
        .collect();
    format::json(view)
}

/// `POST /api/groups/{pid}/members` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MemberPayload {
    contact_pid: Uuid,
}

/// `POST /api/groups/{pid}/members` — add a contact to the roster
/// (`422` if already a member).
#[debug_handler]
async fn add_member(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<MemberPayload>,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let group = find_group(pool, &pid).await?;
    let contact = rows::contact(pool, payload.contact_pid)
        .await?
        .ok_or(Error::NotFound)?;
    let mut txn = pool.begin().await?;
    let already = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM working_group_members WHERE group_pid = $1 \
         AND contact_pid = $2) AS \"found!\"",
        group.pid,
        contact.pid,
    )
    .fetch_one(&mut *txn)
    .await?;
    if already {
        return Err(unprocessable("contact is already a member of this group"));
    }
    sqlx::query!(
        "INSERT INTO working_group_members (group_pid, contact_pid) VALUES ($1, $2)",
        group.pid,
        contact.pid,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "working_group",
        group.pid,
        "member_added",
        caller.actor(),
        Some(serde_json::json!({ "contact_pid": contact.pid })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "group_pid": group.pid, "contact_pid": contact.pid }))
}

/// A member of a group's roster.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "GroupRosterMember")]
pub(crate) struct RosterMember {
    pid: Uuid,
    display_name: String,
    account_pid: Option<Uuid>,
    stakeholder_role: Option<String>,
}

/// `GET /api/groups/{pid}`: the group, its roster and the recent activity of its members.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[allow(clippy::struct_field_names)] // `group` is the response member clients read
pub(crate) struct Group {
    group: working_groups::Model,
    roster: Vec<RosterMember>,
    feed: Vec<activities::Model>,
    /// How the feed was derived.
    feed_note: &'static str,
}

/// `GET /api/groups/{pid}` — the group, its roster (with each
/// member's account), and the recent activity feed derived from the
/// members' recorded activities (cap 50; derivation disclosed).
#[debug_handler]
async fn get_group(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let pool = db::pool(&ctx);
    let group = find_group(pool, &pid).await?;
    let member_pids = sqlx::query_scalar!(
        "SELECT contact_pid FROM working_group_members WHERE group_pid = $1 ORDER BY id",
        group.pid,
    )
    .fetch_all(pool)
    .await?;
    let contact_rows = rows::contacts_page(pool, Some(&member_pids), i64::MAX, 0).await?;
    let roster: Vec<RosterMember> = contact_rows
        .iter()
        .map(|contact| RosterMember {
            pid: contact.pid,
            display_name: contact.display_name.clone(),
            account_pid: contact.account_pid,
            stakeholder_role: contact.stakeholder_role.clone(),
        })
        .collect();
    let feed = if member_pids.is_empty() {
        Vec::new()
    } else {
        rows::recent_activities_of_contacts(pool, &member_pids, 50).await?
    };
    format::json(Group {
        group,
        roster,
        feed,
        feed_note: "recent contact-subject activities of roster members (cap 50)",
    })
}

/// Find one live working group by pid, or 404.
async fn find_group(pool: &PgPool, pid: &str) -> Result<working_groups::Model> {
    rows::working_group(pool, records::parse_pid(pid)?)
        .await?
        .ok_or(Error::NotFound)
}

/// The live account `pid`, or `404`.
async fn find_account(pool: &PgPool, pid: Uuid) -> Result<accounts::Model> {
    rows::account(pool, pid).await?.ok_or(Error::NotFound)
}

/// The engagement routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add(
            "/contacts/{pid}/stakeholder",
            put(declare_contact_stakeholder),
        )
        .add(
            "/accounts/{pid}/stakeholder",
            put(declare_account_stakeholder),
        )
        .add("/accounts/{pid}/partnerships", post(create_partnership))
        .add("/accounts/{pid}/partnerships", get(list_partnerships))
        .add("/partnerships/{pid}/stage", post(partnership_stage))
        .add("/accounts/{pid}/membership", put(upsert_membership))
        .add("/groups", post(create_group))
        .add("/groups", get(list_groups))
        .add("/groups/{pid}", get(get_group))
        .add("/groups/{pid}/members", post(add_member))
}
