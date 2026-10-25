//! Business calendars (CRM-R45): `/api/business-calendars`.
//!
//! A calendar is working hours, a time zone and holidays. An SLA policy may name one, and then
//! the deadlines of the tickets opened under it are counted in working time. The counting is in
//! [`crate::rules::calendar`].

use chrono::NaiveDate;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::merge::conflict;
use super::{PidPath, PidRef, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::calendar::{Calendar, Week};
use crate::validation::Problems;

/// `POST /api/business-calendars` and `PUT /api/business-calendars/{pid}` body (a `PUT` replaces).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "BusinessCalendarPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct CalendarPayload {
    /// Unique, ignoring case.
    name: String,
    /// An IANA time zone name, such as `Europe/London`; the windows are local to it.
    timezone: String,
    /// The working windows of each weekday.
    week: Week,
    /// Days that are not working days, as `YYYY-MM-DD`.
    #[serde(default)]
    holidays: Vec<NaiveDate>,
}

/// One calendar.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "BusinessCalendar")]
pub(crate) struct CalendarView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    timezone: String,
    week: Week,
    holidays: Vec<NaiveDate>,
    /// How many SLA policies use it.
    in_use: i64,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

const COLUMNS: &str = "c.pid, c.name, c.timezone, c.week, c.holidays, \
    (SELECT count(*) FROM sla_policies p WHERE p.calendar_pid = c.pid AND p.deleted_at IS NULL) AS in_use";

fn view(row: &sea_orm::QueryResult) -> Result<CalendarView> {
    let week: serde_json::Value = row.try_get("", "week")?;
    let holidays: serde_json::Value = row.try_get("", "holidays")?;
    Ok(CalendarView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        name: row.try_get("", "name")?,
        timezone: row.try_get("", "timezone")?,
        week: serde_json::from_value(week).unwrap_or_default(),
        holidays: serde_json::from_value(holidays).unwrap_or_default(),
        in_use: row.try_get("", "in_use")?,
    })
}

/// The calendar `pid`, ready to count on; `None` when it is gone or no longer builds.
///
/// # Errors
///
/// A database error.
pub(crate) async fn load<C: ConnectionTrait>(db: &C, pid: Uuid) -> Result<Option<Calendar>> {
    let Some(row) = db
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM business_calendars c WHERE c.pid = $1"),
            [pid.into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    let stored = view(&row)?;
    Ok(Calendar::build(&stored.timezone, &stored.week, &stored.holidays).ok())
}

/// Whether the calendar exists.
pub(crate) async fn exists<C: ConnectionTrait>(db: &C, pid: Uuid) -> Result<bool> {
    Ok(db
        .query_one_raw(stmt(
            "SELECT 1 AS one FROM business_calendars WHERE pid = $1",
            [pid.into()],
        ))
        .await?
        .is_some())
}

fn validate(payload: &CalendarPayload) -> Result<()> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if let Err(found) = Calendar::build(&payload.timezone, &payload.week, &payload.holidays) {
        for (member, message) in found {
            problems.push_field(&member, message);
        }
    }
    problems.finish()
}

fn taken() -> Error {
    unprocessable("a calendar with that name already exists")
}

fn binds(payload: &CalendarPayload) -> Vec<Value> {
    let mut holidays = payload.holidays.clone();
    holidays.sort_unstable();
    holidays.dedup();
    vec![
        payload.name.trim().to_string().into(),
        payload.timezone.clone().into(),
        serde_json::json!(payload.week).into(),
        serde_json::json!(holidays).into(),
    ]
}

/// `GET /api/business-calendars`.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!("SELECT {COLUMNS} FROM business_calendars c ORDER BY lower(c.name), c.id"),
            [],
        ))
        .await?;
    format::json(rows.iter().map(view).collect::<Result<Vec<_>>>()?)
}

/// `GET /api/business-calendars/{pid}`.
#[debug_handler]
async fn show(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    let row = ctx
        .db
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM business_calendars c WHERE c.pid = $1"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    format::json(view(&row)?)
}

/// `POST /api/business-calendars`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<CalendarPayload>,
) -> Result<Response> {
    validate(&payload)?;
    let pid = Uuid::new_v4();
    let mut values = vec![pid.into()];
    values.extend(binds(&payload));
    let txn = ctx.db.begin().await?;
    let inserted = txn
        .execute_raw(stmt(
            "INSERT INTO business_calendars (pid, name, timezone, week, holidays) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            values,
        ))
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record(
        &txn,
        "business_calendar",
        pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `PUT /api/business-calendars/{pid}` — replace a calendar. Tickets already open keep the
/// deadlines they were given; later tickets and priority changes use the new calendar.
#[debug_handler]
async fn replace(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<CalendarPayload>,
) -> Result<Response> {
    validate(&payload)?;
    let mut values = vec![pid.into()];
    values.extend(binds(&payload));
    let txn = ctx.db.begin().await?;
    let changed = txn
        .execute_raw(stmt(
            "UPDATE business_calendars SET name = $2, timezone = $3, week = $4, holidays = $5, \
             updated_at = now() WHERE pid = $1",
            values,
        ))
        .await;
    // The only constraint a replace can break is the unique name.
    if changed.is_err() {
        return Err(taken());
    }
    if changed.is_ok_and(|r| r.rows_affected() == 0) {
        return Err(Error::NotFound);
    }
    Audit::record(
        &txn,
        "business_calendar",
        pid,
        "updated",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `DELETE /api/business-calendars/{pid}` — refused while a policy uses it.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM business_calendars c WHERE c.pid = $1 FOR UPDATE OF c"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    if view(&row)?.in_use > 0 {
        return Err(conflict("an SLA policy still uses this calendar"));
    }
    txn.execute_raw(stmt(
        "DELETE FROM business_calendars WHERE pid = $1",
        [pid.into()],
    ))
    .await?;
    Audit::record(
        &txn,
        "business_calendar",
        pid,
        "deleted",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// The business calendar routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/business-calendars", get(list))
        .add("/business-calendars", post(create))
        .add("/business-calendars/{pid}", get(show))
        .add("/business-calendars/{pid}", put(replace))
        .add("/business-calendars/{pid}", delete(remove))
}
