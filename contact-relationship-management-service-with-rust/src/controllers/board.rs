//! Board edits (CRM-T55): change a card in place, reorder it within its
//! column, and move a follow-up to another day.
//!
//! * `PUT /api/deals/{pid}` — name, amount, expected close (open deals only).
//! * `PUT /api/leads/{pid}` — display name.
//! * `PUT /api/tickets/{pid}` — title.
//! * `PUT /api/{deals,leads,tickets}/{pid}/position` — the 0-based place among
//!   the cards in the same column; the other cards are renumbered.
//! * `PUT /api/activities/{pid}` — due date and summary; a rescheduled task is
//!   audited with the old and new day.
//!
//! Every change is audited with the names of the fields that changed, never
//! their values, except a due date (a day, not personal data).

use crate::controllers::PidPath;
use crate::json::Json;
use axa::prelude::*;
use serde::Deserialize;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::db;
use crate::models::_entities::{activities, deals, leads, tickets};
use crate::models::DateTimeWithTimeZone;
use crate::models::audit_logs::Model as Audit;
use crate::models::encrypted::EncryptedText;
use crate::models::rows;
use crate::validation::Problems;

/// `PUT /api/deals/{pid}` body; absent fields stay as they are.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DealEditPayload {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    amount_minor: Option<i64>,
    #[serde(default)]
    expected_close_on: Option<chrono::NaiveDate>,
}

/// `PUT /api/leads/{pid}` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadEditPayload {
    #[serde(default)]
    display_name: Option<String>,
}

/// `PUT /api/tickets/{pid}` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TicketEditPayload {
    #[serde(default)]
    title: Option<String>,
}

/// `PUT /api/{deals,leads,tickets}/{pid}/position` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PositionPayload {
    /// 0-based place among the cards in the same column.
    position: i32,
}

/// `PUT /api/activities/{pid}` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityEditPayload {
    #[serde(default)]
    due_on: Option<chrono::NaiveDate>,
    #[serde(default)]
    summary: Option<String>,
    /// Who the task is for (a `person:` URN, CRM-D18); tasks only (CRM-R32).
    #[serde(default)]
    assignee_ref: Option<String>,
    /// When to remind the assignee; tasks only. A new time makes a new reminder possible.
    #[serde(default)]
    remind_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

/// The column's pids with `pid` moved to `position` (0-based; clamped to the
/// column). Positions may have gaps or repeats (they start at 0), so callers
/// renumber the whole column from the result.
fn new_order(order: &[Uuid], pid: Uuid, position: i32) -> Vec<Uuid> {
    let mut next: Vec<Uuid> = order.iter().copied().filter(|p| *p != pid).collect();
    let at = usize::try_from(position.max(0))
        .unwrap_or(0)
        .min(next.len());
    next.insert(at, pid);
    next
}

/// A column index as a stored place.
fn place(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

/// The cards of a column, by `id`, whose stored place is not the one `order` gives them.
fn moves(column: &[(i32, Uuid, i32)], pid: Uuid, position: i32) -> Vec<(i32, i32)> {
    let order: Vec<Uuid> = column.iter().map(|(_, p, _)| *p).collect();
    new_order(&order, pid, position)
        .iter()
        .enumerate()
        .filter_map(|(index, p)| {
            column
                .iter()
                .find(|(_, q, _)| q == p)
                .filter(|(_, _, at)| *at != place(index))
                .map(|(id, _, _)| (*id, place(index)))
        })
        .collect()
}

/// The open deal `pid`, locked, that `caller` may change.
async fn open_deal_to_change(
    conn: &mut PgConnection,
    caller: &MaybeAuthUser,
    pid: Uuid,
) -> Result<deals::Model> {
    let deal = rows::deal_for_update(conn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    auth::authorize_record(
        caller,
        authentication_verifier::Action::Write,
        &auth::deal_resource_attrs(&deal),
    )
    .map_err(record_rejection)?;
    if deal.closed_at.is_some() {
        return Err(unprocessable("deal is closed — reopen it first"));
    }
    Ok(deal)
}

/// `PUT /api/deals/{pid}` — edit an open deal in place.
#[debug_handler]
async fn edit_deal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<DealEditPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    if let Some(name) = &payload.name {
        problems.require_text("name", name);
    }
    if payload.amount_minor.is_some_and(|amount| amount < 0) {
        problems.push("amount_minor must not be negative");
    }
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let deal = open_deal_to_change(&mut txn, &caller, pid).await?;
    let mut changed = Vec::new();
    let name = match payload.name {
        Some(name) => {
            changed.push("name");
            name.trim().to_string()
        }
        None => deal.name,
    };
    let amount = match payload.amount_minor {
        Some(amount) => {
            changed.push("amount_minor");
            amount
        }
        None => deal.amount_minor,
    };
    let close_on = match payload.expected_close_on {
        Some(day) => {
            changed.push("expected_close_on");
            Some(day)
        }
        None => deal.expected_close_on,
    };
    if changed.is_empty() {
        return Err(unprocessable("nothing to change"));
    }
    let row = sqlx::query_as!(
        deals::Model,
        "UPDATE deals SET name = $2, amount_minor = $3, expected_close_on = $4 WHERE id = $1 \
            RETURNING created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
            pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
            source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at",
        deal.id,
        name,
        amount,
        close_on,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "deal",
        row.pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "fields": changed })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `PUT /api/leads/{pid}` — edit a lead in place.
#[debug_handler]
async fn edit_lead(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<LeadEditPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    let Some(name) = payload.display_name else {
        return Err(unprocessable("nothing to change"));
    };
    problems.require_text("display_name", &name);
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let row = sqlx::query_as!(
        leads::Model,
        "UPDATE leads SET display_name = $2 WHERE pid = $1 AND deleted_at IS NULL \
            RETURNING created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
            email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\", board_position, owner_ref",
        pid,
        name.trim(),
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    Audit::record_on(
        &mut txn,
        "lead",
        row.pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "fields": ["display_name"] })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `PUT /api/tickets/{pid}` — edit a ticket in place.
#[debug_handler]
async fn edit_ticket(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<TicketEditPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    let Some(title) = payload.title else {
        return Err(unprocessable("nothing to change"));
    };
    problems.require_text("title", &title);
    problems.finish()?;
    let mut txn = db::pool(&ctx).begin().await?;
    let row = sqlx::query_as!(
        tickets::Model,
        "UPDATE tickets SET title = $2 WHERE pid = $1 AND deleted_at IS NULL \
            RETURNING created_at, updated_at, id, pid, contact_pid, account_pid, assignee_ref, title, \
            priority, channel, status, opened_at, first_response_due_at AS \"first_response_due_at: \
            DateTimeWithTimeZone\", resolution_due_at AS \"resolution_due_at: DateTimeWithTimeZone\", \
            first_responded_at AS \"first_responded_at: DateTimeWithTimeZone\", resolved_at AS \
            \"resolved_at: DateTimeWithTimeZone\", first_response_breached, resolution_breached, \
            deleted_at AS \"deleted_at: DateTimeWithTimeZone\", board_position, paused_at AS \"paused_at: \
            DateTimeWithTimeZone\", first_response_paused_seconds, resolution_paused_seconds, \
            wait_limit_exceeded_at AS \"wait_limit_exceeded_at: DateTimeWithTimeZone\"",
        pid,
        title.trim(),
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    Audit::record_on(
        &mut txn,
        "ticket",
        row.pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "fields": ["title"] })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `PUT /api/deals/{pid}/position` — reorder an open deal within its stage.
#[debug_handler]
async fn deal_position(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PositionPayload>,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let mut txn = pool.begin().await?;
    let deal = open_deal_to_change(&mut txn, &caller, pid).await?;
    let column: Vec<_> = rows::deal_column(&mut *txn, deal.stage_pid)
        .await?
        .into_iter()
        .map(|r| (r.id, r.pid, r.kanban_position))
        .collect();
    for (id, at) in moves(&column, pid, payload.position) {
        sqlx::query!(
            "UPDATE deals SET kanban_position = $2 WHERE id = $1 AND deleted_at IS NULL",
            id,
            at
        )
        .execute(&mut *txn)
        .await?;
    }
    Audit::record_on(&mut txn, "deal", pid, "reordered", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(rows::deal(pool, pid).await?.ok_or(Error::NotFound)?)
}

/// `PUT /api/leads/{pid}/position` — reorder a lead within its status column.
#[debug_handler]
async fn lead_position(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PositionPayload>,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let mut txn = pool.begin().await?;
    let lead = rows::lead_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    let column: Vec<_> = rows::lead_column(&mut *txn, &lead.status)
        .await?
        .into_iter()
        .map(|r| (r.id, r.pid, r.board_position))
        .collect();
    for (id, at) in moves(&column, pid, payload.position) {
        sqlx::query!(
            "UPDATE leads SET board_position = $2 WHERE id = $1 AND deleted_at IS NULL",
            id,
            at
        )
        .execute(&mut *txn)
        .await?;
    }
    Audit::record_on(&mut txn, "lead", pid, "reordered", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(rows::lead(pool, pid).await?.ok_or(Error::NotFound)?)
}

/// `PUT /api/tickets/{pid}/position` — reorder a ticket within its status column.
#[debug_handler]
async fn ticket_position(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PositionPayload>,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let mut txn = pool.begin().await?;
    let ticket = rows::ticket_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    let column: Vec<_> = rows::ticket_column(&mut *txn, &ticket.status)
        .await?
        .into_iter()
        .map(|r| (r.id, r.pid, r.board_position))
        .collect();
    for (id, at) in moves(&column, pid, payload.position) {
        sqlx::query!(
            "UPDATE tickets SET board_position = $2 WHERE id = $1 AND deleted_at IS NULL",
            id,
            at
        )
        .execute(&mut *txn)
        .await?;
    }
    Audit::record_on(&mut txn, "ticket", pid, "reordered", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(rows::ticket(pool, pid).await?.ok_or(Error::NotFound)?)
}

/// `PUT /api/activities/{pid}` — change a follow-up's due day or summary.
#[debug_handler]
async fn edit_activity(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(mut payload): Json<ActivityEditPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    if let Some(summary) = &payload.summary {
        problems.require_text("summary", summary);
        problems.cap_text("summary", summary);
    }
    payload.assignee_ref = crate::rules::user_ref::normalise_opt(payload.assignee_ref.take());
    problems.user_opt("assignee_ref", payload.assignee_ref.as_deref());
    problems.finish()?;
    if payload.due_on.is_none()
        && payload.summary.is_none()
        && payload.assignee_ref.is_none()
        && payload.remind_at.is_none()
    {
        return Err(unprocessable("nothing to change"));
    }
    let mut txn = db::pool(&ctx).begin().await?;
    let row = rows::activity_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    if payload.due_on.is_some() && row.due_on.is_none() {
        return Err(unprocessable("this activity has no due day to move"));
    }
    if (payload.assignee_ref.is_some() || payload.remind_at.is_some()) && row.kind != "task" {
        return Err(unprocessable("only a task has an assignee or a reminder"));
    }
    if row.done {
        return Err(unprocessable("a finished activity cannot be changed"));
    }
    let from = row.due_on;
    let mut changed = Vec::new();
    let mut next = row;
    if let Some(day) = payload.due_on {
        next.due_on = Some(day);
        changed.push("due_on");
    }
    if let Some(summary) = payload.summary {
        next.summary = summary.trim().to_string().into();
        changed.push("summary");
    }
    if let Some(assignee) = payload.assignee_ref {
        next.assignee_ref = Some(assignee);
        changed.push("assignee_ref");
    }
    if let Some(at) = payload.remind_at {
        next.remind_at = Some(at);
        changed.push("remind_at");
    }
    let remind_at: Option<DateTimeWithTimeZone> = next.remind_at;
    // `summary` is sealed: binding `EncryptedText` seals it on the way in (CRM-T41).
    let row = sqlx::query_as!(
        activities::Model,
        "UPDATE activities SET due_on = $2, summary = $3, assignee_ref = $4, remind_at = $5 \
            WHERE id = $1 RETURNING created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
            actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
            sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\"",
        next.id,
        next.due_on,
        EncryptedText(next.summary.0.clone()) as _,
        next.assignee_ref,
        remind_at,
    )
    .fetch_one(&mut *txn)
    .await?;
    let details = serde_json::json!({
        "fields": changed,
        "from_due_on": from.map(|d| d.to_string()),
        "to_due_on": row.due_on.map(|d| d.to_string()),
    });
    Audit::record_on(
        &mut txn,
        "activity",
        row.pid,
        if payload.due_on.is_some() {
            "rescheduled"
        } else {
            "updated"
        },
        caller.actor(),
        Some(details),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// The board-edit routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/deals/{pid}", put(edit_deal))
        .add("/deals/{pid}/position", put(deal_position))
        .add("/leads/{pid}", put(edit_lead))
        .add("/leads/{pid}/position", put(lead_position))
        .add("/tickets/{pid}", put(edit_ticket))
        .add("/tickets/{pid}/position", put(ticket_position))
        .add("/activities/{pid}", put(edit_activity))
}

#[cfg(test)]
mod tests {
    use super::new_order;
    use uuid::Uuid;

    fn ids(n: usize) -> Vec<Uuid> {
        (0..n).map(|_| Uuid::new_v4()).collect()
    }

    #[test]
    fn moving_a_card_down_puts_it_after_the_ones_it_passed() {
        let o = ids(4);
        assert_eq!(new_order(&o, o[0], 2), vec![o[1], o[2], o[0], o[3]]);
    }

    #[test]
    fn moving_to_the_same_place_keeps_the_order() {
        let o = ids(3);
        assert_eq!(new_order(&o, o[1], 1), o);
    }

    #[test]
    fn a_position_past_the_end_goes_last_and_a_negative_one_first() {
        let o = ids(3);
        assert_eq!(new_order(&o, o[0], 99), vec![o[1], o[2], o[0]]);
        assert_eq!(new_order(&o, o[2], -5), vec![o[2], o[0], o[1]]);
    }

    #[test]
    fn a_card_not_in_the_column_is_inserted() {
        let o = ids(2);
        let extra = Uuid::new_v4();
        assert_eq!(new_order(&o, extra, 1), vec![o[0], extra, o[1]]);
    }
}
