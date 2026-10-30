//! The report builder (CRM-R48): `/api/reports`.
//!
//! A report counts (or, for deals, sums) records grouped by one field. What it may ask is closed
//! ([`crate::rules::reports`]); the answer is computed here, in the database, and carries the date
//! it was read (`as_of`). Saved reports belong to one worker and can be shared, like saved views;
//! running a shared one is the aggregate read of whoever runs it, so sharing shares no access.

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use loco_rs::prelude::*;

use super::fx_rates::{RateUsed, Unconverted};
use super::views::owner_of;
use super::{PidPath, PidRef, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::reports::{self as rules, Bind, MAX_GROUPS, Spec};
use crate::validation::Problems;

/// `POST /api/reports` and `PUT /api/reports/{pid}` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ReportPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct ReportPayload {
    /// Unique among your own reports, ignoring case.
    name: String,
    /// What the report asks.
    spec: Spec,
    /// Let every signed-in user list and run it.
    #[serde(default)]
    shared: bool,
}

/// One saved report.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Report")]
pub(crate) struct ReportView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    owner_ref: String,
    /// Whether the caller owns it (and so may change it).
    mine: bool,
    name: String,
    spec: Spec,
    shared: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// One row of an answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ReportRow")]
pub(crate) struct ReportRow {
    /// The group's name (a status, a month, a tag, …).
    group: String,
    /// The currency, for a sum of amounts: sums are never added across currencies.
    currency: Option<String>,
    /// The count, or the sum in minor units.
    value: i64,
}

/// What a report found.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ReportResult")]
pub(crate) struct ReportResult {
    spec: Spec,
    /// When it was read.
    as_of: chrono::DateTime<chrono::Utc>,
    /// The groups, largest first.
    rows: Vec<ReportRow>,
    /// There were more groups than are returned.
    truncated: bool,
    /// With `convert_to`: the day whose exchange rates were used.
    rate_date: Option<chrono::NaiveDate>,
    /// With `convert_to`: the rate used for each currency that was converted.
    rates: Vec<RateUsed>,
    /// With `convert_to`: sums that had no rate in force and were left out, never taken at 1.0.
    unconverted: Vec<Unconverted>,
    /// Groups with fewer records than this are left out (CRM-R78); `0` when none are.
    min_cell_size: i64,
    /// How many cells (or, with `convert_to`, groups) were left out for being that small.
    suppressed: usize,
    /// What the figures mean, in a line.
    note: &'static str,
    /// The time zone of the months, fiscal periods and `from`/`to` days (CRM-T171).
    time_zone: String,
    /// The month the fiscal year starts in, for fiscal quarters and years.
    fiscal_year_start_month: u32,
}

/// What a conversion made of the rows of a report.
struct Conversion {
    rows: Vec<ReportRow>,
    rate_date: Option<chrono::NaiveDate>,
    rates: Vec<RateUsed>,
    unconverted: Vec<Unconverted>,
}

/// The rows as the database gave them, or converted to `convert_to` with the rates in force on
/// the rate date (CRM-R50).
async fn converted(ctx: &AppContext, spec: &Spec, raw: Vec<rules::Cell>) -> Result<Conversion> {
    let Some(target) = &spec.convert_to else {
        return Ok(Conversion {
            rows: raw
                .into_iter()
                .map(|c| ReportRow {
                    group: c.group,
                    currency: c.currency,
                    value: c.value,
                })
                .collect(),
            rate_date: None,
            rates: Vec::new(),
            unconverted: Vec::new(),
        });
    };
    if spec.rate_basis.is_some() {
        // Each closing day at its own rate (CRM-R83): no one date stands for the answer.
        let dated: Vec<(String, chrono::NaiveDate, String, i64)> = raw
            .into_iter()
            .filter_map(|c| Some((c.group, c.day?, c.currency.unwrap_or_default(), c.value)))
            .collect();
        let (rows, rates, unconverted) =
            super::fx_rates::convert_rows_dated(ctx, target, &dated).await?;
        return Ok(Conversion {
            rows: rows
                .into_iter()
                .map(|(group, value)| ReportRow {
                    group,
                    currency: Some(target.clone()),
                    value,
                })
                .collect(),
            rate_date: None,
            rates,
            unconverted,
        });
    }
    let sums: Vec<(String, String, i64)> = raw
        .into_iter()
        .map(|c| (c.group, c.currency.unwrap_or_default(), c.value))
        .collect();
    let done =
        super::fx_rates::convert_sums(ctx, target, spec.rate_date, "spec.rate_date", &sums).await?;
    Ok(Conversion {
        rows: done
            .rows
            .into_iter()
            .map(|(group, value)| ReportRow {
                group,
                currency: Some(target.clone()),
                value,
            })
            .collect(),
        rate_date: Some(done.rate_date),
        rates: done.rates,
        unconverted: done.unconverted,
    })
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

fn problems_of(found: Vec<(String, String)>) -> Result<()> {
    let mut problems = Problems::new();
    for (member, message) in found {
        problems.push_field(&format!("spec.{member}"), message);
    }
    problems.finish()
}

/// Answer a specification as the caller.
async fn answer(ctx: &AppContext, caller: &MaybeAuthUser, spec: Spec) -> Result<ReportResult> {
    // Months, fiscal quarters and years in the organization's calendar (CRM-T171).
    let periods = super::organization::periods(&ctx.db).await?;
    let plan = rules::plan_in(&spec, &periods).map_err(|found| match problems_of(found) {
        Err(error) => error,
        Ok(()) => unprocessable("the report cannot be run"),
    })?;
    // A caller whose collection read is masked sees counts, but not money or what is said by tags.
    let masked = auth::aggregate_masked(caller).map_err(record_rejection)?;
    if masked && (plan.uses_money || plan.uses_tags) {
        return Err(record_rejection((
            axum::http::StatusCode::FORBIDDEN,
            "a report on amounts or tags needs an unmasked read".to_string(),
        )));
    }
    let values: Vec<Value> = plan
        .binds
        .iter()
        .map(|b| match b {
            Bind::List(list) => serde_json::json!(list).into(),
            Bind::Day(day) => (*day).into(),
        })
        .collect();
    let found = ctx.db.query_all_raw(stmt(&plan.sql, values)).await?;
    if plan.by_day && found.len() > rules::MAX_DAY_CELLS {
        return Err(unprocessable(
            "that report spans too many closing days to convert at each day's rate; narrow it \
             with from and to, or a filter",
        ));
    }
    let mut cells = Vec::with_capacity(found.len());
    for row in &found {
        cells.push(rules::Cell {
            group: row.try_get::<String>("", "grp")?,
            currency: if plan.per_currency {
                row.try_get("", "currency")?
            } else {
                None
            },
            value: row.try_get::<i64>("", "value")?,
            n: row.try_get::<i64>("", "n")?,
            day: if plan.by_day {
                row.try_get("", "day")?
            } else {
                None
            },
        });
    }
    // A group with only a few records behind it is a few people (or a few deals' amounts): leave it
    // out, and say how many were, so a short table is not mistaken for a complete one (CRM-R78).
    let min_cell_size = crate::settings::current().min_cell_size;
    let (cells, suppressed) = rules::suppress(cells, min_cell_size, spec.convert_to.is_some());
    let Conversion {
        mut rows,
        rate_date,
        rates,
        unconverted,
    } = converted(ctx, &spec, cells).await?;
    let truncated = rows.len() > MAX_GROUPS;
    rows.truncate(MAX_GROUPS);
    Audit::record_read(
        &ctx.db,
        "report",
        Uuid::nil(),
        "report_run",
        caller.actor(),
        Some(serde_json::json!({
            "kind": spec.kind, "group_by": spec.group_by, "measure": spec.measure,
            "groups": rows.len(), "suppressed": suppressed, "min_cell_size": min_cell_size,
        })),
    )
    .await?;
    Ok(ReportResult {
        time_zone: periods.zone.name().to_string(),
        fiscal_year_start_month: periods.fiscal_start_month,
        spec,
        as_of: chrono::Utc::now(),
        rows,
        truncated,
        rate_date,
        rates,
        unconverted,
        min_cell_size,
        suppressed,
        note: "counted over the records that exist now; a record with several tags is in each \
               tag's group; sums are per currency and never added across currencies; a group \
               with fewer records than min_cell_size is left out, never shown as a small number",
    })
}

/// `POST /api/reports/run` — answer a specification, saving nothing.
#[debug_handler]
async fn run_adhoc(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(spec): Json<Spec>,
) -> Result<Response> {
    format::json(answer(&ctx, &caller, spec).await?)
}

const COLUMNS: &str = "pid, owner_ref, name, spec, shared, updated_at";

fn view(row: &sea_orm::QueryResult, me: &str) -> Result<ReportView> {
    let owner: String = row.try_get("", "owner_ref")?;
    let spec: serde_json::Value = row.try_get("", "spec")?;
    Ok(ReportView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        mine: owner == me,
        owner_ref: owner,
        name: row.try_get("", "name")?,
        spec: serde_json::from_value(spec)
            .map_err(|_| Error::string("a stored report does not read"))?,
        shared: row.try_get("", "shared")?,
        updated_at: row.try_get("", "updated_at")?,
    })
}

fn validate(payload: &ReportPayload) -> Result<()> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if payload.name.trim().chars().count() > rules::MAX_NAME {
        problems.push_field(
            "name",
            format!("is longer than {} characters", rules::MAX_NAME),
        );
    }
    if let Err(found) = rules::plan(&payload.spec) {
        for (member, message) in found {
            problems.push_field(&format!("spec.{member}"), message);
        }
    }
    problems.finish()
}

fn taken() -> Error {
    unprocessable("you already have a report with that name")
}

/// `GET /api/reports` — your own reports and everyone's shared ones.
#[debug_handler]
async fn list(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let me = owner_of(&caller)?;
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM reports WHERE owner_ref = $1 OR shared \
                 ORDER BY (owner_ref = $1) DESC, lower(name), id"
            ),
            [me.clone().into()],
        ))
        .await?;
    format::json(
        rows.iter()
            .map(|r| view(r, &me))
            .collect::<Result<Vec<_>>>()?,
    )
}

/// `POST /api/reports`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ReportPayload>,
) -> Result<Response> {
    validate(&payload)?;
    let me = owner_of(&caller)?;
    let pid = Uuid::new_v4();
    let txn = ctx.db.begin().await?;
    let owned: i64 = txn
        .query_one_raw(stmt(
            "SELECT count(*) AS n FROM reports WHERE owner_ref = $1",
            [me.clone().into()],
        ))
        .await?
        .map_or(0, |r| r.try_get::<i64>("", "n").unwrap_or(0));
    if owned >= rules::MAX_PER_OWNER {
        return Err(unprocessable(&format!(
            "you can keep at most {} reports",
            rules::MAX_PER_OWNER
        )));
    }
    let inserted = txn
        .execute_raw(stmt(
            "INSERT INTO reports (pid, owner_ref, name, spec, shared) VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT DO NOTHING",
            [
                pid.into(),
                me.into(),
                payload.name.trim().to_string().into(),
                serde_json::json!(payload.spec).into(),
                payload.shared.into(),
            ],
        ))
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record(
        &txn,
        "report",
        pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "kind": payload.spec.kind, "shared": payload.shared })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// Lock the report and say who owns it, or why the caller cannot change it.
async fn owned(txn: &sea_orm::DatabaseTransaction, pid: Uuid, me: &str) -> Result<bool> {
    let row = txn
        .query_one_raw(stmt(
            "SELECT owner_ref, shared FROM reports WHERE pid = $1 FOR UPDATE",
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let owner: String = row.try_get("", "owner_ref")?;
    let shared: bool = row.try_get("", "shared")?;
    if owner == me {
        Ok(shared)
    } else if shared {
        Err(Error::CustomError(
            axum::http::StatusCode::FORBIDDEN,
            loco_rs::controller::ErrorDetail::new(
                "forbidden",
                "only the owner can change a report",
            ),
        ))
    } else {
        Err(Error::NotFound)
    }
}

/// `PUT /api/reports/{pid}` — replace a report you own.
#[debug_handler]
async fn replace(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ReportPayload>,
) -> Result<Response> {
    validate(&payload)?;
    let me = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    owned(&txn, pid, &me).await?;
    let changed = txn
        .execute_raw(stmt(
            "UPDATE reports SET name = $2, spec = $3, shared = $4, updated_at = now() WHERE pid = $1",
            [
                pid.into(),
                payload.name.trim().to_string().into(),
                serde_json::json!(payload.spec).into(),
                payload.shared.into(),
            ],
        ))
        .await;
    if changed.is_err() {
        return Err(taken());
    }
    Audit::record(
        &txn,
        "report",
        pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "kind": payload.spec.kind, "shared": payload.shared })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `DELETE /api/reports/{pid}` — delete a report you own.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let me = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    owned(&txn, pid, &me).await?;
    txn.execute_raw(stmt("DELETE FROM reports WHERE pid = $1", [pid.into()]))
        .await?;
    Audit::record(&txn, "report", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `GET /api/reports/{pid}/run` — run a saved report: yours, or one that is shared.
#[debug_handler]
async fn run_saved(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let me = owner_of(&caller)?;
    let row = ctx
        .db
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM reports WHERE pid = $1 AND (owner_ref = $2 OR shared)"),
            [pid.into(), me.clone().into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let saved = view(&row, &me)?;
    format::json(answer(&ctx, &caller, saved.spec).await?)
}

/// The report routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/reports", get(list))
        .add("/reports", post(create))
        .add("/reports/run", post(run_adhoc))
        .add("/reports/{pid}", put(replace))
        .add("/reports/{pid}", delete(remove))
        .add("/reports/{pid}/run", get(run_saved))
}
