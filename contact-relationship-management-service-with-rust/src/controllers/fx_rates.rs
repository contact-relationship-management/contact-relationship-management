//! Exchange rates (CRM-R50): `/api/fx-rates`.
//!
//! Rates are entered by hand, one per pair and day, each with its source; nothing is fetched. A
//! report that converts reads them through [`crate::rules::fx`]. Changing a rate is a write and is
//! audited with the pair, the rate, the day and the source.

use chrono::NaiveDate;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::views::owner_of;
use super::{PidPath, PidRef};
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::fx::{self, Rate};
use crate::validation::Problems;

/// Most rates listed.
const MAX_LISTED: i64 = 1000;

/// `PUT /api/fx-rates` body: sets the rate of one pair on one day, replacing any already there.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "FxRatePayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct RatePayload {
    /// The currency being bought with, three capital letters.
    from: String,
    /// The currency it buys.
    to: String,
    /// How many `to` one `from` buys, as a decimal text such as `"1.27"` (at most 9 places).
    rate: String,
    /// The day the rate takes effect.
    effective_on: NaiveDate,
    /// Where it came from, in words.
    source: String,
}

/// One recorded rate.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "FxRate")]
pub(crate) struct RateView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    from: String,
    to: String,
    /// A decimal text, trailing zeros dropped.
    rate: String,
    effective_on: NaiveDate,
    source: String,
    /// Who set it.
    set_by: String,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

const COLUMNS: &str = "pid, from_currency, to_currency, rate_ppb, effective_on, source, set_by";

fn view(row: &sea_orm::QueryResult) -> Result<RateView> {
    Ok(RateView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        from: row
            .try_get::<String>("", "from_currency")?
            .trim()
            .to_string(),
        to: row.try_get::<String>("", "to_currency")?.trim().to_string(),
        rate: fx::format_rate(row.try_get("", "rate_ppb")?),
        effective_on: row.try_get("", "effective_on")?,
        source: row.try_get("", "source")?,
        set_by: row.try_get("", "set_by")?,
    })
}

/// Every recorded rate that involves `currency`, for a conversion into or out of it.
///
/// # Errors
///
/// When the query fails.
pub(crate) async fn rates_involving<C: ConnectionTrait>(
    db: &C,
    currency: &str,
) -> Result<Vec<Rate>> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT from_currency, to_currency, rate_ppb, effective_on, source FROM fx_rates \
             WHERE from_currency = $1 OR to_currency = $1",
            [currency.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Rate {
                from: row
                    .try_get::<String>("", "from_currency")?
                    .trim()
                    .to_string(),
                to: row.try_get::<String>("", "to_currency")?.trim().to_string(),
                ppb: row.try_get("", "rate_ppb")?,
                effective_on: row.try_get("", "effective_on")?,
                source: row.try_get("", "source")?,
            })
        })
        .collect()
}

/// An exchange rate a conversion used (CRM-R50).
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ReportRateUsed")]
pub(crate) struct RateUsed {
    from: String,
    to: String,
    /// How many `to` one `from` buys, as recorded.
    rate: String,
    effective_on: NaiveDate,
    source: String,
    /// The recorded rate is for the reverse pair, so the amount was divided by it.
    inverted: bool,
}

/// Sums in one currency that could not be converted.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ReportUnconverted")]
pub(crate) struct Unconverted {
    pub currency: String,
    /// How many group sums were left out.
    pub groups: usize,
    /// Their total, in that currency's minor units.
    pub amount_minor: i64,
}

/// Sums converted into one currency, and what the conversion used and left out.
pub(crate) struct Done {
    /// `(group, value in the target currency)`, largest first.
    pub rows: Vec<(String, i64)>,
    /// The day whose rates were used.
    pub rate_date: NaiveDate,
    pub rates: Vec<RateUsed>,
    pub unconverted: Vec<Unconverted>,
}

/// Convert `(group, currency, sum)` rows into `target` with the rates in force on `rate_date`
/// (today when absent). A date in the future is a `422` naming `member`.
///
/// # Errors
///
/// A `422` for a future date, or when the rates cannot be read.
pub(crate) async fn convert_sums(
    ctx: &AppContext,
    target: &str,
    rate_date: Option<NaiveDate>,
    member: &str,
    sums: &[(String, String, i64)],
) -> Result<Done> {
    let today = chrono::Utc::now().date_naive();
    let day = rate_date.unwrap_or(today);
    if day > today {
        let mut problems = Problems::new();
        problems.push_field(member, "is in the future");
        problems.finish()?;
    }
    let rates = rates_involving(&ctx.db, target).await?;
    let done = fx::convert_rows(sums, target, day, &rates);
    Ok(Done {
        rows: done.rows,
        rate_date: day,
        rates: done
            .used
            .into_iter()
            .map(|(from, picked)| RateUsed {
                from,
                to: target.to_string(),
                rate: fx::format_rate(picked.rate.ppb),
                effective_on: picked.rate.effective_on,
                source: picked.rate.source,
                inverted: picked.inverted,
            })
            .collect(),
        unconverted: done
            .left_out
            .into_iter()
            .map(|l| Unconverted {
                currency: l.currency,
                groups: l.groups,
                amount_minor: l.amount_minor,
            })
            .collect(),
    })
}

/// `GET /api/fx-rates` — the recorded rates, newest first (at most 1000).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM fx_rates \
                 ORDER BY effective_on DESC, from_currency, to_currency LIMIT $1"
            ),
            [MAX_LISTED.into()],
        ))
        .await?;
    format::json(rows.iter().map(view).collect::<Result<Vec<_>>>()?)
}

/// `PUT /api/fx-rates` — set a rate; audited.
#[debug_handler]
async fn set(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<RatePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    let from = fx::currency(&payload.from)
        .map_err(|why| problems.push_field("from", why))
        .ok();
    let to = fx::currency(&payload.to)
        .map_err(|why| problems.push_field("to", why))
        .ok();
    if from.is_some() && from == to {
        problems.push_field("to", "a rate is between two different currencies");
    }
    let ppb = fx::parse_rate(&payload.rate)
        .map_err(|why| problems.push_field("rate", why))
        .ok();
    let source = fx::source(&payload.source)
        .map_err(|why| problems.push_field("source", why))
        .ok();
    problems.finish()?;
    let (Some(from), Some(to), Some(ppb), Some(source)) = (from, to, ppb, source) else {
        return Err(super::unprocessable("the rate cannot be set"));
    };

    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!(
                "INSERT INTO fx_rates (pid, from_currency, to_currency, rate_ppb, effective_on, \
                                       source, set_by) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7) \
                 ON CONFLICT (from_currency, to_currency, effective_on) DO UPDATE SET \
                   rate_ppb = EXCLUDED.rate_ppb, source = EXCLUDED.source, \
                   set_by = EXCLUDED.set_by, updated_at = now() \
                 RETURNING {COLUMNS}"
            ),
            [
                Uuid::new_v4().into(),
                from.as_str().into(),
                to.as_str().into(),
                ppb.into(),
                payload.effective_on.into(),
                source.as_str().into(),
                who.into(),
            ],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let made = view(&row)?;
    let pid: Uuid = row.try_get("", "pid")?;
    Audit::record(
        &txn,
        "fx_rate",
        pid,
        "set",
        caller.actor(),
        Some(serde_json::json!({
            "from": made.from, "to": made.to, "rate": made.rate,
            "effective_on": made.effective_on, "source": made.source,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(made)
}

/// `DELETE /api/fx-rates/{pid}` — remove a rate; audited. `404` when there is none.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!("DELETE FROM fx_rates WHERE pid = $1 RETURNING {COLUMNS}"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let gone = view(&row)?;
    Audit::record(
        &txn,
        "fx_rate",
        pid,
        "deleted",
        caller.actor(),
        Some(serde_json::json!({
            "from": gone.from, "to": gone.to, "rate": gone.rate,
            "effective_on": gone.effective_on,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// The exchange rate routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/fx-rates", get(list))
        .add("/fx-rates", put(set))
        .add("/fx-rates/{pid}", delete(remove))
}
