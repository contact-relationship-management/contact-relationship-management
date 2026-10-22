//! Outreach insights: contact cadence, engagement, the funnel and membership counts.

use super::{
    AppContext, ColumnTrait, DaysQuery, Deserialize, EntityTrait, HeaderMap, QueryFilter, Response,
    Result, State, Uuid, accounts, activities, audit_logs, conditional, contacts, days_or, deals,
    debug_handler, memberships, pipeline_stages, records, tickets,
};
use crate::models::live::FindLive;

/// `GET /api/insights/cadence?days=` — relationship-cadence aging:
/// contacts and accounts by days since their last recorded touch
/// (account touch = a direct account activity **or** one on any of
/// its contacts — disclosed), plus no-next-touch coverage (rows with
/// no open `due_on` activity).
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over contacts + accounts
pub(super) async fn cadence(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);

    let mut last_contact_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    let mut last_account_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    let mut has_next_contact: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    let account_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = contact_rows
        .iter()
        .map(|c| (c.pid, c.account_pid))
        .collect();
    for activity in &activity_rows {
        let at = activity.occurred_at.to_utc();
        let bump = |map: &mut std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>>,
                    pid: Uuid| {
            let entry = map.entry(pid).or_insert(at);
            if at > *entry {
                *entry = at;
            }
        };
        match activity.subject_kind.as_str() {
            "contact" => {
                bump(&mut last_contact_touch, activity.subject_pid);
                if let Some(Some(account_pid)) = account_of.get(&activity.subject_pid) {
                    bump(&mut last_account_touch, *account_pid);
                }
                if !activity.done && activity.due_on.is_some() {
                    has_next_contact.insert(activity.subject_pid);
                }
            }
            "account" => bump(&mut last_account_touch, activity.subject_pid),
            _ => {}
        }
    }
    let mut untouched_contacts: Vec<(i64, serde_json::Value)> = contact_rows
        .iter()
        .map(|contact| {
            let days = last_contact_touch.get(&contact.pid).map_or_else(
                || (now - contact.created_at.to_utc()).num_days(),
                |at| (now - *at).num_days(),
            );
            (
                days,
                serde_json::json!({
                    "pid": contact.pid,
                    "display_name": contact.display_name,
                    "stakeholder_role": contact.stakeholder_role,
                    "days_since_touch": days,
                    "has_next_touch": has_next_contact.contains(&contact.pid),
                }),
            )
        })
        .filter(|(days, _)| *days > threshold)
        .collect();
    untouched_contacts.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    let mut untouched_accounts: Vec<(i64, serde_json::Value)> = account_rows
        .iter()
        .map(|account| {
            let days = last_account_touch.get(&account.pid).map_or_else(
                || (now - account.created_at.to_utc()).num_days(),
                |at| (now - *at).num_days(),
            );
            (
                days,
                serde_json::json!({
                    "pid": account.pid,
                    "display_name": account.display_name,
                    "stakeholder_role": account.stakeholder_role,
                    "days_since_touch": days,
                }),
            )
        })
        .filter(|(days, _)| *days > threshold)
        .collect();
    untouched_accounts.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    let no_next_touch = contact_rows
        .iter()
        .filter(|c| !has_next_contact.contains(&c.pid))
        .count();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "touch = a recorded activity (account touch includes its \
                           contacts' activities); never-touched rows age from creation; \
                           no-next-touch = no open due-dated activity",
            "threshold_days": threshold,
            "untouched_contacts": untouched_contacts.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "untouched_accounts": untouched_accounts.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "contacts_without_next_touch": no_next_touch,
        }),
    ))
}

/// `GET /api/insights/engagement?days=` — the engagement workload:
/// touches per recorder per month and the per-kind mix over the
/// window, plus recorded-sentiment counts (recorded only — absent
/// sentiment is counted as unrecorded, never guessed).
#[debug_handler]
pub(super) async fn engagement(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window = days_or(query.days, 90);
    let now = chrono::Utc::now();
    let since = now - chrono::Days::new(u64::try_from(window).unwrap_or(90));
    let rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let mut per_recorder_month: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut per_kind: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut sentiment: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut total = 0usize;
    for activity in &rows {
        let at = activity.occurred_at.to_utc();
        if at < since || at > now {
            continue;
        }
        total += 1;
        let recorder = activity
            .actor_ref
            .clone()
            .unwrap_or_else(|| "unattributed".to_string());
        *per_recorder_month
            .entry(format!("{} {}", recorder, at.format("%Y-%m")))
            .or_default() += 1;
        *per_kind.entry(activity.kind.clone()).or_default() += 1;
        let key = activity
            .sentiment
            .clone()
            .unwrap_or_else(|| "unrecorded".to_string());
        *sentiment.entry(key).or_default() += 1;
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "window_days": window,
            "touches": total,
            "per_recorder_month": per_recorder_month,
            "per_kind": per_kind,
            "sentiment": sentiment,
            "note": "sentiment counts recorded declarations only; unrecorded stays unrecorded",
        }),
    ))
}

/// `GET /api/insights/funnel?pipeline=` — stage-entry counts and
/// step conversion through one pipeline, derived from recorded stage
/// audits (`to_stage`): entered(first stage) = deals created into the
/// pipeline; entered(later) = distinct deals whose audits reached that
/// stage. Ratios carry numerator and denominator.
#[debug_handler]
pub(super) async fn funnel(
    axum::extract::Query(query): axum::extract::Query<FunnelQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let pipeline = records::find_pipeline(&ctx.db, records::parse_pid(&query.pipeline)?).await?;
    let mut stages: Vec<pipeline_stages::Model> = live!(pipeline_stages, &ctx.db)
        .into_iter()
        .filter(|stage| stage.pipeline_pid == pipeline.pid)
        .collect();
    stages.sort_by_key(|stage| stage.position);
    let pipeline_deals: Vec<deals::Model> = deals::Entity::find_live()
        .filter(deals::Column::PipelinePid.eq(pipeline.pid))
        .all(&ctx.db)
        .await?;
    let deal_pids: std::collections::HashSet<Uuid> = pipeline_deals.iter().map(|d| d.pid).collect();
    let audit_rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::Entity.eq("deal"))
        .filter(audit_logs::Column::Action.is_in(["deal_stage_changed", "deal_won", "deal_lost"]))
        .all(&ctx.db)
        .await?;
    // Distinct deals that ever entered each stage, from to_stage.
    let mut entered: std::collections::BTreeMap<Uuid, std::collections::HashSet<Uuid>> =
        std::collections::BTreeMap::new();
    for row in &audit_rows {
        if !deal_pids.contains(&row.entity_pid) {
            continue;
        }
        let Some(to_stage) = row
            .snapshot
            .as_ref()
            .and_then(|snap| snap.get("to_stage"))
            .and_then(|v| v.as_str())
            .and_then(|raw| Uuid::parse_str(raw).ok())
        else {
            continue;
        };
        entered.entry(to_stage).or_default().insert(row.entity_pid);
    }
    let mut previous: Option<usize> = None;
    let mut rows: Vec<serde_json::Value> = Vec::new();
    for (index, stage) in stages.iter().enumerate() {
        let count = if index == 0 {
            pipeline_deals.len()
        } else {
            entered
                .get(&stage.pid)
                .map_or(0, std::collections::HashSet::len)
        };
        let conversion = previous.map(|prev| {
            #[allow(clippy::cast_precision_loss)] // display ratio, not money math
            let value = if prev == 0 {
                serde_json::Value::Null
            } else {
                serde_json::json!(count as f64 / prev as f64)
            };
            serde_json::json!({
                "numerator": count,
                "denominator": prev,
                "value": value,
            })
        });
        rows.push(serde_json::json!({
            "stage": stage.name,
            "position": stage.position,
            "is_won": stage.is_won,
            "is_lost": stage.is_lost,
            "entered": count,
            "conversion_from_previous": conversion,
        }));
        if !stage.is_lost {
            previous = Some(count);
        }
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "pipeline": { "pid": pipeline.pid, "name": pipeline.name },
            "derivation": "entered(first stage) = deals created into the pipeline; \
                           entered(later stages) = distinct deals with a recorded \
                           to_stage audit; conversion skips lost stages in the chain",
            "stages": rows,
        }),
    ))
}

/// Query for the funnel: the pipeline pid.
#[derive(Debug, Deserialize)]
pub(super) struct FunnelQuery {
    pipeline: String,
}

/// `GET /api/insights/members?days=` — confederation member-account
/// health: contacts, last touch, open follow-ups, open tickets, and
/// membership state per account, with the silent list (no touch in
/// the window).
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over the account estate
pub(super) async fn members(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let ticket_rows: Vec<tickets::Model> = tickets::Entity::find_live()
        .filter(tickets::Column::Status.is_in(["open", "pending"]))
        .all(&ctx.db)
        .await?;
    let membership_rows: Vec<memberships::Model> = live!(memberships, &ctx.db);
    let membership_of: std::collections::BTreeMap<Uuid, &memberships::Model> =
        membership_rows.iter().map(|m| (m.account_pid, m)).collect();
    let account_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = contact_rows
        .iter()
        .map(|c| (c.pid, c.account_pid))
        .collect();
    let mut last_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    let mut open_followups: std::collections::BTreeMap<Uuid, usize> =
        std::collections::BTreeMap::new();
    for activity in &activity_rows {
        let at = activity.occurred_at.to_utc();
        let account_pid = match activity.subject_kind.as_str() {
            "account" => Some(activity.subject_pid),
            "contact" => account_of.get(&activity.subject_pid).copied().flatten(),
            _ => None,
        };
        let Some(account_pid) = account_pid else {
            continue;
        };
        let entry = last_touch.entry(account_pid).or_insert(at);
        if at > *entry {
            *entry = at;
        }
        if !activity.done && activity.due_on.is_some() {
            *open_followups.entry(account_pid).or_default() += 1;
        }
    }
    let mut open_tickets: std::collections::BTreeMap<Uuid, usize> =
        std::collections::BTreeMap::new();
    for ticket in &ticket_rows {
        if let Some(account_pid) = ticket.account_pid {
            *open_tickets.entry(account_pid).or_default() += 1;
        }
    }
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let mut silent = 0usize;
    for account in &account_rows {
        let contacts_count = contact_rows
            .iter()
            .filter(|c| c.account_pid == Some(account.pid))
            .count();
        let days = last_touch.get(&account.pid).map_or_else(
            || (now - account.created_at.to_utc()).num_days(),
            |at| (now - *at).num_days(),
        );
        let is_silent = days > threshold;
        if is_silent {
            silent += 1;
        }
        let membership = membership_of.get(&account.pid).map(|m| {
            serde_json::json!({ "status": m.status, "joined_on": m.joined_on,
                                "renewal_on": m.renewal_on })
        });
        rows.push(serde_json::json!({
            "pid": account.pid,
            "display_name": account.display_name,
            "tier": account.tier,
            "stakeholder_role": account.stakeholder_role,
            "membership": membership,
            "contacts": contacts_count,
            "days_since_touch": days,
            "silent": is_silent,
            "open_followups": open_followups.get(&account.pid).copied().unwrap_or(0),
            "open_tickets": open_tickets.get(&account.pid).copied().unwrap_or(0),
        }));
    }
    rows.sort_by_key(|row| std::cmp::Reverse(row["days_since_touch"].as_i64().unwrap_or(0)));
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "account touch includes its contacts' activities; \
                           never-touched accounts age from creation",
            "threshold_days": threshold,
            "accounts": rows,
            "silent_accounts": silent,
        }),
    ))
}
