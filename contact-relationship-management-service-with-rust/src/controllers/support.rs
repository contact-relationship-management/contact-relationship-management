//! Customer service & support (CRM-R10–R12): tickets with derived
//! SLA deadlines and breach facts, SLA policies, the breach sweep,
//! and the versioned knowledge base.

use crate::controllers::PidPath;
use crate::json::Json;
use crate::models::live::FindLive;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::paging::{Page, PageParams};
use super::{PidRef, StatusPayload, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::metrics::Metrics;
use crate::models::_entities::{activities, articles, sla_policies, tickets};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::calendar::Calendar;
use crate::rules::{lifecycle, sla, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/sla-policies` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PolicyPayload {
    priority: String,
    first_response_minutes: i32,
    resolution_minutes: i32,
    /// A business calendar (CRM-R45): the targets are then counted in its working time. Leave out
    /// for round-the-clock.
    #[serde(default)]
    calendar_pid: Option<Uuid>,
    /// The most minutes a ticket may wait for the customer (`pending`) before the sweep flags it
    /// (CRM-R64). Leave out for no limit.
    #[serde(default)]
    max_wait_minutes: Option<i32>,
    /// What the sweep does with a ticket past `max_wait_minutes` (CRM-R69): `flag` (the default),
    /// `resolve` it (a status change to `open` reopens it) or `escalate` it to `escalate_to`.
    #[serde(default)]
    wait_limit_action: Option<String>,
    /// The worker (`worker:<id>`) an `escalate` reassigns the ticket to; only with `escalate`.
    #[serde(default)]
    escalate_to: Option<String>,
}

/// `PUT /api/sla-policies/{pid}/calendar` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PolicyCalendarPayload {
    /// The calendar, or `null` to go back to round-the-clock. Tickets already open keep their
    /// deadlines.
    calendar_pid: Option<Uuid>,
}

/// `POST /api/tickets` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TicketPayload {
    title: String,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default = "default_ticket_channel")]
    channel: String,
    #[serde(default)]
    contact_pid: Option<Uuid>,
    #[serde(default)]
    assignee_ref: Option<String>,
}

/// `PUT /api/tickets/{pid}/priority` body (audited; re-derives SLA).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PriorityPayload {
    priority: String,
    reason: String,
}

/// `POST /api/articles` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArticlePayload {
    title: String,
    body: String,
    #[serde(default)]
    keywords: Option<String>,
}

fn default_priority() -> String {
    "normal".to_string()
}
fn default_ticket_channel() -> String {
    "web".to_string()
}

/// The SLA targets for a priority, with the business calendar they are counted on, when a policy
/// exists.
async fn targets_for<C: sea_orm::ConnectionTrait>(
    db: &C,
    priority: &str,
) -> Result<Option<(sla::Targets, Option<Calendar>)>> {
    let policy = sla_policies::Entity::find()
        .filter(sla_policies::Column::Priority.eq(priority))
        .filter(sla_policies::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    let Some(p) = policy else {
        return Ok(None);
    };
    let calendar = match p.calendar_pid {
        Some(pid) => super::calendars::load(db, pid).await?,
        None => None,
    };
    Ok(Some((
        sla::Targets {
            first_response_minutes: p.first_response_minutes,
            resolution_minutes: p.resolution_minutes,
        },
        calendar,
    )))
}

/// `POST /api/sla-policies`.
#[debug_handler]
async fn create_policy(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PolicyPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    if payload.first_response_minutes <= 0 || payload.resolution_minutes <= 0 {
        problems.push("SLA minutes must be positive".to_string());
    }
    if payload.first_response_minutes > payload.resolution_minutes {
        problems.push("first response target exceeds the resolution target".to_string());
    }
    if payload.max_wait_minutes.is_some_and(|m| m <= 0) {
        problems.push_field("max_wait_minutes", "must be positive");
    }
    let action = payload.wait_limit_action.as_deref().unwrap_or("flag");
    if !["flag", "resolve", "escalate"].contains(&action) {
        problems.push_field("wait_limit_action", "one of flag, resolve, escalate");
    } else if action != "flag" && payload.max_wait_minutes.is_none() {
        problems.push_field("wait_limit_action", "needs a max_wait_minutes to act on");
    }
    if action == "escalate" {
        match payload.escalate_to.as_deref() {
            Some(worker) => {
                problems.require_ref("escalate_to", entity_ref::EntityType::Worker, worker);
            }
            None => problems.push_field("escalate_to", "escalating needs a worker"),
        }
    } else if payload.escalate_to.is_some() {
        problems.push_field("escalate_to", "only with the escalate action");
    }
    problems.finish()?;
    if let Some(calendar) = payload.calendar_pid
        && !super::calendars::exists(&ctx.db, calendar).await?
    {
        return Err(unprocessable("that business calendar does not exist"));
    }
    let txn = ctx.db.begin().await?;
    let row = sla_policies::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        priority: ActiveValue::set(payload.priority.clone()),
        first_response_minutes: ActiveValue::set(payload.first_response_minutes),
        resolution_minutes: ActiveValue::set(payload.resolution_minutes),
        calendar_pid: ActiveValue::set(payload.calendar_pid),
        max_wait_minutes: ActiveValue::set(payload.max_wait_minutes),
        wait_limit_action: ActiveValue::set(action.to_string()),
        escalate_to: ActiveValue::set(payload.escalate_to.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "sla_policy", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `PUT /api/sla-policies/{pid}/calendar` — count a policy's targets on a business calendar, or
/// stop (CRM-R45). Tickets already open keep the deadlines they were given.
#[debug_handler]
async fn set_policy_calendar(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PolicyCalendarPayload>,
) -> Result<Response> {
    if let Some(calendar) = payload.calendar_pid
        && !super::calendars::exists(&ctx.db, calendar).await?
    {
        return Err(unprocessable("that business calendar does not exist"));
    }
    let policy = sla_policies::Entity::find_live()
        .filter(sla_policies::Column::Pid.eq(pid))
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let txn = ctx.db.begin().await?;
    let mut active: sla_policies::ActiveModel = policy.into();
    active.calendar_pid = ActiveValue::set(payload.calendar_pid);
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "sla_policy",
        row.pid,
        "calendar_set",
        caller.actor(),
        Some(serde_json::json!({ "calendar_pid": payload.calendar_pid })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/sla-policies`.
#[debug_handler]
async fn list_policies(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = sla_policies::Entity::find_live()
        .order_by_asc(sla_policies::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/tickets` — open; deadlines derive from the priority's
/// policy at open (CRM-R11).
#[debug_handler]
async fn create_ticket(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<TicketPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    problems.require_token("channel", tokens::TICKET_CHANNELS, &payload.channel);
    problems.ref_opt(
        "assignee_ref",
        entity_ref::EntityType::Worker,
        payload.assignee_ref.as_deref(),
    );
    problems.finish()?;
    let account_pid = if let Some(contact) = payload.contact_pid {
        records::find_contact(&ctx.db, contact).await?.account_pid
    } else {
        None
    };
    let opened_at: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let deadlines = targets_for(&ctx.db, &payload.priority)
        .await?
        .map(|(targets, calendar)| sla::deadlines_on(opened_at, targets, calendar.as_ref()));
    let txn = ctx.db.begin().await?;
    let row = tickets::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        contact_pid: ActiveValue::set(payload.contact_pid),
        account_pid: ActiveValue::set(account_pid),
        assignee_ref: ActiveValue::set(payload.assignee_ref.clone()),
        title: ActiveValue::set(payload.title.clone()),
        priority: ActiveValue::set(payload.priority.clone()),
        channel: ActiveValue::set(payload.channel.clone()),
        status: ActiveValue::set("open".to_string()),
        opened_at: ActiveValue::set(opened_at),
        first_response_due_at: ActiveValue::set(
            deadlines.as_ref().map(|d| d.first_response_due_at),
        ),
        resolution_due_at: ActiveValue::set(deadlines.as_ref().map(|d| d.resolution_due_at)),
        first_responded_at: ActiveValue::set(None),
        resolved_at: ActiveValue::set(None),
        first_response_breached: ActiveValue::set(false),
        resolution_breached: ActiveValue::set(false),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "ticket",
        row.pid,
        "ticket_opened",
        caller.actor(),
        None,
    )
    .await?;
    streaming::emit_on(
        &txn,
        "ticket",
        "ticket_opened",
        &row.pid.to_string(),
        &row.title,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/tickets?status=` — the queue with **live** breach flags
/// (computed on read; the sweep persists + emits them).
#[derive(Debug, Deserialize)]
struct TicketListParams {
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn list_tickets(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(params): Query<TicketListParams>,
    Query(paging): Query<PageParams>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 1000)?;
    let mut query = tickets::Entity::find_live();
    if let Some(status) = &params.status {
        query = query.filter(tickets::Column::Status.eq(status));
    }
    let rows = page
        .apply(
            query
                .order_by_asc(tickets::Column::BoardPosition)
                .order_by_asc(tickets::Column::Id),
        )
        .all(&ctx.db)
        .await?;
    let (rows, more) = page.split(rows);
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let rows: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|ticket| {
            // A ticket waiting for the customer has a stopped clock (CRM-R55).
            let clock = sla::clock_now(now, ticket.paused_at);
            let live_first = ticket.first_response_due_at.is_some_and(|due| {
                sla::first_response_breached(clock, due, ticket.first_responded_at)
            });
            let live_resolution = ticket
                .resolution_due_at
                .is_some_and(|due| sla::resolution_breached(clock, due, ticket.resolved_at));
            let mut value = serde_json::to_value(&ticket).unwrap_or_default();
            if let Some(object) = value.as_object_mut() {
                object.insert("live_first_response_breached".into(), live_first.into());
                object.insert("live_resolution_breached".into(), live_resolution.into());
            }
            value
        })
        .collect();
    Audit::record_read(
        &ctx.db,
        "ticket",
        Uuid::nil(),
        "ticket_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    page.respond(rows, more, &uri)
}

/// `GET /api/tickets/{pid}`: the ticket and its activities, oldest first.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct TicketDetail {
    ticket: tickets::Model,
    activities: Vec<activities::Model>,
}

/// What the sweep did with the tickets past their policy's wait limit.
#[derive(Debug, Default)]
struct WaitSweep {
    exceeded: u64,
    resolved: u64,
    escalated: u64,
}

/// Act on each `pending` ticket that has waited longer than its policy's `max_wait_minutes`, once
/// (CRM-R64, CRM-R69): `flag` stores the moment and emits `ticket_wait_limit_exceeded`; `escalate`
/// does that and hands the ticket to the policy's worker; `resolve` resolves it, the clock resuming
/// as for any status change. A ticket stays `pending` unless the policy resolves it.
async fn sweep_waiting(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<WaitSweep> {
    let limits: std::collections::HashMap<String, (i32, String, Option<String>)> =
        sla_policies::Entity::find()
            .filter(sla_policies::Column::DeletedAt.is_null())
            .filter(sla_policies::Column::MaxWaitMinutes.is_not_null())
            .all(&ctx.db)
            .await?
            .into_iter()
            .filter_map(|p| {
                p.max_wait_minutes
                    .map(|m| (p.priority, (m, p.wait_limit_action, p.escalate_to)))
            })
            .collect();
    let mut done = WaitSweep::default();
    if limits.is_empty() {
        return Ok(done);
    }
    let waiting = tickets::Entity::find_live()
        .filter(tickets::Column::Status.eq("pending"))
        .filter(tickets::Column::WaitLimitExceededAt.is_null())
        .all(&ctx.db)
        .await?;
    for ticket in waiting {
        let (Some((limit, action, escalate_to)), Some(since)) =
            (limits.get(&ticket.priority), ticket.paused_at)
        else {
            continue;
        };
        if now - since <= chrono::Duration::minutes(i64::from(*limit)) {
            continue;
        }
        let (pid, title) = (ticket.pid, ticket.title.clone());
        if action == "resolve" {
            transition(ctx, caller, ticket, "resolved", Some("wait_limit")).await?;
            done.resolved += 1;
        } else {
            let txn = ctx.db.begin().await?;
            let mut active: tickets::ActiveModel = ticket.into();
            active.wait_limit_exceeded_at = ActiveValue::set(Some(now));
            if action == "escalate" {
                active.assignee_ref = ActiveValue::set(escalate_to.clone());
                done.escalated += 1;
            }
            active.update(&txn).await?;
            if action == "escalate" {
                Audit::record(
                    &txn,
                    "ticket",
                    pid,
                    "ticket_escalated",
                    caller.actor(),
                    Some(serde_json::json!({ "to": escalate_to, "via": "wait_limit" })),
                )
                .await?;
            }
            txn.commit().await?;
        }
        streaming::emit_on(
            &ctx.db,
            "ticket",
            "ticket_wait_limit_exceeded",
            &pid.to_string(),
            &title,
            caller.actor(),
            Some(serde_json::json!({ "waiting_since": since, "action": action })),
        )
        .await?;
        done.exceeded += 1;
    }
    Ok(done)
}

/// `POST /api/sla/sweep`: how many tickets crossed a deadline in this sweep.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[allow(clippy::struct_field_names)] // the `new_` names are what clients read
pub(crate) struct SlaSweep {
    new_breaches: u64,
    /// Waiting tickets found past their policy's `max_wait_minutes` in this sweep (CRM-R64).
    new_wait_limits_exceeded: u64,
    /// Of those, tickets the policy resolved (CRM-R69).
    new_wait_limit_resolved: u64,
    /// Of those, tickets the policy reassigned to its worker (CRM-R69).
    new_wait_limit_escalated: u64,
}

/// `GET /api/tickets/{pid}` — one ticket + its activities.
#[debug_handler]
async fn get_ticket(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let ticket = records::find_ticket(&ctx.db, pid).await?;
    Audit::record_read(
        &ctx.db,
        "ticket",
        ticket.pid,
        "ticket_read",
        caller.actor(),
        None,
    )
    .await?;
    let activity_rows = activities::Entity::find()
        .filter(activities::Column::SubjectKind.eq("ticket"))
        .filter(activities::Column::SubjectPid.eq(ticket.pid))
        .filter(activities::Column::DeletedAt.is_null())
        .order_by_asc(activities::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(TicketDetail {
        ticket,
        activities: activity_rows,
    })
}

/// What leaving `pending` does to a ticket's clock.
struct Resumed {
    /// The waiting that now counts against each deadline, the earlier waiting included.
    paused: sla::Paused,
    /// The deadlines derived again with it; `None` when no policy covers the ticket any more.
    deadlines: Option<sla::Deadlines>,
}

/// Work out what resuming a ticket that was paused at `paused_at` does at `now`: each deadline moves
/// by the waiting that counted against it, in working time on the policy's calendar.
async fn resume_clock<C: sea_orm::ConnectionTrait>(
    db: &C,
    ticket: &tickets::Model,
    paused_at: chrono::DateTime<chrono::FixedOffset>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<Resumed> {
    let policy = targets_for(db, &ticket.priority).await?;
    let calendar = policy.as_ref().and_then(|(_, calendar)| calendar.as_ref());
    let paused = sla::Paused {
        first_response_seconds: ticket.first_response_paused_seconds
            + sla::pause_counted(
                calendar,
                paused_at,
                now,
                ticket.first_response_due_at,
                ticket.first_responded_at.is_some(),
            ),
        resolution_seconds: ticket.resolution_paused_seconds
            + sla::pause_counted(
                calendar,
                paused_at,
                now,
                ticket.resolution_due_at,
                ticket.resolved_at.is_some(),
            ),
    };
    let deadlines = policy.as_ref().map(|(targets, calendar)| {
        sla::deadlines_paused(ticket.opened_at, *targets, calendar.as_ref(), paused)
    });
    Ok(Resumed { paused, deadlines })
}

/// `POST /api/tickets/{pid}/status` — one lifecycle transition;
/// `resolved` stamps `resolved_at`.
#[debug_handler]
async fn ticket_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::TICKET_STATUSES, &payload.to);
    problems.finish()?;
    let ticket = records::find_ticket(&ctx.db, pid).await?;
    transition(&ctx, &caller, ticket, &payload.to, None).await
}

/// `POST /api/tickets/{pid}/customer-reply` — the customer has answered (CRM-R63): a ticket
/// waiting for them goes back to `open` and its SLA clock starts again, exactly as a status
/// change would. A ticket already `open` is returned as it is; one that is resolved or closed is
/// refused, because a reply does not reopen it.
#[debug_handler]
async fn ticket_customer_reply(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let ticket = records::find_ticket(&ctx.db, pid).await?;
    match ticket.status.as_str() {
        "pending" => transition(&ctx, &caller, ticket, "open", Some("customer_reply")).await,
        "open" => format::json(ticket),
        other => Err(unprocessable(&format!(
            "a customer reply does not reopen a {other} ticket"
        ))),
    }
}

/// Move `ticket` to `to`, resuming or stopping its SLA clock (CRM-R55); `via` names what caused it.
async fn transition(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    ticket: tickets::Model,
    to: &str,
    via: Option<&str>,
) -> Result<Response> {
    let payload = StatusPayload { to: to.to_string() };
    lifecycle::check("ticket", lifecycle::TICKET, &ticket.status, &payload.to)
        .map_err(|e| unprocessable(&e.to_string()))?;
    // The SLA clock stops while the ticket waits for the customer and starts again when it leaves
    // `pending`, moving each deadline by the waiting that counted against it (CRM-R55).
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let resumed = match ticket.paused_at {
        Some(paused_at) if ticket.status == "pending" => {
            Some(resume_clock(&ctx.db, &ticket, paused_at, now).await?)
        }
        _ => None,
    };
    let txn = ctx.db.begin().await?;
    let from = ticket.status.clone();
    let title = ticket.title.clone();
    let mut active: tickets::ActiveModel = ticket.into();
    active.status = ActiveValue::set(payload.to.clone());
    if payload.to == "pending" {
        active.paused_at = ActiveValue::set(Some(now));
    } else if let Some(resumed) = resumed {
        active.paused_at = ActiveValue::set(None);
        active.wait_limit_exceeded_at = ActiveValue::set(None);
        active.first_response_paused_seconds =
            ActiveValue::set(resumed.paused.first_response_seconds);
        active.resolution_paused_seconds = ActiveValue::set(resumed.paused.resolution_seconds);
        if let Some(deadlines) = resumed.deadlines {
            active.first_response_due_at = ActiveValue::set(Some(deadlines.first_response_due_at));
            active.resolution_due_at = ActiveValue::set(Some(deadlines.resolution_due_at));
        }
    }
    if payload.to == "resolved" {
        active.resolved_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    } else if payload.to == "open" && from == "resolved" {
        active.resolved_at = ActiveValue::set(None); // reopen clears it
    }
    let row = active.update(&txn).await?;
    let kind = match payload.to.as_str() {
        "resolved" => "ticket_resolved",
        "closed" => "ticket_closed",
        _ => "ticket_status_changed",
    };
    Audit::record(
        &txn,
        "ticket",
        row.pid,
        kind,
        caller.actor(),
        Some(match via {
            Some(via) => serde_json::json!({ "from": from, "via": via }),
            None => serde_json::json!({ "from": from }),
        }),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "ticket",
        kind,
        &row.pid.to_string(),
        &title,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `PUT /api/tickets/{pid}/priority` — audited priority change;
/// re-derives the deadlines from the opened time (CRM-R11).
#[debug_handler]
async fn ticket_priority(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PriorityPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    problems.require_text("reason", &payload.reason);
    problems.finish()?;
    let ticket = records::find_ticket(&ctx.db, pid).await?;
    // Waiting for the customer already counted stays counted (CRM-R55).
    let paused = sla::Paused {
        first_response_seconds: ticket.first_response_paused_seconds,
        resolution_seconds: ticket.resolution_paused_seconds,
    };
    let deadlines = targets_for(&ctx.db, &payload.priority)
        .await?
        .map(|(targets, calendar)| {
            sla::deadlines_paused(ticket.opened_at, targets, calendar.as_ref(), paused)
        });
    let txn = ctx.db.begin().await?;
    let from = ticket.priority.clone();
    let mut active: tickets::ActiveModel = ticket.into();
    active.priority = ActiveValue::set(payload.priority.clone());
    active.first_response_due_at =
        ActiveValue::set(deadlines.as_ref().map(|d| d.first_response_due_at));
    active.resolution_due_at = ActiveValue::set(deadlines.as_ref().map(|d| d.resolution_due_at));
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "ticket",
        row.pid,
        "priority_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.priority, "reason": payload.reason })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/sla/sweep` — persist breach facts and emit
/// `sla_breached` **once per breach** (idempotent: only rows whose
/// stored flag is still false flip and emit).
#[debug_handler]
async fn sla_sweep(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let open = tickets::Entity::find_live()
        .filter(tickets::Column::Status.is_in(["open", "pending", "resolved"]))
        .all(&ctx.db)
        .await?;
    let mut breaches = 0_u64;
    for ticket in open {
        let clock = sla::clock_now(now, ticket.paused_at);
        let first = ticket
            .first_response_due_at
            .is_some_and(|due| sla::first_response_breached(clock, due, ticket.first_responded_at));
        let resolution = ticket
            .resolution_due_at
            .is_some_and(|due| sla::resolution_breached(clock, due, ticket.resolved_at));
        let new_first = first && !ticket.first_response_breached;
        let new_resolution = resolution && !ticket.resolution_breached;
        if !new_first && !new_resolution {
            continue;
        }
        let txn = ctx.db.begin().await?;
        let pid = ticket.pid;
        let mut active: tickets::ActiveModel = ticket.into();
        if new_first {
            active.first_response_breached = ActiveValue::set(true);
        }
        if new_resolution {
            active.resolution_breached = ActiveValue::set(true);
        }
        active.update(&txn).await?;
        streaming::emit_on(
            &txn,
            "ticket",
            "sla_breached",
            &pid.to_string(),
            "",
            caller.actor(),
            Some(serde_json::json!({
                "first_response": new_first, "resolution": new_resolution,
            })),
        )
        .await?;
        txn.commit().await?;
        breaches += u64::from(new_first) + u64::from(new_resolution);
        Metrics::global().sla_breached_total.inc();
    }
    let waited = sweep_waiting(&ctx, &caller, now).await?;
    format::json(SlaSweep {
        new_breaches: breaches,
        new_wait_limits_exceeded: waited.exceeded,
        new_wait_limit_resolved: waited.resolved,
        new_wait_limit_escalated: waited.escalated,
    })
}

/// `POST /api/articles` — draft an article.
#[debug_handler]
async fn create_article(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ArticlePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    problems.require_text("body", &payload.body);
    problems.cap_opt("keywords", payload.keywords.as_deref());
    problems.finish()?;
    let txn = ctx.db.begin().await?;
    let row = articles::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        title: ActiveValue::set(payload.title.clone()),
        body: ActiveValue::set(payload.body.clone()),
        keywords: ActiveValue::set(payload.keywords.clone()),
        status: ActiveValue::set("draft".to_string()),
        version: ActiveValue::set(1),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "article", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `PUT /api/articles/{pid}` — edit; a **published** edit bumps the
/// version (CRM-R12).
#[debug_handler]
async fn update_article(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ArticlePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    problems.require_text("body", &payload.body);
    problems.finish()?;
    let article = records::find_article(&ctx.db, pid).await?;
    if article.status == "archived" {
        return Err(unprocessable("archived articles are read-only"));
    }
    let txn = ctx.db.begin().await?;
    let published = article.status == "published";
    let version = article.version;
    let mut active: articles::ActiveModel = article.into();
    active.title = ActiveValue::set(payload.title.clone());
    active.body = ActiveValue::set(payload.body.clone());
    active.keywords = ActiveValue::set(payload.keywords.clone());
    if published {
        active.version = ActiveValue::set(version + 1);
    }
    let row = active.update(&txn).await?;
    Audit::record(&txn, "article", row.pid, "updated", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/articles/{pid}/status` — publish / archive.
#[debug_handler]
async fn article_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::ARTICLE_STATUSES, &payload.to);
    problems.finish()?;
    let article = records::find_article(&ctx.db, pid).await?;
    lifecycle::check("article", lifecycle::ARTICLE, &article.status, &payload.to)
        .map_err(|e| unprocessable(&e.to_string()))?;
    let txn = ctx.db.begin().await?;
    let mut active: articles::ActiveModel = article.into();
    active.status = ActiveValue::set(payload.to.clone());
    let row = active.update(&txn).await?;
    let kind = if payload.to == "published" {
        "article_published"
    } else {
        "updated"
    };
    Audit::record(&txn, "article", row.pid, kind, caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "article",
        kind,
        &row.pid.to_string(),
        &row.title,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/articles?q=` — keyword search (ILIKE-style contains over
/// title/keywords; published first).
#[derive(Debug, Deserialize)]
struct ArticleSearchParams {
    #[serde(default)]
    q: Option<String>,
}

#[debug_handler]
async fn list_articles(
    State(ctx): State<AppContext>,
    Query(params): Query<ArticleSearchParams>,
    Query(paging): Query<PageParams>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 500)?;
    let mut query = articles::Entity::find_live();
    // Search in SQL, before the page is cut: filtering after the cut would
    // hide matches that sit beyond the first page.
    if let Some(q) = params.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let escaped = q
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        query = query.filter(sea_orm::sea_query::Expr::cust_with_values(
            "(lower(title) LIKE $1 OR lower(coalesce(keywords, '')) LIKE $1)",
            [format!("%{escaped}%")],
        ));
    }
    let rows = page
        .apply(query.order_by_asc(articles::Column::Id))
        .all(&ctx.db)
        .await?;
    let (rows, more) = page.split(rows);
    page.respond(rows, more, &uri)
}

/// An article that may answer a ticket.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct SuggestedArticle {
    #[schemars(with = "uuid::Uuid")]
    article_pid: String,
    title: String,
    /// Words of the ticket's title the article shares; headline words count 2, text words 1.
    score: u32,
    matched: Vec<String>,
}

/// `GET /api/tickets/{pid}/suggested-articles` — the published articles that share words with the
/// ticket's title, best first, at most 5 (CRM-R66).
#[debug_handler]
async fn suggested_articles(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let ticket = records::find_ticket(&ctx.db, pid).await?;
    let published = articles::Entity::find_live()
        .filter(articles::Column::Status.eq("published"))
        .order_by_asc(articles::Column::Id)
        .all(&ctx.db)
        .await?;
    let texts: Vec<_> = published
        .iter()
        .map(|a| (a.title.clone(), a.keywords.clone(), a.body.clone()))
        .collect();
    format::json(
        crate::rules::suggest::suggest(&ticket.title, &texts)
            .into_iter()
            .map(|m| SuggestedArticle {
                article_pid: published[m.index].pid.to_string(),
                title: published[m.index].title.clone(),
                score: m.score,
                matched: m.matched,
            })
            .collect::<Vec<_>>(),
    )
}

/// The support routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sla-policies", post(create_policy))
        .add("/sla-policies", get(list_policies))
        .add("/sla-policies/{pid}/calendar", put(set_policy_calendar))
        .add("/tickets", post(create_ticket))
        .add("/tickets", get(list_tickets))
        .add("/tickets/{pid}", get(get_ticket))
        .add("/tickets/{pid}/suggested-articles", get(suggested_articles))
        .add("/tickets/{pid}/status", post(ticket_status))
        .add("/tickets/{pid}/customer-reply", post(ticket_customer_reply))
        .add("/tickets/{pid}/priority", put(ticket_priority))
        .add("/sla/sweep", post(sla_sweep))
        .add("/articles", post(create_article))
        .add("/articles", get(list_articles))
        .add("/articles/{pid}", put(update_article))
        .add("/articles/{pid}/status", post(article_status))
}
