//! Support insights: the SLA register and per-assignee workload.

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
            // Record-level read decision per ticket (CRM-T321), as the ticket list makes it: a
            // denied ticket is left out of the register, a masked one keeps its row without the
            // title. The workload counts still include it.
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
