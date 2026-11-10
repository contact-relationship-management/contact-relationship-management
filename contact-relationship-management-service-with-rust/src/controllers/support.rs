//! Customer service & support (CRM-R10–R12): tickets with derived
//! SLA deadlines and breach facts, SLA policies, the breach sweep,
//! and the versioned knowledge base.

use crate::controllers::PidPath;
use crate::json::Json;
use axa::extract::Query;
use axa::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::paging::Page;
use super::{PidRef, StatusPayload, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::db;
use crate::metrics::Metrics;
use crate::models::_entities::{activities, tickets};
use crate::models::DateTimeWithTimeZone;
use crate::models::audit_logs::Model as Audit;
use crate::models::rows;
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
    /// The user (`person:<id>`; `worker:` is read as `person:`, CRM-D18) an `escalate` reassigns
    /// the ticket to; only with `escalate`.
    #[serde(default)]
    escalate_to: Option<String>,
    /// Days (1 to 365) a ticket may stay `resolved` before the sweep closes it (CRM-R82). Leave out
    /// to leave closing to a person.
    #[serde(default)]
    auto_close_days: Option<i32>,
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
async fn targets_for(
    conn: &mut PgConnection,
    priority: &str,
) -> Result<Option<(sla::Targets, Option<Calendar>)>> {
    let Some(p) = rows::sla_policy_of_priority(&mut *conn, priority).await? else {
        return Ok(None);
    };
    let calendar = match p.calendar_pid {
        Some(pid) => super::calendars::load(&mut *conn, pid).await?,
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

/// The live ticket `pid`, or `404`.
async fn find_ticket<'e>(db: impl sqlx::PgExecutor<'e>, pid: Uuid) -> Result<tickets::Model> {
    rows::ticket(db, pid).await?.ok_or(Error::NotFound)
}

/// `POST /api/sla-policies`.
#[debug_handler]
async fn create_policy(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<PolicyPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    payload.escalate_to = crate::rules::user_ref::normalise_opt(payload.escalate_to.take());
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
    if payload
        .auto_close_days
        .is_some_and(|d| !(1..=365).contains(&d))
    {
        problems.push_field("auto_close_days", "between 1 and 365");
    }
    if action == "escalate" {
        match payload.escalate_to.as_deref() {
            Some(user) => problems.require_user("escalate_to", user),
            None => problems.push_field("escalate_to", "escalating needs a worker"),
        }
    } else if payload.escalate_to.is_some() {
        problems.push_field("escalate_to", "only with the escalate action");
    }
    problems.finish()?;
    let pool = db::pool(&ctx);
    if let Some(calendar) = payload.calendar_pid
        && !super::calendars::exists(pool, calendar).await?
    {
        return Err(unprocessable("that business calendar does not exist"));
    }
    let pid = Uuid::new_v4();
    let mut txn = pool.begin().await?;
    sqlx::query!(
        "INSERT INTO sla_policies (pid, priority, first_response_minutes, resolution_minutes, \
         calendar_pid, max_wait_minutes, wait_limit_action, escalate_to, auto_close_days) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        pid,
        payload.priority,
        payload.first_response_minutes,
        payload.resolution_minutes,
        payload.calendar_pid,
        payload.max_wait_minutes,
        action,
        payload.escalate_to,
        payload.auto_close_days,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(&mut txn, "sla_policy", pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
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
    let pool = db::pool(&ctx);
    if let Some(calendar) = payload.calendar_pid
        && !super::calendars::exists(pool, calendar).await?
    {
        return Err(unprocessable("that business calendar does not exist"));
    }
    let mut txn = pool.begin().await?;
    let row = sqlx::query_as!(
        crate::models::_entities::sla_policies::Model,
        "UPDATE sla_policies SET calendar_pid = $2 WHERE pid = $1 AND deleted_at IS NULL \
            RETURNING created_at, updated_at, id, pid, priority, first_response_minutes, \
            resolution_minutes, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", calendar_pid, \
            max_wait_minutes, wait_limit_action, escalate_to, auto_close_days",
        pid,
        payload.calendar_pid,
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    Audit::record_on(
        &mut txn,
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
    format::json(rows::sla_policies(db::pool(&ctx)).await?)
}

/// `POST /api/tickets` — open; deadlines derive from the priority's
/// policy at open (CRM-R11).
#[debug_handler]
async fn create_ticket(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<TicketPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    problems.require_token("channel", tokens::TICKET_CHANNELS, &payload.channel);
    payload.assignee_ref = crate::rules::user_ref::normalise_opt(payload.assignee_ref.take());
    problems.user_opt("assignee_ref", payload.assignee_ref.as_deref());
    problems.finish()?;
    let pool = db::pool(&ctx);
    let account_pid = if let Some(contact) = payload.contact_pid {
        rows::contact(pool, contact)
            .await?
            .ok_or(Error::NotFound)?
            .account_pid
    } else {
        None
    };
    let opened_at: DateTimeWithTimeZone = crate::clock::now().into();
    let mut txn = pool.begin().await?;
    let deadlines = targets_for(&mut txn, &payload.priority)
        .await?
        .map(|(targets, calendar)| sla::deadlines_on(opened_at, targets, calendar.as_ref()));
    let pid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO tickets (pid, contact_pid, account_pid, assignee_ref, title, priority, \
         channel, status, opened_at, first_response_due_at, resolution_due_at, \
         first_response_breached, resolution_breached) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'open', $8, $9, $10, false, false)",
        pid,
        payload.contact_pid,
        account_pid,
        payload.assignee_ref,
        payload.title,
        payload.priority,
        payload.channel,
        opened_at,
        deadlines.as_ref().map(|d| d.first_response_due_at),
        deadlines.as_ref().map(|d| d.resolution_due_at),
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "ticket",
        pid,
        "ticket_opened",
        caller.actor(),
        None,
    )
    .await?;
    streaming::emit_in(
        &mut txn,
        "ticket",
        "ticket_opened",
        &pid.to_string(),
        &payload.title,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
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
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&uri, 1000)?;
    let (limit, offset) = page.bounds();
    let pool = db::pool(&ctx);
    let rows = rows::tickets_page(pool, params.status.as_deref(), limit, offset).await?;
    let (rows, more) = page.split(rows);
    let now: DateTimeWithTimeZone = crate::clock::now().into();
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
    Audit::record_read_on(
        pool,
        "ticket",
        Uuid::nil(),
        "ticket_list_read",
        caller.actor(),
        Some(serde_json::json!({ "returned": rows.len() })),
    )
    .await?;
    page.respond(rows, more)
}

/// `GET /api/tickets/{pid}`: the ticket and its activities, oldest first.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct TicketDetail {
    ticket: tickets::Model,
    /// What a customer's reply puts in its subject, in square brackets, so that the reply finds
    /// this ticket (CRM-R88).
    reference: String,
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
    now: DateTimeWithTimeZone,
) -> Result<WaitSweep> {
    let pool = db::pool(ctx);
    let limits: std::collections::HashMap<String, (i32, String, Option<String>)> =
        rows::sla_policies(pool)
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
    let waiting = rows::tickets_with_status(pool, &["pending"]).await?;
    for ticket in waiting
        .into_iter()
        .filter(|t| t.wait_limit_exceeded_at.is_none())
    {
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
            transition(ctx, caller, pid, "resolved", Some("wait_limit")).await?;
            done.resolved += 1;
        } else {
            let escalate = action == "escalate";
            let mut txn = pool.begin().await?;
            sqlx::query!(
                "UPDATE tickets SET wait_limit_exceeded_at = $2, \
                 assignee_ref = CASE WHEN $3 THEN $4 ELSE assignee_ref END \
                 WHERE id = $1 AND deleted_at IS NULL",
                ticket.id,
                now,
                escalate,
                escalate_to.as_deref(),
            )
            .execute(&mut *txn)
            .await?;
            if escalate {
                done.escalated += 1;
                Audit::record_on(
                    &mut txn,
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
        let mut conn = pool.acquire().await?;
        streaming::emit_in(
            &mut conn,
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

/// Close each `resolved` ticket that has stayed so for longer than its policy's `auto_close_days`
/// (CRM-R82), as a status change audited with `via: auto_close`. Returns how many it closed. A
/// reopened ticket has no `resolved_at`, so it is not counted from its first resolution.
async fn sweep_closing(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    now: DateTimeWithTimeZone,
) -> Result<u64> {
    let pool = db::pool(ctx);
    let days: std::collections::HashMap<String, i32> = rows::sla_policies(pool)
        .await?
        .into_iter()
        .filter_map(|p| p.auto_close_days.map(|d| (p.priority, d)))
        .collect();
    if days.is_empty() {
        return Ok(0);
    }
    let resolved = rows::tickets_with_status(pool, &["resolved"]).await?;
    let mut closed = 0;
    for ticket in resolved {
        let (Some(limit), Some(since)) = (days.get(&ticket.priority), ticket.resolved_at) else {
            continue;
        };
        if now - since <= chrono::Duration::days(i64::from(*limit)) {
            continue;
        }
        transition(ctx, caller, ticket.pid, "closed", Some("auto_close")).await?;
        closed += 1;
    }
    Ok(closed)
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
    /// Resolved tickets the policy closed after their days were up (CRM-R82).
    new_auto_closed: u64,
}

/// `GET /api/tickets/{pid}` — one ticket + its activities.
#[debug_handler]
async fn get_ticket(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let ticket = find_ticket(pool, pid).await?;
    Audit::record_read_on(
        pool,
        "ticket",
        ticket.pid,
        "ticket_read",
        caller.actor(),
        None,
    )
    .await?;
    let activity_rows = rows::activities_of(pool, "ticket", ticket.pid).await?;
    format::json(TicketDetail {
        reference: reference(ticket.pid),
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
async fn resume_clock(
    conn: &mut PgConnection,
    ticket: &tickets::Model,
    paused_at: DateTimeWithTimeZone,
    now: DateTimeWithTimeZone,
) -> Result<Resumed> {
    let policy = targets_for(conn, &ticket.priority).await?;
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
    transition(&ctx, &caller, pid, &payload.to, None).await
}

/// The reference a reply carries for the ticket `pid` (CRM-R88): `T-` and the first eight hex
/// digits of its pid.
fn reference(pid: Uuid) -> String {
    format!("T-{}", &pid.simple().to_string()[..8])
}

/// The first `[T-xxxxxxxx]` in `subject`, as its eight hex digits.
fn reference_in(subject: &str) -> Option<String> {
    let lower = subject.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(at) = rest.find("[t-") {
        let after = &rest[at + 3..];
        if let Some(digits) = after.get(..8)
            && after.get(8..9) == Some("]")
            && digits.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Some(digits.to_string());
        }
        rest = after;
    }
    None
}

/// The mailbox in a `From` value: what is inside `<...>` when there is such a part, in lower case.
fn address_in(from: &str) -> String {
    let inner = match (from.rfind('<'), from.rfind('>')) {
        (Some(open), Some(close)) if open < close => &from[open + 1..close],
        _ => from,
    };
    inner.trim().to_ascii_lowercase()
}

/// `POST /webhooks/inbound-email` body: the two things a reply is recognised by. Anything else a
/// sender includes (the text, the headers) is not read and is not kept.
#[derive(Debug, Deserialize)]
struct InboundMail {
    /// The sender's `From` value, with or without a display name.
    from: String,
    /// The subject, which carries the ticket's reference.
    subject: String,
}

/// What became of one inbound message (CRM-R88).
async fn recognise_reply(ctx: &AppContext, mail: &InboundMail) -> Result<&'static str> {
    let Some(digits) = reference_in(&mail.subject) else {
        return Ok("no_reference");
    };
    let pool = db::pool(ctx);
    let found = sqlx::query_scalar!(
        "SELECT pid FROM tickets WHERE deleted_at IS NULL AND pid::text LIKE $1 LIMIT 2",
        format!("{digits}%"),
    )
    .fetch_all(pool)
    .await?;
    let pid: Uuid = match found.as_slice() {
        [] => return Ok("unknown_ticket"),
        [one] => *one,
        _ => return Ok("ambiguous_reference"),
    };
    let ticket = find_ticket(pool, pid).await?;
    // The sender must be the person the ticket is about: the mailbox the person service gives for
    // its contact, not an address the message names for itself.
    let contact = match ticket.contact_pid {
        Some(contact_pid) => rows::contact(pool, contact_pid).await?,
        None => None,
    };
    let known = match &contact {
        Some(contact) => crate::clients::email_address(&contact.person_ref).await,
        None => None,
    };
    if known.is_none_or(|address| address_in(&address) != address_in(&mail.from)) {
        return Ok("unverified_sender");
    }
    match ticket.status.as_str() {
        "pending" => {
            transition(
                ctx,
                &MaybeAuthUser(None),
                ticket.pid,
                "open",
                Some("inbound_mail"),
            )
            .await?;
            Ok("resumed")
        }
        "open" => Ok("already_open"),
        _ => Ok("ignored"),
    }
}

/// `POST /webhooks/inbound-email` — a customer's reply, as a mail service forwards it (CRM-R88).
/// Signed like `/webhooks/email`; `404` when no secret is configured. It answers `200` with the
/// outcome for every well-formed message, so a mail service does not retry one it was right to
/// ignore.
#[debug_handler]
async fn inbound_email(
    State(ctx): State<AppContext>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response> {
    use axum::http::StatusCode;
    match crate::webhook::signed_request(&headers, &body) {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(false) => return Ok(StatusCode::UNAUTHORIZED.into_response()),
        Some(true) => {}
    }
    let Ok(mail) = serde_json::from_slice::<InboundMail>(&body) else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };
    format::json(serde_json::json!({ "result": recognise_reply(&ctx, &mail).await? }))
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
    let ticket = find_ticket(db::pool(&ctx), pid).await?;
    match ticket.status.as_str() {
        "pending" => transition(&ctx, &caller, pid, "open", Some("customer_reply")).await,
        "open" => format::json(ticket),
        other => Err(unprocessable(&format!(
            "a customer reply does not reopen a {other} ticket"
        ))),
    }
}

/// Move the ticket `pid` to `to`, resuming or stopping its SLA clock (CRM-R55); `via` names what
/// caused it. The ticket is read again and locked in the transaction that changes it, so the
/// change is decided on the row as it is.
async fn transition(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    pid: Uuid,
    to: &str,
    via: Option<&str>,
) -> Result<Response> {
    let mut txn = db::pool(ctx).begin().await?;
    let ticket = rows::ticket_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    lifecycle::check("ticket", lifecycle::TICKET, &ticket.status, to)
        .map_err(|e| unprocessable(&e.to_string()))?;
    // The SLA clock stops while the ticket waits for the customer and starts again when it leaves
    // `pending`, moving each deadline by the waiting that counted against it (CRM-R55).
    let now: DateTimeWithTimeZone = crate::clock::now().into();
    let resumed = match ticket.paused_at {
        Some(paused_at) if ticket.status == "pending" => {
            Some(resume_clock(&mut txn, &ticket, paused_at, now).await?)
        }
        _ => None,
    };
    let from = ticket.status.clone();
    let title = ticket.title.clone();
    let mut next = ticket;
    next.status = to.to_string();
    if to == "pending" {
        next.paused_at = Some(now);
    } else if let Some(resumed) = resumed {
        next.paused_at = None;
        next.wait_limit_exceeded_at = None;
        next.first_response_paused_seconds = resumed.paused.first_response_seconds;
        next.resolution_paused_seconds = resumed.paused.resolution_seconds;
        if let Some(deadlines) = resumed.deadlines {
            next.first_response_due_at = Some(deadlines.first_response_due_at);
            next.resolution_due_at = Some(deadlines.resolution_due_at);
        }
    }
    if to == "resolved" {
        next.resolved_at = Some(crate::clock::now().into());
    } else if to == "open" && from == "resolved" {
        next.resolved_at = None; // reopen clears it
    }
    let row = sqlx::query_as!(
        tickets::Model,
        "UPDATE tickets SET status = $2, paused_at = $3, wait_limit_exceeded_at = $4, \
            first_response_paused_seconds = $5, resolution_paused_seconds = $6, \
            first_response_due_at = $7, resolution_due_at = $8, resolved_at = $9 WHERE id = $1 \
            RETURNING created_at, updated_at, id, pid, contact_pid, account_pid, assignee_ref, title, \
            priority, channel, status, opened_at, first_response_due_at AS \"first_response_due_at: \
            DateTimeWithTimeZone\", resolution_due_at AS \"resolution_due_at: DateTimeWithTimeZone\", \
            first_responded_at AS \"first_responded_at: DateTimeWithTimeZone\", resolved_at AS \
            \"resolved_at: DateTimeWithTimeZone\", first_response_breached, resolution_breached, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\", board_position, paused_at AS \"paused_at: \
            DateTimeWithTimeZone\", first_response_paused_seconds, resolution_paused_seconds, \
            wait_limit_exceeded_at AS \"wait_limit_exceeded_at: DateTimeWithTimeZone\"",
        next.id,
        next.status,
        next.paused_at,
        next.wait_limit_exceeded_at,
        next.first_response_paused_seconds,
        next.resolution_paused_seconds,
        next.first_response_due_at,
        next.resolution_due_at,
        next.resolved_at,
    )
    .fetch_one(&mut *txn)
    .await?;
    let kind = match to {
        "resolved" => "ticket_resolved",
        "closed" => "ticket_closed",
        _ => "ticket_status_changed",
    };
    Audit::record_on(
        &mut txn,
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
    streaming::emit_in(
        &mut txn,
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
    let mut txn = db::pool(&ctx).begin().await?;
    let ticket = rows::ticket_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    // Waiting for the customer already counted stays counted (CRM-R55).
    let paused = sla::Paused {
        first_response_seconds: ticket.first_response_paused_seconds,
        resolution_seconds: ticket.resolution_paused_seconds,
    };
    let deadlines = targets_for(&mut txn, &payload.priority)
        .await?
        .map(|(targets, calendar)| {
            sla::deadlines_paused(ticket.opened_at, targets, calendar.as_ref(), paused)
        });
    let from = ticket.priority.clone();
    let row = sqlx::query_as!(
        tickets::Model,
        "UPDATE tickets SET priority = $2, first_response_due_at = $3, resolution_due_at = $4 \
            WHERE id = $1 RETURNING created_at, updated_at, id, pid, contact_pid, account_pid, assignee_ref, title, \
            priority, channel, status, opened_at, first_response_due_at AS \"first_response_due_at: \
            DateTimeWithTimeZone\", resolution_due_at AS \"resolution_due_at: DateTimeWithTimeZone\", \
            first_responded_at AS \"first_responded_at: DateTimeWithTimeZone\", resolved_at AS \
            \"resolved_at: DateTimeWithTimeZone\", first_response_breached, resolution_breached, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\", board_position, paused_at AS \"paused_at: \
            DateTimeWithTimeZone\", first_response_paused_seconds, resolution_paused_seconds, \
            wait_limit_exceeded_at AS \"wait_limit_exceeded_at: DateTimeWithTimeZone\"",
        ticket.id,
        payload.priority,
        deadlines.as_ref().map(|d| d.first_response_due_at),
        deadlines.as_ref().map(|d| d.resolution_due_at),
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
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
    let now: DateTimeWithTimeZone = crate::clock::now().into();
    let pool = db::pool(&ctx);
    let open = rows::tickets_with_status(pool, &["open", "pending", "resolved"]).await?;
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
        let mut txn = pool.begin().await?;
        sqlx::query!(
            "UPDATE tickets SET first_response_breached = first_response_breached OR $2, \
             resolution_breached = resolution_breached OR $3 WHERE id = $1 AND deleted_at IS NULL",
            ticket.id,
            new_first,
            new_resolution,
        )
        .execute(&mut *txn)
        .await?;
        streaming::emit_in(
            &mut txn,
            "ticket",
            "sla_breached",
            &ticket.pid.to_string(),
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
    let auto_closed = sweep_closing(&ctx, &caller, now).await?;
    format::json(SlaSweep {
        new_breaches: breaches,
        new_wait_limits_exceeded: waited.exceeded,
        new_wait_limit_resolved: waited.resolved,
        new_wait_limit_escalated: waited.escalated,
        new_auto_closed: auto_closed,
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
    let pid = Uuid::new_v4();
    let mut txn = db::pool(&ctx).begin().await?;
    sqlx::query!(
        "INSERT INTO articles (pid, title, body, keywords, status, version) \
         VALUES ($1, $2, $3, $4, 'draft', 1)",
        pid,
        payload.title,
        payload.body,
        payload.keywords,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(&mut txn, "article", pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
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
    let mut txn = db::pool(&ctx).begin().await?;
    let article = rows::article_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    if article.status == "archived" {
        return Err(unprocessable("archived articles are read-only"));
    }
    let version = if article.status == "published" {
        article.version + 1
    } else {
        article.version
    };
    let row = sqlx::query_as!(
        crate::models::_entities::articles::Model,
        "UPDATE articles SET title = $2, body = $3, keywords = $4, version = $5 WHERE id = $1 \
            RETURNING created_at, updated_at, id, pid, title, body, keywords, status, version, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\"",
        article.id,
        payload.title,
        payload.body,
        payload.keywords,
        version,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "article",
        row.pid,
        "updated",
        caller.actor(),
        None,
    )
    .await?;
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
    let mut txn = db::pool(&ctx).begin().await?;
    let article = rows::article_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    lifecycle::check("article", lifecycle::ARTICLE, &article.status, &payload.to)
        .map_err(|e| unprocessable(&e.to_string()))?;
    let row = sqlx::query_as!(
        crate::models::_entities::articles::Model,
        "UPDATE articles SET status = $2 WHERE id = $1 RETURNING created_at, updated_at, id, pid, title, body, keywords, status, version, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\"",
        article.id,
        payload.to,
    )
    .fetch_one(&mut *txn)
    .await?;
    let kind = if payload.to == "published" {
        "article_published"
    } else {
        "updated"
    };
    Audit::record_on(&mut txn, "article", row.pid, kind, caller.actor(), None).await?;
    streaming::emit_in(
        &mut txn,
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
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&uri, 500)?;
    let (limit, offset) = page.bounds();
    // Search in SQL, before the page is cut: filtering after the cut would
    // hide matches that sit beyond the first page.
    let pattern = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| {
            format!(
                "%{}%",
                axa::cursor_pagination::escape_like(&q.to_lowercase())
            )
        });
    let rows = rows::articles_page(db::pool(&ctx), pattern.as_deref(), limit, offset).await?;
    let (rows, more) = page.split(rows);
    page.respond(rows, more)
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
    let pool = db::pool(&ctx);
    let ticket = find_ticket(pool, pid).await?;
    let published = rows::published_articles_by_id(pool).await?;
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

/// The public route for a customer's reply by e-mail (outside `/api`; see `auth::is_public_path`).
pub fn public_routes() -> Routes {
    Routes::new().add("/webhooks/inbound-email", post(inbound_email))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_is_t_and_eight_hex_digits_of_the_pid() {
        let pid = Uuid::parse_str("0a1b2c3d-1111-2222-3333-444455556666").unwrap();
        assert_eq!(reference(pid), "T-0a1b2c3d");
    }

    #[test]
    fn a_reply_is_found_by_the_reference_in_its_subject_however_it_is_dressed() {
        for subject in [
            "Re: Your request [T-0a1b2c3d]",
            "RE: [t-0A1B2C3D] printer",
            "Fwd: Re: Re: [T-0a1b2c3d] and more",
            "[T-zzzzzzzz] [T-0a1b2c3d]",
        ] {
            assert_eq!(
                reference_in(subject).as_deref(),
                Some("0a1b2c3d"),
                "{subject}"
            );
        }
        for subject in [
            "no reference",
            "T-0a1b2c3d without brackets",
            "[T-0a1b2c]",
            "[T-0a1b2c3d4]",
            "[T-0a1b2c3g]",
            "[CRM-0a1b2c3d]",
            "",
        ] {
            assert_eq!(reference_in(subject), None, "{subject}");
        }
    }

    #[test]
    fn the_sender_is_the_address_not_the_display_name() {
        assert_eq!(
            address_in("Ada Lovelace <Ada@Example.test>"),
            "ada@example.test"
        );
        assert_eq!(address_in("  ada@example.test "), "ada@example.test");
        assert_eq!(
            address_in("\"Ada <x@y.z>\" <ada@example.test>"),
            "ada@example.test"
        );
        assert_eq!(address_in("<ada@example.test>"), "ada@example.test");
    }
}
