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
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use super::{ensure_valid, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{activities, deals, leads, tickets};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::validation::Problems;

/// `PUT /api/deals/{pid}` body; absent fields stay as they are.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DealEditPayload {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    amount_minor: Option<i64>,
    #[serde(default)]
    expected_close_on: Option<chrono::NaiveDate>,
}

/// `PUT /api/leads/{pid}` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LeadEditPayload {
    #[serde(default)]
    display_name: Option<String>,
}

/// `PUT /api/tickets/{pid}` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TicketEditPayload {
    #[serde(default)]
    title: Option<String>,
}

/// `PUT /api/{deals,leads,tickets}/{pid}/position` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PositionPayload {
    /// 0-based place among the cards in the same column.
    position: i32,
}

/// `PUT /api/activities/{pid}` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivityEditPayload {
    #[serde(default)]
    due_on: Option<chrono::NaiveDate>,
    #[serde(default)]
    summary: Option<String>,
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
    ensure_valid(&problems.into_vec())?;
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
    let mut changed = Vec::new();
    let mut active: deals::ActiveModel = deal.into();
    if let Some(name) = payload.name {
        active.name = ActiveValue::set(name.trim().to_string());
        changed.push("name");
    }
    if let Some(amount) = payload.amount_minor {
        active.amount_minor = ActiveValue::set(amount);
        changed.push("amount_minor");
    }
    if let Some(day) = payload.expected_close_on {
        active.expected_close_on = ActiveValue::set(Some(day));
        changed.push("expected_close_on");
    }
    if changed.is_empty() {
        return Err(unprocessable("nothing to change"));
    }
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
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
    ensure_valid(&problems.into_vec())?;
    let lead = records::find_lead(&ctx.db, pid).await?;
    let txn = ctx.db.begin().await?;
    let mut active: leads::ActiveModel = lead.into();
    active.display_name = ActiveValue::set(name.trim().to_string());
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
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
    ensure_valid(&problems.into_vec())?;
    let ticket = records::find_ticket(&ctx.db, pid).await?;
    let txn = ctx.db.begin().await?;
    let mut active: tickets::ActiveModel = ticket.into();
    active.title = ActiveValue::set(title.trim().to_string());
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
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
    let column = deals::Entity::find()
        .filter(deals::Column::StagePid.eq(deal.stage_pid))
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .order_by_asc(deals::Column::KanbanPosition)
        .order_by_asc(deals::Column::Id)
        .all(&txn)
        .await?;
    let order: Vec<Uuid> = column.iter().map(|r| r.pid).collect();
    for (index, p) in new_order(&order, pid, payload.position).iter().enumerate() {
        if let Some(row) = column.iter().find(|r| r.pid == *p)
            && row.kanban_position != place(index)
        {
            let mut active: deals::ActiveModel = row.clone().into();
            active.kanban_position = ActiveValue::set(place(index));
            active.update(&txn).await?;
        }
    }
    Audit::record(&txn, "deal", pid, "reordered", caller.actor(), None).await?;
    txn.commit().await?;
    let row = records::find_deal(&ctx.db, pid).await?;
    format::json(row)
}

/// `PUT /api/leads/{pid}/position` — reorder a lead within its status column.
#[debug_handler]
async fn lead_position(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PositionPayload>,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let lead = leads::Entity::find()
        .filter(leads::Column::Pid.eq(pid))
        .filter(leads::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    let column = leads::Entity::find()
        .filter(leads::Column::Status.eq(lead.status.clone()))
        .filter(leads::Column::DeletedAt.is_null())
        .order_by_asc(leads::Column::BoardPosition)
        .order_by_desc(leads::Column::Score)
        .order_by_asc(leads::Column::Id)
        .all(&txn)
        .await?;
    let order: Vec<Uuid> = column.iter().map(|r| r.pid).collect();
    for (index, p) in new_order(&order, pid, payload.position).iter().enumerate() {
        if let Some(row) = column.iter().find(|r| r.pid == *p)
            && row.board_position != place(index)
        {
            let mut active: leads::ActiveModel = row.clone().into();
            active.board_position = ActiveValue::set(place(index));
            active.update(&txn).await?;
        }
    }
    Audit::record(&txn, "lead", pid, "reordered", caller.actor(), None).await?;
    txn.commit().await?;
    let row = records::find_lead(&ctx.db, pid).await?;
    format::json(row)
}

/// `PUT /api/tickets/{pid}/position` — reorder a ticket within its status column.
#[debug_handler]
async fn ticket_position(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<PositionPayload>,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let ticket = tickets::Entity::find()
        .filter(tickets::Column::Pid.eq(pid))
        .filter(tickets::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    let column = tickets::Entity::find()
        .filter(tickets::Column::Status.eq(ticket.status.clone()))
        .filter(tickets::Column::DeletedAt.is_null())
        .order_by_asc(tickets::Column::BoardPosition)
        .order_by_asc(tickets::Column::Id)
        .all(&txn)
        .await?;
    let order: Vec<Uuid> = column.iter().map(|r| r.pid).collect();
    for (index, p) in new_order(&order, pid, payload.position).iter().enumerate() {
        if let Some(row) = column.iter().find(|r| r.pid == *p)
            && row.board_position != place(index)
        {
            let mut active: tickets::ActiveModel = row.clone().into();
            active.board_position = ActiveValue::set(place(index));
            active.update(&txn).await?;
        }
    }
    Audit::record(&txn, "ticket", pid, "reordered", caller.actor(), None).await?;
    txn.commit().await?;
    let row = records::find_ticket(&ctx.db, pid).await?;
    format::json(row)
}

/// `PUT /api/activities/{pid}` — change a follow-up's due day or summary.
#[debug_handler]
async fn edit_activity(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ActivityEditPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    if let Some(summary) = &payload.summary {
        problems.require_text("summary", summary);
        problems.cap_text("summary", summary);
    }
    ensure_valid(&problems.into_vec())?;
    if payload.due_on.is_none() && payload.summary.is_none() {
        return Err(unprocessable("nothing to change"));
    }
    let row = activities::Entity::find()
        .filter(activities::Column::Pid.eq(pid))
        .filter(activities::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    if payload.due_on.is_some() && row.due_on.is_none() {
        return Err(unprocessable("this activity has no due day to move"));
    }
    if row.done {
        return Err(unprocessable("a finished activity cannot be changed"));
    }
    let from = row.due_on;
    let txn = ctx.db.begin().await?;
    let mut changed = Vec::new();
    let mut active: activities::ActiveModel = row.into();
    if let Some(day) = payload.due_on {
        active.due_on = ActiveValue::set(Some(day));
        changed.push("due_on");
    }
    if let Some(summary) = payload.summary {
        active.summary = ActiveValue::set(summary.trim().to_string().into());
        changed.push("summary");
    }
    let row = active.update(&txn).await?;
    let details = serde_json::json!({
        "fields": changed,
        "from_due_on": from.map(|d| d.to_string()),
        "to_due_on": row.due_on.map(|d| d.to_string()),
    });
    Audit::record(
        &txn,
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
