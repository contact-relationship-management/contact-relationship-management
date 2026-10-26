//! Support insights: the SLA register and per-assignee workload.

use super::{
    AppContext, ColumnTrait, HeaderMap, QueryFilter, Response, Result, State, conditional,
    debug_handler, tickets,
};
use crate::models::live::FindLive;

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
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// `GET /api/insights/sla` — the breach register (open/pending tickets
/// past a due stamp, most-overdue first) and the per-assignee
/// workload (open counts, due-within-4h at-risk counts — the window
/// is disclosed).
#[debug_handler]
pub(super) async fn sla_register(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let at_risk_horizon = now + chrono::Duration::hours(4);
    let open: Vec<tickets::Model> = tickets::Entity::find_live()
        .filter(tickets::Column::Status.is_in(["open", "pending"]))
        .all(&ctx.db)
        .await?;
    let mut breaches: Vec<(i64, Breach)> = Vec::new();
    let mut workload: std::collections::BTreeMap<String, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for ticket in &open {
        let key = ticket
            .assignee_ref
            .clone()
            .unwrap_or_else(|| "unassigned".to_string());
        let entry = workload.entry(key).or_default();
        entry.0 += 1;
        let mut worst_overdue: Option<(&str, i64)> = None;
        // A ticket waiting for the customer has a stopped clock, so it is not running toward a
        // deadline (CRM-R55); one that was already late when paused stays late.
        let paused = ticket.paused_at.map(|at| at.to_utc());
        let now = paused.map_or(now, |at| at.min(now));
        for (which, due) in [
            ("first_response", ticket.first_response_due_at),
            ("resolution", ticket.resolution_due_at),
        ] {
            let Some(due) = due else { continue };
            let due = due.to_utc();
            if due < now {
                let hours = (now - due).num_hours();
                if worst_overdue.is_none_or(|(_, worst)| hours > worst) {
                    worst_overdue = Some((which, hours));
                }
            } else if paused.is_none() && due <= at_risk_horizon {
                entry.2 += 1;
            }
        }
        if let Some((which, hours)) = worst_overdue {
            entry.1 += 1;
            breaches.push((
                hours,
                Breach {
                    pid: ticket.pid,
                    title: ticket.title.clone(),
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
