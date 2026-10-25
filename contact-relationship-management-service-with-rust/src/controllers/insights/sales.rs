//! Sales insights: stale-deal aging, follow-ups, pipeline hygiene, the executive pack and forecast trends.

use super::{
    AppContext, ColumnTrait, DaysQuery, Deserialize, EntityTrait, HeaderMap, MaybeAuthUser,
    PaginatorTrait, QueryFilter, QueryOrder, Response, Result, State, Uuid, WindowQuery,
    activities, audit_logs, auth, conditional, consent_events, days_or, deals, debug_handler,
    forecast_snapshots, leads, parse_instant, pipeline_stages, record_rejection, tickets,
};
use crate::models::live::FindLive;

// Each view carries `as_of: Option<..>`, left `None` here: [`conditional`] stamps it after the
// `ETag` is taken, and the schema says it is always there.

/// One open deal in the stale-deals view. The amount is `null` when masked.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "StaleDeal")]
pub(crate) struct StaleDeal {
    pid: Uuid,
    name: String,
    stage: Option<String>,
    owner_ref: Option<String>,
    amount_minor: Option<i64>,
    currency: String,
    days_in_stage: i64,
    stale: bool,
}

/// `GET /api/insights/stale-deals`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[allow(clippy::struct_field_names)] // `stale_deals` is the response member clients read
pub(crate) struct StaleDeals {
    derivation: &'static str,
    threshold_days: i64,
    open_deals: usize,
    stale_deals: usize,
    deals: Vec<StaleDeal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// An activity with a due day, in the follow-ups view.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct Followup {
    pid: Uuid,
    kind: String,
    summary: String,
    subject_kind: String,
    subject_pid: Uuid,
    actor_ref: Option<String>,
    /// Always set: only activities with a due day are listed.
    #[schemars(with = "chrono::NaiveDate")]
    due_on: Option<chrono::NaiveDate>,
    overdue_days: Option<i64>,
}

/// `GET /api/insights/followups`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct Followups {
    note: &'static str,
    overdue: Vec<Followup>,
    upcoming_30d: Vec<Followup>,
    open_by_recorder: std::collections::BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// The deal a hygiene finding is about.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "HygieneDeal")]
pub(crate) struct HygieneDeal {
    pid: Uuid,
    name: String,
}

/// One rule that matched. Which of `deal`, `lead_pid`, `score` and `expected_close_on` appear depends on the rule.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "HygieneFinding")]
pub(crate) struct HygieneFinding {
    rule: &'static str,
    detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    deal: Option<HygieneDeal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lead_pid: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_close_on: Option<chrono::NaiveDate>,
}

impl HygieneFinding {
    fn on_deal(rule: &'static str, detail: String, deal: &HygieneDeal) -> Self {
        Self {
            rule,
            detail,
            deal: Some(deal.clone()),
            lead_pid: None,
            score: None,
            expected_close_on: None,
        }
    }
}

/// `GET /api/insights/pipeline-hygiene`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct HygieneFindings {
    threshold_days: i64,
    findings: Vec<HygieneFinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// The period an executive pack covers.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "ExecutiveWindow")]
pub(crate) struct ExecutiveWindow {
    from: chrono::DateTime<chrono::Utc>,
    to: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/insights/executive`. Won value is `null` when masked.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct ExecutivePack {
    window: ExecutiveWindow,
    deals_won: usize,
    deals_lost: usize,
    won_value_by_currency_minor: Option<std::collections::BTreeMap<String, i64>>,
    lost_reasons: std::collections::BTreeMap<String, usize>,
    new_leads: u64,
    tickets_opened: u64,
    tickets_resolved: usize,
    campaigns_started: usize,
    activities_logged: u64,
    consent_withdrawals: u64,
    note: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// One stored forecast snapshot. Totals are `null` when masked.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "ForecastTrendPoint")]
pub(crate) struct TrendPoint {
    taken_on: chrono::NaiveDate,
    #[schemars(with = "Option<std::collections::BTreeMap<String, i64>>")]
    totals: Option<serde_json::Value>,
}

/// `GET /api/insights/forecast-trends`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct ForecastTrends {
    note: &'static str,
    series: Vec<TrendPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

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
    let open: Vec<deals::Model> = deals::Entity::find_live()
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
    let mut rows: Vec<(i64, StaleDeal)> = open
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
            let masked = obligations.iter().any(|o| o == "mask");
            Some((
                days_in_stage,
                StaleDeal {
                    pid: deal.pid,
                    name: deal.name.clone(),
                    stage: stage_names.get(&deal.stage_pid).cloned(),
                    owner_ref: deal.owner_ref.clone(),
                    amount_minor: (!masked).then_some(deal.amount_minor),
                    currency: deal.currency.clone(),
                    days_in_stage,
                    stale: days_in_stage > threshold,
                },
            ))
        })
        .collect();
    rows.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    let stale_count = rows.iter().filter(|(days, _)| *days > threshold).count();
    Ok(conditional(
        &headers,
        serde_json::json!(StaleDeals {
            derivation: "stage entry = newest deal_stage_changed audit, else deal creation \
                         (never moved); stale = days_in_stage over the threshold",
            threshold_days: threshold,
            open_deals: rows.len(),
            stale_deals: stale_count,
            deals: rows.into_iter().map(|(_, v)| v).collect(),
            as_of: None,
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
    let mut find = activities::Entity::find_live()
        .filter(activities::Column::Done.eq(false))
        .filter(activities::Column::DueOn.is_not_null());
    if let Some(kind) = query.kind.as_deref() {
        find = find.filter(activities::Column::Kind.eq(kind));
    }
    let rows: Vec<activities::Model> = find.all(&ctx.db).await?;
    let view = |activity: &activities::Model| Followup {
        pid: activity.pid,
        kind: activity.kind.clone(),
        summary: activity.summary.to_string(),
        subject_kind: activity.subject_kind.clone(),
        subject_pid: activity.subject_pid,
        actor_ref: activity.actor_ref.clone(),
        due_on: activity.due_on,
        overdue_days: activity
            .due_on
            .filter(|due| *due < today)
            .map(|due| (today - due).num_days()),
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
        serde_json::json!(Followups {
            note: "open follow-ups only (due_on set, not done); actor_ref is the \
                   recording actor, not necessarily an assignee",
            overdue: overdue.iter().map(|a| view(a)).collect(),
            upcoming_30d: upcoming.iter().map(|a| view(a)).collect(),
            open_by_recorder: per_recorder,
            as_of: None,
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
    let open: Vec<deals::Model> = deals::Entity::find_live()
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    // Only the leads the rule is about, not all of them.
    let lead_rows: Vec<leads::Model> = leads::Entity::find_live()
        .filter(leads::Column::Status.is_in(["new", "contacted"]))
        .all(&ctx.db)
        .await?;
    let touches: std::collections::HashMap<(String, Uuid), chrono::DateTime<chrono::Utc>> =
        super::rollup::touches(&ctx.db)
            .await?
            .into_iter()
            .map(|t| ((t.subject_kind, t.subject_pid), t.last_at))
            .collect();
    let last_touch = |kind: &str, pid: Uuid| -> Option<chrono::DateTime<chrono::Utc>> {
        touches.get(&(kind.to_string(), pid)).copied()
    };
    let mut findings: Vec<HygieneFinding> = Vec::new();
    for deal in &open {
        let deal_ref = HygieneDeal {
            pid: deal.pid,
            name: deal.name.clone(),
        };
        if deal.amount_minor == 0 {
            findings.push(HygieneFinding::on_deal(
                "open_deal_without_amount",
                "an open deal carries no amount (forecast blind spot)".to_string(),
                &deal_ref,
            ));
        }
        if deal.expected_close_on.is_none() {
            findings.push(HygieneFinding::on_deal(
                "open_deal_without_expected_close",
                "an open deal has no expected close date".to_string(),
                &deal_ref,
            ));
        } else if deal.expected_close_on.is_some_and(|d| d < today) {
            findings.push(HygieneFinding {
                expected_close_on: deal.expected_close_on,
                ..HygieneFinding::on_deal(
                    "open_deal_past_expected_close",
                    "an open deal is past its expected close date".to_string(),
                    &deal_ref,
                )
            });
        }
        let untouched = match last_touch("deal", deal.pid) {
            Some(at) => (now - at).num_days() > threshold,
            None => true,
        };
        if untouched {
            findings.push(HygieneFinding::on_deal(
                "open_deal_without_recent_activity",
                format!("no recorded activity in {threshold} days"),
                &deal_ref,
            ));
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
            findings.push(HygieneFinding {
                rule: "unworked_lead",
                detail: format!(
                    "a {} lead with no activity in {threshold} days",
                    lead.status
                ),
                deal: None,
                lead_pid: Some(lead.pid),
                score: Some(lead.score),
                expected_close_on: None,
            });
        }
    }
    Ok(conditional(
        &headers,
        serde_json::json!(HygieneFindings {
            threshold_days: threshold,
            findings,
            as_of: None,
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

    // Only the deals closed in the window.
    let deal_rows: Vec<deals::Model> = deals::Entity::find_live()
        .filter(deals::Column::ClosedAt.gte(from))
        .filter(deals::Column::ClosedAt.lte(to))
        .all(&ctx.db)
        .await?;
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
    let new_leads = leads::Entity::find_live()
        .filter(leads::Column::CreatedAt.gte(from))
        .filter(leads::Column::CreatedAt.lte(to))
        .count(&ctx.db)
        .await?;
    let tickets_opened = tickets::Entity::find_live()
        .filter(tickets::Column::OpenedAt.gte(from))
        .filter(tickets::Column::OpenedAt.lte(to))
        .count(&ctx.db)
        .await?;
    // Counted in the database, not by loading every audit row of the window.
    let audited = |entity: &'static str, action: &'static str| {
        audit_logs::Entity::find()
            .filter(audit_logs::Column::CreatedAt.gte(from))
            .filter(audit_logs::Column::CreatedAt.lte(to))
            .filter(audit_logs::Column::Entity.eq(entity))
            .filter(audit_logs::Column::Action.eq(action))
            .count(&ctx.db)
    };
    let tickets_resolved =
        usize::try_from(audited("ticket", "resolved").await?).unwrap_or(usize::MAX);
    let campaigns_started =
        usize::try_from(audited("campaign", "running").await?).unwrap_or(usize::MAX);
    let activities_logged = activities::Entity::find_live()
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
    Ok(conditional(
        &headers,
        serde_json::json!(ExecutivePack {
            window: ExecutiveWindow { from, to },
            deals_won: won,
            deals_lost: lost,
            won_value_by_currency_minor: (!masked).then_some(won_value),
            lost_reasons,
            new_leads,
            tickets_opened,
            tickets_resolved,
            campaigns_started,
            activities_logged,
            consent_withdrawals,
            note: "per-currency won value is never merged or converted; \
                   tickets_resolved / campaigns_started derive from status audits",
            as_of: None,
        }),
    ))
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
    let rows: Vec<forecast_snapshots::Model> = forecast_snapshots::Entity::find_live()
        .order_by_asc(forecast_snapshots::Column::TakenOn)
        .all(&ctx.db)
        .await?;
    let series: Vec<TrendPoint> = rows
        .iter()
        .take(200)
        .map(|row| TrendPoint {
            taken_on: row.taken_on,
            totals: (!masked).then(|| row.totals.clone()),
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(ForecastTrends {
            note: "stored snapshots only (POST /api/forecast/snapshot); no interpolated history",
            series,
            as_of: None,
        }),
    ))
}
