//! Support insights: the SLA register and per-assignee workload, and the tickets nothing has touched
//! for a while.

use super::{AppContext, HeaderMap, Response, Result, State, conditional, debug_handler, tickets};
use crate::arith::OrOverflow;
use crate::arith::Tally;

// Each view carries `as_of: Option<..>`, left `None` here: [`conditional`] stamps it after the
// `ETag` is taken, and the schema says it is always there.

/// An open ticket past a due stamp.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "SlaBreach")]
pub(crate) struct Breach {
    pid: uuid::Uuid,
    title: String,
    priority: String,
    status: String,
    assignee_ref: Option<String>,
    /// The worst deadline missed: `first_response` or `resolution`.
    breached: &'static str,
    overdue_hours: i64,
}

/// One assignee's open load.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "SlaWorkload")]
pub(crate) struct Workload {
    assignee_ref: String,
    open: usize,
    breached: usize,
    at_risk_4h: usize,
}

/// `GET /api/insights/sla`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct SlaRegister {
    derivation: &'static str,
    breaches: Vec<Breach>,
    workload: Vec<Workload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "crate::time::Timestamp")]
    as_of: Option<crate::time::Timestamp>,
}

/// `GET /api/insights/sla` — the breach register (open/pending tickets
/// past a due stamp, most-overdue first) and the per-assignee
/// workload (open counts, due-within-4h at-risk counts — the window
/// is disclosed).
#[debug_handler]
pub(super) async fn sla_register(
    State(ctx): State<AppContext>,
    caller: crate::auth::MaybeAuthUser,
    headers: HeaderMap,
) -> Result<Response> {
    let now = crate::clock::now();
    let at_risk_horizon = now
        .checked_add(jiff::SignedDuration::from_hours(4))
        .or_overflow("at_risk_horizon")?;
    let open: Vec<tickets::Model> =
        crate::models::rows::open_tickets(crate::db::pool(&ctx)).await?;
    let mut breaches: Vec<(i64, Breach)> = Vec::new();
    let mut workload: std::collections::BTreeMap<String, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for ticket in &open {
        // Record-level read decision per ticket (CRM-T321), as the ticket list makes it: a denied
        // ticket is in neither the register nor the workload counts, so a count does not reveal it
        // (owner's decision, 2026-11-26); a masked one keeps its row without the title.
        let Ok(obligations) = crate::auth::authorize_record(
            &caller,
            authentication_verifier::Action::Read,
            &crate::auth::ticket_resource_attrs(ticket),
        ) else {
            continue;
        };
        let key = ticket
            .assignee_ref
            .clone()
            .unwrap_or_else(|| "unassigned".to_string());
        let entry = workload.entry(key).or_default();
        entry.0 = entry.0.tally(1);
        let mut worst_overdue: Option<(&str, i64)> = None;
        // A ticket waiting for the customer has a stopped clock, so it is not running toward a
        // deadline (CRM-R55); one that was already late when paused stays late.
        let now = ticket.paused_at.map_or(now, |at| at.min(now));
        for (which, due) in [
            ("first_response", ticket.first_response_due_at),
            ("resolution", ticket.resolution_due_at),
        ] {
            let Some(due) = due else { continue };
            if due < now {
                let hours = now.duration_since(due).as_hours();
                if worst_overdue.is_none_or(|(_, worst)| hours > worst) {
                    worst_overdue = Some((which, hours));
                }
            } else if ticket.paused_at.is_none() && due <= at_risk_horizon {
                entry.2 = entry.2.tally(1);
            }
        }
        if let Some((which, hours)) = worst_overdue {
            entry.1 = entry.1.tally(1);
            let title = if obligations.iter().any(|o| o == "mask") {
                crate::auth::MASKED.to_string()
            } else {
                ticket.title.clone()
            };
            breaches.push((
                hours,
                Breach {
                    pid: ticket.pid,
                    title,
                    priority: ticket.priority.clone(),
                    status: ticket.status.clone(),
                    assignee_ref: ticket.assignee_ref.clone(),
                    breached: which,
                    overdue_hours: hours,
                },
            ));
        }
    }
    breaches.sort_by_key(|(hours, _)| std::cmp::Reverse(*hours));
    let workload_view: Vec<Workload> = workload
        .into_iter()
        .map(|(assignee_ref, (open, breached, at_risk_4h))| Workload {
            assignee_ref,
            open,
            breached,
            at_risk_4h,
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(SlaRegister {
            derivation: "breach = an open/pending ticket past a due stamp (worst deadline \
                         reported); at_risk = a deadline within the disclosed 4h window",
            breaches: breaches.into_iter().map(|(_, v)| v).collect(),
            workload: workload_view,
            as_of: None,
        }),
    ))
}

/// An open or pending ticket nothing has touched for a while (CRM-T317).
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "StuckTicket")]
pub(crate) struct Stuck {
    pid: uuid::Uuid,
    title: String,
    priority: String,
    status: String,
    assignee_ref: Option<String>,
    /// The latest of the ticket's own last change, its last activity and its last handover.
    #[schemars(with = "crate::time::Timestamp")]
    last_touched_at: crate::time::Timestamp,
    /// Whole days since then.
    idle_days: i64,
}

/// `GET /api/insights/stuck-tickets`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct StuckTickets {
    derivation: &'static str,
    days: i64,
    tickets: Vec<Stuck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "crate::time::Timestamp")]
    as_of: Option<crate::time::Timestamp>,
}

/// `GET /api/insights/stuck-tickets?days=` — the open and pending tickets nothing has touched for
/// `days` (7 by default, 1 to 365) or more, the longest idle first (CRM-T317). A touch is a change
/// to the ticket, an activity on it, or a handover; the list names tickets, never ranks the people
/// who hold them. Each ticket takes its record-level read decision (CRM-T321): one the caller may not
/// read is left out, one read masked shows no title.
#[debug_handler]
pub(super) async fn stuck_tickets(
    axa::extract::Query(query): axa::extract::Query<super::DaysQuery>,
    State(ctx): State<AppContext>,
    caller: crate::auth::MaybeAuthUser,
    headers: HeaderMap,
) -> Result<Response> {
    let days = super::days_or(query.days, 7)?;
    let pool = crate::db::pool(&ctx);
    let now = crate::clock::now();
    let open = crate::models::rows::open_tickets(pool).await?;
    let touched: std::collections::HashMap<uuid::Uuid, crate::time::Timestamp> = sqlx::query!(
        "SELECT t.pid AS \"pid!\", GREATEST(t.updated_at, \
           (SELECT max(a.occurred_at) FROM activities a WHERE a.subject_kind = 'ticket' \
              AND a.subject_pid = t.pid AND a.deleted_at IS NULL), \
           (SELECT max(m.created_at) FROM ticket_movements m WHERE m.ticket_pid = t.pid)) \
           AS \"last!: crate::models::DateTimeWithTimeZone\" \
         FROM tickets t WHERE t.deleted_at IS NULL AND t.status IN ('open', 'pending')",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| (row.pid, row.last))
    .collect();
    let mut stuck: Vec<Stuck> = Vec::new();
    for ticket in &open {
        let Some(&last) = touched.get(&ticket.pid) else {
            continue; // changed status since the first read
        };
        let idle_days = crate::time::whole_days(now.duration_since(last));
        if idle_days < days {
            continue;
        }
        let Ok(obligations) = crate::auth::authorize_record(
            &caller,
            authentication_verifier::Action::Read,
            &crate::auth::ticket_resource_attrs(ticket),
        ) else {
            continue;
        };
        let title = if obligations.iter().any(|o| o == "mask") {
            crate::auth::MASKED.to_string()
        } else {
            ticket.title.clone()
        };
        stuck.push(Stuck {
            pid: ticket.pid,
            title,
            priority: ticket.priority.clone(),
            status: ticket.status.clone(),
            assignee_ref: ticket.assignee_ref.clone(),
            last_touched_at: last,
            idle_days,
        });
    }
    stuck.sort_by_key(|s| (s.last_touched_at, s.pid));
    Ok(conditional(
        &headers,
        serde_json::json!(StuckTickets {
            derivation: "stuck = an open or pending ticket whose latest change, activity or \
                         handover is at least `days` whole days old; longest idle first",
            days,
            tickets: stuck,
            as_of: None,
        }),
    ))
}
