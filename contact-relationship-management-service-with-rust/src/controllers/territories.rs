//! Territories of named accounts and the revenue won in them (CRM-R68).
//!
//! `/api/quotas/territories` is kept by hand and audited; `GET /api/quotas/territory-attainment`
//! adds up the won deals on each territory's accounts. Both sit under the admin-only `/api/quotas`
//! prefix, like the quotas themselves.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use chrono::NaiveDate;
use entity_ref::{EntityRef, EntityType};
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::views::owner_of;
use super::{PidPath, PidRef};
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::quota;
use crate::rules::user_ref;
use crate::validation::Problems;

/// Most accounts in a territory.
const MAX_ACCOUNTS: usize = 200;
/// Most territories listed.
const MAX_LISTED: i64 = 500;
/// Longest period the attainment may cover, in days.
const MAX_PERIOD_DAYS: i64 = 366;
/// The period read when none is asked for, in days.
const DEFAULT_PERIOD_DAYS: i64 = 90;

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// `PUT /api/quotas/territories` body: sets the territory of that name (any case), replacing its
/// accounts and worker.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "TerritoryPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct TerritoryPayload {
    /// Its name, up to 80 characters.
    name: String,
    /// The user (`person:<id>`; `worker:` is read as `person:`, CRM-D18) it is assigned to, if any.
    owner_ref: Option<String>,
    /// The accounts in it (`organization:<id>`), from 1 to 200, each named once; an account is in
    /// at most one territory.
    accounts: Vec<String>,
}

/// A territory.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Territory")]
pub(crate) struct TerritoryView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    owner_ref: Option<String>,
    accounts: Vec<String>,
    set_by: String,
}

async fn territories<C: ConnectionTrait>(db: &C) -> Result<Vec<TerritoryView>> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT pid, name, owner_ref, set_by FROM territories ORDER BY lower(name) LIMIT $1",
            [MAX_LISTED.into()],
        ))
        .await?;
    let mut by_pid: BTreeMap<Uuid, Vec<String>> = BTreeMap::new();
    for row in db
        .query_all_raw(stmt(
            "SELECT territory_pid, organization_ref FROM territory_accounts ORDER BY organization_ref",
            [],
        ))
        .await?
    {
        by_pid
            .entry(row.try_get("", "territory_pid")?)
            .or_default()
            .push(row.try_get("", "organization_ref")?);
    }
    rows.iter()
        .map(|row| {
            let pid: Uuid = row.try_get("", "pid")?;
            Ok(TerritoryView {
                pid: pid.to_string(),
                name: row.try_get("", "name")?,
                owner_ref: row.try_get("", "owner_ref")?,
                accounts: by_pid.remove(&pid).unwrap_or_default(),
                set_by: row.try_get("", "set_by")?,
            })
        })
        .collect()
}

/// `GET /api/quotas/territories` — the territories, by name.
#[debug_handler]
async fn list_territories(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(territories(&ctx.db).await?)
}

/// `PUT /api/quotas/territories` — set a territory; audited. `422` naming `/name`, `/owner_ref` or
/// `/accounts` (also when an account is already in another territory).
#[debug_handler]
async fn set_territory(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<TerritoryPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    let name = quota::name(&payload.name)
        .map_err(|why| problems.push_field("name", why.replace("a plan", "a territory")))
        .ok();
    payload.owner_ref = user_ref::normalise_opt(payload.owner_ref.take());
    problems.user_opt("owner_ref", payload.owner_ref.as_deref());
    if payload.accounts.is_empty() || payload.accounts.len() > MAX_ACCOUNTS {
        problems.push_field("accounts", format!("from 1 to {MAX_ACCOUNTS} accounts"));
    }
    let mut refs = Vec::with_capacity(payload.accounts.len());
    for account in &payload.accounts {
        problems.require_ref("accounts", EntityType::Organization, account);
        if let Ok(r) = EntityRef::from_str(account) {
            refs.push(r.to_string());
        }
    }
    if refs.iter().collect::<BTreeSet<_>>().len() != refs.len() {
        problems.push_field("accounts", "names an account twice");
    }
    problems.finish()?;
    let Some(name) = name else {
        return Err(super::unprocessable("the territory cannot be set"));
    };
    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    let taken = txn
        .query_all_raw(stmt(
            "SELECT a.organization_ref, t.name FROM territory_accounts a \
             JOIN territories t ON t.pid = a.territory_pid \
             WHERE a.organization_ref = ANY($1) AND lower(t.name) <> lower($2) \
             ORDER BY a.organization_ref LIMIT 1",
            [refs.clone().into(), name.as_str().into()],
        ))
        .await?;
    if let Some(row) = taken.first() {
        let other: String = row.try_get("", "name")?;
        return Err(super::unprocessable(&format!(
            "/accounts: {} is already in the territory {other}",
            row.try_get::<String>("", "organization_ref")?
        )));
    }
    let row = txn
        .query_one_raw(stmt(
            "INSERT INTO territories (pid, name, owner_ref, set_by) VALUES ($1, $2, $3, $4) \
             ON CONFLICT ((lower(name))) DO UPDATE SET name = EXCLUDED.name, \
               owner_ref = EXCLUDED.owner_ref, set_by = EXCLUDED.set_by, updated_at = now() \
             RETURNING pid",
            [
                Uuid::new_v4().into(),
                name.as_str().into(),
                payload.owner_ref.clone().into(),
                who.into(),
            ],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let pid: Uuid = row.try_get("", "pid")?;
    txn.execute_raw(stmt(
        "DELETE FROM territory_accounts WHERE territory_pid = $1",
        [pid.into()],
    ))
    .await?;
    for account in &refs {
        txn.execute_raw(stmt(
            "INSERT INTO territory_accounts (organization_ref, territory_pid) VALUES ($1, $2)",
            [account.as_str().into(), pid.into()],
        ))
        .await?;
    }
    Audit::record(
        &txn,
        "territory",
        pid,
        "set",
        caller.actor(),
        Some(serde_json::json!({
            "name": name, "owner_ref": payload.owner_ref, "accounts": refs
        })),
    )
    .await?;
    let made = territories(&txn)
        .await?
        .into_iter()
        .find(|t| t.pid == pid.to_string())
        .ok_or(Error::NotFound)?;
    txn.commit().await?;
    format::json(made)
}

/// `DELETE /api/quotas/territories/{pid}` — remove a territory (its accounts are free again);
/// audited. `404` when there is none.
#[debug_handler]
async fn remove_territory(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let gone = txn
        .execute_raw(stmt("DELETE FROM territories WHERE pid = $1", [pid.into()]))
        .await?;
    if gone.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Audit::record(&txn, "territory", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// What `GET /api/quotas/territory-attainment` may be asked.
#[derive(Debug, Deserialize)]
struct TerritoryQuery {
    /// First day of the period (90 days before `to` when absent).
    from: Option<NaiveDate>,
    /// Last day of the period (today when absent).
    to: Option<NaiveDate>,
}

/// Won revenue in one currency.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TerritoryRevenue")]
pub(crate) struct TerritoryRevenue {
    currency: String,
    revenue_minor: i64,
    won_deals: i64,
}

/// One territory's won revenue.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TerritoryStanding")]
pub(crate) struct TerritoryStanding {
    #[schemars(with = "uuid::Uuid")]
    territory_pid: String,
    name: String,
    owner_ref: Option<String>,
    accounts: usize,
    /// One entry per currency; currencies are never added together.
    currencies: Vec<TerritoryRevenue>,
}

/// The answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TerritoryAttainmentReport")]
pub(crate) struct TerritoryReport {
    as_of: chrono::DateTime<chrono::Utc>,
    from: NaiveDate,
    to: NaiveDate,
    rows: Vec<TerritoryStanding>,
    /// Won revenue on accounts in no territory, and on deals with no account.
    unassigned: Vec<TerritoryRevenue>,
    note: &'static str,
}

fn too_large() -> Error {
    super::unprocessable("a total is too large to report")
}

/// `GET /api/quotas/territory-attainment?from=&to=` — the won, live deals closed in the period,
/// added up per currency for each territory's accounts. An audited read.
#[debug_handler]
async fn territory_attainment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<TerritoryQuery>,
) -> Result<Response> {
    let to = match query.to {
        Some(to) => to,
        None => super::organization::caller_today(&ctx.db, &headers).await?,
    };
    let from = query
        .from
        .unwrap_or_else(|| to - chrono::Duration::days(DEFAULT_PERIOD_DAYS));
    if from > to || (to - from).num_days() > MAX_PERIOD_DAYS {
        return Err(super::unprocessable(&format!(
            "/from: not after /to, and at most {MAX_PERIOD_DAYS} days before it"
        )));
    }
    let all = territories(&ctx.db).await?;
    // The days deals closed on are the organization's (CRM-T181).
    let calendar = super::organization::periods(&ctx.db).await?;
    let closed_on = crate::rules::periods::local_day("d.closed_at", &calendar);
    let deals = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT a.territory_pid, trim(d.currency) AS currency, d.amount_minor \
                 FROM deals d \
                 LEFT JOIN accounts c ON c.pid = d.account_pid \
                 LEFT JOIN territory_accounts a ON a.organization_ref = c.organization_ref \
                 WHERE d.won AND d.deleted_at IS NULL AND d.closed_at IS NOT NULL \
                   AND {closed_on} BETWEEN $1 AND $2"
            ),
            [from.into(), to.into()],
        ))
        .await?;
    // (territory, currency) -> (minor units, deals); the territory is `None` when unassigned.
    let mut sums: BTreeMap<(Option<Uuid>, String), (i64, i64)> = BTreeMap::new();
    for deal in &deals {
        let slot = sums
            .entry((
                deal.try_get("", "territory_pid")?,
                deal.try_get("", "currency")?,
            ))
            .or_default();
        slot.0 = slot
            .0
            .checked_add(deal.try_get::<i64>("", "amount_minor")?)
            .ok_or_else(too_large)?;
        slot.1 += 1;
    }
    let revenue = |owner: Option<Uuid>| -> Vec<TerritoryRevenue> {
        sums.iter()
            .filter(|((o, _), _)| *o == owner)
            .map(|((_, currency), (minor, won))| TerritoryRevenue {
                currency: currency.clone(),
                revenue_minor: *minor,
                won_deals: *won,
            })
            .collect()
    };
    let rows = all
        .iter()
        .map(|t| TerritoryStanding {
            territory_pid: t.pid.clone(),
            name: t.name.clone(),
            owner_ref: t.owner_ref.clone(),
            accounts: t.accounts.len(),
            currencies: Uuid::parse_str(&t.pid)
                .map(|pid| revenue(Some(pid)))
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    Audit::record_read(
        &ctx.db,
        "territory",
        Uuid::nil(),
        "attainment_read",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": to, "territories": rows.len() })),
    )
    .await?;
    format::json(TerritoryReport {
        as_of: chrono::Utc::now(),
        from,
        to,
        rows,
        unassigned: revenue(None),
        note: "a deal counts whole in the territory holding its account, whoever it is credited \
               to; currencies are listed, never added or converted",
    })
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/quotas/territories", get(list_territories))
        .add("/quotas/territories", put(set_territory))
        .add("/quotas/territories/{pid}", delete(remove_territory))
        .add("/quotas/territory-attainment", get(territory_attainment))
}
