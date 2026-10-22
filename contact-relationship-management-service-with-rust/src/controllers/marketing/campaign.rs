//! Campaigns: create, status, the send run and its worker, deliveries, suppression, funnel (CRM-R8).

use super::{
    ActiveModelTrait, ActiveValue, AppContext, Audit, BackgroundWorker, ColumnTrait, Deserialize,
    EntityTrait, Error, MaybeAuthUser, PidRef, Problems, QueryFilter, QueryOrder, Response, Result,
    State, TransactionTrait, Uuid, activities, analytics, async_trait, campaigns, clients,
    contacts, deals, debug_handler, deliveries, ensure_valid, evaluate_segment, format, lifecycle,
    mail, records, streaming, tokens, unprocessable,
};
use crate::controllers::PidPath;
use crate::json::Json;
use crate::models::live::FindLive;

/// `POST /api/campaigns` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CampaignPayload {
    name: String,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    cost_minor: i64,
    #[serde(default = "default_currency")]
    currency: String,
    #[serde(default)]
    segment_pid: Option<Uuid>,
    /// Message subject; the campaign name when absent.
    #[serde(default)]
    subject: Option<String>,
    /// Plain-text message body. Required before a campaign can really send.
    #[serde(default)]
    body: Option<String>,
}

/// Longest campaign message body accepted.
pub(super) const MAX_BODY_LEN: usize = 20_000;

/// `POST /api/campaigns/{pid}/status` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CampaignStatusPayload {
    to: String,
}

pub(super) fn default_kind() -> String {
    "email".to_string()
}

pub(super) fn default_currency() -> String {
    "GBP".to_string()
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
    problems.require_text("currency", &payload.currency);
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
    ensure_valid(&problems.into_vec())?;
    if let Some(segment) = payload.segment_pid {
        records::find_segment(&ctx.db, segment).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = campaigns::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        kind: ActiveValue::set(payload.kind.clone()),
        name: ActiveValue::set(payload.name.clone()),
        status: ActiveValue::set("draft".to_string()),
        cost_minor: ActiveValue::set(payload.cost_minor),
        currency: ActiveValue::set(payload.currency.clone()),
        segment_pid: ActiveValue::set(payload.segment_pid),
        recipients: ActiveValue::set(0),
        delivered: ActiveValue::set(0),
        opened: ActiveValue::set(0),
        clicked: ActiveValue::set(0),
        unsubscribed: ActiveValue::set(0),
        subject: ActiveValue::set(payload.subject.clone()),
        body: ActiveValue::set(payload.body.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "campaign", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
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
    let rows = campaigns::Entity::find_live()
        .order_by_asc(campaigns::Column::Id)
        .all(&ctx.db)
        .await?;
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
    Json(payload): Json<CampaignStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::CAMPAIGN_STATUSES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let campaign = records::find_campaign(&ctx.db, pid).await?;
    lifecycle::check(
        "campaign",
        lifecycle::CAMPAIGN,
        &campaign.status,
        &payload.to,
    )
    .map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let from = campaign.status.clone();
    let name = campaign.name.clone();
    let mut active: campaigns::ActiveModel = campaign.into();
    active.status = ActiveValue::set(payload.to.clone());
    let row = active.update(&txn).await?;
    let kind = match payload.to.as_str() {
        "running" => "campaign_started",
        "completed" => "campaign_completed",
        _ => "campaign_status_changed",
    };
    Audit::record(
        &txn,
        "campaign",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from })),
    )
    .await?;
    streaming::emit_on(
        &txn,
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

/// Why a consented contact is still not mailed this time, if so.
pub(super) async fn skip_reason(
    db: &sea_orm::DatabaseConnection,
    contact_pid: Uuid,
    audience: &std::collections::HashSet<Uuid>,
) -> Result<Option<&'static str>> {
    if deliveries::is_suppressed(db, contact_pid).await? {
        return Ok(Some("mailbox suppressed after a bounce or complaint"));
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
    // The audience was listed a moment ago and consent can change at any time,
    // so read the contact again immediately before sending (CRM-D6).
    let contact = contacts::Entity::find()
        .filter(contacts::Column::Pid.eq(item.contact_pid))
        .filter(contacts::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?;
    let Some(contact) = contact.filter(|c| c.marketing_consent == "granted") else {
        deliveries::settle(
            &ctx.db,
            item.id,
            "suppressed",
            None,
            Some("consent not granted at send time"),
        )
        .await?;
        outcome.suppressed += 1;
        return Ok(());
    };
    if let Some(reason) = skip_reason(&ctx.db, contact.pid, audience).await? {
        deliveries::settle(&ctx.db, item.id, "suppressed", None, Some(reason)).await?;
        outcome.suppressed += 1;
        return Ok(());
    }
    let (message_id, provider_id) = if sender.needs_address() {
        let Some(address) = clients::email_address(&contact.person_ref).await else {
            deliveries::settle(
                &ctx.db,
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
        let message = mail::Message {
            to: address,
            to_name: contact.display_name.clone(),
            subject: campaign
                .subject
                .clone()
                .unwrap_or_else(|| campaign.name.clone()),
            text: with_unsubscribe(
                campaign.body.clone().unwrap_or_default(),
                unsubscribe_url.as_deref(),
            ),
            unsubscribe_url,
            message_id: sender.new_message_id(),
        };
        match sender.send(&message).await {
            Ok(provider_id) => (message.message_id, provider_id),
            Err(error) => {
                deliveries::settle(&ctx.db, item.id, "failed", None, Some(&error.describe()))
                    .await?;
                outcome.failed += 1;
                return Ok(());
            }
        }
    } else {
        (sender.new_message_id(), None)
    };
    // The delivery row and the touch activity commit together.
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let txn = ctx.db.begin().await?;
    deliveries::settle(&txn, item.id, "sent", Some(&message_id), None).await?;
    if let Some(provider_id) = &provider_id {
        deliveries::note_provider_id(&txn, item.id, provider_id).await?;
    }
    activities::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set("contact".to_string()),
        subject_pid: ActiveValue::set(contact.pid),
        kind: ActiveValue::set("email".to_string()),
        occurred_at: ActiveValue::set(now),
        actor_ref: ActiveValue::set(None),
        summary: ActiveValue::set(format!("Campaign touch: {}", campaign.name).into()),
        due_on: ActiveValue::set(None),
        done: ActiveValue::set(false),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
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
    let campaign = records::find_campaign(&ctx.db, pid).await?;
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
    let row = records::find_campaign(&ctx.db, campaign.pid).await?;
    // Queued (still `scheduled`) or running: the work is not finished.
    let status = if row.status == "completed" {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::ACCEPTED
    };
    format::render().status(status).json(row)
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
    let campaign = records::find_campaign(&ctx.db, campaign_pid).await?;
    let sender = mail::current();
    check_runnable(&campaign, &sender)?;
    let segment_pid = campaign
        .segment_pid
        .ok_or_else(|| unprocessable("campaign has no segment"))?;
    let segment = records::find_segment(&ctx.db, segment_pid).await?;
    let audience = evaluate_segment(&ctx.db, &segment).await?;
    let audience_pids: std::collections::HashSet<Uuid> = audience.iter().map(|c| c.pid).collect();
    let pids: Vec<Uuid> = audience.iter().map(|c| c.pid).collect();
    deliveries::ensure_rows(&ctx.db, campaign.pid, &pids).await?;

    let campaign_pid = campaign.pid;
    let name = campaign.name.clone();
    let was_scheduled = campaign.status == "scheduled";
    let mut campaign = campaign;
    if was_scheduled {
        let mut active: campaigns::ActiveModel = campaign.clone().into();
        active.status = ActiveValue::set("running".to_string());
        campaign = active.update(&ctx.db).await?;
    }

    let limit = if sender.needs_address() {
        batch_size()
    } else {
        i64::from(i32::MAX)
    };
    let claimed = deliveries::claim(&ctx.db, campaign_pid, limit).await?;
    let mut outcome = RunOutcome::default();
    for item in claimed {
        deliver_one(ctx, &campaign, &sender, item, &audience_pids, &mut outcome).await?;
    }

    let remaining = deliveries::retriable(&ctx.db, campaign_pid).await?;
    let counts = deliveries::summary(&ctx.db, campaign_pid).await?;
    let sent_total = counts.get("sent").copied().unwrap_or(0);
    let recipients = i32::try_from(sent_total).unwrap_or(i32::MAX);
    if sender.needs_address() {
        Audit::record(
            &ctx.db,
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
    }
    let txn = ctx.db.begin().await?;
    let mut active: campaigns::ActiveModel = campaign.into();
    active.recipients = ActiveValue::set(recipients);
    if remaining == 0 {
        active.status = ActiveValue::set("completed".to_string());
        if sender.is_simulated() {
            // Deterministic demo engagement: all delivered, 3/5 opened, 1/4
            // clicked, none unsubscribed by the simulation. A real sender
            // leaves these at zero: SMTP acceptance is not delivery, and the
            // provider's delivery, open and click events are not wired yet.
            active.delivered = ActiveValue::set(recipients);
            active.opened = ActiveValue::set(recipients * 3 / 5);
            active.clicked = ActiveValue::set(recipients / 4);
        }
    }
    let row = active.update(&txn).await?;
    if remaining == 0 {
        Audit::record(
            &txn,
            "campaign",
            campaign_pid,
            "campaign_completed",
            actor,
            Some(serde_json::json!({ "recipients": recipients })),
        )
        .await?;
        streaming::emit_on(
            &txn,
            "campaign",
            "campaign_completed",
            &campaign_pid.to_string(),
            &name,
            actor,
            None,
        )
        .await?;
    }
    txn.commit().await?;
    Ok((row, outcome.sent))
}

/// `GET /api/campaigns/{pid}/deliveries` — how a send is going: rows per state
/// and how many a further `run` could still send. Counts only; no addresses.
#[debug_handler]
pub(super) async fn campaign_deliveries(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let campaign = records::find_campaign(&ctx.db, pid).await?;
    let by_state = deliveries::summary(&ctx.db, campaign.pid).await?;
    let total: i64 = by_state.values().sum();
    let remaining = deliveries::retriable(&ctx.db, campaign.pid).await?;
    format::json(serde_json::json!({
        "campaign_pid": campaign.pid,
        "total": total,
        "remaining": remaining,
        "by_state": by_state,
    }))
}

/// `GET /api/contacts/{pid}/email-suppression` — whether the contact's mailbox
/// is suppressed after a hard bounce or complaint, and why. Audited read.
#[debug_handler]
pub(super) async fn email_suppression(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, pid).await?;
    let found = deliveries::suppression(&ctx.db, contact.pid).await?;
    Audit::record(
        &ctx.db,
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
    let contact = records::find_contact(&ctx.db, pid).await?;
    let txn = ctx.db.begin().await?;
    let removed = deliveries::unsuppress(&txn, contact.pid).await?;
    if !removed {
        return Err(Error::NotFound);
    }
    Audit::record(
        &txn,
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
pub(super) async fn campaign_funnel(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let masked =
        crate::auth::aggregate_masked(&caller).map_err(crate::controllers::record_rejection)?;
    let campaign = records::find_campaign(&ctx.db, pid).await?;
    let lead_rows = crate::models::_entities::leads::Entity::find()
        .filter(crate::models::_entities::leads::Column::CampaignPid.eq(campaign.pid))
        .filter(crate::models::_entities::leads::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let attributed_deals = deals::Entity::find()
        .filter(deals::Column::SourceCampaignPid.eq(campaign.pid))
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::Won.eq(true))
        .all(&ctx.db)
        .await?;
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
    let roi = analytics::roi(same_currency_revenue, campaign.cost_minor);
    let mut body = serde_json::json!({
        "campaign": campaign,
        "leads": lead_rows.len(),
        "won_deals": attributed_deals.len(),
        "won_revenue_minor": same_currency_revenue,
        "won_revenue_other_currencies": other_currency,
        "roi": roi,
    });
    if masked {
        // Cost, revenue and ROI are commercial; counts and structure stay.
        body = crate::auth::mask_json(
            body,
            &["won_revenue_minor", "won_revenue_other_currencies", "roi"],
        );
        if let Some(campaign) = body.get_mut("campaign") {
            *campaign = crate::auth::mask_json(campaign.take(), &["cost_minor"]);
        }
    }
    format::json(body)
}
