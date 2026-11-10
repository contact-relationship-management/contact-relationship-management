//! Sales automation (CRM-R3–R5): leads with deterministic scoring +
//! conversion, pipelines + stages, deals with Kanban stage moves and
//! terminal closes, and the stage-weighted forecast.

use crate::controllers::PidPath;
use crate::db;
use crate::json::Json;
use crate::models::DateTimeWithTimeZone;
use crate::models::encrypted::EncryptedText;
use crate::models::rows;
use axa::extract::Query;
use axa::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::paging::Page;
use super::{PidRef, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::metrics::Metrics;
use crate::models::_entities::{deals, forecast_snapshots, leads, pipeline_stages, pipelines};
use crate::models::audit_logs::Model as Audit;
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
    /// The amount in minor units of `currency` (0 places for JPY, 2 for EUR, 3 for KWD).
    amount_minor: i64,
    /// An ISO 4217 code (CRM-T168): required, never assumed.
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
async fn score_lead<'e>(
    db: impl sqlx::PgExecutor<'e>,
    lead: &leads::Model,
) -> Result<scoring::ScoreBreakdown> {
    let activity = sqlx::query!(
        "SELECT COUNT(*) AS \"n!\", MAX(occurred_at) AS last FROM activities \
         WHERE subject_kind = 'lead' AND subject_pid = $1 AND deleted_at IS NULL",
        lead.pid,
    )
    .fetch_one(db)
    .await?;
    let now = crate::clock::now();
    let days_since_last = activity.last.map(|at| (now - at).num_days());
    let facts = scoring::LeadFacts {
        source: lead.source.clone(),
        campaign_attributed: lead.campaign_pid.is_some(),
        known_contact: lead.contact_pid.is_some(),
        email_domain: lead.email_domain.clone(),
        days_since_last_activity: days_since_last,
        activity_count: activity.n,
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
/// caller's transaction. Shared by `POST /api/leads`, the web form (CRM-R31) and the lead import.
pub(crate) async fn capture_lead(
    txn: &mut PgConnection,
    new: &NewLead<'_>,
    actor: Option<&str>,
) -> Result<(leads::Model, scoring::ScoreBreakdown)> {
    let row = sqlx::query_as!(
        leads::Model,
        "INSERT INTO leads (pid, source, campaign_pid, contact_pid, display_name, email, \
            email_domain, score, campaign_click, unsubscribed, status) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, 0, false, false, 'new') RETURNING created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
            email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\", board_position, owner_ref",
        Uuid::new_v4(),
        new.source,
        new.campaign_pid,
        new.contact_pid,
        new.display_name,
        new.email,
        new.email_domain,
    )
    .fetch_one(&mut *txn)
    .await?;
    let breakdown = score_lead(&mut *txn, &row).await?;
    let mut row = sqlx::query_as!(
        leads::Model,
        "UPDATE leads SET score = $2 WHERE id = $1 RETURNING created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
            email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\", board_position, owner_ref",
        row.id,
        breakdown.score,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(txn, "lead", row.pid, "lead_captured", actor, None).await?;
    streaming::emit_in(
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
        // The workflow engine takes a `ConnectionTrait`; `Bridge` runs it in this transaction.
        crate::workflows::fire(txn, &event, 0, actor).await?;
        // An action may have changed the lead (assigned an owner).
        row = rows::lead(&mut *txn, row.pid)
            .await?
            .ok_or(Error::NotFound)?;
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
    let pool = db::pool(&ctx);
    if let Some(contact) = payload.contact_pid {
        rows::contact(pool, contact).await?.ok_or(Error::NotFound)?;
    }
    let email_domain = payload
        .email
        .as_deref()
        .and_then(|e| e.rsplit_once('@'))
        .map(|(_, domain)| domain.to_ascii_lowercase());
    let mut txn = pool.begin().await?;
    let (row, breakdown) = capture_lead(
        &mut txn,
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
    Query(tag): Query<super::tags::TagFilter>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&uri, 1000)?;
    let (limit, offset) = page.bounds();
    let pool = db::pool(&ctx);
    let tagged: Option<Vec<Uuid>> = super::tags::filter_pids(&ctx.db, "lead", &tag)
        .await?
        .map(|pids| pids.into_iter().collect());
    let rows = rows::leads_page(pool, tagged.as_deref(), limit, offset).await?;
    let (rows, more) = page.split(rows);
    Audit::record_read_on(
        pool,
        "lead",
        Uuid::nil(),
        "lead_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    page.respond(rows, more)
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
    /// With `convert_to`: the currency of `converted_minor` (CRM-R53).
    convert_to: Option<String>,
    /// With `convert_to`: every currency's total converted and added, in minor units of
    /// `convert_to`; what had no rate is left out (see `unconverted`). `null` when masked.
    converted_minor: Option<i64>,
    /// With `convert_to`: the day whose exchange rates were used.
    rate_date: Option<chrono::NaiveDate>,
    /// With `convert_to`: the rate used for each currency that was converted.
    rates: Vec<super::fx_rates::RateUsed>,
    /// With `convert_to`: totals that had no rate in force, left out and never taken at 1.0.
    unconverted: Vec<super::fx_rates::Unconverted>,
}

/// What `GET /api/forecast` may be asked.
#[derive(Debug, Deserialize)]
struct ForecastQuery {
    /// Also show the forecast in this currency, an ISO 4217 code (CRM-R53, CRM-T168).
    convert_to: Option<String>,
    /// The day whose exchange rates are used (today when absent); only with `convert_to`.
    rate_date: Option<chrono::NaiveDate>,
}

/// `GET /api/leads/{pid}` — the lead + its live score breakdown.
#[debug_handler]
async fn get_lead(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let lead = rows::lead(pool, pid).await?.ok_or(Error::NotFound)?;
    let breakdown = score_lead(pool, &lead).await?;
    Audit::record_read_on(pool, "lead", lead.pid, "lead_read", caller.actor(), None).await?;
    format::json(LeadDetail {
        lead,
        score: breakdown,
    })
}

/// On conversion: the contact the lead becomes. A lead already linked to a
/// contact keeps it; otherwise a new, never-consented contact is created from
/// the lead and the `person_ref` the caller supplied.
async fn contact_for_conversion(
    txn: &mut PgConnection,
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
    // The lead's owner carries over to the contact it becomes. `preferred_channel` is sealed:
    // binding `EncryptedText` seals it (CRM-T41).
    let pid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO contacts (pid, person_ref, owner_ref, display_name, status, \
         preferred_channel, marketing_consent) VALUES ($1, $2, $3, $4, 'active', $5, 'never')",
        pid,
        person_ref,
        lead.owner_ref,
        lead.display_name,
        EncryptedText::from("email") as _,
    )
    .execute(&mut *txn)
    .await?;
    Ok(pid)
}

/// On conversion: open the deal in the first stage of its pipeline, with the
/// converted contact as primary contact and, unless the request names one, the
/// lead's campaign as its source.
async fn open_deal_for_conversion(
    txn: &mut PgConnection,
    caller: &MaybeAuthUser,
    lead: &leads::Model,
    contact_pid: Uuid,
    deal: &DealPayload,
) -> Result<Uuid> {
    let pipeline = rows::pipeline(&mut *txn, deal.pipeline_pid)
        .await?
        .ok_or(Error::NotFound)?;
    let first_stage = rows::first_stage_of(&mut *txn, pipeline.pid)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no stages"))?;
    if deal.amount_minor < 0 {
        return Err(unprocessable("amount_minor must be non-negative"));
    }
    let row = insert_deal(
        &mut *txn,
        deal,
        Some(contact_pid),
        (pipeline.pid, first_stage.pid),
        deal.source_campaign_pid.or(lead.campaign_pid),
    )
    .await?;
    streaming::emit_in(
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

/// A new open deal in `stage` of `pipeline`, at the top of its column.
async fn insert_deal<'e>(
    db: impl sqlx::PgExecutor<'e>,
    deal: &DealPayload,
    primary_contact: Option<Uuid>,
    (pipeline, stage): (Uuid, Uuid),
    source_campaign: Option<Uuid>,
) -> Result<deals::Model> {
    Ok(sqlx::query_as!(
        deals::Model,
        "INSERT INTO deals (pid, account_pid, primary_contact_pid, owner_ref, pipeline_pid, \
            stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
            source_campaign_pid, won) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0, $11, false) RETURNING created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
            pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
            source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at",
        Uuid::new_v4(),
        deal.account_pid,
        primary_contact,
        deal.owner_ref,
        pipeline,
        stage,
        deal.name,
        deal.amount_minor,
        deal.currency,
        deal.expected_close_on,
        source_campaign,
    )
    .fetch_one(db)
    .await?)
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
    if let Some(deal) = payload.deal.as_ref() {
        problems.currency("deal.currency", &deal.currency);
    }
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let lead = rows::lead_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    lifecycle::check("lead", lifecycle::LEAD, &lead.status, &payload.to)
        .map_err(|e| unprocessable(&e.to_string()))?;
    let from = lead.status.clone();
    let converting = payload.to == "converted";
    let mut contact_pid = lead.contact_pid;
    let mut deal_pid = None;
    if converting {
        let contact = contact_for_conversion(&mut txn, &lead, payload.person_ref.as_ref()).await?;
        contact_pid = Some(contact);
        if let Some(deal) = &payload.deal {
            deal_pid =
                Some(open_deal_for_conversion(&mut txn, &caller, &lead, contact, deal).await?);
        }
    }
    let row = sqlx::query_as!(
        leads::Model,
        "UPDATE leads SET status = $2, contact_pid = $3 WHERE id = $1 RETURNING created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
            email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\", board_position, owner_ref",
        lead.id,
        payload.to,
        contact_pid,
    )
    .fetch_one(&mut *txn)
    .await?;
    let kind = if converting {
        "lead_converted"
    } else {
        "lead_status_changed"
    };
    Audit::record_on(
        &mut txn,
        "lead",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    streaming::emit_in(
        &mut txn,
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
    let pipeline = Uuid::new_v4();
    let mut txn = db::pool(&ctx).begin().await?;
    sqlx::query!(
        "INSERT INTO pipelines (pid, name) VALUES ($1, $2)",
        pipeline,
        payload.name,
    )
    .execute(&mut *txn)
    .await?;
    let mut stage_pids = Vec::with_capacity(payload.stages.len());
    for (position, stage) in payload.stages.iter().enumerate() {
        let pid = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO pipeline_stages (pid, pipeline_pid, name, position, \
             probability_percent, is_won, is_lost) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            pid,
            pipeline,
            stage.name,
            i32::try_from(position).unwrap_or(i32::MAX),
            stage.probability_percent,
            stage.is_won,
            stage.is_lost,
        )
        .execute(&mut *txn)
        .await?;
        stage_pids.push(pid.to_string());
    }
    Audit::record_on(
        &mut txn,
        "pipeline",
        pipeline,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pid": pipeline, "stage_pids": stage_pids }))
}

/// `GET /api/pipelines` — pipelines with their stages.
#[debug_handler]
async fn list_pipelines(State(ctx): State<AppContext>) -> Result<Response> {
    let pool = db::pool(&ctx);
    let rows = rows::pipelines(pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for pipeline in rows {
        let stages = rows::stages_of(pool, pipeline.pid).await?;
        out.push(PipelineWithStages { pipeline, stages });
    }
    format::json(out)
}

/// `POST /api/deals` — open a deal in its pipeline's first stage.
#[debug_handler]
async fn create_deal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<DealPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.currency("currency", &payload.currency);
    payload.owner_ref = crate::rules::user_ref::normalise_opt(payload.owner_ref.take());
    problems.user_opt("owner_ref", payload.owner_ref.as_deref());
    if payload.amount_minor < 0 {
        problems.push("amount_minor must be non-negative".to_string());
    }
    problems.finish()?;
    let pool = db::pool(&ctx);
    let pipeline = rows::pipeline(pool, payload.pipeline_pid)
        .await?
        .ok_or(Error::NotFound)?;
    let first_stage = rows::first_stage_of(pool, pipeline.pid)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no stages"))?;
    if let Some(account) = payload.account_pid {
        rows::account(pool, account).await?.ok_or(Error::NotFound)?;
    }
    let mut txn = pool.begin().await?;
    let row = insert_deal(
        &mut *txn,
        &payload,
        payload.primary_contact_pid,
        (pipeline.pid, first_stage.pid),
        payload.source_campaign_pid,
    )
    .await?;
    Audit::record_on(
        &mut txn,
        "deal",
        row.pid,
        "deal_opened",
        caller.actor(),
        None,
    )
    .await?;
    streaming::emit_in(
        &mut txn,
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

/// One deal as `GET /api/deals` lists it: the record, and whether it is stalled (CRM-R4,
/// CRM-D20). The amount is `null` when masked.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ListedDeal")]
pub(crate) struct ListedDeal {
    #[serde(flatten)]
    deal: deals::Model,
    /// Open, with no stage move and no activity on the deal for the organization's
    /// `stalled_after_days` or more. Derived on every read, never stored.
    stalled: bool,
    /// The later of the last stage move and the last activity on the deal that has happened.
    last_touched_at: chrono::DateTime<chrono::Utc>,
}

/// The newest `occurred_at` that has happened of the live activities on each of `pids`.
async fn last_activities(
    pool: &PgPool,
    pids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, chrono::DateTime<chrono::Utc>>> {
    if pids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    Ok(sqlx::query!(
        "SELECT subject_pid, MAX(occurred_at) AS \"at!\" FROM activities \
         WHERE subject_kind = 'deal' AND deleted_at IS NULL AND occurred_at <= $1 \
         AND subject_pid = ANY($2) GROUP BY subject_pid",
        crate::clock::now(),
        pids,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| (row.subject_pid, row.at))
    .collect())
}

/// The deals with their stalled flag, at `now`.
async fn listed(ctx: &AppContext, rows: Vec<deals::Model>) -> Result<Vec<ListedDeal>> {
    use crate::rules::stalled;
    let days = super::organization::stalled_after_days(crate::db::pool(ctx)).await?;
    let pids: Vec<Uuid> = rows.iter().map(|d| d.pid).collect();
    let activity = last_activities(db::pool(ctx), &pids).await?;
    let now = crate::clock::now();
    Ok(rows
        .into_iter()
        .map(|deal| {
            let touched = stalled::last_touch(
                deal.stage_moved_at.to_utc(),
                activity.get(&deal.pid).copied(),
                now,
            );
            ListedDeal {
                stalled: stalled::is_stalled(deal.closed_at.is_some(), touched, now, days),
                last_touched_at: touched,
                deal,
            }
        })
        .collect())
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
    Query(tag): Query<super::tags::TagFilter>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&uri, 2000)?;
    let (limit, offset) = page.bounds();
    let pool = db::pool(&ctx);
    let tagged: Option<Vec<Uuid>> = super::tags::filter_pids(&ctx.db, "deal", &tag)
        .await?
        .map(|pids| pids.into_iter().collect());
    let rows = rows::deals_page(pool, params.pipeline, tagged.as_deref(), limit, offset).await?;
    let (rows, more) = page.split(rows);
    Audit::record_read_on(
        pool,
        "deal",
        Uuid::nil(),
        "deal_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    let rows = listed(&ctx, rows).await?;
    page.respond(readable_deals(&rows, &caller, tagged.is_some()), more)
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
    rows: &[ListedDeal],
    caller: &MaybeAuthUser,
    drop_masked: bool,
) -> Vec<serde_json::Value> {
    rows.iter()
        .filter_map(|row| {
            let obligations = auth::authorize_record(
                caller,
                authentication_verifier::Action::Read,
                &auth::deal_resource_attrs(&row.deal),
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
    let mut txn = db::pool(&ctx).begin().await?;
    let deal = rows::deal_for_update(&mut *txn, pid)
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
    let stage = rows::stage(&mut *txn, payload.stage_pid)
        .await?
        .ok_or(Error::NotFound)?;
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
    // A stage move is a touch (CRM-D20); a move into the stage it is in still counts.
    let now: DateTimeWithTimeZone = crate::clock::now().into();
    let terminal = stage.is_won || stage.is_lost;
    let row = sqlx::query_as!(
        deals::Model,
        "UPDATE deals SET stage_pid = $2, stage_moved_at = $3, \
            kanban_position = COALESCE($4, kanban_position), \
            closed_at = CASE WHEN $5 THEN $3 ELSE closed_at END, \
            won = CASE WHEN $5 THEN $6 ELSE won END, \
            lost_reason = CASE WHEN $5 THEN $7 ELSE lost_reason END \
            WHERE id = $1 RETURNING created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
            pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
            source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at",
        deal.id,
        stage.pid,
        now,
        payload.kanban_position,
        terminal,
        stage.is_won,
        payload.lost_reason,
    )
    .fetch_one(&mut *txn)
    .await?;
    let kind = if stage.is_won {
        "deal_won"
    } else if stage.is_lost {
        "deal_lost"
    } else {
        "deal_stage_changed"
    };
    Audit::record_on(
        &mut txn,
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
    streaming::emit_in(
        &mut txn,
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
        // The workflow engine takes a `ConnectionTrait`; `Bridge` runs it in this transaction.
        crate::workflows::fire(&mut txn, &event, 0, caller.actor()).await?;
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
    let mut txn = db::pool(&ctx).begin().await?;
    let deal = rows::deal_for_update(&mut *txn, pid)
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
    let target = rows::last_open_stage_of(&mut *txn, deal.pipeline_pid)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no open stage"))?;
    let now: DateTimeWithTimeZone = crate::clock::now().into();
    let row = sqlx::query_as!(
        deals::Model,
        "UPDATE deals SET stage_pid = $2, stage_moved_at = $3, closed_at = NULL, won = false, \
            lost_reason = NULL WHERE id = $1 RETURNING created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
            pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
            source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at",
        deal.id,
        target.pid,
        now,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
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

/// Each open deal with its stage's probability. A deal whose stage is gone is `404`, as before.
async fn forecast_inputs(pool: &PgPool, open: &[deals::Model]) -> Result<Vec<analytics::OpenDeal>> {
    let stages: std::collections::HashMap<Uuid, i32> = rows::pipeline_stages(pool)
        .await?
        .into_iter()
        .map(|stage| (stage.pid, stage.probability_percent))
        .collect();
    open.iter()
        .map(|deal| {
            Ok(analytics::OpenDeal {
                amount_minor: deal.amount_minor,
                currency: deal.currency.clone(),
                probability_percent: *stages.get(&deal.stage_pid).ok_or(Error::NotFound)?,
            })
        })
        .collect()
}

/// `GET /api/forecast` — the live stage-weighted forecast per
/// currency over open deals (CRM-R5); stamped `as_of`.
#[debug_handler]
async fn forecast(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    axa::extract::Query(query): axa::extract::Query<ForecastQuery>,
) -> Result<Response> {
    let mut problems = Problems::new();
    if let Some(target) = &query.convert_to {
        if let Err(why) = crate::rules::fx::currency(target) {
            problems.push_field("convert_to", why);
        }
    } else if query.rate_date.is_some() {
        problems.push_field("rate_date", "needs convert_to");
    }
    problems.finish()?;
    let masked = auth::aggregate_masked(&caller).map_err(record_rejection)?;
    let pool = db::pool(&ctx);
    let open = rows::open_deals(pool).await?;
    let inputs = forecast_inputs(pool, &open).await?;
    let totals = analytics::forecast_by_currency(&inputs).map_err(|e| unprocessable(&e))?;
    Metrics::global()
        .deals_open
        .set(i64::try_from(open.len()).unwrap_or(i64::MAX));
    // With `convert_to`, each currency's total is converted once and the results added.
    let mut conversion = None;
    if let (Some(target), false) = (&query.convert_to, masked) {
        let sums: Vec<(String, String, i64)> = totals
            .iter()
            .map(|(currency, total)| ("forecast".to_string(), currency.clone(), *total))
            .collect();
        conversion = Some(
            super::fx_rates::convert_sums(&ctx, target, query.rate_date, "rate_date", &sums)
                .await?,
        );
    }
    let (converted_minor, rate_date, rates, unconverted) = match conversion {
        Some(done) => (
            Some(done.rows.first().map_or(0, |row| row.1)),
            Some(done.rate_date),
            done.rates,
            done.unconverted,
        ),
        None => (None, None, Vec::new(), Vec::new()),
    };
    format::json(Forecast {
        convert_to: query.convert_to.clone(),
        converted_minor,
        rate_date,
        rates,
        unconverted,
        as_of: crate::clock::now(),
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
    let pool = db::pool(&ctx);
    let open = rows::open_deals(pool).await?;
    let inputs = forecast_inputs(pool, &open).await?;
    let totals = analytics::forecast_by_currency(&inputs).map_err(|e| unprocessable(&e))?;
    // The day in the organization's zone, not UTC's (CRM-T171).
    let taken_on = super::organization::organization_today(crate::db::pool(&ctx)).await?;
    let mut txn = pool.begin().await?;
    let row = sqlx::query_as!(
        forecast_snapshots::Model,
        "INSERT INTO forecast_snapshots (pid, taken_on, totals) VALUES ($1, $2, $3) RETURNING created_at, updated_at, id, pid, taken_on, totals, deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\"",
        Uuid::new_v4(),
        taken_on,
        serde_json::to_value(&totals).unwrap_or_default(),
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
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
pub(crate) async fn closed_counts(pool: &PgPool) -> Result<(u64, u64)> {
    let counts = sqlx::query!(
        "SELECT COUNT(*) FILTER (WHERE won) AS \"won!\", \
         COUNT(*) FILTER (WHERE closed_at IS NOT NULL AND NOT won) AS \"lost!\" \
         FROM deals WHERE deleted_at IS NULL",
    )
    .fetch_one(pool)
    .await?;
    Ok((
        u64::try_from(counts.won).unwrap_or(0),
        u64::try_from(counts.lost).unwrap_or(0),
    ))
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
