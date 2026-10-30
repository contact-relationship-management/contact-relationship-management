//! Quotas, commission plans and attainment (CRM-R59).
//!
//! `/api/commission-plans` and `/api/quotas` are kept by hand and audited; `GET
//! /api/quotas/attainment` reads the won deals against them. All three are admin-only (see
//! [`crate::auth::policy::ADMIN_ONLY_PREFIXES`]): what a person is paid is neither theirs to set
//! nor everyone's to read.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::merge::conflict;
use super::views::owner_of;
use super::{PidPath, PidRef};
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::analytics::Ratio;
use crate::rules::fx;
use crate::rules::quota::{self, Band, Tier};
use crate::rules::user_ref;
use crate::validation::Problems;

/// Most rows listed.
const MAX_LISTED: i64 = 1000;
/// Longest period a quota may cover, in days.
const MAX_PERIOD_DAYS: i64 = 366;

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

// ------------------------------------------------------------------ plans

/// `PUT /api/commission-plans` body: sets the plan of that name (any case), replacing its tiers.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "CommissionPlanPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanPayload {
    /// Its name, unique in any case.
    name: String,
    /// The tiers, marginal: the first starts at `0` and each starts above the one before.
    tiers: Vec<Tier>,
}

/// A commission plan.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "CommissionPlan")]
pub(crate) struct PlanView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    tiers: Vec<Tier>,
    /// How many quotas are paid under it.
    quotas: i64,
    /// Who set it.
    set_by: String,
}

const PLAN_COLUMNS: &str = "p.pid, p.name, p.tiers, p.set_by, \
     (SELECT count(*) FROM quotas q WHERE q.plan_pid = p.pid)::bigint AS in_use";

fn plan_view(row: &sea_orm::QueryResult) -> Result<PlanView> {
    let tiers: serde_json::Value = row.try_get("", "tiers")?;
    Ok(PlanView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        name: row.try_get("", "name")?,
        tiers: serde_json::from_value(tiers).map_err(|e| Error::string(&e.to_string()))?,
        quotas: row.try_get("", "in_use")?,
        set_by: row.try_get("", "set_by")?,
    })
}

/// `GET /api/commission-plans` — the plans, by name.
#[debug_handler]
async fn list_plans(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {PLAN_COLUMNS} FROM commission_plans p ORDER BY lower(p.name) LIMIT $1"
            ),
            [MAX_LISTED.into()],
        ))
        .await?;
    format::json(rows.iter().map(plan_view).collect::<Result<Vec<_>>>()?)
}

/// `PUT /api/commission-plans` — set a plan; audited.
#[debug_handler]
async fn set_plan(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PlanPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    let name = quota::name(&payload.name)
        .map_err(|why| problems.push_field("name", why))
        .ok();
    if let Err(why) = quota::check_tiers(&payload.tiers) {
        problems.push_field("tiers", why);
    }
    problems.finish()?;
    let Some(name) = name else {
        return Err(super::unprocessable("the plan cannot be set"));
    };
    let who = owner_of(&caller)?;
    let tiers = serde_json::to_value(&payload.tiers).map_err(|e| Error::string(&e.to_string()))?;
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!(
                "WITH p AS ( \
                   INSERT INTO commission_plans (pid, name, tiers, set_by) \
                   VALUES ($1, $2, $3::jsonb, $4) \
                   ON CONFLICT ((lower(name))) DO UPDATE SET name = EXCLUDED.name, \
                     tiers = EXCLUDED.tiers, set_by = EXCLUDED.set_by, updated_at = now() \
                   RETURNING * ) \
                 SELECT {PLAN_COLUMNS} FROM p"
            ),
            [
                Uuid::new_v4().into(),
                name.as_str().into(),
                tiers.into(),
                who.into(),
            ],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let made = plan_view(&row)?;
    let pid: Uuid = row.try_get("", "pid")?;
    Audit::record(
        &txn,
        "commission_plan",
        pid,
        "set",
        caller.actor(),
        Some(serde_json::json!({ "name": made.name, "tiers": made.tiers })),
    )
    .await?;
    txn.commit().await?;
    format::json(made)
}

/// `DELETE /api/commission-plans/{pid}` — remove a plan no quota uses; audited. `404` when there is
/// none, `409` when a quota is paid under it.
#[debug_handler]
async fn remove_plan(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!(
                "SELECT {PLAN_COLUMNS} FROM commission_plans p WHERE p.pid = $1 FOR UPDATE OF p"
            ),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let plan = plan_view(&row)?;
    if plan.quotas > 0 {
        return Err(conflict("a quota is still paid under this plan"));
    }
    txn.execute_raw(stmt(
        "DELETE FROM commission_plans WHERE pid = $1",
        [pid.into()],
    ))
    .await?;
    Audit::record(
        &txn,
        "commission_plan",
        pid,
        "deleted",
        caller.actor(),
        Some(serde_json::json!({ "name": plan.name })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

// ----------------------------------------------------------------- quotas

/// `PUT /api/quotas` body: sets the quota of a worker for a period in a currency, replacing any
/// already there for exactly that period.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "QuotaPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct QuotaPayload {
    /// The user, `person:<id>` (`worker:` is read as `person:`, CRM-D18).
    owner_ref: String,
    /// First day of the period.
    period_from: NaiveDate,
    /// Last day of the period, inclusive; at most 366 days after the first.
    period_to: NaiveDate,
    /// An ISO 4217 code (CRM-T168).
    currency: String,
    /// What is to be won, in minor units; above zero.
    amount_minor: i64,
    /// The commission plan it is paid under, if any.
    #[schemars(with = "Option<uuid::Uuid>")]
    plan_pid: Option<Uuid>,
}

/// A quota.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Quota")]
pub(crate) struct QuotaView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    owner_ref: String,
    period_from: NaiveDate,
    period_to: NaiveDate,
    currency: String,
    amount_minor: i64,
    #[schemars(with = "Option<uuid::Uuid>")]
    plan_pid: Option<String>,
    set_by: String,
}

const QUOTA_COLUMNS: &str =
    "pid, owner_ref, period_from, period_to, currency, amount_minor, plan_pid, set_by";

fn quota_view(row: &sea_orm::QueryResult) -> Result<QuotaView> {
    Ok(QuotaView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        owner_ref: row.try_get("", "owner_ref")?,
        period_from: row.try_get("", "period_from")?,
        period_to: row.try_get("", "period_to")?,
        currency: row.try_get::<String>("", "currency")?.trim().to_string(),
        amount_minor: row.try_get("", "amount_minor")?,
        plan_pid: row
            .try_get::<Option<Uuid>>("", "plan_pid")?
            .map(|p| p.to_string()),
        set_by: row.try_get("", "set_by")?,
    })
}

/// What `GET /api/quotas` may be asked.
#[derive(Debug, Deserialize)]
struct QuotaQuery {
    /// Only this worker's quotas.
    owner_ref: Option<String>,
}

/// `GET /api/quotas?owner_ref=` — the quotas, latest period first.
#[debug_handler]
async fn list_quotas(
    State(ctx): State<AppContext>,
    axum::extract::Query(query): axum::extract::Query<QuotaQuery>,
) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {QUOTA_COLUMNS} FROM quotas WHERE ($1::text IS NULL OR owner_ref = $1) \
                 ORDER BY period_from DESC, owner_ref, currency LIMIT $2"
            ),
            [
                user_ref::normalise_opt(query.owner_ref).into(),
                MAX_LISTED.into(),
            ],
        ))
        .await?;
    format::json(rows.iter().map(quota_view).collect::<Result<Vec<_>>>()?)
}

/// `PUT /api/quotas` — set a quota; audited.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one validation per member, then the write
async fn set_quota(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<QuotaPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    payload.owner_ref = user_ref::normalise(&payload.owner_ref);
    problems.require_user("owner_ref", &payload.owner_ref);
    let currency = fx::currency(&payload.currency)
        .map_err(|why| problems.push_field("currency", why))
        .ok();
    if payload.amount_minor <= 0 {
        problems.push_field("amount_minor", "a quota is more than zero");
    }
    if payload.period_to < payload.period_from {
        problems.push_field("period_to", "the period ends before it starts");
    } else if (payload.period_to - payload.period_from).num_days() >= MAX_PERIOD_DAYS {
        problems.push_field(
            "period_to",
            format!("a period is at most {MAX_PERIOD_DAYS} days"),
        );
    }
    problems.finish()?;
    let Some(currency) = currency else {
        return Err(super::unprocessable("the quota cannot be set"));
    };

    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    let mut later = Problems::new();
    if let Some(plan) = payload.plan_pid
        && txn
            .query_one_raw(stmt(
                "SELECT 1 FROM commission_plans WHERE pid = $1",
                [plan.into()],
            ))
            .await?
            .is_none()
    {
        later.push_field("plan_pid", "there is no such plan");
    }
    // One quota covers a day for a worker and currency, so attainment is never ambiguous.
    if txn
        .query_one_raw(stmt(
            "SELECT 1 FROM quotas WHERE owner_ref = $1 AND currency = $2 \
               AND period_from <= $4 AND period_to >= $3 \
               AND NOT (period_from = $3 AND period_to = $4)",
            [
                payload.owner_ref.as_str().into(),
                currency.as_str().into(),
                payload.period_from.into(),
                payload.period_to.into(),
            ],
        ))
        .await?
        .is_some()
    {
        later.push_field(
            "period_from",
            "overlaps another quota of this worker in this currency",
        );
    }
    later.finish()?;
    let row = txn
        .query_one_raw(stmt(
            &format!(
                "INSERT INTO quotas (pid, owner_ref, period_from, period_to, currency, \
                                     amount_minor, plan_pid, set_by) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                 ON CONFLICT (owner_ref, period_from, period_to, currency) DO UPDATE SET \
                   amount_minor = EXCLUDED.amount_minor, plan_pid = EXCLUDED.plan_pid, \
                   set_by = EXCLUDED.set_by, updated_at = now() \
                 RETURNING {QUOTA_COLUMNS}"
            ),
            [
                Uuid::new_v4().into(),
                payload.owner_ref.as_str().into(),
                payload.period_from.into(),
                payload.period_to.into(),
                currency.as_str().into(),
                payload.amount_minor.into(),
                payload.plan_pid.into(),
                who.into(),
            ],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let made = quota_view(&row)?;
    Audit::record(
        &txn,
        "quota",
        row.try_get("", "pid")?,
        "set",
        caller.actor(),
        Some(serde_json::json!({
            "owner_ref": made.owner_ref, "period_from": made.period_from,
            "period_to": made.period_to, "currency": made.currency,
            "amount_minor": made.amount_minor, "plan_pid": made.plan_pid,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(made)
}

/// `DELETE /api/quotas/{pid}` — remove a quota; audited. `404` when there is none.
#[debug_handler]
async fn remove_quota(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!("DELETE FROM quotas WHERE pid = $1 RETURNING {QUOTA_COLUMNS}"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let gone = quota_view(&row)?;
    Audit::record(
        &txn,
        "quota",
        pid,
        "deleted",
        caller.actor(),
        Some(serde_json::json!({
            "owner_ref": gone.owner_ref, "period_from": gone.period_from,
            "period_to": gone.period_to, "currency": gone.currency,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

// ----------------------------------------------------------------- splits

/// Most workers a deal may be split between.
const MAX_SHARES: usize = 8;

/// One worker's share of a deal.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[schemars(rename = "DealShare")]
#[serde(deny_unknown_fields)]
pub(crate) struct Share {
    /// The user, `person:<id>` (`worker:` is read as `person:`, CRM-D18).
    owner_ref: String,
    /// Their share in basis points, 1 to 10000.
    share_bps: u32,
}

/// `PUT /api/quotas/splits/{pid}` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "DealSplitPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct SplitPayload {
    /// Who is credited with the deal when it is won; the shares add up to 10000 and name each
    /// worker once. At most 8.
    shares: Vec<Share>,
}

/// How a deal is credited for quotas.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "DealSplit")]
pub(crate) struct SplitView {
    #[schemars(with = "uuid::Uuid")]
    deal_pid: String,
    /// Empty when the deal is credited wholly to its owner.
    shares: Vec<Share>,
}

async fn shares_of<C: ConnectionTrait>(db: &C, deal: Uuid) -> Result<Vec<Share>> {
    db.query_all_raw(stmt(
        "SELECT owner_ref, share_bps FROM deal_splits WHERE deal_pid = $1 ORDER BY id",
        [deal.into()],
    ))
    .await?
    .iter()
    .map(|r| {
        Ok(Share {
            owner_ref: r.try_get("", "owner_ref")?,
            share_bps: u32::try_from(r.try_get::<i32>("", "share_bps")?).unwrap_or(0),
        })
    })
    .collect()
}

async fn deal_exists<C: ConnectionTrait>(db: &C, deal: Uuid) -> Result<()> {
    db.query_one_raw(stmt(
        "SELECT 1 FROM deals WHERE pid = $1 AND deleted_at IS NULL",
        [deal.into()],
    ))
    .await?
    .map(|_| ())
    .ok_or(Error::NotFound)
}

/// `GET /api/quotas/splits/{pid}` — how a deal is credited. `404` for an unknown deal.
#[debug_handler]
async fn get_split(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    deal_exists(&ctx.db, pid).await?;
    format::json(SplitView {
        deal_pid: pid.to_string(),
        shares: shares_of(&ctx.db, pid).await?,
    })
}

/// `PUT /api/quotas/splits/{pid}` — credit a deal to several workers, replacing any earlier
/// split; audited (CRM-R65).
#[debug_handler]
async fn set_split(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(mut payload): Json<SplitPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    if payload.shares.is_empty() || payload.shares.len() > MAX_SHARES {
        problems.push_field("shares", format!("from 1 to {MAX_SHARES} workers"));
    }
    for share in &mut payload.shares {
        share.owner_ref = user_ref::normalise(&share.owner_ref);
    }
    for share in &payload.shares {
        problems.require_user("shares", &share.owner_ref);
        if !(1..=10_000).contains(&share.share_bps) {
            problems.push_field("shares", "a share is from 1 to 10000 basis points");
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    if !payload
        .shares
        .iter()
        .all(|s| seen.insert(s.owner_ref.as_str()))
    {
        problems.push_field("shares", "names a worker twice");
    }
    if payload
        .shares
        .iter()
        .map(|s| u64::from(s.share_bps))
        .sum::<u64>()
        != 10_000
    {
        problems.push_field("shares", "the shares add up to 10000");
    }
    problems.finish()?;
    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    deal_exists(&txn, pid).await?;
    txn.execute_raw(stmt(
        "DELETE FROM deal_splits WHERE deal_pid = $1",
        [pid.into()],
    ))
    .await?;
    for share in &payload.shares {
        txn.execute_raw(stmt(
            "INSERT INTO deal_splits (deal_pid, owner_ref, share_bps, set_by) VALUES ($1, $2, $3, $4)",
            [
                pid.into(),
                share.owner_ref.as_str().into(),
                i32::try_from(share.share_bps).unwrap_or(0).into(),
                who.as_str().into(),
            ],
        ))
        .await?;
    }
    Audit::record(
        &txn,
        "deal_split",
        pid,
        "set",
        caller.actor(),
        Some(serde_json::json!({ "shares": payload.shares })),
    )
    .await?;
    txn.commit().await?;
    format::json(SplitView {
        deal_pid: pid.to_string(),
        shares: payload.shares,
    })
}

/// `DELETE /api/quotas/splits/{pid}` — credit the deal wholly to its owner again; audited.
/// `404` when it was not split.
#[debug_handler]
async fn remove_split(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let gone = txn
        .execute_raw(stmt(
            "DELETE FROM deal_splits WHERE deal_pid = $1",
            [pid.into()],
        ))
        .await?;
    if gone.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Audit::record(&txn, "deal_split", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

// -------------------------------------------------------------- clawbacks

/// Longest reason, in characters.
const MAX_REASON: usize = 200;

/// `PUT /api/quotas/clawbacks/{pid}` body (the deal's pid).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ClawbackPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct ClawbackPayload {
    /// The user (`person:<id>`; `worker:` is read as `person:`, CRM-D18) whose credit for the deal
    /// is reduced.
    owner_ref: String,
    /// What is taken back, in minor units: above zero and no more than their credit for the deal.
    amount_minor: i64,
    /// Why, up to 200 characters.
    reason: Option<String>,
}

/// A clawback.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Clawback")]
pub(crate) struct ClawbackView {
    owner_ref: String,
    amount_minor: i64,
    reason: Option<String>,
    set_by: String,
}

/// What `DELETE /api/quotas/clawbacks/{pid}` is asked.
#[derive(Debug, Deserialize)]
struct ClawbackQuery {
    owner_ref: String,
}

/// What a worker is credited for a won, live deal before any clawback: `(currency, minor units)`,
/// or `None` when the deal is not won or does not credit them.
async fn credit_of<C: ConnectionTrait>(
    db: &C,
    deal: Uuid,
    owner: &str,
) -> Result<Option<(String, i64)>> {
    let Some(row) = db
        .query_one_raw(stmt(
            "SELECT trim(currency) AS currency, amount_minor, owner_ref FROM deals \
             WHERE pid = $1 AND won AND deleted_at IS NULL",
            [deal.into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    let amount: i64 = row.try_get("", "amount_minor")?;
    let shares = shares_of(db, deal).await?;
    let part = if shares.is_empty() {
        (row.try_get::<Option<String>>("", "owner_ref")?.as_deref() == Some(owner))
            .then_some(amount)
    } else {
        let bps: Vec<u32> = shares.iter().map(|s| s.share_bps).collect();
        shares
            .iter()
            .position(|s| s.owner_ref == owner)
            .map(|i| quota::split(amount, &bps)[i])
    };
    Ok(part.map(|p| (row.try_get("", "currency").unwrap_or_default(), p)))
}

async fn clawbacks_of<C: ConnectionTrait>(db: &C, deal: Uuid) -> Result<Vec<ClawbackView>> {
    db.query_all_raw(stmt(
        "SELECT owner_ref, amount_minor, reason, set_by FROM deal_clawbacks \
         WHERE deal_pid = $1 ORDER BY id",
        [deal.into()],
    ))
    .await?
    .iter()
    .map(|r| {
        Ok(ClawbackView {
            owner_ref: r.try_get("", "owner_ref")?,
            amount_minor: r.try_get("", "amount_minor")?,
            reason: r.try_get("", "reason")?,
            set_by: r.try_get("", "set_by")?,
        })
    })
    .collect()
}

/// `GET /api/quotas/clawbacks/{pid}` — what has been taken back on a deal. `404` for an unknown
/// deal.
#[debug_handler]
async fn get_clawbacks(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    deal_exists(&ctx.db, pid).await?;
    format::json(clawbacks_of(&ctx.db, pid).await?)
}

/// `PUT /api/quotas/clawbacks/{pid}` — take an amount back from one worker's credit for a won
/// deal, replacing their earlier clawback on it; audited (CRM-R68).
#[debug_handler]
async fn set_clawback(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(mut payload): Json<ClawbackPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    payload.owner_ref = user_ref::normalise(&payload.owner_ref);
    problems.require_user("owner_ref", &payload.owner_ref);
    if payload.amount_minor < 1 {
        problems.push_field("amount_minor", "above zero");
    }
    let reason = payload
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    if reason.is_some_and(|r| r.chars().count() > MAX_REASON) {
        problems.push_field("reason", format!("up to {MAX_REASON} characters"));
    }
    problems.finish()?;
    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    deal_exists(&txn, pid).await?;
    let Some((_, credit)) = credit_of(&txn, pid, &payload.owner_ref).await? else {
        return Err(super::unprocessable(
            "/owner_ref: the deal is not won, or does not credit that worker",
        ));
    };
    if payload.amount_minor > credit {
        return Err(super::unprocessable(
            "/amount_minor: no more than the worker's credit for the deal",
        ));
    }
    txn.execute_raw(stmt(
        "INSERT INTO deal_clawbacks (deal_pid, owner_ref, amount_minor, reason, set_by) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (deal_pid, owner_ref) DO UPDATE SET amount_minor = EXCLUDED.amount_minor, \
           reason = EXCLUDED.reason, set_by = EXCLUDED.set_by",
        [
            pid.into(),
            payload.owner_ref.as_str().into(),
            payload.amount_minor.into(),
            reason.map(str::to_string).into(),
            who.as_str().into(),
        ],
    ))
    .await?;
    Audit::record(
        &txn,
        "deal_clawback",
        pid,
        "set",
        caller.actor(),
        Some(serde_json::json!({
            "owner_ref": payload.owner_ref, "amount_minor": payload.amount_minor, "reason": reason
        })),
    )
    .await?;
    let all = clawbacks_of(&txn, pid).await?;
    txn.commit().await?;
    format::json(all)
}

/// `DELETE /api/quotas/clawbacks/{pid}?owner_ref=` — give the worker their credit back; audited.
/// `404` when there is no such clawback.
#[debug_handler]
async fn remove_clawback(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    axum::extract::Query(mut query): axum::extract::Query<ClawbackQuery>,
) -> Result<Response> {
    query.owner_ref = user_ref::normalise(&query.owner_ref);
    let txn = ctx.db.begin().await?;
    let gone = txn
        .execute_raw(stmt(
            "DELETE FROM deal_clawbacks WHERE deal_pid = $1 AND owner_ref = $2",
            [pid.into(), query.owner_ref.as_str().into()],
        ))
        .await?;
    if gone.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Audit::record(
        &txn,
        "deal_clawback",
        pid,
        "deleted",
        caller.actor(),
        Some(serde_json::json!({ "owner_ref": query.owner_ref })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

// ------------------------------------------------------------- attainment

/// What `GET /api/quotas/attainment` may be asked.
#[derive(Debug, Deserialize)]
struct AttainmentQuery {
    /// Only this worker.
    owner_ref: Option<String>,
    /// The day the quota's period must contain (today when absent).
    at: Option<NaiveDate>,
}

/// One worker's attainment of one quota.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[allow(clippy::struct_field_names)] // `attainment` is the ratio the page and clients read
#[schemars(rename = "QuotaAttainment")]
pub(crate) struct Attainment {
    #[schemars(with = "uuid::Uuid")]
    quota_pid: String,
    pub(super) owner_ref: String,
    period_from: NaiveDate,
    period_to: NaiveDate,
    pub(super) currency: String,
    pub(super) quota_minor: i64,
    /// Won revenue of the worker's deals closed in the period, in the quota's currency.
    pub(super) revenue_minor: i64,
    /// How many deals made it.
    pub(super) won_deals: i64,
    /// `revenue ÷ quota`, with both numbers.
    attainment: Ratio,
    /// What is still to win; zero once the quota is met.
    remaining_minor: i64,
    /// Won revenue in other currencies, never added in or converted.
    pub(super) other_currencies: BTreeMap<String, i64>,
    /// The plan the quota is paid under, when it has one.
    plan: Option<String>,
    /// What the plan pays on `revenue_minor`; `null` without a plan.
    commission_minor: Option<i64>,
    /// Revenue and commission band by band; empty without a plan.
    bands: Vec<Band>,
}

/// The answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "QuotaAttainmentReport")]
pub(crate) struct AttainmentReport {
    as_of: chrono::DateTime<chrono::Utc>,
    /// The day the periods contain.
    at: NaiveDate,
    rows: Vec<Attainment>,
    note: &'static str,
}

/// The won, live deals closed in the period that are credited to `owner`, as `(currency, amount
/// credited)`: the whole amount for a deal they own that is not split, and their share of one that
/// is (CRM-R65).
async fn credited_deals<C: ConnectionTrait>(
    db: &C,
    owner: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(String, i64)>> {
    let deals = db
        .query_all_raw(stmt(
            "SELECT d.pid, trim(d.currency) AS currency, d.amount_minor, d.owner_ref \
             FROM deals d \
             WHERE d.won AND d.deleted_at IS NULL AND d.closed_at IS NOT NULL \
               AND d.closed_at::date BETWEEN $2 AND $3 \
               AND (d.owner_ref = $1 OR EXISTS \
                    (SELECT 1 FROM deal_splits s WHERE s.deal_pid = d.pid AND s.owner_ref = $1))",
            [owner.into(), from.into(), to.into()],
        ))
        .await?;
    let mut credited = Vec::new();
    for deal in &deals {
        let pid: Uuid = deal.try_get("", "pid")?;
        let amount: i64 = deal.try_get("", "amount_minor")?;
        let shares = shares_of(db, pid).await?;
        let mine = if shares.is_empty() {
            (deal.try_get::<Option<String>>("", "owner_ref")?.as_deref() == Some(owner))
                .then_some(amount)
        } else {
            let bps: Vec<u32> = shares.iter().map(|s| s.share_bps).collect();
            shares
                .iter()
                .position(|s| s.owner_ref == owner)
                .map(|i| quota::split(amount, &bps)[i])
        };
        // A clawback takes from the worker's credit, never below nothing; a deal wholly clawed back
        // no longer counts (CRM-R68).
        let clawed: Option<i64> = db
            .query_one_raw(stmt(
                "SELECT amount_minor FROM deal_clawbacks WHERE deal_pid = $1 AND owner_ref = $2",
                [pid.into(), owner.into()],
            ))
            .await?
            .map(|r| r.try_get("", "amount_minor"))
            .transpose()?;
        let mine = mine.map(|part| part.saturating_sub(clawed.unwrap_or(0)).max(0));
        if let Some(part) = mine.filter(|p| *p > 0) {
            credited.push((deal.try_get("", "currency")?, part));
        }
    }
    Ok(credited)
}

/// Every quota whose period contains `at` (of `owner_filter` alone, when given) with the revenue its
/// worker's credited deals reached and the commission its plan pays.
pub(super) async fn standings<C: ConnectionTrait>(
    db: &C,
    at: NaiveDate,
    owner_filter: Option<String>,
) -> Result<Vec<Attainment>> {
    let quotas = db
        .query_all_raw(stmt(
            "SELECT q.pid, q.owner_ref, q.period_from, q.period_to, q.currency, q.amount_minor, \
                    p.name AS plan_name, p.tiers AS plan_tiers \
             FROM quotas q LEFT JOIN commission_plans p ON p.pid = q.plan_pid \
             WHERE q.period_from <= $1 AND q.period_to >= $1 \
               AND ($2::text IS NULL OR q.owner_ref = $2) \
             ORDER BY q.owner_ref, q.currency LIMIT $3",
            [
                at.into(),
                owner_filter.map_or(Value::String(None), Value::from),
                MAX_LISTED.into(),
            ],
        ))
        .await?;
    let mut rows = Vec::with_capacity(quotas.len());
    for q in &quotas {
        let owner: String = q.try_get("", "owner_ref")?;
        let (from, to): (NaiveDate, NaiveDate) =
            (q.try_get("", "period_from")?, q.try_get("", "period_to")?);
        let currency = q.try_get::<String>("", "currency")?.trim().to_string();
        let quota_minor: i64 = q.try_get("", "amount_minor")?;
        let (mut revenue, mut deals, mut other) = (0_i64, 0_i64, BTreeMap::new());
        for (c, credited) in credited_deals(db, &owner, from, to).await? {
            if c == currency {
                revenue = revenue.saturating_add(credited);
                deals += 1;
            } else {
                let slot: &mut i64 = other.entry(c).or_default();
                *slot = slot.saturating_add(credited);
            }
        }
        let tiers: Option<Vec<Tier>> = q
            .try_get::<Option<serde_json::Value>>("", "plan_tiers")?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| Error::string(&e.to_string()))?;
        let bands = tiers
            .as_deref()
            .map(|t| quota::bands(quota_minor, revenue, t))
            .unwrap_or_default();
        rows.push(Attainment {
            quota_pid: q.try_get::<Uuid>("", "pid")?.to_string(),
            owner_ref: owner,
            period_from: from,
            period_to: to,
            currency,
            quota_minor,
            revenue_minor: revenue,
            won_deals: deals,
            attainment: quota::attainment(quota_minor, revenue),
            remaining_minor: quota::remaining(quota_minor, revenue),
            other_currencies: other,
            plan: q.try_get("", "plan_name")?,
            commission_minor: tiers.as_ref().map(|_| quota::commission(&bands)),
            bands,
        });
    }
    Ok(rows)
}

/// `GET /api/quotas/attainment?owner_ref=&at=` — each quota whose period contains `at`, with the
/// revenue the worker's won deals reached in it and the commission its plan pays. An audited read.
#[debug_handler]
async fn attainment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<AttainmentQuery>,
) -> Result<Response> {
    // Today in the caller's zone, else the organization's (CRM-T171), not UTC's.
    let at = match query.at {
        Some(at) => at,
        None => super::organization::caller_today(&ctx.db, &headers).await?,
    };
    let rows = standings(&ctx.db, at, user_ref::normalise_opt(query.owner_ref)).await?;
    Audit::record_read(
        &ctx.db,
        "quota",
        Uuid::nil(),
        "attainment_read",
        caller.actor(),
        Some(serde_json::json!({ "at": at, "quotas": rows.len() })),
    )
    .await?;
    format::json(AttainmentReport {
        as_of: chrono::Utc::now(),
        at,
        rows,
        note: "revenue is the amount of the worker's won, live deals closed in the period, in the \
               quota's currency only (other currencies are listed, never added or converted); \
               commission is a statement of what the plan pays, not a payment",
    })
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/commission-plans", get(list_plans))
        .add("/commission-plans", put(set_plan))
        .add("/commission-plans/{pid}", delete(remove_plan))
        .add("/quotas", get(list_quotas))
        .add("/quotas", put(set_quota))
        .add("/quotas/attainment", get(attainment))
        .add("/quotas/splits/{pid}", get(get_split))
        .add("/quotas/splits/{pid}", put(set_split))
        .add("/quotas/splits/{pid}", delete(remove_split))
        .add("/quotas/clawbacks/{pid}", get(get_clawbacks))
        .add("/quotas/clawbacks/{pid}", put(set_clawback))
        .add("/quotas/clawbacks/{pid}", delete(remove_clawback))
        .add("/quotas/{pid}", delete(remove_quota))
}
