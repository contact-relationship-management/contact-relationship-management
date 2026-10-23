//! The relationship layer (CRM-R1, CRM-R2): contacts + accounts as
//! URN wrappers, the merged timeline, the manual repoint endpoint,
//! and activities. Every mutation runs on one transaction (CRM-D9).

use crate::controllers::PidPath;
use crate::json::Json;
use crate::models::live::FindLive;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use contact_relationship_management_matcher::{Contact as MatchContact, Matcher, Ranked, Verdict};
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::paging::{Page, PageParams};
use super::{record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{accounts, activities, contacts, deals, tickets};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::tokens;
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/contacts` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContactPayload {
    person_ref: String,
    display_name: String,
    #[serde(default)]
    account_pid: Option<Uuid>,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    job_title: Option<String>,
    #[serde(default = "default_channel")]
    preferred_channel: String,
}

/// `POST /api/accounts` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccountPayload {
    organization_ref: String,
    display_name: String,
    #[serde(default = "default_tier")]
    tier: String,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    industry: Option<String>,
}

/// `POST /api/contacts/{pid}/repoint` body — the manual post-merge
/// repoint (CRM-D1).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepointPayload {
    person_ref: String,
    reason: String,
}

/// `POST /api/activities` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityPayload {
    subject_kind: String,
    subject_pid: Uuid,
    kind: String,
    summary: String,
    #[serde(default)]
    occurred_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    #[serde(default)]
    actor_ref: Option<String>,
    #[serde(default)]
    due_on: Option<chrono::NaiveDate>,
    /// Optional recorded sentiment (`positive` / `neutral` / `negative`).
    #[serde(default)]
    sentiment: Option<String>,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

fn default_channel() -> String {
    "email".to_string()
}
fn default_tier() -> String {
    "prospect".to_string()
}

/// `POST /api/contacts` — create the wrapper (consent starts `never`).
#[debug_handler]
async fn create_contact(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ContactPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_ref(
        "person_ref",
        entity_ref::EntityType::Person,
        &payload.person_ref,
    );
    problems.require_text("display_name", &payload.display_name);
    problems.ref_opt(
        "owner_ref",
        entity_ref::EntityType::Worker,
        payload.owner_ref.as_deref(),
    );
    problems.require_token(
        "preferred_channel",
        tokens::CHANNELS,
        &payload.preferred_channel,
    );
    problems.cap_opt("job_title", payload.job_title.as_deref());
    problems.finish()?;
    if let Some(account) = payload.account_pid {
        records::find_account(&ctx.db, account).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = contacts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        account_pid: ActiveValue::set(payload.account_pid),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        display_name: ActiveValue::set(payload.display_name.clone()),
        status: ActiveValue::set("active".to_string()),
        job_title: ActiveValue::set(payload.job_title.clone()),
        preferred_channel: ActiveValue::set(payload.preferred_channel.clone().into()),
        marketing_consent: ActiveValue::set("never".to_string()),
        consent_changed_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "contact",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "owner": row.owner_ref })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "contact",
        "created",
        &row.pid.to_string(),
        &row.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/contacts` — active contacts.
#[debug_handler]
async fn list_contacts(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(paging): Query<PageParams>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 1000)?;
    let rows = page
        .apply(contacts::Entity::find_live().order_by_asc(contacts::Column::Id))
        .all(&ctx.db)
        .await?;
    let (rows, more) = page.split(rows);
    let readable = readable_contacts(&rows, &caller);
    Audit::record_read(
        &ctx.db,
        "contact",
        Uuid::nil(),
        "contact_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": readable.len() })),
    )
    .await?;
    page.respond(readable, more, &uri)
}

/// Filter `rows` down to the ones `caller`'s record-level ABAC policy
/// allows to read, masking `preferred_channel` on a row whose decision
/// carries the `mask` obligation. A denied row is **omitted** — with
/// enforcement on and a policy that denies read on some contacts, a
/// collection read must not disclose more than the equivalent single
/// `GET /{pid}` would (CRM-T24; `agents/share/security.md` invariant 5).
/// With enforcement off (the default) or no deny rule configured,
/// `authorize_record` never denies, so this is a no-op.
fn readable_contacts(rows: &[contacts::Model], caller: &MaybeAuthUser) -> Vec<serde_json::Value> {
    rows.iter()
        .filter_map(|row| {
            let obligations = auth::authorize_record(
                caller,
                authentication_verifier::Action::Read,
                &auth::contact_resource_attrs(row),
            )
            .ok()?;
            let mut value = serde_json::json!(row);
            if obligations.iter().any(|o| o == "mask") {
                value = auth::mask_text_json(value, &["preferred_channel"]);
            }
            Some(value)
        })
        .collect()
}

/// Serialize rows to JSON values for masking.
fn to_values<T: serde::Serialize>(rows: &[T]) -> Vec<serde_json::Value> {
    rows.iter().map(|row| serde_json::json!(row)).collect()
}

/// The most activities, deals and tickets `GET /api/contacts/{pid}` returns each.
const DETAIL_ROWS: u64 = 200;

/// Drop the one row read past [`DETAIL_ROWS`]; true when there was one.
fn cut<T>(rows: &mut Vec<T>) -> bool {
    let more = rows.len() as u64 > DETAIL_ROWS;
    rows.truncate(usize::try_from(DETAIL_ROWS).unwrap_or(usize::MAX));
    more
}

/// `GET /api/contacts/{pid}` — the contact + its merged timeline
/// (activities, deals, tickets — chronological; CRM-R1).
///
/// Record-level ABAC (CRM-T24): a `mask` obligation on the decision
/// redacts the contact's `preferred_channel`
/// (`auth::mask_json`/`auth::MASKED` names it as the channel-detail
/// field CRM-R15 masks). The bundled activities/deals/tickets rows are
/// **not** re-authorized or masked here — each carries its own
/// resource attributes a caller might read differently under, and
/// folding that in is a separate, larger change (spec CRM-T24 note).
///
/// # Errors
///
/// `403` when the record-level ABAC policy denies reading this contact.
#[debug_handler]
async fn get_contact(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, pid).await?;
    let obligations = auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::contact_resource_attrs(&contact),
    )
    .map_err(record_rejection)?;
    // Each list is cut at DETAIL_ROWS, newest first, and `truncated` says which
    // were cut: one row more is read to know.
    let mut activity_rows = activities::Entity::find_live()
        .filter(activities::Column::SubjectKind.eq("contact"))
        .filter(activities::Column::SubjectPid.eq(contact.pid))
        .order_by_desc(activities::Column::OccurredAt)
        .limit(DETAIL_ROWS + 1)
        .all(&ctx.db)
        .await?;
    let mut deal_rows = deals::Entity::find_live()
        .filter(deals::Column::PrimaryContactPid.eq(contact.pid))
        .order_by_desc(deals::Column::Id)
        .limit(DETAIL_ROWS + 1)
        .all(&ctx.db)
        .await?;
    let mut ticket_rows = tickets::Entity::find_live()
        .filter(tickets::Column::ContactPid.eq(contact.pid))
        .order_by_desc(tickets::Column::Id)
        .limit(DETAIL_ROWS + 1)
        .all(&ctx.db)
        .await?;
    let truncated = serde_json::json!({
        "activities": cut(&mut activity_rows),
        "deals": cut(&mut deal_rows),
        "tickets": cut(&mut ticket_rows),
    });
    let mut contact_json = serde_json::json!(contact);
    let masked = obligations.iter().any(|o| o == "mask");
    if masked {
        contact_json = auth::mask_text_json(contact_json, &["preferred_channel"]);
    }
    Audit::record_read(
        &ctx.db,
        "contact",
        contact.pid,
        "contact_read",
        caller.actor(),
        Some(serde_json::json!({
            "masked": masked,
            "break_glass": auth::break_glass_active(&caller),
        })),
    )
    .await?;
    // The bundled rows follow the contact's own decision: a masked contact
    // read must not leak, through its timeline, what a masked deal or
    // activity list would hide (CRM-T32).
    let (activities_json, deals_json, tickets_json) = if masked {
        let each = |rows: &[serde_json::Value],
                    f: &dyn Fn(serde_json::Value) -> serde_json::Value| {
            rows.iter().cloned().map(f).collect::<Vec<_>>()
        };
        (
            each(&to_values(&activity_rows), &|v| {
                auth::mask_text_json(v, &["summary"])
            }),
            each(&to_values(&deal_rows), &|v| {
                auth::mask_json(v, &["amount_minor"])
            }),
            each(&to_values(&ticket_rows), &|v| {
                auth::mask_text_json(v, &["description"])
            }),
        )
    } else {
        (
            to_values(&activity_rows),
            to_values(&deal_rows),
            to_values(&ticket_rows),
        )
    };
    format::json(serde_json::json!({
        "contact": contact_json,
        "activities": activities_json,
        "deals": deals_json,
        "tickets": tickets_json,
        "truncated": truncated,
    }))
}

/// Contacts compared against the one asked about. A larger table is cut off
/// here and the answer says so (`truncated`).
const DUPLICATE_POOL: usize = 5000;
/// How many of the best candidates get their email address fetched and the
/// comparison repeated. The CRM holds no addresses (they live on the person
/// record), so each one costs a call to the person service.
const DUPLICATE_EMAIL_CHECKS: usize = 20;

/// `GET /api/contacts/{pid}/duplicates` query.
#[derive(Debug, Deserialize)]
struct DuplicateParams {
    /// Most candidates to return (default 20, at most 100).
    limit: Option<usize>,
}

fn matcher() -> &'static Matcher {
    static MATCHER: OnceLock<Matcher> = OnceLock::new();
    MATCHER.get_or_init(Matcher::default)
}

/// A contact as the matcher sees it: the name, title and employer the CRM
/// holds, and the person reference.
fn match_contact(contact: &contacts::Model, accounts: &HashMap<Uuid, String>) -> MatchContact {
    let mut m = MatchContact::new()
        .person_ref(&contact.person_ref)
        .display_name(&contact.display_name);
    if let Some(title) = &contact.job_title {
        m = m.job_title(title);
    }
    if let Some(name) = contact.account_pid.and_then(|pid| accounts.get(&pid)) {
        m = m.organization(name);
    }
    m
}

/// Display names of the accounts the given contacts belong to.
async fn account_names<'a>(
    db: &DatabaseConnection,
    contacts: impl Iterator<Item = &'a contacts::Model>,
) -> Result<HashMap<Uuid, String>> {
    let pids: HashSet<Uuid> = contacts.filter_map(|c| c.account_pid).collect();
    if pids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(accounts::Entity::find()
        .filter(accounts::Column::Pid.is_in(pids))
        .all(db)
        .await?
        .into_iter()
        .map(|a| (a.pid, a.display_name))
        .collect())
}

/// Second look at the best few candidates, with email addresses fetched from
/// the person service. Rescored entries replace their first score and the
/// list is re-sorted. Returns the candidates whose
/// address was used. Does nothing when the target has no address.
async fn recheck_with_email(
    target: &contacts::Model,
    rows: &[contacts::Model],
    query: &MatchContact,
    candidates: &[MatchContact],
    ranked: &mut [Ranked],
) -> HashSet<usize> {
    let mut emailed = HashSet::new();
    let Some(target_email) = crate::clients::email_address(&target.person_ref).await else {
        return emailed;
    };
    let query = query.clone().email(target_email);
    for entry in ranked.iter_mut().take(DUPLICATE_EMAIL_CHECKS) {
        let Some(email) = crate::clients::email_address(&rows[entry.index].person_ref).await else {
            continue;
        };
        entry.result = matcher().compare(&query, &candidates[entry.index].clone().email(email));
        emailed.insert(entry.index);
    }
    sort_ranked(ranked);
    emailed
}

/// Highest probability first; ties by position, so the order is stable.
fn sort_ranked(ranked: &mut [Ranked]) {
    ranked.sort_by(|x, y| {
        y.result
            .probability
            .total_cmp(&x.result.probability)
            .then(x.index.cmp(&y.index))
    });
}

/// `GET /api/contacts/{pid}/duplicates` — other contacts that are probably
/// the same person (CRM-R23).
///
/// Candidates are the other active contacts the caller may read (record-level
/// ABAC, so a denied contact is neither compared nor disclosed). They are
/// scored on name, job title, employer and `person_ref`; the best few then
/// have their email addresses fetched from the person service and are scored
/// again, because a shared address is the strongest evidence there is. The
/// best few are chosen by their name-and-employer score whatever its verdict,
/// so a changed surname can be rescued by a shared address. Only
/// `match` and `review` verdicts are returned, best first. Each carries the
/// per-field breakdown (outcome, similarity, weight, never the values).
///
/// The answer is advice: nothing here merges, deletes or changes a contact.
///
/// # Errors
///
/// `404` for an unknown contact; `403` when the caller may not read it.
#[debug_handler]
async fn contact_duplicates(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Query(params): Query<DuplicateParams>,
) -> Result<Response> {
    let target = records::find_contact(&ctx.db, pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::contact_resource_attrs(&target),
    )
    .map_err(record_rejection)?;
    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    let mut rows = contacts::Entity::find_live()
        .filter(contacts::Column::Pid.ne(target.pid))
        .order_by_asc(contacts::Column::Id)
        .limit(u64::try_from(DUPLICATE_POOL + 1).unwrap_or(u64::MAX))
        .all(&ctx.db)
        .await?;
    let truncated = rows.len() > DUPLICATE_POOL;
    rows.truncate(DUPLICATE_POOL);
    rows.retain(|row| {
        auth::authorize_record(
            &caller,
            authentication_verifier::Action::Read,
            &auth::contact_resource_attrs(row),
        )
        .is_ok()
    });

    let account_names = account_names(&ctx.db, rows.iter().chain(std::iter::once(&target))).await?;

    let query = match_contact(&target, &account_names);
    let candidates: Vec<MatchContact> = rows
        .iter()
        .map(|r| match_contact(r, &account_names))
        .collect();
    // Every candidate, best first, whatever the verdict: the email look
    // below can rescue a contact whose names alone said "no" (a married name).
    let mut ranked: Vec<Ranked> = matcher()
        .compare_one_to_many(&query, &candidates)
        .into_iter()
        .enumerate()
        .map(|(index, result)| Ranked { index, result })
        .collect();
    sort_ranked(&mut ranked);

    let emailed = recheck_with_email(&target, &rows, &query, &candidates, &mut ranked).await;

    let duplicates: Vec<serde_json::Value> = ranked
        .iter()
        .filter(|entry| entry.result.verdict != Verdict::NonMatch)
        .take(limit)
        .map(|entry| {
            let row = &rows[entry.index];
            serde_json::json!({
                "contact": {
                    "pid": row.pid,
                    "display_name": row.display_name,
                    "account_pid": row.account_pid,
                    "job_title": row.job_title,
                    "status": row.status,
                },
                "probability": entry.result.probability,
                "verdict": entry.result.verdict,
                "deterministic": entry.result.deterministic,
                "email_checked": emailed.contains(&entry.index),
                "fields": entry.result.fields,
            })
        })
        .collect();
    Audit::record_read(
        &ctx.db,
        "contact",
        target.pid,
        "contact_duplicates_read",
        caller.actor(),
        Some(serde_json::json!({
            "compared": candidates.len(),
            "returned": duplicates.len(),
            "truncated": truncated,
        })),
    )
    .await?;
    format::json(serde_json::json!({
        "contact_pid": target.pid,
        "compared": candidates.len(),
        "truncated": truncated,
        "duplicates": duplicates,
    }))
}

/// `POST /api/contacts/{pid}/repoint` — repoint the wrapper after an
/// upstream registry merge (manual v1; audited with the reason).
#[debug_handler]
async fn repoint_contact(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<RepointPayload>,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::contact_resource_attrs(&contact),
    )
    .map_err(record_rejection)?;
    let mut problems = Problems::new();
    problems.require_ref(
        "person_ref",
        entity_ref::EntityType::Person,
        &payload.person_ref,
    );
    problems.require_text("reason", &payload.reason);
    problems.finish()?;
    let txn = ctx.db.begin().await?;
    let from = contact.person_ref.clone();
    let mut active: contacts::ActiveModel = contact.into();
    active.person_ref = ActiveValue::set(payload.person_ref.clone());
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "contact",
        row.pid,
        "repointed",
        caller.actor(),
        Some(
            serde_json::json!({ "from": from, "to": payload.person_ref, "reason": payload.reason }),
        ),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/accounts`.
#[debug_handler]
async fn create_account(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<AccountPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_ref(
        "organization_ref",
        entity_ref::EntityType::Organization,
        &payload.organization_ref,
    );
    problems.require_text("display_name", &payload.display_name);
    problems.require_token("tier", tokens::ACCOUNT_TIERS, &payload.tier);
    problems.ref_opt(
        "owner_ref",
        entity_ref::EntityType::Worker,
        payload.owner_ref.as_deref(),
    );
    problems.cap_opt("industry", payload.industry.as_deref());
    problems.finish()?;
    let txn = ctx.db.begin().await?;
    let row = accounts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        organization_ref: ActiveValue::set(payload.organization_ref.clone()),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        display_name: ActiveValue::set(payload.display_name.clone()),
        tier: ActiveValue::set(payload.tier.clone()),
        industry: ActiveValue::set(payload.industry.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "account", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "account",
        "created",
        &row.pid.to_string(),
        &row.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/accounts`.
#[debug_handler]
async fn list_accounts(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(paging): Query<PageParams>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 500)?;
    let rows = page
        .apply(accounts::Entity::find_live().order_by_asc(accounts::Column::Id))
        .all(&ctx.db)
        .await?;
    let (rows, more) = page.split(rows);
    Audit::record_read(
        &ctx.db,
        "account",
        Uuid::nil(),
        "account_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    page.respond(rows, more, &uri)
}

/// `GET /api/accounts/{pid}` — account + its contacts and deals.
#[debug_handler]
async fn get_account(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let account = records::find_account(&ctx.db, pid).await?;
    let contact_rows = contacts::Entity::find()
        .filter(contacts::Column::AccountPid.eq(account.pid))
        .filter(contacts::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let deal_rows = deals::Entity::find()
        .filter(deals::Column::AccountPid.eq(account.pid))
        .filter(deals::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    // The bundled contacts and deals follow the same record-level decisions
    // as their own endpoints (CRM-T32): a denied row is omitted, a masked
    // one loses its channel detail or amount.
    let contacts_json = readable_contacts(&contact_rows, &caller);
    let deals_json: Vec<serde_json::Value> = deal_rows
        .iter()
        .filter_map(|deal| {
            let obligations = auth::authorize_record(
                &caller,
                authentication_verifier::Action::Read,
                &auth::deal_resource_attrs(deal),
            )
            .ok()?;
            let value = serde_json::json!(deal);
            Some(if obligations.iter().any(|o| o == "mask") {
                auth::mask_json(value, &["amount_minor"])
            } else {
                value
            })
        })
        .collect();
    Audit::record_read(
        &ctx.db,
        "account",
        account.pid,
        "account_read",
        caller.actor(),
        None,
    )
    .await?;
    format::json(serde_json::json!({
        "account": account, "contacts": contacts_json, "deals": deals_json,
    }))
}

/// `POST /api/activities` — log an interaction against any
/// relationship object (CRM-R2). Stamps the ticket first-response
/// when an assignee's outbound call/email hits an open ticket.
#[debug_handler]
async fn create_activity(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ActivityPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token(
        "subject_kind",
        tokens::ACTIVITY_SUBJECTS,
        &payload.subject_kind,
    );
    problems.token_opt(
        "sentiment",
        crate::rules::engagement::SENTIMENTS,
        payload.sentiment.as_deref(),
    );
    problems.require_token("kind", tokens::ACTIVITY_KINDS, &payload.kind);
    problems.require_text("summary", &payload.summary);
    problems.ref_opt(
        "actor_ref",
        entity_ref::EntityType::Worker,
        payload.actor_ref.as_deref(),
    );
    problems.finish()?;
    let occurred_at = payload
        .occurred_at
        .unwrap_or_else(|| chrono::Utc::now().into());
    let txn = ctx.db.begin().await?;
    let row = activities::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set(payload.subject_kind.clone()),
        sentiment: ActiveValue::set(payload.sentiment.clone()),
        subject_pid: ActiveValue::set(payload.subject_pid),
        kind: ActiveValue::set(payload.kind.clone()),
        occurred_at: ActiveValue::set(occurred_at),
        actor_ref: ActiveValue::set(payload.actor_ref.clone()),
        summary: ActiveValue::set(payload.summary.clone().into()),
        due_on: ActiveValue::set(payload.due_on),
        done: ActiveValue::set(false),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    // First-response stamping (CRM-R10): the assignee's first
    // outbound call/email on an open/pending ticket.
    if payload.subject_kind == "ticket"
        && (payload.kind == "call" || payload.kind == "email")
        && let Ok(ticket) = records::find_ticket(&txn, payload.subject_pid).await
    {
        {
            let is_assignee =
                ticket.assignee_ref.is_some() && ticket.assignee_ref == payload.actor_ref;
            if is_assignee && ticket.first_responded_at.is_none() && ticket.status != "closed" {
                let ticket_pid = ticket.pid;
                let mut active: tickets::ActiveModel = ticket.into();
                active.first_responded_at = ActiveValue::set(Some(occurred_at));
                active.update(&txn).await?;
                streaming::emit_on(
                    &txn,
                    "ticket",
                    "ticket_first_response",
                    &ticket_pid.to_string(),
                    "",
                    caller.actor(),
                    None,
                )
                .await?;
            }
        }
    }
    Audit::record(&txn, "activity", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/activities?subject_kind=&subject_pid=` — one object's
/// activities, or the recent feed with no filter (CRM-R14).
#[derive(Debug, Deserialize)]
struct ActivityListParams {
    #[serde(default)]
    subject_kind: Option<String>,
    #[serde(default)]
    subject_pid: Option<Uuid>,
}

#[debug_handler]
async fn list_activities(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(params): Query<ActivityListParams>,
    Query(paging): Query<PageParams>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 200)?;
    let mut query = activities::Entity::find_live();
    if let Some(kind) = &params.subject_kind {
        query = query.filter(activities::Column::SubjectKind.eq(kind));
    }
    if let Some(pid) = params.subject_pid {
        query = query.filter(activities::Column::SubjectPid.eq(pid));
    }
    let rows = page
        .apply(
            query
                .order_by_desc(activities::Column::OccurredAt)
                .order_by_desc(activities::Column::Id),
        )
        .all(&ctx.db)
        .await?;
    let (rows, more) = page.split(rows);
    Audit::record_read(
        &ctx.db,
        "activity",
        Uuid::nil(),
        "activity_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    page.respond(rows, more, &uri)
}

/// `PUT /api/activities/{pid}/done` — tick a task activity.
#[debug_handler]
async fn activity_done(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let row = activities::Entity::find()
        .filter(activities::Column::Pid.eq(pid))
        .filter(activities::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    if row.kind != "task" {
        return Err(unprocessable("only task activities can be marked done"));
    }
    let txn = ctx.db.begin().await?;
    let mut active: activities::ActiveModel = row.into();
    active.done = ActiveValue::set(true);
    let row = active.update(&txn).await?;
    Audit::record(&txn, "activity", row.pid, "updated", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// The relationship routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts", post(create_contact))
        .add("/contacts", get(list_contacts))
        .add("/contacts/{pid}", get(get_contact))
        .add("/contacts/{pid}/duplicates", get(contact_duplicates))
        .add("/contacts/{pid}/repoint", post(repoint_contact))
        .add("/accounts", post(create_account))
        .add("/accounts", get(list_accounts))
        .add("/accounts/{pid}", get(get_account))
        .add("/activities", post(create_activity))
        .add("/activities", get(list_activities))
        .add("/activities/{pid}/done", put(activity_done))
}
