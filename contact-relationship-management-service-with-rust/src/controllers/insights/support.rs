//! Support insights: the SLA register and per-assignee workload.

use super::{
    AppContext, ColumnTrait, HeaderMap, QueryFilter, Response, Result, State, conditional,
    debug_handler, tickets,
};
use crate::models::live::FindLive;

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
    let mut breaches: Vec<(i64, serde_json::Value)> = Vec::new();
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
            } else if due <= at_risk_horizon {
                entry.2 += 1;
            }
        }
        if let Some((which, hours)) = worst_overdue {
            entry.1 += 1;
            breaches.push((
                hours,
                serde_json::json!({
                    "pid": ticket.pid,
                    "title": ticket.title,
                    "priority": ticket.priority,
                    "status": ticket.status,
                    "assignee_ref": ticket.assignee_ref,
                    "breached": which,
                    "overdue_hours": hours,
                }),
            ));
        }
    }
    breaches.sort_by_key(|(hours, _)| std::cmp::Reverse(*hours));
    let workload_view: Vec<serde_json::Value> = workload
        .iter()
        .map(|(assignee, (open, breached, at_risk))| {
            serde_json::json!({
                "assignee_ref": assignee,
                "open": open,
                "breached": breached,
                "at_risk_4h": at_risk,
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "breach = an open/pending ticket past a due stamp (worst deadline \
                           reported); at_risk = a deadline within the disclosed 4h window",
            "breaches": breaches.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "workload": workload_view,
        }),
    ))
}
