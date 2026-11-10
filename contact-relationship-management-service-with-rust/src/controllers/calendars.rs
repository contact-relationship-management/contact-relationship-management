//! Business calendars (CRM-R45): `/api/business-calendars`.
//!
//! A calendar is working hours, a time zone and holidays. An SLA policy may name one, and then
//! the deadlines of the tickets opened under it are counted in working time. The counting is in
//! [`crate::rules::calendar`].
//!
//! The routes run on `sqlx` (CRM-D24, CRM-T205), and so do [`load`] and [`exists`], which the SLA
//! handlers in `support.rs` call (CRM-T208).

use axa::prelude::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::merge::conflict;
use super::{PidPath, PidRef, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::db;
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

/// A stored calendar and how many live SLA policies name it.
struct Row {
    pid: Uuid,
    name: String,
    timezone: String,
    week: serde_json::Value,
    holidays: serde_json::Value,
    in_use: i64,
}

fn view(row: Row) -> CalendarView {
    CalendarView {
        pid: row.pid.to_string(),
        name: row.name,
        timezone: row.timezone,
        week: serde_json::from_value(row.week).unwrap_or_default(),
        holidays: serde_json::from_value(row.holidays).unwrap_or_default(),
        in_use: row.in_use,
    }
}

/// The calendar `pid`, ready to count on; `None` when it is gone or no longer builds.
///
/// # Errors
///
/// A database error.
pub(crate) async fn load<'e>(db: impl PgExecutor<'e>, pid: Uuid) -> Result<Option<Calendar>> {
    let Some(row) = sqlx::query!(
        "SELECT timezone, week, holidays FROM business_calendars WHERE pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await?
    else {
        return Ok(None);
    };
    let week: Week = serde_json::from_value(row.week).unwrap_or_default();
    let holidays: Vec<NaiveDate> = serde_json::from_value(row.holidays).unwrap_or_default();
    Ok(Calendar::build(&row.timezone, &week, &holidays).ok())
}

/// Whether the calendar exists.
pub(crate) async fn exists<'e>(db: impl PgExecutor<'e>, pid: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM business_calendars WHERE pid = $1) AS \"exists!\"",
        pid,
    )
    .fetch_one(db)
    .await?)
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

/// The holidays as stored: in order, each once.
fn holidays_of(payload: &CalendarPayload) -> serde_json::Value {
    let mut holidays = payload.holidays.clone();
    holidays.sort_unstable();
    holidays.dedup();
    serde_json::json!(holidays)
}

/// `GET /api/business-calendars`.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = sqlx::query_as!(
        Row,
        r#"SELECT c.pid, c.name, c.timezone, c.week, c.holidays,
             (SELECT count(*) FROM sla_policies p
              WHERE p.calendar_pid = c.pid AND p.deleted_at IS NULL) AS "in_use!"
           FROM business_calendars c ORDER BY lower(c.name), c.id"#,
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    format::json(rows.into_iter().map(view).collect::<Vec<_>>())
}

/// `GET /api/business-calendars/{pid}`.
#[debug_handler]
async fn show(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    let row = sqlx::query_as!(
        Row,
        r#"SELECT c.pid, c.name, c.timezone, c.week, c.holidays,
             (SELECT count(*) FROM sla_policies p
              WHERE p.calendar_pid = c.pid AND p.deleted_at IS NULL) AS "in_use!"
           FROM business_calendars c WHERE c.pid = $1"#,
        pid,
    )
    .fetch_optional(db::pool(&ctx))
    .await?
    .ok_or(Error::NotFound)?;
    format::json(view(row))
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
    let mut txn = db::pool(&ctx).begin().await?;
    let inserted = sqlx::query!(
        "INSERT INTO business_calendars (pid, name, timezone, week, holidays) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
        pid,
        payload.name.trim(),
        payload.timezone,
        serde_json::json!(payload.week),
        holidays_of(&payload),
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record_on(
        &mut txn,
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
    let mut txn = db::pool(&ctx).begin().await?;
    let changed = sqlx::query!(
        "UPDATE business_calendars SET name = $2, timezone = $3, week = $4, holidays = $5, \
         updated_at = now() WHERE pid = $1",
        pid,
        payload.name.trim(),
        payload.timezone,
        serde_json::json!(payload.week),
        holidays_of(&payload),
    )
    .execute(&mut *txn)
    .await;
    // The only constraint a replace can break is the unique name.
    if changed.is_err() {
        return Err(taken());
    }
    if changed.is_ok_and(|r| r.rows_affected() == 0) {
        return Err(Error::NotFound);
    }
    Audit::record_on(
        &mut txn,
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
    let mut txn = db::pool(&ctx).begin().await?;
    let in_use = sqlx::query_scalar!(
        r#"SELECT (SELECT count(*) FROM sla_policies p
              WHERE p.calendar_pid = c.pid AND p.deleted_at IS NULL) AS "in_use!"
           FROM business_calendars c WHERE c.pid = $1 FOR UPDATE OF c"#,
        pid,
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    if in_use > 0 {
        return Err(conflict("an SLA policy still uses this calendar"));
    }
    sqlx::query!("DELETE FROM business_calendars WHERE pid = $1", pid)
        .execute(&mut *txn)
        .await?;
    Audit::record_on(
        &mut txn,
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
