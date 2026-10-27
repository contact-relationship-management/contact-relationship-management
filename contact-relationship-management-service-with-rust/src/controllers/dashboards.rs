//! Analytics & reporting (CRM-R13, CRM-R14): every number is a
//! pure-core derivation from recorded facts (CRM-D4) — per-currency,
//! honest ratios, `as_of`-stamped, ETag-conditional.

use crate::controllers::PidPath;
use crate::models::live::FindLive;
use std::hash::{DefaultHasher, Hash, Hasher};

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use loco_rs::prelude::*;
use sea_orm::QuerySelect;
use serde::{Deserialize, Serialize};

use super::sales;
use crate::auth::MaybeAuthUser;
use crate::metrics::Metrics;
use crate::models::_entities::{activities, deals, tickets};
use crate::models::records;
use crate::rules::{analytics, sla};

/// Weak-ETag helper over the payload **without** `as_of` (the tag
/// must be stable across reads of unchanged data).
pub(crate) fn etag_of(value: &serde_json::Value) -> String {
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    format!("W/\"{:x}\"", hasher.finish())
}

/// Wrap a dashboard payload: 304 on a matching `If-None-Match`, else
/// the payload + `as_of` with the `ETag` header.
pub(crate) fn conditional(headers: &HeaderMap, payload: serde_json::Value) -> Response {
    let tag = etag_of(&payload);
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(tag.as_str())
    {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, tag)]).into_response();
    }
    let mut body = payload;
    if let Some(object) = body.as_object_mut() {
        object.insert("as_of".into(), serde_json::json!(chrono::Utc::now()));
    }
    (StatusCode::OK, [(header::ETAG, tag)], axum::Json(body)).into_response()
}

/// Stage name to (open-deal count, currency to total in minor units), and the
/// number of open deals. One grouped query, whatever the number of deals; a
/// total beyond `i64` saturates, as the sum always did.
async fn open_deals_by_stage(
    db: &sea_orm::DatabaseConnection,
) -> Result<(
    std::collections::BTreeMap<String, (usize, std::collections::BTreeMap<String, i64>)>,
    usize,
)> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT s.name AS stage, d.currency AS currency, COUNT(*) AS n, \
                    SUM(d.amount_minor)::text AS total \
             FROM deals d JOIN pipeline_stages s ON s.pid = d.stage_pid \
             WHERE d.deleted_at IS NULL AND d.closed_at IS NULL \
             GROUP BY s.name, d.currency",
        ))
        .await?;
    let model = |e: sea_orm::DbErr| Error::Model(ModelError::from(e));
    let mut totals: std::collections::BTreeMap<
        String,
        (usize, std::collections::BTreeMap<String, i64>),
    > = std::collections::BTreeMap::new();
    let mut open_deals = 0usize;
    for row in &rows {
        let stage: String = row.try_get("", "stage").map_err(model)?;
        let currency: String = row.try_get("", "currency").map_err(model)?;
        let n: i64 = row.try_get("", "n").map_err(model)?;
        let total: String = row.try_get("", "total").map_err(model)?;
        let n = usize::try_from(n).unwrap_or(0);
        let total = total.parse::<i128>().map_or(0, |t| {
            i64::try_from(t).unwrap_or(if t < 0 { i64::MIN } else { i64::MAX })
        });
        let (count, by_currency) = totals.entry(stage).or_default();
        *count += n;
        open_deals += n;
        let slot = by_currency.entry(currency).or_insert(0);
        *slot = slot.saturating_add(total);
    }
    Ok((totals, open_deals))
}

/// One stage of the sales dashboard.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "SalesDashboardStage")]
pub(crate) struct StageTotals {
    count: usize,
    /// Minor units per ISO 4217 code, never summed across currencies; `null` when masked.
    totals_minor: Option<std::collections::BTreeMap<String, i64>>,
}

/// `GET /api/dashboards/sales`. `as_of` is stamped by [`conditional`], after the `ETag` is taken.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct SalesDashboard {
    win_rate: analytics::Ratio,
    open_deals: usize,
    pipeline_by_stage: std::collections::BTreeMap<String, StageTotals>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// One priority row of the SLA dashboard.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "SlaDashboardPriority")]
pub(crate) struct PriorityRow {
    priority: String,
    open: i64,
    breached: i64,
}

/// `GET /api/dashboards/sla`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct SlaDashboard {
    open_tickets: usize,
    by_priority: Vec<PriorityRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// `GET /api/accounts/{pid}/clv`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[allow(clippy::struct_field_names)] // `clv_minor` is the response member clients read
pub(crate) struct Clv {
    account_pid: Uuid,
    won_deals: usize,
    /// Minor units per ISO 4217 code, never summed across currencies; `null` when masked.
    clv_minor: Option<std::collections::BTreeMap<String, i64>>,
    as_of: chrono::DateTime<chrono::Utc>,
    /// With `convert_to`: the currency of `converted_minor` (CRM-R54).
    convert_to: Option<String>,
    /// With `convert_to`: every currency's CLV converted and added, in minor units of
    /// `convert_to`; what had no rate is left out (see `unconverted`). `null` when masked.
    converted_minor: Option<i64>,
    /// With `convert_to`: the day whose exchange rates were used.
    rate_date: Option<chrono::NaiveDate>,
    /// With `convert_to`: the rate used for each currency that was converted.
    rates: Vec<super::fx_rates::RateUsed>,
    /// With `convert_to`: CLV that had no rate in force, left out and never taken at 1.0.
    unconverted: Vec<super::fx_rates::Unconverted>,
    /// With `margin_bps`: the margin on each currency's CLV, minor units; `null` otherwise or
    /// when masked (CRM-R60).
    margin_minor: Option<std::collections::BTreeMap<String, i64>>,
    /// With `retention_bps` and `years`: the look-ahead (CRM-R60); `null` otherwise or when masked.
    projection: Option<ClvProjection>,
    /// With `convert_to` and `margin_bps`: the margin of every currency converted and added, minor
    /// units of `convert_to`, at the rates named in `rates` (CRM-R62).
    margin_converted_minor: Option<i64>,
}

/// The look-ahead of `GET /api/accounts/{pid}/clv` (CRM-R60), per currency, never converted.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ClvProjection {
    years: u32,
    /// The retention asked for; `null` when it was read from history (`retention_from=history`).
    retention_bps: Option<u32>,
    /// With `retention_from=history`: each currency's retention from its own won deals, `null`
    /// where nothing was won in the year before (CRM-R61).
    retention_history_bps: Option<std::collections::BTreeMap<String, Option<u32>>>,
    discount_bps: u32,
    /// The revenue (or its margin, with `margin_bps`) of deals won in the 365 days to `as_of`.
    annual_base_minor: std::collections::BTreeMap<String, i64>,
    /// What the base is expected to bring over `years`, discounted.
    projected_minor: std::collections::BTreeMap<String, i64>,
    /// What has been won (its margin, with `margin_bps`) plus `projected_minor`.
    total_minor: std::collections::BTreeMap<String, i64>,
    /// With `convert_to`: the three sums above, every currency converted and added (CRM-R62).
    converted: Option<ClvProjectionConverted>,
}

/// The look-ahead in one currency: each currency's figure converted once, then added.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ClvProjectionConverted {
    /// Minor units of `convert_to`.
    annual_base_minor: i64,
    projected_minor: i64,
    total_minor: i64,
}

/// What `GET /api/accounts/{pid}/clv` may be asked.
#[derive(Debug, Deserialize)]
struct ClvQuery {
    /// Also show the CLV in this currency, three capital letters (CRM-R54).
    convert_to: Option<String>,
    /// The day whose exchange rates are used (today when absent); only with `convert_to`.
    rate_date: Option<chrono::NaiveDate>,
    /// Also show the margin on the CLV, in basis points of revenue, 0 to 10000 (CRM-R60).
    margin_bps: Option<u32>,
    /// With `years`: the share of a year's revenue expected again the next year, in basis points.
    retention_bps: Option<u32>,
    /// `history`: take the retention from the account's own won deals instead of `retention_bps` (CRM-R61).
    retention_from: Option<String>,
    /// With `retention_bps` and `years`: the yearly discount rate, in basis points (0 when absent).
    discount_bps: Option<u32>,
    /// With `retention_bps`: how many years to look ahead, 1 to 20.
    years: Option<u32>,
}

/// `GET /api/dashboards/sales` — win rate + pipeline value by stage
/// (per currency) over live data.
#[debug_handler]
async fn sales_dashboard(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
) -> Result<Response> {
    let masked = crate::auth::aggregate_masked(&caller).map_err(super::record_rejection)?;
    let (won, lost) = sales::closed_counts(&ctx.db).await?;
    let rate = analytics::win_rate(
        i64::try_from(won).unwrap_or(i64::MAX),
        i64::try_from(lost).unwrap_or(i64::MAX),
    );
    // Pipeline value by stage, per currency.
    let (totals, open_deals) = open_deals_by_stage(&ctx.db).await?;
    // Counts and structure stay visible when masked; the money is absent, never zero.
    let by_stage: std::collections::BTreeMap<String, StageTotals> = totals
        .into_iter()
        .map(|(stage, (count, by_currency))| {
            (
                stage,
                StageTotals {
                    count,
                    totals_minor: (!masked).then_some(by_currency),
                },
            )
        })
        .collect();
    Metrics::global()
        .deals_open
        .set(i64::try_from(open_deals).unwrap_or(i64::MAX));
    Ok(conditional(
        &headers,
        serde_json::json!(SalesDashboard {
            win_rate: rate,
            open_deals,
            pipeline_by_stage: by_stage,
            as_of: None,
        }),
    ))
}

/// `GET /api/dashboards/sla` — open tickets by priority × breach
/// state; live breach truth (CRM-R13).
#[debug_handler]
async fn sla_dashboard(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let open = tickets::Entity::find_live()
        .filter(tickets::Column::Status.is_in(["open", "pending"]))
        .all(&ctx.db)
        .await?;
    let mut by_priority: std::collections::BTreeMap<String, (i64, i64)> =
        std::collections::BTreeMap::new();
    for ticket in &open {
        let clock = sla::clock_now(now, ticket.paused_at);
        let breached = ticket
            .first_response_due_at
            .is_some_and(|due| sla::first_response_breached(clock, due, ticket.first_responded_at))
            || ticket
                .resolution_due_at
                .is_some_and(|due| sla::resolution_breached(clock, due, ticket.resolved_at));
        let slot = by_priority.entry(ticket.priority.clone()).or_default();
        slot.0 += 1;
        if breached {
            slot.1 += 1;
        }
    }
    Metrics::global()
        .tickets_open
        .set(i64::try_from(open.len()).unwrap_or(i64::MAX));
    let rows: Vec<PriorityRow> = by_priority
        .into_iter()
        .map(|(priority, (open, breached))| PriorityRow {
            priority,
            open,
            breached,
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(SlaDashboard {
            open_tickets: open.len(),
            by_priority: rows,
            as_of: None,
        }),
    ))
}

/// What `margin_bps` and `retention_bps` with `years` add to the CLV (CRM-R60).
type Outlook = (
    Option<std::collections::BTreeMap<String, i64>>,
    Option<ClvProjection>,
);

fn clv_outlook(
    query: &ClvQuery,
    totals: &std::collections::BTreeMap<String, i64>,
    won: &[deals::Model],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Outlook> {
    let (mut margin_minor, mut projection) = (None, None);
    {
        let overflow = |e: String| super::unprocessable(&e);
        let kept = |amount: i64| match query.margin_bps {
            Some(bps) => analytics::margin(amount, bps),
            None => Ok(amount),
        };
        if query.margin_bps.is_some() {
            let mut shown = std::collections::BTreeMap::new();
            for (currency, total) in totals {
                shown.insert(currency.clone(), kept(*total).map_err(overflow)?);
            }
            margin_minor = Some(shown);
        }
        if let (true, Some(years)) = (
            query.retention_bps.is_some() || query.retention_from.is_some(),
            query.years,
        ) {
            let discount = query.discount_bps.unwrap_or(0);
            let since = now - chrono::Duration::days(365);
            let before = since - chrono::Duration::days(365);
            let (mut recent, mut prior): (
                std::collections::BTreeMap<String, i64>,
                std::collections::BTreeMap<String, i64>,
            ) = (
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
            );
            for deal in won {
                let Some(at) = deal.closed_at else { continue };
                let year = if at > since {
                    &mut recent
                } else if at > before {
                    &mut prior
                } else {
                    continue;
                };
                let slot = year.entry(deal.currency.clone()).or_insert(0);
                *slot = slot
                    .checked_add(deal.amount_minor)
                    .ok_or_else(|| super::unprocessable("clv sum overflows"))?;
            }
            let from_history = query.retention_from.is_some();
            let mut history = std::collections::BTreeMap::new();
            let (mut base, mut ahead, mut total) = (
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
            );
            for currency in totals.keys() {
                let annual = kept(recent.get(currency).copied().unwrap_or(0)).map_err(overflow)?;
                let retention = if from_history {
                    let found = analytics::retention_from_history(
                        recent.get(currency).copied().unwrap_or(0),
                        prior.get(currency).copied().unwrap_or(0),
                    )
                    .map_err(overflow)?;
                    history.insert(currency.clone(), found);
                    found
                } else {
                    query.retention_bps
                };
                // Nothing to measure against means nothing is projected, never a guess.
                let coming = match retention {
                    Some(r) => {
                        analytics::projected(annual, r, discount, years).map_err(overflow)?
                    }
                    None => 0,
                };
                let won_so_far = kept(totals[currency]).map_err(overflow)?;
                base.insert(currency.clone(), annual);
                total.insert(
                    currency.clone(),
                    won_so_far
                        .checked_add(coming)
                        .ok_or_else(|| super::unprocessable("clv sum overflows"))?,
                );
                ahead.insert(currency.clone(), coming);
            }
            projection = Some(ClvProjection {
                years,
                retention_bps: query.retention_bps,
                retention_history_bps: from_history.then_some(history),
                discount_bps: discount,
                annual_base_minor: base,
                projected_minor: ahead,
                total_minor: total,
                converted: None,
            });
        }
    }
    Ok((margin_minor, projection))
}

/// `GET /api/accounts/{pid}/clv` — CLV per currency over won deals
/// (CRM-R13; the v1 revenue-sum definition).
#[debug_handler]
async fn account_clv(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    axum::extract::Query(query): axum::extract::Query<ClvQuery>,
) -> Result<Response> {
    let mut problems = crate::validation::Problems::new();
    if let Some(target) = &query.convert_to {
        if let Err(why) = crate::rules::fx::currency(target) {
            problems.push_field("convert_to", why);
        }
    } else if query.rate_date.is_some() {
        problems.push_field("rate_date", "needs convert_to");
    }
    for (field, value) in [
        ("margin_bps", query.margin_bps),
        ("retention_bps", query.retention_bps),
        ("discount_bps", query.discount_bps),
    ] {
        if value.is_some_and(|bps| bps > analytics::MAX_BPS) {
            problems.push_field(field, "at most 10000");
        }
    }
    if query
        .years
        .is_some_and(|y| !(1..=analytics::MAX_YEARS).contains(&y))
    {
        problems.push_field("years", "from 1 to 20");
    }
    if let Some(source) = &query.retention_from {
        if source != "history" {
            problems.push_field("retention_from", "only history");
        }
        if query.retention_bps.is_some() {
            problems.push_field("retention_from", "not with retention_bps");
        }
    }
    let has_retention = query.retention_bps.is_some() || query.retention_from.is_some();
    if has_retention != query.years.is_some() {
        problems.push_field(
            "years",
            "retention_bps or retention_from, and years, go together",
        );
    }
    if query.discount_bps.is_some() && query.years.is_none() {
        problems.push_field("discount_bps", "needs retention_bps and years");
    }
    problems.finish()?;
    // CLV is money: a masked collection read gets the count of won deals, not the figures.
    let masked = crate::auth::aggregate_masked(&caller).map_err(super::record_rejection)?;
    let account = records::find_account(&ctx.db, pid).await?;
    let won = deals::Entity::find_live()
        .filter(deals::Column::AccountPid.eq(account.pid))
        .filter(deals::Column::Won.eq(true))
        .all(&ctx.db)
        .await?;
    let inputs: Vec<(i64, String)> = won
        .iter()
        .map(|deal| (deal.amount_minor, deal.currency.clone()))
        .collect();
    let totals = analytics::clv_by_currency(&inputs).map_err(|e| super::unprocessable(&e))?;
    // With `convert_to`, each currency's CLV is converted once and the results added.
    let mut conversion = None;
    if let (Some(target), false) = (&query.convert_to, masked) {
        let sums: Vec<(String, String, i64)> = totals
            .iter()
            .map(|(currency, total)| ("clv".to_string(), currency.clone(), *total))
            .collect();
        conversion = Some(
            super::fx_rates::convert_sums(&ctx, target, query.rate_date, "rate_date", &sums)
                .await?,
        );
    }
    let (converted_minor, rate_date, rates, unconverted) = match conversion {
        Some(done) => (
            Some(done.rows.first().map_or(0, |row| row.1)),
            Some(done.rate_date),
            done.rates,
            done.unconverted,
        ),
        None => (None, None, Vec::new(), Vec::new()),
    };
    let now = chrono::Utc::now();
    let (margin_minor, mut projection) = if masked {
        (None, None)
    } else {
        clv_outlook(&query, &totals, &won, now)?
    };
    // The margin and the look-ahead are converted at the same rates as the CLV: the same
    // currencies, so the same ones are left out (CRM-R62).
    let mut margin_converted_minor = None;
    if let (Some(target), false) = (&query.convert_to, masked) {
        let mut sums = Vec::new();
        for (group, amounts) in [
            ("margin", margin_minor.as_ref()),
            ("base", projection.as_ref().map(|p| &p.annual_base_minor)),
            ("projected", projection.as_ref().map(|p| &p.projected_minor)),
            ("total", projection.as_ref().map(|p| &p.total_minor)),
        ] {
            for (currency, amount) in amounts.into_iter().flatten() {
                sums.push((group.to_string(), currency.clone(), *amount));
            }
        }
        if !sums.is_empty() {
            let done =
                super::fx_rates::convert_sums(&ctx, target, query.rate_date, "rate_date", &sums)
                    .await?;
            let of = |group: &str| {
                done.rows
                    .iter()
                    .find(|row| row.0 == group)
                    .map_or(0, |row| row.1)
            };
            margin_converted_minor = margin_minor.is_some().then(|| of("margin"));
            if let Some(p) = projection.as_mut() {
                p.converted = Some(ClvProjectionConverted {
                    annual_base_minor: of("base"),
                    projected_minor: of("projected"),
                    total_minor: of("total"),
                });
            }
        }
    }
    format::json(Clv {
        account_pid: account.pid,
        won_deals: won.len(),
        clv_minor: (!masked).then_some(totals),
        as_of: now,
        convert_to: query.convert_to.clone(),
        converted_minor,
        rate_date,
        rates,
        unconverted,
        margin_minor,
        projection,
        margin_converted_minor,
    })
}

/// `GET /api/dashboards/activity?days=`: activity counts by kind in the window.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ActivityDashboard {
    window_days: i64,
    total: i64,
    by_kind: std::collections::BTreeMap<String, i64>,
    as_of: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/dashboards/activity?days=` — per-kind activity counts
/// over the window (CRM-R14).
#[derive(Debug, Deserialize)]
struct ActivityParams {
    #[serde(default = "default_days")]
    days: i64,
}

const fn default_days() -> i64 {
    30
}

/// `GET /api/dashboards/activity?days=` — counts by kind, computed in SQL
/// over **every** matching activity (no row cap), so the totals are exact.
#[debug_handler]
async fn activity_dashboard(
    State(ctx): State<AppContext>,
    Query(params): Query<ActivityParams>,
) -> Result<Response> {
    let since = chrono::Utc::now() - chrono::Duration::days(params.days.clamp(1, 365));
    let counts: Vec<(String, i64)> = activities::Entity::find()
        .select_only()
        .column(activities::Column::Kind)
        .column_as(activities::Column::Id.count(), "n")
        .filter(activities::Column::DeletedAt.is_null())
        .filter(activities::Column::OccurredAt.gte(since))
        .group_by(activities::Column::Kind)
        .into_tuple()
        .all(&ctx.db)
        .await?;
    let total: i64 = counts.iter().map(|(_, n)| n).sum();
    let by_kind: std::collections::BTreeMap<String, i64> = counts.into_iter().collect();
    format::json(ActivityDashboard {
        window_days: params.days,
        total,
        by_kind,
        as_of: chrono::Utc::now(),
    })
}

/// The dashboard routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/dashboards/sales", get(sales_dashboard))
        .add("/dashboards/sla", get(sla_dashboard))
        .add("/dashboards/activity", get(activity_dashboard))
        .add("/accounts/{pid}/clv", get(account_clv))
}
