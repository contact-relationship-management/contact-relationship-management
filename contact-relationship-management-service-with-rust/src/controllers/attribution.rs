//! Multi-touch attribution (CRM-R49): `GET /api/attribution`.
//!
//! Which campaigns touched the people who bought, and how the revenue of the won deals is shared
//! among them under a model the caller chooses. The sharing itself is [`crate::rules::attribution`];
//! this reads the touches and adds the shares up, per campaign and per currency.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use loco_rs::prelude::*;

use super::fx_rates::{self, RateUsed, Unconverted};
use super::record_rejection;
use crate::auth::{self, MaybeAuthUser};
use crate::models::audit_logs::Model as Audit;
use crate::rules::attribution::{self as rules, Model, Touch};
use crate::rules::fx;
use crate::validation::Problems;

/// The group key of the deals no campaign touched, in a conversion.
const UNATTRIBUTED: &str = "unattributed";

/// Most won deals read in one ask.
const MAX_DEALS: usize = 5000;

/// What `GET /api/attribution` may be asked.
#[derive(Debug, Deserialize)]
struct AttributionQuery {
    /// `first`, `last`, `linear` (default), `time_decay` or `position`.
    model: Option<String>,
    /// Which e-mail touches count, a comma-separated list of `sent` (default), `opened` and
    /// `clicked` (CRM-R58). Campaign leads and the recorded source always count.
    touches: Option<String>,
    /// How far back from the day a deal closed a touch still counts, 1 to 365 (default 90).
    window_days: Option<i64>,
    /// With `time_decay`: a touch's weight halves for every this many days before the deal closed,
    /// 4 to 90 (default 7; CRM-R89).
    half_life_days: Option<i64>,
    /// With `position`: the share, in percent, each of the first and last touches gets, 1 to 49
    /// (default 40; CRM-R90). The touches between share the rest.
    end_percent: Option<i64>,
    /// An open reported less than this many seconds after the e-mail was sent is taken for a mail
    /// program loading it ahead of the reader, and does not count; 1 to 86,400, and only with
    /// `touches` including `opened` (CRM-R89). Clicks are not affected.
    min_open_seconds: Option<i64>,
    /// A click reported less than this many seconds after the e-mail was sent is taken for a link
    /// scanner and does not count; 1 to 86,400, and only with `touches` including `clicked`
    /// (CRM-R91). Opens are not affected.
    min_click_seconds: Option<i64>,
    /// First day a deal may have closed on.
    from: Option<NaiveDate>,
    /// Last day a deal may have closed on.
    to: Option<NaiveDate>,
    /// Show every figure in this currency (CRM-R51), an ISO 4217 code (CRM-T168).
    convert_to: Option<String>,
    /// The day whose exchange rates are used (today when absent); only with `convert_to`.
    rate_date: Option<NaiveDate>,
    /// `close_day`: convert each deal's share at the rate of the day the deal closed (CRM-R85);
    /// only with `convert_to`, and not with `rate_date`.
    rate_basis: Option<String>,
}

/// An amount in one currency.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "AttributionAmount")]
pub(crate) struct Amount {
    currency: String,
    /// Minor units.
    amount_minor: i64,
}

/// What one campaign earns.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "AttributionCampaign")]
pub(crate) struct CampaignCredit {
    #[schemars(with = "uuid::Uuid")]
    campaign_pid: String,
    /// The campaign's name; `(deleted campaign)` when it is gone.
    name: String,
    /// Its touches on the deals considered.
    touches: usize,
    /// The deals it touched.
    deals: usize,
    /// Its credit, per currency; currencies are never added together.
    credit: Vec<Amount>,
}

/// The answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Attribution")]
pub(crate) struct Attribution {
    /// The model used.
    model: String,
    /// The window used, in days.
    window_days: i64,
    /// With `time_decay`: the half-life used, in days (CRM-R89); `null` for the other models.
    half_life_days: Option<i64>,
    /// With `position`: the share each of the first and last touches got, in percent (CRM-R90);
    /// `null` for the other models.
    end_percent: Option<i64>,
    /// The seconds after sending before which an open does not count, when one was asked for.
    min_open_seconds: Option<i64>,
    /// The seconds after sending before which a click does not count, when one was asked for.
    min_click_seconds: Option<i64>,
    /// The e-mail touches counted: `sent`, `opened`, `clicked`.
    touches: Vec<String>,
    as_of: DateTime<Utc>,
    /// Won deals considered.
    deals: usize,
    /// Deals with no touch at all, and what they were worth: kept apart, never dropped.
    unattributed_deals: usize,
    unattributed: Vec<Amount>,
    /// Most deals touched first.
    campaigns: Vec<CampaignCredit>,
    /// More deals matched than are read.
    truncated: bool,
    /// With `convert_to`: the currency every figure is in.
    convert_to: Option<String>,
    /// With `convert_to`: the day whose exchange rates were used; `null` with `rate_basis`, where
    /// each deal has its own.
    rate_date: Option<NaiveDate>,
    /// `close_day` when each deal's share was converted at the rate of the day it closed.
    rate_basis: Option<String>,
    /// With `convert_to`: the rate used for each currency that was converted.
    rates: Vec<RateUsed>,
    /// With `convert_to`: credit that had no rate in force and was left out, never taken at 1.0.
    /// Each entry counts a campaign's credit in one currency (or the unattributed deals').
    unconverted: Vec<Unconverted>,
    /// What the figures mean, in a line.
    note: &'static str,
}

/// A won deal, as far as attribution needs it.
struct Won {
    pid: Uuid,
    amount_minor: i64,
    currency: String,
    source_campaign: Option<Uuid>,
    created_at: DateTime<Utc>,
    closed_at: DateTime<Utc>,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// The kinds of e-mail touch that count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kinds {
    sent: bool,
    opened: bool,
    clicked: bool,
}

impl Kinds {
    /// The kinds asked for (`sent` alone when none is asked), or the reason a name is wrong.
    fn parse(asked: Option<&str>) -> std::result::Result<Self, String> {
        let Some(text) = asked else {
            return Ok(Self {
                sent: true,
                opened: false,
                clicked: false,
            });
        };
        let mut kinds = Self {
            sent: false,
            opened: false,
            clicked: false,
        };
        for name in text.split(',').map(str::trim) {
            match name {
                "sent" => kinds.sent = true,
                "opened" => kinds.opened = true,
                "clicked" => kinds.clicked = true,
                _ => return Err("touches is a list of sent, opened, clicked".to_string()),
            }
        }
        Ok(kinds)
    }

    /// The names, in the order they are listed in an answer.
    fn names(self) -> Vec<String> {
        [
            (self.sent, "sent"),
            (self.opened, "opened"),
            (self.clicked, "clicked"),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| name.to_string())
        .collect()
    }
}

/// The model, the window and the e-mail touches, or a `422` naming each wrong member.
fn validate(query: &AttributionQuery) -> Result<Asked> {
    let mut problems = Problems::new();
    let model = match query.model.as_deref() {
        None => Some(Model::Linear),
        Some(name) => Model::parse(name)
            .map_err(|why| problems.push_field("model", why))
            .ok(),
    };
    let kinds = Kinds::parse(query.touches.as_deref())
        .map_err(|why| problems.push_field("touches", why))
        .ok();
    let days = rules::window_days(query.window_days)
        .map_err(|why| problems.push_field("window_days", why))
        .ok();
    if let (Some(from), Some(to)) = (query.from, query.to)
        && from > to
    {
        problems.push_field("from", "from is after to");
    }
    if let Some(target) = &query.convert_to {
        if let Err(why) = fx::currency(target) {
            problems.push_field("convert_to", why);
        }
    } else if query.rate_date.is_some() {
        problems.push_field("rate_date", "needs convert_to");
    }
    if let Some(basis) = &query.rate_basis {
        if query.convert_to.is_none() {
            problems.push_field("rate_basis", "needs convert_to");
        } else if basis != "close_day" {
            problems.push_field("rate_basis", "only close_day");
        } else if query.rate_date.is_some() {
            problems.push_field(
                "rate_date",
                "not with rate_basis: each deal has its own day",
            );
        }
    }
    if query.half_life_days.is_some() && model != Some(Model::TimeDecay) {
        problems.push_field("half_life_days", "only with the time_decay model");
    }
    let half_life = rules::half_life_days(query.half_life_days)
        .map_err(|why| problems.push_field("half_life_days", why))
        .ok();
    if query.end_percent.is_some() && model != Some(Model::Position) {
        problems.push_field("end_percent", "only with the position model");
    }
    let end = rules::end_percent(query.end_percent)
        .map_err(|why| problems.push_field("end_percent", why))
        .ok();
    if let Some(seconds) = query.min_open_seconds {
        if !(1..=rules::MAX_MIN_OPEN_SECONDS).contains(&seconds) {
            problems.push_field(
                "min_open_seconds",
                format!("1 to {} seconds", rules::MAX_MIN_OPEN_SECONDS),
            );
        } else if kinds.is_some_and(|k| !k.opened) {
            problems.push_field("min_open_seconds", "needs touches to include opened");
        }
    }
    if let Some(seconds) = query.min_click_seconds {
        if !(1..=rules::MAX_MIN_OPEN_SECONDS).contains(&seconds) {
            problems.push_field(
                "min_click_seconds",
                format!("1 to {} seconds", rules::MAX_MIN_OPEN_SECONDS),
            );
        } else if kinds.is_some_and(|k| !k.clicked) {
            problems.push_field("min_click_seconds", "needs touches to include clicked");
        }
    }
    problems.finish()?;
    match (model, days, kinds, half_life, end) {
        (Some(model), Some(days), Some(kinds), Some(half_life_days), Some(end_percent)) => {
            Ok(Asked {
                model,
                days,
                kinds,
                half_life_days,
                end_percent,
                min_open_seconds: query.min_open_seconds.unwrap_or(0),
                min_click_seconds: query.min_click_seconds.unwrap_or(0),
            })
        }
        _ => Err(super::unprocessable("the request cannot be answered")),
    }
}

/// What a valid ask came to.
struct Asked {
    model: Model,
    days: i64,
    kinds: Kinds,
    /// The time-decay half-life, in days.
    half_life_days: i64,
    /// The position model's share for each end, in percent.
    end_percent: i64,
    /// An open sooner than this after sending does not count; `0` for none.
    min_open_seconds: i64,
    /// A click sooner than this after sending does not count; `0` for none.
    min_click_seconds: i64,
}

/// The won deals closed in the range, newest first, one more than the cap so a cut is seen.
async fn won_deals(ctx: &AppContext, query: &AttributionQuery) -> Result<Vec<Won>> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            "SELECT pid, amount_minor, currency, source_campaign_pid, created_at, closed_at FROM deals \
             WHERE won AND deleted_at IS NULL AND closed_at IS NOT NULL \
               AND ($1::date IS NULL OR closed_at::date >= $1::date) \
               AND ($2::date IS NULL OR closed_at::date <= $2::date) \
             ORDER BY closed_at DESC, id LIMIT $3",
            [
                query.from.into(),
                query.to.into(),
                i64::try_from(MAX_DEALS + 1).unwrap_or(i64::MAX).into(),
            ],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Won {
                pid: row.try_get("", "pid")?,
                amount_minor: row.try_get("", "amount_minor")?,
                currency: row.try_get("", "currency")?,
                source_campaign: row.try_get("", "source_campaign_pid")?,
                created_at: row.try_get("", "created_at")?,
                closed_at: row.try_get("", "closed_at")?,
            })
        })
        .collect()
}

/// The touches on those deals: e-mails sent to the deal's primary contact, and campaign leads of
/// that contact, from `days` before the deal closed up to the moment it closed.
async fn touches_of(
    ctx: &AppContext,
    deals: &[Won],
    days: i64,
    kinds: Kinds,
    min_open_seconds: i64,
    min_click_seconds: i64,
) -> Result<HashMap<Uuid, Vec<Touch>>> {
    // The engagement events asked for: an open or a click the provider reported, at the moment it
    // was reported (CRM-R58).
    let engagement: Vec<&str> = [(kinds.opened, "opened"), (kinds.clicked, "clicked")]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| name)
        .collect();
    let pids: Vec<String> = deals.iter().map(|d| d.pid.to_string()).collect();
    let rows = ctx
        .db
        .query_all_raw(stmt(
            "SELECT d.pid AS deal_pid, e.campaign_pid, e.sent_at AS at \
               FROM deals d JOIN email_deliveries e ON e.contact_pid = d.primary_contact_pid \
              WHERE d.pid IN (SELECT jsonb_array_elements_text($1::jsonb)::uuid) \
                AND $3::bool AND e.state = 'sent' AND e.sent_at IS NOT NULL \
                AND e.sent_at <= d.closed_at \
                AND e.sent_at >= d.closed_at - make_interval(days => $2::int) \
             UNION ALL \
             SELECT d.pid, e.campaign_pid, ev.received_at \
               FROM deals d JOIN email_deliveries e ON e.contact_pid = d.primary_contact_pid \
               JOIN email_events ev ON ev.message_id = e.message_id \
              WHERE d.pid IN (SELECT jsonb_array_elements_text($1::jsonb)::uuid) \
                AND ev.event IN (SELECT jsonb_array_elements_text($4::jsonb)) \
                AND ($5::int = 0 OR ev.event <> 'opened' \
                     OR ev.received_at >= e.sent_at + make_interval(secs => $5::int)) \
                AND ($6::int = 0 OR ev.event <> 'clicked' \
                     OR ev.received_at >= e.sent_at + make_interval(secs => $6::int)) \
                AND ev.received_at <= d.closed_at \
                AND ev.received_at >= d.closed_at - make_interval(days => $2::int) \
             UNION ALL \
             SELECT d.pid, l.campaign_pid, l.created_at \
               FROM deals d JOIN leads l ON l.contact_pid = d.primary_contact_pid \
              WHERE d.pid IN (SELECT jsonb_array_elements_text($1::jsonb)::uuid) \
                AND l.campaign_pid IS NOT NULL AND l.deleted_at IS NULL \
                AND l.created_at <= d.closed_at \
                AND l.created_at >= d.closed_at - make_interval(days => $2::int)",
            [
                serde_json::json!(pids).into(),
                i32::try_from(days).unwrap_or(i32::MAX).into(),
                kinds.sent.into(),
                serde_json::json!(engagement).into(),
                i32::try_from(min_open_seconds).unwrap_or(i32::MAX).into(),
                i32::try_from(min_click_seconds).unwrap_or(i32::MAX).into(),
            ],
        ))
        .await?;
    let mut by_deal: HashMap<Uuid, Vec<Touch>> = HashMap::new();
    for row in &rows {
        by_deal
            .entry(row.try_get("", "deal_pid")?)
            .or_default()
            .push(Touch {
                campaign: row.try_get("", "campaign_pid")?,
                at: row.try_get("", "at")?,
            });
    }
    Ok(by_deal)
}

/// Names of the campaigns, by id.
async fn names_of(ctx: &AppContext, ids: &BTreeSet<Uuid>) -> Result<HashMap<Uuid, String>> {
    let list: Vec<String> = ids.iter().map(ToString::to_string).collect();
    let rows = ctx
        .db
        .query_all_raw(stmt(
            "SELECT pid, name FROM campaigns \
             WHERE pid IN (SELECT jsonb_array_elements_text($1::jsonb)::uuid)",
            [serde_json::json!(list).into()],
        ))
        .await?;
    rows.iter()
        .map(|row| Ok((row.try_get("", "pid")?, row.try_get("", "name")?)))
        .collect()
}

/// Add `amount` to a currency's total.
fn add(totals: &mut BTreeMap<String, i64>, currency: &str, amount: i64) {
    let total = totals.entry(currency.to_string()).or_default();
    *total = total.saturating_add(amount);
}

fn amounts(totals: BTreeMap<String, i64>) -> Vec<Amount> {
    totals
        .into_iter()
        .map(|(currency, amount_minor)| Amount {
            currency,
            amount_minor,
        })
        .collect()
}

/// What one campaign has earned so far.
#[derive(Default)]
struct Tally {
    touches: usize,
    deals: usize,
    credit: BTreeMap<String, i64>,
    /// Each share with the day its deal closed, for conversion at close-day rates (CRM-R85).
    dated: Vec<(NaiveDate, String, i64)>,
}

/// What sharing the deals out came to.
struct Shared {
    tallies: BTreeMap<Uuid, Tally>,
    unattributed: BTreeMap<String, i64>,
    unattributed_dated: Vec<(NaiveDate, String, i64)>,
    unattributed_deals: usize,
}

/// Share every deal among its touches: each campaign's tally, and what no campaign touched.
fn share_out(
    model: Model,
    half_life_days: i64,
    end_percent: i64,
    deals: &[Won],
    touches: &mut HashMap<Uuid, Vec<Touch>>,
) -> Shared {
    let mut tallies: BTreeMap<Uuid, Tally> = BTreeMap::new();
    let mut unattributed: BTreeMap<String, i64> = BTreeMap::new();
    let mut unattributed_dated = Vec::new();
    let mut unattributed_deals = 0;
    for deal in deals {
        let mut list = touches.remove(&deal.pid).unwrap_or_default();
        // The recorded source campaign is always a touch, at the deal's creation (CRM-R49).
        if let Some(campaign) = deal.source_campaign {
            list.push(Touch {
                campaign,
                at: deal.created_at,
            });
        }
        if list.is_empty() {
            unattributed_deals += 1;
            add(&mut unattributed, &deal.currency, deal.amount_minor);
            unattributed_dated.push((
                deal.closed_at.date_naive(),
                deal.currency.clone(),
                deal.amount_minor,
            ));
            continue;
        }
        for touch in &list {
            tallies.entry(touch.campaign).or_default().touches += 1;
        }
        for campaign in list.iter().map(|t| t.campaign).collect::<BTreeSet<_>>() {
            tallies.entry(campaign).or_default().deals += 1;
        }
        for (campaign, share) in rules::credit_tuned(
            model,
            deal.amount_minor,
            &list,
            deal.closed_at,
            half_life_days,
            end_percent,
        ) {
            let tally = tallies.entry(campaign).or_default();
            add(&mut tally.credit, &deal.currency, share);
            tally
                .dated
                .push((deal.closed_at.date_naive(), deal.currency.clone(), share));
        }
    }
    Shared {
        tallies,
        unattributed,
        unattributed_dated,
        unattributed_deals,
    }
}

/// With `convert_to`, each campaign's credit in each currency is converted once and merged.
async fn convert_credit(
    ctx: &AppContext,
    query: &AttributionQuery,
    shared: &Shared,
) -> Result<Option<fx_rates::Done>> {
    let Some(target) = &query.convert_to else {
        return Ok(None);
    };
    let Shared {
        tallies,
        unattributed,
        unattributed_dated,
        ..
    } = shared;
    if query.rate_basis.is_some() {
        // Each deal's share at the rate of the day the deal closed (CRM-R85), converted once.
        let mut rows: Vec<(String, NaiveDate, String, i64)> = Vec::new();
        for (pid, tally) in tallies {
            for (day, currency, amount) in &tally.dated {
                rows.push((pid.to_string(), *day, currency.clone(), *amount));
            }
        }
        for (day, currency, amount) in unattributed_dated {
            rows.push((UNATTRIBUTED.to_string(), *day, currency.clone(), *amount));
        }
        let (rows, rates, unconverted) = fx_rates::convert_rows_dated(ctx, target, &rows).await?;
        return Ok(Some(fx_rates::Done {
            rows,
            // Not shown: with a basis each deal has its own day, and the answer says `null`.
            rate_date: super::organization::organization_today(&ctx.db).await?,
            rates,
            unconverted,
        }));
    }
    let mut sums: Vec<(String, String, i64)> = Vec::new();
    for (pid, tally) in tallies {
        for (currency, amount) in &tally.credit {
            sums.push((pid.to_string(), currency.clone(), *amount));
        }
    }
    for (currency, amount) in unattributed {
        sums.push((UNATTRIBUTED.to_string(), currency.clone(), *amount));
    }
    fx_rates::convert_sums(ctx, target, query.rate_date, "rate_date", &sums)
        .await
        .map(Some)
}

/// `GET /api/attribution?model=&window_days=&from=&to=` — share the revenue of won deals among
/// the campaigns that touched the buyer.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one handler: read, share, convert and answer
async fn run(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    axum::extract::Query(query): axum::extract::Query<AttributionQuery>,
) -> Result<Response> {
    let Asked {
        model,
        days,
        kinds,
        half_life_days,
        end_percent,
        min_open_seconds,
        min_click_seconds,
    } = validate(&query)?;
    // The answer is money, so a masked collection read may not have it.
    if auth::aggregate_masked(&caller).map_err(record_rejection)? {
        return Err(record_rejection((
            axum::http::StatusCode::FORBIDDEN,
            "attribution is about amounts and needs an unmasked read".to_string(),
        )));
    }
    let mut deals = won_deals(&ctx, &query).await?;
    let truncated = deals.len() > MAX_DEALS;
    deals.truncate(MAX_DEALS);
    let mut touches = touches_of(
        &ctx,
        &deals,
        days,
        kinds,
        min_open_seconds,
        min_click_seconds,
    )
    .await?;

    let shared = share_out(model, half_life_days, end_percent, &deals, &mut touches);
    let mut conversion = convert_credit(&ctx, &query, &shared).await?;
    let Shared {
        tallies,
        unattributed,
        unattributed_deals,
        ..
    } = shared;

    let names = names_of(&ctx, &tallies.keys().copied().collect()).await?;
    let converted: BTreeMap<String, i64> = conversion
        .as_ref()
        .map(|done| done.rows.iter().cloned().collect())
        .unwrap_or_default();
    let in_target = |key: &str| {
        let target = query.convert_to.clone().unwrap_or_default();
        converted
            .get(key)
            .map(|amount_minor| {
                vec![Amount {
                    currency: target,
                    amount_minor: *amount_minor,
                }]
            })
            .unwrap_or_default()
    };
    let mut campaigns: Vec<CampaignCredit> = tallies
        .into_iter()
        .map(|(pid, tally)| CampaignCredit {
            campaign_pid: pid.to_string(),
            name: names
                .get(&pid)
                .cloned()
                .unwrap_or_else(|| "(deleted campaign)".to_string()),
            touches: tally.touches,
            deals: tally.deals,
            credit: if conversion.is_some() {
                in_target(&pid.to_string())
            } else {
                amounts(tally.credit)
            },
        })
        .collect();
    campaigns.sort_by(|a, b| {
        b.deals
            .cmp(&a.deals)
            .then(b.touches.cmp(&a.touches))
            .then_with(|| a.name.cmp(&b.name))
    });

    Audit::record_read(
        &ctx.db,
        "attribution",
        Uuid::nil(),
        "attribution_run",
        caller.actor(),
        Some(serde_json::json!({
            "model": model.name(), "window_days": days, "deals": deals.len(),
            "half_life_days": (model == Model::TimeDecay).then_some(half_life_days),
            "end_percent": (model == Model::Position).then_some(end_percent),
            "min_open_seconds": (min_open_seconds > 0).then_some(min_open_seconds),
            "min_click_seconds": (min_click_seconds > 0).then_some(min_click_seconds),
        })),
    )
    .await?;
    format::json(Attribution {
        model: model.name().to_string(),
        window_days: days,
        half_life_days: (model == Model::TimeDecay).then_some(half_life_days),
        end_percent: (model == Model::Position).then_some(end_percent),
        min_open_seconds: (min_open_seconds > 0).then_some(min_open_seconds),
        min_click_seconds: (min_click_seconds > 0).then_some(min_click_seconds),
        touches: kinds.names(),
        as_of: Utc::now(),
        deals: deals.len(),
        unattributed_deals,
        unattributed: if conversion.is_some() {
            in_target(UNATTRIBUTED)
        } else {
            amounts(unattributed)
        },
        campaigns,
        truncated,
        convert_to: query.convert_to.clone(),
        rate_date: conversion
            .as_ref()
            .filter(|_| query.rate_basis.is_none())
            .map(|d| d.rate_date),
        rate_basis: query.rate_basis.clone(),
        rates: conversion
            .as_mut()
            .map(|d| std::mem::take(&mut d.rates))
            .unwrap_or_default(),
        unconverted: conversion
            .as_mut()
            .map(|d| std::mem::take(&mut d.unconverted))
            .unwrap_or_default(),
        note: "each won deal's amount is shared among the campaigns that touched its primary \
               contact (e-mails sent, campaign leads) in the window before it closed, and its \
               recorded source campaign; shares add up to the amount; per currency, never \
               added across currencies",
    })
}

/// The routes of this module.
pub fn routes() -> Routes {
    Routes::new().prefix("/api").add("/attribution", get(run))
}
