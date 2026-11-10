//! Campaigns: create, status, the send run and its worker, deliveries, suppression, funnel (CRM-R8).

use super::{
    AppContext, Audit, BackgroundWorker, Deserialize, Error, MaybeAuthUser, PidRef, Problems,
    Response, Result, State, StatusPayload, Uuid, analytics, async_trait, campaigns, clients,
    contacts, debug_handler, deliveries, evaluate_segment, format, lifecycle, mail, streaming,
    tokens, unprocessable,
};
use sqlx::PgConnection;

use crate::controllers::PidPath;
use crate::controllers::templates;
use crate::db::{self, Bridge};
use crate::json::Json;
use crate::models::DateTimeWithTimeZone;
use crate::models::encrypted::EncryptedText;
use crate::models::rows;

/// The live campaign `pid`, or `404`.
pub(super) async fn find_campaign<'e>(
    db: impl sqlx::PgExecutor<'e>,
    pid: Uuid,
) -> Result<campaigns::Model> {
    rows::campaign(db, pid).await?.ok_or(Error::NotFound)
}

/// `POST /api/campaigns` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CampaignPayload {
    name: String,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    cost_minor: i64,
    /// The ISO 4217 code the cost and the ROI are in (CRM-T168): required, never assumed.
    currency: String,
    #[serde(default)]
    segment_pid: Option<Uuid>,
    /// Message subject; the campaign name when absent.
    #[serde(default)]
    subject: Option<String>,
    /// Plain-text message body. Required before a campaign can really send.
    #[serde(default)]
    body: Option<String>,
    /// An email template (CRM-R36) to make the message from instead of `subject` and `body`.
    #[serde(default)]
    template_pid: Option<Uuid>,
    /// The consent purpose it is sent under (`marketing` when absent): only contacts who
    /// granted *that* purpose are sent to (CRM-R29).
    #[serde(default)]
    purpose: Option<String>,
}

/// Longest campaign message body accepted.
pub(super) const MAX_BODY_LEN: usize = 20_000;

pub(super) fn default_kind() -> String {
    "email".to_string()
}

/// `POST /api/campaigns`.
#[debug_handler]
pub(super) async fn create_campaign(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<CampaignPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.currency("currency", &payload.currency);
    problems.token_opt("purpose", tokens::PURPOSES, payload.purpose.as_deref());
    if payload.kind != "email" {
        problems.push("kind must be email (v1)".to_string());
    }
    if payload.cost_minor < 0 {
        problems.push("cost_minor must be non-negative".to_string());
    }
    if let Some(subject) = payload.subject.as_deref() {
        problems.cap_opt("subject", Some(subject));
        if subject.contains(['\r', '\n']) {
            problems.push("subject must be a single line".to_string());
        }
    }
    if payload
        .body
        .as_deref()
        .is_some_and(|body| body.len() > MAX_BODY_LEN)
    {
        problems.push(format!("body exceeds {MAX_BODY_LEN} characters"));
    }
    if payload.template_pid.is_some() && (payload.subject.is_some() || payload.body.is_some()) {
        problems.push_field(
            "template_pid",
            "cannot be given together with a subject or body",
        );
    }
    problems.finish()?;
    let pool = db::pool(&ctx);
    if let Some(segment) = payload.segment_pid {
        rows::segment(pool, segment).await?.ok_or(Error::NotFound)?;
    }
    if let Some(template) = payload.template_pid
        && !templates::exists(&ctx.db, template).await?
    {
        return Err(unprocessable("template_pid is not an email template"));
    }
    let pid = Uuid::new_v4();
    let mut txn = pool.begin().await?;
    sqlx::query!(
        "INSERT INTO campaigns (pid, kind, purpose, name, status, cost_minor, currency, \
         segment_pid, recipients, delivered, opened, clicked, unsubscribed, subject, body, \
         template_pid) VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7, 0, 0, 0, 0, 0, $8, $9, $10)",
        pid,
        payload.kind,
        payload
            .purpose
            .as_deref()
            .unwrap_or(tokens::DEFAULT_PURPOSE),
        payload.name,
        payload.cost_minor,
        payload.currency,
        payload.segment_pid,
        payload.subject,
        payload.body,
        payload.template_pid,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(&mut txn, "campaign", pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `GET /api/campaigns`.
#[debug_handler]
pub(super) async fn list_campaigns(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let masked =
        crate::auth::aggregate_masked(&caller).map_err(crate::controllers::record_rejection)?;
    let rows = rows::campaigns(db::pool(&ctx)).await?;
    if masked {
        let masked_rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| crate::auth::mask_json(serde_json::json!(row), &["cost_minor"]))
            .collect();
        return format::json(masked_rows);
    }
    format::json(rows)
}

/// `POST /api/campaigns/{pid}/status` — one lifecycle transition.
#[debug_handler]
pub(super) async fn campaign_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::CAMPAIGN_STATUSES, &payload.to);
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let campaign = rows::campaign_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    lifecycle::check(
        "campaign",
        lifecycle::CAMPAIGN,
        &campaign.status,
        &payload.to,
    )
    .map_err(|e| unprocessable(&e.to_string()))?;
    let from = campaign.status.clone();
    let name = campaign.name.clone();
    let row = sqlx::query_as!(
        campaigns::Model,
        "UPDATE campaigns SET status = $2 WHERE id = $1 RETURNING created_at, updated_at, id, pid, kind, name, status, cost_minor, currency, \
            segment_pid, recipients, delivered, opened, clicked, unsubscribed, deleted_at AS \
            \"deleted_at: DateTimeWithTimeZone\", subject, body, purpose, template_pid, \
            consent_jurisdiction",
        campaign.id,
        payload.to,
    )
    .fetch_one(&mut *txn)
    .await?;
    let kind = match payload.to.as_str() {
        "running" => "campaign_started",
        "completed" => "campaign_completed",
        _ => "campaign_status_changed",
    };
    Audit::record_on(
        &mut txn,
        "campaign",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from })),
    )
    .await?;
    streaming::emit_in(
        &mut txn,
        "campaign",
        kind,
        &row.pid.to_string(),
        &name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// Contacts handled per `run` call when mail really leaves
/// (`CRM_MAIL_BATCH`, default 200): a send is network I/O inside a request, so
/// a long audience is walked in several calls, each resuming where the last
/// stopped.
pub(super) fn batch_size() -> i64 {
    crate::settings::current().mail_batch
}

/// What one `run` call did.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct RunOutcome {
    sent: u64,
    failed: u64,
    no_address: u64,
    suppressed: u64,
}

/// The body with the unsubscribe line every message ends with.
pub(super) fn with_unsubscribe(body: String, url: Option<&str>) -> String {
    match url {
        Some(url) => format!("{body}\n\n--\nTo stop receiving these emails: {url}\n"),
        None => body,
    }
}

/// The subject and text of a campaign's message for one contact: from its email template when it
/// has one (which carries its own unsubscribe link), else its own subject and body with the
/// generic unsubscribe footer. `None` when the template is gone.
async fn campaign_content(
    db: &impl sea_orm::ConnectionTrait,
    campaign: &campaigns::Model,
    contact: &contacts::Model,
    unsubscribe_url: Option<&str>,
) -> Result<Option<(String, String)>> {
    if let Some(template) = campaign.template_pid {
        return templates::content_for(db, template, contact, unsubscribe_url).await;
    }
    Ok(Some((
        campaign
            .subject
            .clone()
            .unwrap_or_else(|| campaign.name.clone()),
        with_unsubscribe(campaign.body.clone().unwrap_or_default(), unsubscribe_url),
    )))
}

/// Why a consented contact is still not mailed this time, if so.
pub(super) async fn skip_reason(
    conn: &mut PgConnection,
    contact_pid: Uuid,
    audience: &std::collections::HashSet<Uuid>,
) -> Result<Option<&'static str>> {
    if deliveries::is_suppressed(&mut *conn, contact_pid).await? {
        return Ok(Some("mailbox suppressed after a bounce or complaint"));
    }
    // The employment helpers are still on `SeaORM`; `Bridge` runs this one on `conn`.
    if crate::models::employments::is_employee(&Bridge::new(conn), contact_pid).await? {
        return Ok(Some("an employee, by the worker service: marketing paused"));
    }
    if !audience.contains(&contact_pid) {
        return Ok(Some("no longer in the segment"));
    }
    Ok(None)
}

/// Settle one claimed delivery: re-check consent against the live row,
/// resolve the address, hand the message to the sender, and record the result.
pub(super) async fn deliver_one(
    ctx: &AppContext,
    campaign: &campaigns::Model,
    sender: &mail::Sender,
    item: deliveries::Claimed,
    audience: &std::collections::HashSet<Uuid>,
    outcome: &mut RunOutcome,
) -> Result<()> {
    let pool = db::pool(ctx);
    let mut conn = pool.acquire().await?;
    // The audience was listed a moment ago and consent can change at any time,
    // so read the contact again immediately before sending (CRM-D6).
    let contact = rows::contact(&mut *conn, item.contact_pid).await?;
    let granted = match &contact {
        Some(c) => crate::consents::state_of(&mut *conn, c, &campaign.purpose).await? == "granted",
        None => false,
    };
    let Some(contact) = contact.filter(|_| granted) else {
        deliveries::settle(
            &mut *conn,
            item.id,
            "suppressed",
            None,
            Some("consent not granted at send time"),
        )
        .await?;
        outcome.suppressed += 1;
        return Ok(());
    };
    if let Some(reason) = skip_reason(&mut conn, contact.pid, audience).await? {
        deliveries::settle(&mut *conn, item.id, "suppressed", None, Some(reason)).await?;
        outcome.suppressed += 1;
        return Ok(());
    }
    let (message_id, provider_id) = if sender.needs_address() {
        let Some(address) = clients::email_address(&contact.person_ref).await else {
            deliveries::settle(
                &mut *conn,
                item.id,
                "no_address",
                None,
                Some("no address on file"),
            )
            .await?;
            outcome.no_address += 1;
            return Ok(());
        };
        let unsubscribe_url =
            crate::unsubscribe::current().map(|u| u.url(contact.pid, campaign.pid));
        let Some((subject, text)) =
            // On the connection this send holds: a second one from the pool would wait on it.
            campaign_content(
                &Bridge::new(&mut conn),
                campaign,
                &contact,
                unsubscribe_url.as_deref(),
            )
            .await?
        else {
            let why = "permanent: the email template is gone";
            deliveries::settle(&mut *conn, item.id, "failed", None, Some(why)).await?;
            outcome.failed += 1;
            return Ok(());
        };
        let message = mail::Message {
            to: address,
            to_name: contact.display_name.clone(),
            subject,
            text,
            unsubscribe_url,
            message_id: sender.new_message_id(),
        };
        match sender.send(&message).await {
            Ok(provider_id) => (message.message_id, provider_id),
            Err(error) => {
                deliveries::settle(&mut *conn, item.id, "failed", None, Some(&error.describe()))
                    .await?;
                outcome.failed += 1;
                return Ok(());
            }
        }
    } else {
        (sender.new_message_id(), None)
    };
    drop(conn);
    // The delivery row and the touch activity commit together.
    let now: DateTimeWithTimeZone = crate::clock::now().into();
    let mut txn = pool.begin().await?;
    deliveries::settle(&mut *txn, item.id, "sent", Some(&message_id), None).await?;
    if let Some(provider_id) = &provider_id {
        deliveries::note_provider_id(&mut *txn, item.id, provider_id).await?;
    }
    // `summary` is sealed: binding `EncryptedText` seals it (CRM-T41).
    sqlx::query!(
        "INSERT INTO activities (pid, subject_kind, subject_pid, kind, occurred_at, summary, \
         done) VALUES ($1, 'contact', $2, 'email', $3, $4, false)",
        Uuid::new_v4(),
        contact.pid,
        now,
        EncryptedText(format!("Campaign touch: {}", campaign.name)) as _,
    )
    .execute(&mut *txn)
    .await?;
    txn.commit().await?;
    outcome.sent += 1;
    Ok(())
}

/// Why a campaign cannot be run now, if so.
pub(super) fn check_runnable(campaign: &campaigns::Model, sender: &mail::Sender) -> Result<()> {
    if campaign.status != "scheduled" && campaign.status != "running" {
        return Err(unprocessable(&format!(
            "campaign is {} — schedule it first",
            campaign.status
        )));
    }
    if campaign.segment_pid.is_none() {
        return Err(unprocessable("campaign has no segment"));
    }
    if sender.needs_address()
        && campaign.template_pid.is_none()
        && campaign
            .body
            .as_deref()
            .is_none_or(|body| body.trim().is_empty())
    {
        return Err(unprocessable("campaign has no body to send"));
    }
    Ok(())
}

/// `POST /api/campaigns/{pid}/run` — send the campaign (CRM-R8, CRM-T39).
///
/// The default sender is **simulated**: the whole audience is handled inside
/// the request and the engagement counters are the deterministic demo values.
/// With a real sender (`CRM_MAIL_MODE=smtp`) the call checks the campaign,
/// marks it `running`, and hands the sending to [`CampaignSendWorker`] (a
/// background job when `workers.mode` is `BackgroundQueue`, inline otherwise),
/// answering `202` while it is still running. A re-run sends only what is not
/// yet sent, so a contact is never mailed twice, and resumes a campaign that
/// stalled on transient failures.
#[debug_handler]
pub(super) async fn run_campaign(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let campaign = find_campaign(pool, pid).await?;
    let sender = mail::current();
    check_runnable(&campaign, &sender)?;
    if !sender.needs_address() {
        let (row, _) = run_batch(&ctx, campaign.pid, caller.actor()).await?;
        return format::json(row);
    }
    CampaignSendWorker::perform_later(
        &ctx,
        CampaignSendArgs {
            campaign_pid: campaign.pid,
            actor: caller.actor().map(ToString::to_string),
        },
    )
    .await?;
    let row = find_campaign(pool, campaign.pid).await?;
    // Queued (still `scheduled`) or running: the work is not finished.
    let status = if row.status == "completed" {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::ACCEPTED
    };
    Ok(axum::response::IntoResponse::into_response((
        status,
        axum::Json(row),
    )))
}

/// The job that sends a campaign.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CampaignSendArgs {
    /// The campaign to send.
    pub campaign_pid: Uuid,
    /// Who asked, for the audit trail.
    pub actor: Option<String>,
}

/// Sends a campaign batch by batch until nothing more can be sent. A pass that
/// sends nothing (every remaining contact is failing) stops the job, leaving
/// the campaign `running`; a later `run` resumes it.
pub struct CampaignSendWorker {
    ctx: AppContext,
}

#[async_trait]
impl BackgroundWorker<CampaignSendArgs> for CampaignSendWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: CampaignSendArgs) -> Result<()> {
        loop {
            let (row, progressed) =
                run_batch(&self.ctx, args.campaign_pid, args.actor.as_deref()).await?;
            if row.status != "running" || progressed == 0 {
                return Ok(());
            }
        }
    }
}

/// One batch of a campaign: list the segment (consent is structural in the
/// evaluator, CRM-D6), record a delivery row per contact, claim a batch
/// atomically, and for each contact re-check consent, resolve the address and
/// send through the configured [`mail::Sender`]. Returns the campaign row and
/// how many messages this batch sent.
pub(super) async fn run_batch(
    ctx: &AppContext,
    campaign_pid: Uuid,
    actor: Option<&str>,
) -> Result<(campaigns::Model, u64)> {
    let pool = db::pool(ctx);
    let campaign = find_campaign(pool, campaign_pid).await?;
    let sender = mail::current();
    check_runnable(&campaign, &sender)?;
    let segment_pid = campaign
        .segment_pid
        .ok_or_else(|| unprocessable("campaign has no segment"))?;
    let segment = rows::segment(pool, segment_pid)
        .await?
        .ok_or(Error::NotFound)?;
    let audience = evaluate_segment(pool, &segment, &campaign.purpose).await?;
    let audience_pids: std::collections::HashSet<Uuid> = audience.iter().map(|c| c.pid).collect();
    let pids: Vec<Uuid> = audience.iter().map(|c| c.pid).collect();
    deliveries::ensure_rows(&mut *pool.acquire().await?, campaign.pid, &pids).await?;

    let campaign_pid = campaign.pid;
    let mut campaign = campaign;
    if campaign.status == "scheduled" {
        campaign = sqlx::query_as!(
            campaigns::Model,
            "UPDATE campaigns SET status = 'running' WHERE id = $1 RETURNING created_at, updated_at, id, pid, kind, name, status, cost_minor, currency, \
            segment_pid, recipients, delivered, opened, clicked, unsubscribed, deleted_at AS \
            \"deleted_at: DateTimeWithTimeZone\", subject, body, purpose, template_pid, \
            consent_jurisdiction",
            campaign.id,
        )
        .fetch_one(pool)
        .await?;
    }

    let limit = if sender.needs_address() {
        batch_size()
    } else {
        i64::from(i32::MAX)
    };
    let claimed = deliveries::claim(pool, campaign_pid, limit).await?;
    let mut outcome = RunOutcome::default();
    for item in claimed {
        deliver_one(ctx, &campaign, &sender, item, &audience_pids, &mut outcome).await?;
    }

    let remaining = deliveries::retriable(pool, campaign_pid).await?;
    let counts = deliveries::summary(pool, campaign_pid).await?;
    let sent_total = counts.get("sent").copied().unwrap_or(0);
    let recipients = i32::try_from(sent_total).unwrap_or(i32::MAX);
    if sender.needs_address() {
        let mut txn = pool.begin().await?;
        Audit::record_on(
            &mut txn,
            "campaign",
            campaign_pid,
            "campaign_run",
            actor,
            Some(serde_json::json!({
                "sent": outcome.sent, "failed": outcome.failed,
                "no_address": outcome.no_address, "suppressed": outcome.suppressed,
                "remaining": remaining,
            })),
        )
        .await?;
        txn.commit().await?;
    }
    let row = close_batch(pool, &campaign, recipients, remaining == 0, &sender, actor).await?;
    Ok((row, outcome.sent))
}

/// The closing update of one batch: the recipients so far and, when nothing is left to send, the
/// campaign completed, audited and announced.
async fn close_batch(
    pool: &sqlx::PgPool,
    campaign: &campaigns::Model,
    recipients: i32,
    done: bool,
    sender: &mail::Sender,
    actor: Option<&str>,
) -> Result<campaigns::Model> {
    let (campaign_pid, name) = (campaign.pid, &campaign.name);
    // Deterministic demo engagement: all delivered, 3/5 opened, 1/4 clicked, none unsubscribed by
    // the simulation. A real sender leaves these alone: SMTP acceptance is not delivery, and the
    // provider's events count them as they come, so they are never written back from this run.
    let simulated = done && sender.is_simulated();
    let mut txn = pool.begin().await?;
    let row = sqlx::query_as!(
        campaigns::Model,
        "UPDATE campaigns SET recipients = $2, \
            status = CASE WHEN $3 THEN 'completed' ELSE status END, \
            delivered = CASE WHEN $4 THEN $2 ELSE delivered END, \
            opened = CASE WHEN $4 THEN $5 ELSE opened END, \
            clicked = CASE WHEN $4 THEN $6 ELSE clicked END \
            WHERE id = $1 RETURNING created_at, updated_at, id, pid, kind, name, status, cost_minor, currency, \
            segment_pid, recipients, delivered, opened, clicked, unsubscribed, deleted_at AS \
            \"deleted_at: DateTimeWithTimeZone\", subject, body, purpose, template_pid, \
            consent_jurisdiction",
        campaign.id,
        recipients,
        done,
        simulated,
        recipients * 3 / 5,
        recipients / 4,
    )
    .fetch_one(&mut *txn)
    .await?;
    if done {
        Audit::record_on(
            &mut txn,
            "campaign",
            campaign_pid,
            "campaign_completed",
            actor,
            Some(serde_json::json!({ "recipients": recipients })),
        )
        .await?;
        streaming::emit_in(
            &mut txn,
            "campaign",
            "campaign_completed",
            &campaign_pid.to_string(),
            name,
            actor,
            None,
        )
        .await?;
    }
    txn.commit().await?;
    Ok(row)
}

/// `GET /api/campaigns/{pid}/deliveries`: the send, counted by state.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct Deliveries {
    campaign_pid: Uuid,
    total: i64,
    /// Rows a further run could still send.
    remaining: i64,
    by_state: std::collections::BTreeMap<String, i64>,
}

/// `GET /api/campaigns/{pid}/funnel`: leads, won revenue and ROI. Revenue and ROI are `null` when masked.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct CampaignFunnel {
    /// The campaign; its cost is `null` when masked.
    #[schemars(with = "campaigns::Model")]
    campaign: serde_json::Value,
    leads: usize,
    won_deals: usize,
    /// Won revenue in the campaign's own currency.
    won_revenue_minor: Option<i64>,
    /// Won revenue in other currencies, never summed in.
    won_revenue_other_currencies: Option<std::collections::BTreeMap<String, i64>>,
    roi: Option<analytics::Ratio>,
    /// With `convert=true`: the day whose exchange rates were used (CRM-R52); `null` with
    /// `rate_basis`, where each deal has its own.
    rate_date: Option<chrono::NaiveDate>,
    /// `close_day` when each deal was converted at the rate of the day it closed (CRM-R86).
    rate_basis: Option<String>,
    /// With `convert=true`: the rate used for each currency that was converted.
    rates: Vec<crate::controllers::fx_rates::RateUsed>,
    /// With `convert=true`: revenue that had no rate in force and stays out of the figures.
    unconverted: Vec<crate::controllers::fx_rates::Unconverted>,
}

/// What `GET /api/campaigns/{pid}/funnel` may be asked.
#[derive(Debug, Deserialize)]
pub(super) struct FunnelQuery {
    /// Convert revenue won in other currencies into the campaign's currency (CRM-R52).
    convert: Option<bool>,
    /// The day whose exchange rates are used (today when absent); only with `convert=true`.
    rate_date: Option<chrono::NaiveDate>,
    /// `close_day`: convert each deal at the rate of the day it closed (CRM-R86); only with
    /// `convert=true`, and not with `rate_date`.
    rate_basis: Option<String>,
}

/// `GET /api/campaigns/{pid}/deliveries` — how a send is going: rows per state
/// and how many a further `run` could still send. Counts only; no addresses.
#[debug_handler]
pub(super) async fn campaign_deliveries(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let campaign = find_campaign(pool, pid).await?;
    let by_state = deliveries::summary(pool, campaign.pid).await?;
    let total: i64 = by_state.values().sum();
    let remaining = deliveries::retriable(pool, campaign.pid).await?;
    format::json(Deliveries {
        campaign_pid: campaign.pid,
        total,
        remaining,
        by_state,
    })
}

/// `GET /api/contacts/{pid}/email-suppression` — whether the contact's mailbox
/// is suppressed after a hard bounce or complaint, and why. Audited read.
#[debug_handler]
pub(super) async fn email_suppression(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let contact = rows::contact(pool, pid).await?.ok_or(Error::NotFound)?;
    let found = deliveries::suppression(pool, contact.pid).await?;
    Audit::record_read_on(
        pool,
        "contact",
        contact.pid,
        "email_suppression_read",
        caller.actor(),
        None,
    )
    .await?;
    format::json(serde_json::json!({
        "suppressed": found.is_some(),
        "reason": found.as_ref().map(|f| f.0.clone()),
        "since": found.map(|f| f.1),
    }))
}

/// `DELETE /api/contacts/{pid}/email-suppression` — lift a suppression, for
/// example after the person confirms a corrected address. It does not restore
/// marketing consent: a contact who complained stays withdrawn until they
/// grant it again. Audited; `404` when the contact is not suppressed.
#[debug_handler]
pub(super) async fn clear_email_suppression(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let contact = rows::contact(pool, pid).await?.ok_or(Error::NotFound)?;
    let mut txn = pool.begin().await?;
    let removed = deliveries::unsuppress(&mut *txn, contact.pid).await?;
    if !removed {
        return Err(Error::NotFound);
    }
    Audit::record_on(
        &mut txn,
        "contact",
        contact.pid,
        "email_suppression_cleared",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// `GET /api/campaigns/{pid}/funnel` — the funnel + ROI (CRM-R8):
/// recipients → delivered → opened → clicked → leads → won revenue;
/// ROI reports `null` on a zero cost, with absolutes alongside.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one handler: read, convert (two ways) and answer
pub(super) async fn campaign_funnel(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    axa::extract::Query(query): axa::extract::Query<FunnelQuery>,
) -> Result<Response> {
    let convert = query.convert.unwrap_or(false);
    let mut problems = Problems::new();
    if !convert && query.rate_date.is_some() {
        problems.push_field("rate_date", "needs convert=true");
    }
    if let Some(basis) = &query.rate_basis {
        if !convert {
            problems.push_field("rate_basis", "needs convert=true");
        } else if basis != "close_day" {
            problems.push_field("rate_basis", "only close_day");
        } else if query.rate_date.is_some() {
            problems.push_field(
                "rate_date",
                "not with rate_basis: each deal has its own day",
            );
        }
    }
    problems.finish()?;
    let masked =
        crate::auth::aggregate_masked(&caller).map_err(crate::controllers::record_rejection)?;
    let pool = db::pool(&ctx);
    let campaign = find_campaign(pool, pid).await?;
    let leads = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"n!\" FROM leads WHERE campaign_pid = $1 AND deleted_at IS NULL",
        campaign.pid,
    )
    .fetch_one(pool)
    .await?;
    let attributed_deals = rows::won_deals_of_campaign(pool, campaign.pid).await?;
    // Won revenue in the campaign's currency only (per-currency
    // honesty: other currencies are listed, never summed in).
    let mut same_currency_revenue: i64 = 0;
    let mut other_currency: std::collections::BTreeMap<String, i64> =
        std::collections::BTreeMap::new();
    for deal in &attributed_deals {
        if deal.currency.eq_ignore_ascii_case(&campaign.currency) {
            same_currency_revenue = same_currency_revenue.saturating_add(deal.amount_minor);
        } else {
            *other_currency.entry(deal.currency.clone()).or_insert(0) += deal.amount_minor;
        }
    }
    // With `convert=true`, revenue in other currencies is converted into the campaign's currency
    // (once per currency) and counts toward ROI; what has no rate stays listed apart.
    let mut conversion = None;
    let mut dated = None;
    if convert && query.rate_basis.is_some() && !other_currency.is_empty() {
        // Each deal at the rate of the day it closed, rounded once per deal (CRM-R86); what had
        // no rate on its day is all that stays listed apart. The day is the organization's
        // (CRM-T181).
        let calendar = crate::controllers::organization::periods(crate::db::pool(&ctx)).await?;
        let foreign: Vec<(chrono::NaiveDate, String, i64)> = attributed_deals
            .iter()
            .filter(|deal| !deal.currency.eq_ignore_ascii_case(&campaign.currency))
            .map(|deal| {
                (
                    crate::rules::periods::day_of(
                        deal.closed_at.unwrap_or(deal.updated_at).to_utc(),
                        &calendar,
                    ),
                    deal.currency.clone(),
                    deal.amount_minor,
                )
            })
            .collect();
        let (total, rates, unconverted) =
            crate::controllers::fx_rates::convert_dated(&ctx, &campaign.currency, &foreign).await?;
        same_currency_revenue = same_currency_revenue.saturating_add(total);
        other_currency = unconverted
            .iter()
            .map(|u| (u.currency.clone(), u.amount_minor))
            .collect();
        dated = Some((rates, unconverted));
    } else if convert && !other_currency.is_empty() {
        let sums: Vec<(String, String, i64)> = other_currency
            .iter()
            .map(|(currency, amount)| ("revenue".to_string(), currency.clone(), *amount))
            .collect();
        let done = crate::controllers::fx_rates::convert_sums(
            &ctx,
            &campaign.currency,
            query.rate_date,
            "rate_date",
            &sums,
        )
        .await?;
        let converted = done.rows.first().map_or(0, |row| row.1);
        same_currency_revenue = same_currency_revenue.saturating_add(converted);
        let left: std::collections::BTreeSet<&str> = done
            .unconverted
            .iter()
            .map(|u| u.currency.as_str())
            .collect();
        other_currency.retain(|currency, _| left.contains(currency.as_str()));
        conversion = Some(done);
    }
    let (rate_date, rates, unconverted) = match conversion {
        _ if query.rate_basis.is_some() => {
            let (rates, unconverted) = dated.unwrap_or_default();
            (None, rates, unconverted)
        }
        Some(done) => (Some(done.rate_date), done.rates, done.unconverted),
        None if convert => (
            Some(match query.rate_date {
                Some(day) => day,
                // The exchange-rate "today" is the organization's (CRM-T171, CRM-T179).
                None => {
                    crate::controllers::organization::organization_today(crate::db::pool(&ctx))
                        .await?
                }
            }),
            Vec::new(),
            Vec::new(),
        ),
        None => (None, Vec::new(), Vec::new()),
    };
    let roi = analytics::roi(same_currency_revenue, campaign.cost_minor);
    // Cost, revenue and ROI are commercial and masked; counts and structure stay.
    let mut campaign = serde_json::json!(campaign);
    if masked {
        campaign = crate::auth::mask_json(campaign, &["cost_minor"]);
    }
    format::json(CampaignFunnel {
        campaign,
        leads: usize::try_from(leads).unwrap_or(0),
        won_deals: attributed_deals.len(),
        won_revenue_minor: (!masked).then_some(same_currency_revenue),
        won_revenue_other_currencies: (!masked).then_some(other_currency),
        roi: (!masked).then_some(roi),
        rate_date,
        rate_basis: query.rate_basis.clone(),
        rates: if masked { Vec::new() } else { rates },
        unconverted: if masked { Vec::new() } else { unconverted },
    })
}
