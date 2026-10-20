//! Sales insights: stale-deal aging, follow-ups, pipeline hygiene, the executive pack and forecast trends.

use super::{
    AppContext, ColumnTrait, DaysQuery, Deserialize, EntityTrait, HeaderMap, MaybeAuthUser,
    PaginatorTrait, QueryFilter, QueryOrder, Response, Result, State, Uuid, WindowQuery,
    activities, audit_logs, auth, conditional, consent_events, days_or, deals, debug_handler,
    forecast_snapshots, leads, parse_instant, pipeline_stages, record_rejection, tickets,
};

/// `GET /api/insights/stale-deals?days=` — open deals by days in their
/// current stage, oldest first. Stage entry = the newest
/// `deal_stage_changed` audit for the deal, else the deal's creation
/// (never moved) — the derivation is served.
#[debug_handler]
pub(super) async fn stale_deals(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 14);
    let now = chrono::Utc::now();
    let open: Vec<deals::Model> = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let stage_names: std::collections::BTreeMap<Uuid, String> = live!(pipeline_stages, &ctx.db)
        .into_iter()
        .map(|stage| (stage.pid, stage.name))
        .collect();
    let moves = audit_logs::Entity::find()
        .filter(audit_logs::Column::Entity.eq("deal"))
        .filter(audit_logs::Column::Action.eq("deal_stage_changed"))
        .all(&ctx.db)
        .await?;
    let mut last_move: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    for row in &moves {
        let at = row.created_at.to_utc();
        let entry = last_move.entry(row.entity_pid).or_insert(at);
        if at > *entry {
            *entry = at;
        }
    }
    let mut rows: Vec<(i64, serde_json::Value)> = open
        .iter()
        .filter_map(|deal| {
            // Record-level read decision per deal (CRM-T32): a denied deal is
            // omitted, a masked one keeps its row without the amount.
            let obligations = auth::authorize_record(
                &caller,
                authentication_verifier::Action::Read,
                &auth::deal_resource_attrs(deal),
            )
            .ok()?;
            let entered = last_move
                .get(&deal.pid)
                .copied()
                .unwrap_or_else(|| deal.created_at.to_utc());
            let days_in_stage = (now - entered).num_days();
            let mut row = serde_json::json!({
                "pid": deal.pid,
                "name": deal.name,
                "stage": stage_names.get(&deal.stage_pid),
                "owner_ref": deal.owner_ref,
                "amount_minor": deal.amount_minor,
                "currency": deal.currency,
                "days_in_stage": days_in_stage,
                "stale": days_in_stage > threshold,
            });
            if obligations.iter().any(|o| o == "mask") {
                row = auth::mask_json(row, &["amount_minor"]);
            }
            Some((days_in_stage, row))
        })
        .collect();
    rows.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    let stale_count = rows.iter().filter(|(days, _)| *days > threshold).count();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "stage entry = newest deal_stage_changed audit, else deal creation \
                           (never moved); stale = days_in_stage over the threshold",
            "threshold_days": threshold,
            "open_deals": rows.len(),
            "stale_deals": stale_count,
            "deals": rows.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
        }),
    ))
}

/// Query for the follow-ups view: optional activity-kind filter (the
/// renewals convention: log renewals as due-dated `task` activities
/// and filter here).
#[derive(Debug, Deserialize)]
pub(super) struct FollowupsQuery {
    kind: Option<String>,
}

/// `GET /api/insights/followups?kind=` — activities carrying a
/// `due_on` and not `done`: the overdue list (oldest first, with age)
/// and the next 30 days, plus per-recorder counts (`actor_ref` is the
/// recorder — said, not assumed to be an assignee).
#[debug_handler]
pub(super) async fn followups(
    axum::extract::Query(query): axum::extract::Query<FollowupsQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let horizon = today + chrono::Days::new(30);
    let mut find = activities::Entity::find()
        .filter(activities::Column::DeletedAt.is_null())
        .filter(activities::Column::Done.eq(false))
        .filter(activities::Column::DueOn.is_not_null());
    if let Some(kind) = query.kind.as_deref() {
        find = find.filter(activities::Column::Kind.eq(kind));
    }
    let rows: Vec<activities::Model> = find.all(&ctx.db).await?;
    let view = |activity: &activities::Model| {
        serde_json::json!({
            "pid": activity.pid,
            "kind": activity.kind,
            "summary": activity.summary,
            "subject_kind": activity.subject_kind,
            "subject_pid": activity.subject_pid,
            "actor_ref": activity.actor_ref,
            "due_on": activity.due_on,
            "overdue_days": activity
                .due_on
                .filter(|due| *due < today)
                .map(|due| (today - due).num_days()),
        })
    };
    let mut overdue: Vec<&activities::Model> = rows
        .iter()
        .filter(|a| a.due_on.is_some_and(|due| due < today))
        .collect();
    overdue.sort_by_key(|a| a.due_on);
    let mut upcoming: Vec<&activities::Model> = rows
        .iter()
        .filter(|a| a.due_on.is_some_and(|due| due >= today && due <= horizon))
        .collect();
    upcoming.sort_by_key(|a| a.due_on);
    let mut per_recorder: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for activity in &rows {
        let key = activity
            .actor_ref
            .clone()
            .unwrap_or_else(|| "unattributed".to_string());
        *per_recorder.entry(key).or_default() += 1;
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "open follow-ups only (due_on set, not done); actor_ref is the \
                     recording actor, not necessarily an assignee",
            "overdue": overdue.iter().map(|a| view(a)).collect::<Vec<_>>(),
            "upcoming_30d": upcoming.iter().map(|a| view(a)).collect::<Vec<_>>(),
            "open_by_recorder": per_recorder,
        }),
    ))
}

/// `GET /api/insights/pipeline-hygiene?days=` — rule-disclosed
/// findings over the open pipeline: no invented health score, just
/// what each rule matched.
#[debug_handler]
pub(super) async fn pipeline_hygiene(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 14);
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let open: Vec<deals::Model> = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let lead_rows: Vec<leads::Model> = live!(leads, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let last_touch = |kind: &str, pid: Uuid| -> Option<chrono::DateTime<chrono::Utc>> {
        activity_rows
            .iter()
            .filter(|a| a.subject_kind == kind && a.subject_pid == pid)
            .map(|a| a.occurred_at.to_utc())
            .max()
    };
    let mut findings: Vec<serde_json::Value> = Vec::new();
    for deal in &open {
        let deal_ref = serde_json::json!({ "pid": deal.pid, "name": deal.name });
        if deal.amount_minor == 0 {
            findings.push(serde_json::json!({
                "rule": "open_deal_without_amount",
                "detail": "an open deal carries no amount (forecast blind spot)",
                "deal": deal_ref,
            }));
        }
        if deal.expected_close_on.is_none() {
            findings.push(serde_json::json!({
                "rule": "open_deal_without_expected_close",
                "detail": "an open deal has no expected close date",
                "deal": deal_ref,
            }));
        } else if deal.expected_close_on.is_some_and(|d| d < today) {
            findings.push(serde_json::json!({
                "rule": "open_deal_past_expected_close",
                "detail": "an open deal is past its expected close date",
                "deal": deal_ref,
                "expected_close_on": deal.expected_close_on,
            }));
        }
        let untouched = match last_touch("deal", deal.pid) {
            Some(at) => (now - at).num_days() > threshold,
            None => true,
        };
        if untouched {
            findings.push(serde_json::json!({
                "rule": "open_deal_without_recent_activity",
                "detail": format!("no recorded activity in {threshold} days"),
                "deal": deal_ref,
            }));
        }
    }
    for lead in lead_rows
        .iter()
        .filter(|l| matches!(l.status.as_str(), "new" | "contacted"))
    {
        let untouched = match last_touch("lead", lead.pid) {
            Some(at) => (now - at).num_days() > threshold,
            None => (now - lead.created_at.to_utc()).num_days() > threshold,
        };
        if untouched {
            findings.push(serde_json::json!({
                "rule": "unworked_lead",
                "detail": format!("a {} lead with no activity in {threshold} days", lead.status),
                "lead_pid": lead.pid,
                "score": lead.score,
            }));
        }
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "threshold_days": threshold,
            "findings": findings,
        }),
    ))
}

/// `GET /api/insights/executive?from=&to=` — the period sales pack:
/// deals won/lost in the window (per-currency won value, never
/// merged; lost reasons verbatim), new leads, tickets opened /
/// resolved (from status audits), activities logged, campaigns
/// started, and consent withdrawals.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over the period's facts
pub(super) async fn executive(
    axum::extract::Query(query): axum::extract::Query<WindowQuery>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
) -> Result<Response> {
    let masked = auth::aggregate_masked(&caller).map_err(record_rejection)?;
    let now = chrono::Utc::now();
    let to = query
        .to
        .as_deref()
        .and_then(|raw| parse_instant(raw, true))
        .unwrap_or(now);
    let from = query
        .from
        .as_deref()
        .and_then(|raw| parse_instant(raw, false))
        .unwrap_or(to - chrono::Days::new(30));
    let in_window = |at: chrono::DateTime<chrono::Utc>| at >= from && at <= to;

    let deal_rows: Vec<deals::Model> = live!(deals, &ctx.db);
    let mut won = 0usize;
    let mut lost = 0usize;
    let mut won_value: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut lost_reasons: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for deal in deal_rows
        .iter()
        .filter(|d| d.closed_at.is_some_and(|at| in_window(at.to_utc())))
    {
        if deal.won {
            won += 1;
            let entry = won_value.entry(deal.currency.clone()).or_default();
            *entry = entry.saturating_add(deal.amount_minor);
        } else {
            lost += 1;
            let reason = deal
                .lost_reason
                .clone()
                .unwrap_or_else(|| "(no reason recorded)".to_string());
            *lost_reasons.entry(reason).or_default() += 1;
        }
    }
    let new_leads = leads::Entity::find()
        .filter(leads::Column::DeletedAt.is_null())
        .filter(leads::Column::CreatedAt.gte(from))
        .filter(leads::Column::CreatedAt.lte(to))
        .count(&ctx.db)
        .await?;
    let tickets_opened = tickets::Entity::find()
        .filter(tickets::Column::DeletedAt.is_null())
        .filter(tickets::Column::OpenedAt.gte(from))
        .filter(tickets::Column::OpenedAt.lte(to))
        .count(&ctx.db)
        .await?;
    let audit_rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::CreatedAt.gte(from))
        .filter(audit_logs::Column::CreatedAt.lte(to))
        .all(&ctx.db)
        .await?;
    let tickets_resolved = audit_rows
        .iter()
        .filter(|row| row.entity == "ticket" && row.action == "resolved")
        .count();
    let campaigns_started = audit_rows
        .iter()
        .filter(|row| row.entity == "campaign" && row.action == "running")
        .count();
    let activities_logged = activities::Entity::find()
        .filter(activities::Column::DeletedAt.is_null())
        .filter(activities::Column::OccurredAt.gte(from))
        .filter(activities::Column::OccurredAt.lte(to))
        .count(&ctx.db)
        .await?;
    let consent_withdrawals = consent_events::Entity::find()
        .filter(consent_events::Column::Action.eq("withdrawn"))
        .filter(consent_events::Column::OccurredAt.gte(from))
        .filter(consent_events::Column::OccurredAt.lte(to))
        .count(&ctx.db)
        .await?;
    let mut body = serde_json::json!({
        "window": { "from": from, "to": to },
        "deals_won": won,
        "deals_lost": lost,
        "won_value_by_currency_minor": won_value,
        "lost_reasons": lost_reasons,
        "new_leads": new_leads,
        "tickets_opened": tickets_opened,
        "tickets_resolved": tickets_resolved,
        "campaigns_started": campaigns_started,
        "activities_logged": activities_logged,
        "consent_withdrawals": consent_withdrawals,
        "note": "per-currency won value is never merged or converted; \
                 tickets_resolved / campaigns_started derive from status audits",
    });
    if masked {
        body = auth::mask_json(body, &["won_value_by_currency_minor"]);
    }
    Ok(conditional(&headers, body))
}

/// `GET /api/insights/forecast-trends` — the stored forecast-snapshot
/// series, oldest first (cap 200). History is only ever what
/// `POST /api/forecast/snapshot` actually captured — no interpolation.
#[debug_handler]
pub(super) async fn forecast_trends(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
) -> Result<Response> {
    let masked = auth::aggregate_masked(&caller).map_err(record_rejection)?;
    let rows: Vec<forecast_snapshots::Model> = forecast_snapshots::Entity::find()
        .filter(forecast_snapshots::Column::DeletedAt.is_null())
        .order_by_asc(forecast_snapshots::Column::TakenOn)
        .all(&ctx.db)
        .await?;
    let series: Vec<serde_json::Value> = rows
        .iter()
        .take(200)
        .map(|row| {
            let entry = serde_json::json!({ "taken_on": row.taken_on, "totals": row.totals });
            if masked {
                auth::mask_json(entry, &["totals"])
            } else {
                entry
            }
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "stored snapshots only (POST /api/forecast/snapshot); no interpolated history",
            "series": series,
        }),
    ))
}
