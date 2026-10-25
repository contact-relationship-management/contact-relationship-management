//! Sales automation (CRM-R3–R5): leads with deterministic scoring +
//! conversion, pipelines + stages, deals with Kanban stage moves and
//! terminal closes, and the stage-weighted forecast.

use crate::controllers::PidPath;
use crate::json::Json;
use crate::models::live::FindLive;
use loco_rs::prelude::*;
use sea_orm::DatabaseTransaction;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::paging::{Page, PageParams};
use super::{PidRef, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::metrics::Metrics;
use crate::models::_entities::{
    activities, contacts, deals, forecast_snapshots, leads, pipeline_stages, pipelines,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{analytics, lifecycle, scoring, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/leads` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadPayload {
    display_name: String,
    source: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    campaign_pid: Option<Uuid>,
    #[serde(default)]
    contact_pid: Option<Uuid>,
}

/// `POST /api/leads/{pid}/status` body. `converted` may open a deal.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadStatusPayload {
    to: String,
    /// On `converted`: the person URN for the new contact (when the
    /// lead is not yet linked to one).
    #[serde(default)]
    person_ref: Option<String>,
    /// On `converted`: optionally open a deal in this pipeline.
    #[serde(default)]
    deal: Option<DealPayload>,
}

/// `POST /api/pipelines` body: the named, ordered stage list.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PipelinePayload {
    name: String,
    stages: Vec<StagePayload>,
}

/// One stage in a pipeline payload.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "StageInput")]
pub(crate) struct StagePayload {
    name: String,
    probability_percent: i32,
    #[serde(default)]
    is_won: bool,
    #[serde(default)]
    is_lost: bool,
}

/// `POST /api/deals` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DealPayload {
    name: String,
    pipeline_pid: Uuid,
    amount_minor: i64,
    currency: String,
    #[serde(default)]
    account_pid: Option<Uuid>,
    #[serde(default)]
    primary_contact_pid: Option<Uuid>,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    expected_close_on: Option<chrono::NaiveDate>,
    #[serde(default)]
    source_campaign_pid: Option<Uuid>,
}

/// `POST /api/deals/{pid}/stage` body — the Kanban move.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DealStagePayload {
    stage_pid: Uuid,
    #[serde(default)]
    kanban_position: Option<i32>,
    /// Required when the target stage `is_lost`.
    #[serde(default)]
    lost_reason: Option<String>,
}

/// `POST /api/deals/{pid}/reopen` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReopenPayload {
    reason: String,
}

/// Derive the scoring facts for a lead from its rows (activities +
/// linkage), then run the pure scorer.
async fn score_lead<C: sea_orm::ConnectionTrait>(
    db: &C,
    lead: &leads::Model,
) -> Result<scoring::ScoreBreakdown> {
    let activity_rows = activities::Entity::find()
        .filter(activities::Column::SubjectKind.eq("lead"))
        .filter(activities::Column::SubjectPid.eq(lead.pid))
        .filter(activities::Column::DeletedAt.is_null())
        .order_by_desc(activities::Column::OccurredAt)
        .all(db)
        .await?;
    let now = chrono::Utc::now();
    let days_since_last = activity_rows
        .first()
        .map(|a| (now - a.occurred_at.with_timezone(&chrono::Utc)).num_days());
    let facts = scoring::LeadFacts {
        source: lead.source.clone(),
        campaign_attributed: lead.campaign_pid.is_some(),
        known_contact: lead.contact_pid.is_some(),
        email_domain: lead.email_domain.clone(),
        days_since_last_activity: days_since_last,
        activity_count: i64::try_from(activity_rows.len()).unwrap_or(i64::MAX),
        campaign_click: lead.campaign_click,
        unsubscribed: lead.unsubscribed,
    };
    Ok(scoring::score(&facts, &scoring::Weights::default()))
}

/// What a new lead is made from.
pub(crate) struct NewLead<'a> {
    pub source: &'a str,
    pub campaign_pid: Option<Uuid>,
    pub contact_pid: Option<Uuid>,
    pub display_name: &'a str,
    pub email: Option<&'a str>,
    pub email_domain: Option<String>,
}

/// Insert a lead, score it, and write the `lead_captured` audit row and event, all in the
/// caller's transaction. Shared by `POST /api/leads` and the web form (CRM-R31).
pub(crate) async fn capture_lead(
    txn: &sea_orm::DatabaseTransaction,
    new: &NewLead<'_>,
    actor: Option<&str>,
) -> Result<(leads::Model, scoring::ScoreBreakdown)> {
    let mut row = leads::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        source: ActiveValue::set(new.source.to_string()),
        campaign_pid: ActiveValue::set(new.campaign_pid),
        contact_pid: ActiveValue::set(new.contact_pid),
        display_name: ActiveValue::set(new.display_name.to_string()),
        email: ActiveValue::set(new.email.map(ToString::to_string)),
        email_domain: ActiveValue::set(new.email_domain.clone()),
        score: ActiveValue::set(0),
        campaign_click: ActiveValue::set(false),
        unsubscribed: ActiveValue::set(false),
        status: ActiveValue::set("new".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    let breakdown = score_lead(txn, &row).await?;
    let mut active: leads::ActiveModel = row.clone().into();
    active.score = ActiveValue::set(breakdown.score);
    row = active.update(txn).await?;
    Audit::record(txn, "lead", row.pid, "lead_captured", actor, None).await?;
    streaming::emit_on(
        txn,
        "lead",
        "lead_captured",
        &row.pid.to_string(),
        &row.display_name,
        actor,
        None,
    )
    .await?;
    Metrics::global().lead_captured_total.inc();
    // A lead brought in by a list import starts no workflow (CRM-R43), like imported contacts.
    if new.source != "import" {
        let event = crate::workflows::Event::lead(row.pid);
        crate::workflows::fire(txn, &event, 0, actor).await?;
        // An action may have changed the lead (assigned an owner).
        row = records::find_lead(txn, row.pid).await?;
    }
    Ok((row, breakdown))
}

/// `POST /api/leads` — capture (scored immediately).
#[debug_handler]
async fn create_lead(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<LeadPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("display_name", &payload.display_name);
    problems.require_token("source", tokens::LEAD_SOURCES, &payload.source);
    problems.cap_opt("email", payload.email.as_deref());
    problems.finish()?;
    if let Some(contact) = payload.contact_pid {
        records::find_contact(&ctx.db, contact).await?;
    }
    let email_domain = payload
        .email
        .as_deref()
        .and_then(|e| e.rsplit_once('@'))
        .map(|(_, domain)| domain.to_ascii_lowercase());
    let txn = ctx.db.begin().await?;
    let (row, breakdown) = capture_lead(
        &txn,
        &NewLead {
            source: &payload.source,
            campaign_pid: payload.campaign_pid,
            contact_pid: payload.contact_pid,
            display_name: &payload.display_name,
            email: payload.email.as_deref(),
            email_domain,
        },
        caller.actor(),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pid": row.pid, "score": breakdown }))
}

/// `GET /api/leads` — the queue, score-sorted (CRM-R3).
#[debug_handler]
async fn list_leads(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(paging): Query<PageParams>,
    Query(tag): Query<super::tags::TagFilter>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 1000)?;
    let mut query = leads::Entity::find_live()
        .order_by_asc(leads::Column::BoardPosition)
        .order_by_desc(leads::Column::Score)
        .order_by_asc(leads::Column::Id);
    if let Some(pids) = super::tags::filter_pids(&ctx.db, "lead", &tag).await? {
        query = query.filter(leads::Column::Pid.is_in(pids));
    }
    let rows = page.apply(query).all(&ctx.db).await?;
    let (rows, more) = page.split(rows);
    Audit::record_read(
        &ctx.db,
        "lead",
        Uuid::nil(),
        "lead_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    page.respond(rows, more, &uri)
}

/// `GET /api/leads/{pid}`: the lead and its live score breakdown.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct LeadDetail {
    lead: leads::Model,
    score: scoring::ScoreBreakdown,
}

/// One pipeline with its stages, in order.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Pipeline")]
pub(crate) struct PipelineWithStages {
    pipeline: pipelines::Model,
    stages: Vec<pipeline_stages::Model>,
}

/// `GET /api/forecast`: the stage-weighted forecast per currency over open deals.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct Forecast {
    as_of: chrono::DateTime<chrono::Utc>,
    open_deals: usize,
    /// Minor units per currency, never summed across currencies; `null` when masked.
    #[schemars(with = "Option<std::collections::BTreeMap<String, i64>>")]
    totals_minor: serde_json::Value,
}

/// `GET /api/leads/{pid}` — the lead + its live score breakdown.
#[debug_handler]
async fn get_lead(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let lead = records::find_lead(&ctx.db, pid).await?;
    let breakdown = score_lead(&ctx.db, &lead).await?;
    Audit::record_read(&ctx.db, "lead", lead.pid, "lead_read", caller.actor(), None).await?;
    format::json(LeadDetail {
        lead,
        score: breakdown,
    })
}

/// On conversion: the contact the lead becomes. A lead already linked to a
/// contact keeps it; otherwise a new, never-consented contact is created from
/// the lead and the `person_ref` the caller supplied.
async fn contact_for_conversion(
    txn: &DatabaseTransaction,
    lead: &leads::Model,
    person_ref: Option<&String>,
) -> Result<Uuid> {
    if let Some(linked) = lead.contact_pid {
        return Ok(linked);
    }
    let person_ref = person_ref
        .ok_or_else(|| unprocessable("conversion requires person_ref (no linked contact)"))?;
    let mut problems = Problems::new();
    problems.require_ref("person_ref", entity_ref::EntityType::Person, person_ref);
    problems.finish()?;
    let contact = contacts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(person_ref.clone()),
        account_pid: ActiveValue::set(None),
        // The lead's owner carries over to the contact it becomes.
        owner_ref: ActiveValue::set(lead.owner_ref.clone()),
        display_name: ActiveValue::set(lead.display_name.clone()),
        status: ActiveValue::set("active".to_string()),
        job_title: ActiveValue::set(None),
        given_name: ActiveValue::set(None),
        family_name: ActiveValue::set(None),
        preferred_channel: ActiveValue::set("email".into()),
        marketing_consent: ActiveValue::set("never".to_string()),
        consent_changed_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(contact.pid)
}

/// On conversion: open the deal in the first stage of its pipeline, with the
/// converted contact as primary contact and, unless the request names one, the
/// lead's campaign as its source.
async fn open_deal_for_conversion(
    txn: &DatabaseTransaction,
    caller: &MaybeAuthUser,
    lead: &leads::Model,
    contact_pid: Uuid,
    deal: &DealPayload,
) -> Result<Uuid> {
    let pipeline = records::find_pipeline(txn, deal.pipeline_pid).await?;
    let first_stage = pipeline_stages::Entity::find()
        .filter(pipeline_stages::Column::PipelinePid.eq(pipeline.pid))
        .filter(pipeline_stages::Column::DeletedAt.is_null())
        .order_by_asc(pipeline_stages::Column::Position)
        .one(txn)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no stages"))?;
    if deal.amount_minor < 0 {
        return Err(unprocessable("amount_minor must be non-negative"));
    }
    let row = deals::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        account_pid: ActiveValue::set(deal.account_pid),
        primary_contact_pid: ActiveValue::set(Some(contact_pid)),
        owner_ref: ActiveValue::set(deal.owner_ref.clone()),
        pipeline_pid: ActiveValue::set(pipeline.pid),
        stage_pid: ActiveValue::set(first_stage.pid),
        name: ActiveValue::set(deal.name.clone()),
        amount_minor: ActiveValue::set(deal.amount_minor),
        currency: ActiveValue::set(deal.currency.clone()),
        expected_close_on: ActiveValue::set(deal.expected_close_on),
        kanban_position: ActiveValue::set(0),
        source_campaign_pid: ActiveValue::set(deal.source_campaign_pid.or(lead.campaign_pid)),
        closed_at: ActiveValue::set(None),
        won: ActiveValue::set(false),
        lost_reason: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    streaming::emit_on(
        txn,
        "deal",
        "deal_opened",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    Ok(row.pid)
}

/// `POST /api/leads/{pid}/status` — one lifecycle transition;
/// `converted` creates/links the Contact (+ optional deal) in one
/// transaction (CRM-R3, CRM-D9).
#[debug_handler]
async fn lead_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<LeadStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::LEAD_STATUSES, &payload.to);
    problems.finish()?;
    let lead = records::find_lead(&ctx.db, pid).await?;
    lifecycle::check("lead", lifecycle::LEAD, &lead.status, &payload.to)
        .map_err(|e| unprocessable(&e.to_string()))?;
    let txn = ctx.db.begin().await?;
    let from = lead.status.clone();
    let converting = payload.to == "converted";
    let mut contact_pid = lead.contact_pid;
    let mut deal_pid = None;
    if converting {
        let contact = contact_for_conversion(&txn, &lead, payload.person_ref.as_ref()).await?;
        contact_pid = Some(contact);
        if let Some(deal) = &payload.deal {
            deal_pid = Some(open_deal_for_conversion(&txn, &caller, &lead, contact, deal).await?);
        }
    }
    let mut active: leads::ActiveModel = lead.into();
    active.status = ActiveValue::set(payload.to.clone());
    active.contact_pid = ActiveValue::set(contact_pid);
    let row = active.update(&txn).await?;
    let kind = if converting {
        "lead_converted"
    } else {
        "lead_status_changed"
    };
    Audit::record(
        &txn,
        "lead",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "lead",
        kind,
        &row.pid.to_string(),
        &row.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    if converting {
        Metrics::global().lead_converted_total.inc();
    }
    format::json(serde_json::json!({
        "pid": row.pid, "status": row.status,
        "contact_pid": contact_pid, "deal_pid": deal_pid,
    }))
}

/// `POST /api/pipelines` — create with its ordered stages.
#[debug_handler]
async fn create_pipeline(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PipelinePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if payload.stages.len() < 2 || payload.stages.len() > 20 {
        problems.push("a pipeline needs 2-20 stages".to_string());
    }
    for stage in &payload.stages {
        problems.require_text("stages[].name", &stage.name);
        if !(0..=100).contains(&stage.probability_percent) {
            problems.push(format!(
                "probability {} out of range 0-100",
                stage.probability_percent
            ));
        }
        if stage.is_won && stage.is_lost {
            problems.push("a stage cannot be both won and lost".to_string());
        }
    }
    problems.finish()?;
    let txn = ctx.db.begin().await?;
    let pipeline = pipelines::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let mut stage_pids = Vec::with_capacity(payload.stages.len());
    for (position, stage) in payload.stages.iter().enumerate() {
        let row = pipeline_stages::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            pipeline_pid: ActiveValue::set(pipeline.pid),
            name: ActiveValue::set(stage.name.clone()),
            position: ActiveValue::set(i32::try_from(position).unwrap_or(i32::MAX)),
            probability_percent: ActiveValue::set(stage.probability_percent),
            is_won: ActiveValue::set(stage.is_won),
            is_lost: ActiveValue::set(stage.is_lost),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        stage_pids.push(row.pid.to_string());
    }
    Audit::record(
        &txn,
        "pipeline",
        pipeline.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pid": pipeline.pid, "stage_pids": stage_pids }))
}

/// `GET /api/pipelines` — pipelines with their stages.
#[debug_handler]
async fn list_pipelines(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = pipelines::Entity::find_live()
        .order_by_asc(pipelines::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for pipeline in rows {
        let stages = pipeline_stages::Entity::find()
            .filter(pipeline_stages::Column::PipelinePid.eq(pipeline.pid))
            .filter(pipeline_stages::Column::DeletedAt.is_null())
            .order_by_asc(pipeline_stages::Column::Position)
            .all(&ctx.db)
            .await?;
        out.push(PipelineWithStages { pipeline, stages });
    }
    format::json(out)
}

/// `POST /api/deals` — open a deal in its pipeline's first stage.
#[debug_handler]
async fn create_deal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<DealPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.require_text("currency", &payload.currency);
    problems.ref_opt(
        "owner_ref",
        entity_ref::EntityType::Worker,
        payload.owner_ref.as_deref(),
    );
    if payload.amount_minor < 0 {
        problems.push("amount_minor must be non-negative".to_string());
    }
    problems.finish()?;
    let pipeline = records::find_pipeline(&ctx.db, payload.pipeline_pid).await?;
    let first_stage = pipeline_stages::Entity::find()
        .filter(pipeline_stages::Column::PipelinePid.eq(pipeline.pid))
        .filter(pipeline_stages::Column::DeletedAt.is_null())
        .order_by_asc(pipeline_stages::Column::Position)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no stages"))?;
    if let Some(account) = payload.account_pid {
        records::find_account(&ctx.db, account).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = deals::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        account_pid: ActiveValue::set(payload.account_pid),
        primary_contact_pid: ActiveValue::set(payload.primary_contact_pid),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        pipeline_pid: ActiveValue::set(pipeline.pid),
        stage_pid: ActiveValue::set(first_stage.pid),
        name: ActiveValue::set(payload.name.clone()),
        amount_minor: ActiveValue::set(payload.amount_minor),
        currency: ActiveValue::set(payload.currency.clone()),
        expected_close_on: ActiveValue::set(payload.expected_close_on),
        kanban_position: ActiveValue::set(0),
        source_campaign_pid: ActiveValue::set(payload.source_campaign_pid),
        closed_at: ActiveValue::set(None),
        won: ActiveValue::set(false),
        lost_reason: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "deal", row.pid, "deal_opened", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "deal",
        "deal_opened",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/deals?pipeline=<pid>` — the board rows (open + closed).
#[derive(Debug, Deserialize)]
struct DealListParams {
    #[serde(default)]
    pipeline: Option<Uuid>,
}

#[debug_handler]
async fn list_deals(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(params): Query<DealListParams>,
    Query(paging): Query<PageParams>,
    Query(tag): Query<super::tags::TagFilter>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 2000)?;
    let mut query = deals::Entity::find_live();
    if let Some(pipeline) = params.pipeline {
        query = query.filter(deals::Column::PipelinePid.eq(pipeline));
    }
    let tagged = super::tags::filter_pids(&ctx.db, "deal", &tag).await?;
    if let Some(pids) = &tagged {
        query = query.filter(deals::Column::Pid.is_in(pids.clone()));
    }
    let rows = page
        .apply(
            query
                .order_by_asc(deals::Column::KanbanPosition)
                .order_by_asc(deals::Column::Id),
        )
        .all(&ctx.db)
        .await?;
    let (rows, more) = page.split(rows);
    Audit::record_read(
        &ctx.db,
        "deal",
        Uuid::nil(),
        "deal_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    page.respond(readable_deals(&rows, &caller, tagged.is_some()), more, &uri)
}

/// Filter `rows` down to the ones `caller`'s record-level ABAC policy
/// allows to read, masking `amount_minor` on a row whose decision
/// carries the `mask` obligation. A denied row is **omitted** — with
/// enforcement on and a policy that denies read on some deals, a
/// collection read must not disclose more than the equivalent single
/// read would (CRM-T24; `agents/share/security.md` invariant 5, same
/// reasoning `relationships.rs`'s `readable_contacts` applies to
/// contacts). With enforcement off (the default) or no deny rule
/// configured, `authorize_record` never denies, so this is a no-op.
fn readable_deals(
    rows: &[deals::Model],
    caller: &MaybeAuthUser,
    drop_masked: bool,
) -> Vec<serde_json::Value> {
    rows.iter()
        .filter_map(|row| {
            let obligations = auth::authorize_record(
                caller,
                authentication_verifier::Action::Read,
                &auth::deal_resource_attrs(row),
            )
            .ok()?;
            let mut value = serde_json::json!(row);
            if obligations.iter().any(|o| o == "mask") {
                // Narrowing by tag must not show which masked records carry it.
                if drop_masked {
                    return None;
                }
                value = auth::mask_json(value, &["amount_minor"]);
            }
            Some(value)
        })
        .collect()
}

/// `POST /api/deals/{pid}/stage` — the Kanban stage move: validates
/// pipeline membership, serializes on the locked deal row, closes on
/// a terminal stage (lost requires a reason) (CRM-R4, CRM-D9).
#[debug_handler]
async fn deal_stage(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<DealStagePayload>,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let deal = deals::Entity::find()
        .filter(deals::Column::Pid.eq(pid))
        .filter(deals::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::deal_resource_attrs(&deal),
    )
    .map_err(record_rejection)?;
    if deal.closed_at.is_some() {
        return Err(unprocessable("deal is closed — reopen it first"));
    }
    let stage = records::find_stage(&txn, payload.stage_pid).await?;
    if stage.pipeline_pid != deal.pipeline_pid {
        return Err(unprocessable("stage belongs to a different pipeline"));
    }
    if stage.is_lost
        && payload
            .lost_reason
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(unprocessable("a lost close requires lost_reason"));
    }
    let from_stage = deal.stage_pid;
    let name = deal.name.clone();
    let mut active: deals::ActiveModel = deal.into();
    active.stage_pid = ActiveValue::set(stage.pid);
    if let Some(position) = payload.kanban_position {
        active.kanban_position = ActiveValue::set(position);
    }
    let terminal = stage.is_won || stage.is_lost;
    if terminal {
        active.closed_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        active.won = ActiveValue::set(stage.is_won);
        active.lost_reason = ActiveValue::set(payload.lost_reason.clone());
    }
    let row = active.update(&txn).await?;
    let kind = if stage.is_won {
        "deal_won"
    } else if stage.is_lost {
        "deal_lost"
    } else {
        "deal_stage_changed"
    };
    Audit::record(
        &txn,
        "deal",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({
            "from_stage": from_stage, "to_stage": stage.pid,
            "lost_reason": payload.lost_reason,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "deal",
        kind,
        &row.pid.to_string(),
        &name,
        caller.actor(),
        None,
    )
    .await?;
    if kind == "deal_stage_changed" {
        let event = crate::workflows::Event::deal_stage(row.pid, row.primary_contact_pid);
        crate::workflows::fire(&txn, &event, 0, caller.actor()).await?;
    }
    txn.commit().await?;
    match kind {
        "deal_won" => Metrics::global().deal_won_total.inc(),
        "deal_lost" => Metrics::global().deal_lost_total.inc(),
        _ => {}
    }
    format::json(row)
}

/// `POST /api/deals/{pid}/reopen` — reasoned reopen to the prior
/// (first non-terminal) stage (CRM-R4).
#[debug_handler]
async fn reopen_deal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ReopenPayload>,
) -> Result<Response> {
    if payload.reason.trim().is_empty() {
        return Err(unprocessable("a reopen requires a reason"));
    }
    let txn = ctx.db.begin().await?;
    let deal = deals::Entity::find()
        .filter(deals::Column::Pid.eq(pid))
        .filter(deals::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::deal_resource_attrs(&deal),
    )
    .map_err(record_rejection)?;
    if deal.closed_at.is_none() {
        return Err(unprocessable("deal is not closed"));
    }
    // Reopen into the pipeline's last non-terminal stage.
    let target = pipeline_stages::Entity::find()
        .filter(pipeline_stages::Column::PipelinePid.eq(deal.pipeline_pid))
        .filter(pipeline_stages::Column::DeletedAt.is_null())
        .filter(pipeline_stages::Column::IsWon.eq(false))
        .filter(pipeline_stages::Column::IsLost.eq(false))
        .order_by_desc(pipeline_stages::Column::Position)
        .one(&txn)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no open stage"))?;
    let mut active: deals::ActiveModel = deal.into();
    active.stage_pid = ActiveValue::set(target.pid);
    active.closed_at = ActiveValue::set(None);
    active.won = ActiveValue::set(false);
    active.lost_reason = ActiveValue::set(None);
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "deal",
        row.pid,
        "deal_reopened",
        caller.actor(),
        Some(serde_json::json!({ "reason": payload.reason })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/forecast` — the live stage-weighted forecast per
/// currency over open deals (CRM-R5); stamped `as_of`.
#[debug_handler]
async fn forecast(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let masked = auth::aggregate_masked(&caller).map_err(record_rejection)?;
    let open = deals::Entity::find_live()
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut inputs = Vec::with_capacity(open.len());
    for deal in &open {
        let stage = records::find_stage(&ctx.db, deal.stage_pid).await?;
        inputs.push(analytics::OpenDeal {
            amount_minor: deal.amount_minor,
            currency: deal.currency.clone(),
            probability_percent: stage.probability_percent,
        });
    }
    let totals = analytics::forecast_by_currency(&inputs).map_err(|e| unprocessable(&e))?;
    Metrics::global()
        .deals_open
        .set(i64::try_from(open.len()).unwrap_or(i64::MAX));
    format::json(Forecast {
        as_of: chrono::Utc::now(),
        open_deals: open.len(),
        totals_minor: if masked {
            serde_json::Value::Null
        } else {
            serde_json::json!(totals)
        },
    })
}

/// `POST /api/forecast/snapshot` — freeze the current roll-up
/// (CRM-R5: an output, never an input).
#[debug_handler]
async fn forecast_snapshot(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let masked = auth::aggregate_masked(&caller).map_err(record_rejection)?;
    let open = deals::Entity::find_live()
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut inputs = Vec::with_capacity(open.len());
    for deal in &open {
        let stage = records::find_stage(&ctx.db, deal.stage_pid).await?;
        inputs.push(analytics::OpenDeal {
            amount_minor: deal.amount_minor,
            currency: deal.currency.clone(),
            probability_percent: stage.probability_percent,
        });
    }
    let totals = analytics::forecast_by_currency(&inputs).map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let row = forecast_snapshots::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        taken_on: ActiveValue::set(chrono::Utc::now().date_naive()),
        totals: ActiveValue::set(serde_json::to_value(&totals).unwrap_or_default()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "forecast_snapshot",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    // The snapshot is stored in full; a masked caller's response omits the totals.
    if masked {
        return format::json(auth::mask_json(serde_json::json!(row), &["totals"]));
    }
    format::json(row)
}

/// Count helper for the win-rate dashboard (exposed here so sales
/// owns the deal queries).
pub(crate) async fn closed_counts(db: &DatabaseConnection) -> Result<(u64, u64)> {
    let won = deals::Entity::find_live()
        .filter(deals::Column::Won.eq(true))
        .count(db)
        .await?;
    let lost = deals::Entity::find_live()
        .filter(deals::Column::ClosedAt.is_not_null())
        .filter(deals::Column::Won.eq(false))
        .count(db)
        .await?;
    Ok((won, lost))
}

/// The sales routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/leads", post(create_lead))
        .add("/leads", get(list_leads))
        .add("/leads/{pid}", get(get_lead))
        .add("/leads/{pid}/status", post(lead_status))
        .add("/pipelines", post(create_pipeline))
        .add("/pipelines", get(list_pipelines))
        .add("/deals", post(create_deal))
        .add("/deals", get(list_deals))
        .add("/deals/{pid}/stage", post(deal_stage))
        .add("/deals/{pid}/reopen", post(reopen_deal))
        .add("/forecast", get(forecast))
        .add("/forecast/snapshot", post(forecast_snapshot))
}
