//! The organization's calendar (CRM-T171) and when a deal is stalled (CRM-T183):
//! `/api/settings/organization`.
//!
//! One row (`organization_settings`): the IANA time zone that decides "today" in the service and
//! the months of reports, the month the fiscal year starts, and after how many days without a
//! stage move or an activity an open deal is stalled (CRM-D20). Anyone signed in may read it; only
//! an administrator may change it (the policy derives `Destructive` for the `PUT`), and a change
//! is audited with both values.
//!
//! The two routes run on `sqlx` (CRM-D24). The public readers below stay on `ConnectionTrait` until
//! their callers move, because `workflows.rs` calls one inside a `SeaORM` transaction; they read
//! the same row, and what a row means is in `periods_from` and `stalled_from` for both.

use axa::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::views::owner_of;
use crate::auth::MaybeAuthUser;
use crate::db;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::periods::{self, Periods, USER_ZONE_HEADER};
use crate::rules::stalled;
use crate::validation::Problems;

/// The organization's calendar as the API shows it.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "OrganizationSettings")]
pub(crate) struct SettingsView {
    /// The IANA time zone of "today" and of report periods (`UTC` until one is set).
    time_zone: String,
    /// The month (1 to 12) the fiscal year starts in (1 until one is set).
    fiscal_year_start_month: u32,
    /// After how many days without a stage move or an activity an open deal is stalled (1 to
    /// 365; 14 until one is set).
    stalled_after_days: u32,
    /// When they were last changed.
    updated_at: chrono::DateTime<chrono::Utc>,
    /// Who changed them last; absent while they are the starting values.
    set_by: Option<String>,
    /// "Today" in the organization's zone.
    today: chrono::NaiveDate,
}

/// `PUT /api/settings/organization` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "OrganizationSettingsPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct SettingsPayload {
    /// An IANA time zone, such as `Asia/Tokyo`.
    time_zone: String,
    /// The month (1 to 12) the fiscal year starts in.
    fiscal_year_start_month: i64,
    /// After how many days (1 to 365) an open deal with no stage move or activity is stalled;
    /// absent keeps the stored value.
    #[serde(default)]
    stalled_after_days: Option<i64>,
}

/// The stored row, if there is one.
struct Stored {
    time_zone: String,
    fiscal_year_start_month: i16,
    stalled_after_days: i16,
    updated_at: chrono::DateTime<chrono::Utc>,
    set_by: Option<String>,
}

async fn stored(pool: &PgPool) -> Result<Option<Stored>> {
    Ok(sqlx::query_as!(
        Stored,
        "SELECT time_zone, fiscal_year_start_month, stalled_after_days, updated_at, set_by \
         FROM organization_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?)
}

async fn row(db: &impl ConnectionTrait) -> Result<Option<sea_orm::QueryResult>> {
    Ok(db
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT time_zone, fiscal_year_start_month, stalled_after_days \
             FROM organization_settings WHERE id = 1",
        ))
        .await?)
}

/// The calendar a stored zone and month stand for; a value no longer known falls back.
fn periods_from(zone: &str, month: i16) -> Periods {
    let fallback = Periods::default();
    Periods {
        zone: periods::zone(zone).unwrap_or(fallback.zone),
        fiscal_start_month: periods::fiscal_start_month(i64::from(month))
            .unwrap_or(fallback.fiscal_start_month),
    }
}

/// The days a stored value stands for; one out of range falls back.
fn stalled_from(days: i16) -> u32 {
    stalled::days(i64::from(days)).unwrap_or(stalled::DEFAULT_DAYS)
}

/// The organization's calendar; the starting values when none is stored or the stored zone is
/// no longer known.
///
/// # Errors
///
/// A database error.
pub async fn periods(db: &impl ConnectionTrait) -> Result<Periods> {
    let Some(row) = row(db).await? else {
        return Ok(Periods::default());
    };
    let zone: String = row.try_get("", "time_zone")?;
    let month: i16 = row.try_get("", "fiscal_year_start_month")?;
    Ok(periods_from(&zone, month))
}

/// After how many days an open deal is stalled; the starting 14 when none is stored.
///
/// # Errors
///
/// A database error.
pub async fn stalled_after_days(db: &impl ConnectionTrait) -> Result<u32> {
    let Some(row) = row(db).await? else {
        return Ok(stalled::DEFAULT_DAYS);
    };
    let days: i16 = row.try_get("", "stalled_after_days")?;
    Ok(stalled_from(days))
}

/// "Today" for the caller of a request: in the zone it names in `CRM-Time-Zone`, else the
/// organization's.
///
/// # Errors
///
/// A database error.
pub async fn caller_today(
    db: &impl ConnectionTrait,
    headers: &axum::http::HeaderMap,
) -> Result<chrono::NaiveDate> {
    let org = periods(db).await?;
    let named = headers.get(USER_ZONE_HEADER).and_then(|v| v.to_str().ok());
    Ok(periods::today_in(
        chrono::Utc::now(),
        periods::caller_zone(named, &org),
    ))
}

/// "Today" in the organization's zone, for what is not one caller's (a stored snapshot).
///
/// # Errors
///
/// A database error.
pub async fn organization_today(db: &impl ConnectionTrait) -> Result<chrono::NaiveDate> {
    Ok(periods::today_in(
        chrono::Utc::now(),
        periods(db).await?.zone,
    ))
}

async fn view(pool: &PgPool) -> Result<SettingsView> {
    let row = stored(pool).await?;
    let found = row.as_ref().map_or_else(Periods::default, |row| {
        periods_from(&row.time_zone, row.fiscal_year_start_month)
    });
    let stalled_days = row.as_ref().map_or(stalled::DEFAULT_DAYS, |row| {
        stalled_from(row.stalled_after_days)
    });
    let (updated_at, set_by) = match row {
        Some(row) => (row.updated_at, row.set_by),
        None => (chrono::Utc::now(), None),
    };
    Ok(SettingsView {
        time_zone: found.zone.name().to_string(),
        fiscal_year_start_month: found.fiscal_start_month,
        stalled_after_days: stalled_days,
        updated_at,
        set_by,
        today: periods::today_in(chrono::Utc::now(), found.zone),
    })
}

/// `GET /api/settings/organization`.
#[debug_handler]
async fn show(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(view(db::pool(&ctx)).await?)
}

/// `PUT /api/settings/organization` — an administrator sets the zone and the fiscal year's start.
#[debug_handler]
async fn set(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SettingsPayload>,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let mut problems = Problems::new();
    let zone = periods::zone(&payload.time_zone)
        .map_err(|why| problems.push_field("time_zone", why))
        .ok();
    let month = periods::fiscal_start_month(payload.fiscal_year_start_month)
        .map_err(|why| problems.push_field("fiscal_year_start_month", why))
        .ok();
    let stalled_days = match payload.stalled_after_days {
        Some(raw) => stalled::days(raw)
            .map_err(|why| problems.push_field("stalled_after_days", why))
            .ok(),
        None => Some(stored(pool).await?.map_or(stalled::DEFAULT_DAYS, |row| {
            stalled_from(row.stalled_after_days)
        })),
    };
    problems.finish()?;
    let (Some(zone), Some(month), Some(stalled_days)) = (zone, month, stalled_days) else {
        return Err(super::unprocessable("the settings cannot be set"));
    };
    let who = owner_of(&caller)?;
    let mut txn = pool.begin().await?;
    sqlx::query!(
        "INSERT INTO organization_settings \
         (id, time_zone, fiscal_year_start_month, stalled_after_days, set_by) \
         VALUES (1, $1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET time_zone = EXCLUDED.time_zone, \
         fiscal_year_start_month = EXCLUDED.fiscal_year_start_month, \
         stalled_after_days = EXCLUDED.stalled_after_days, set_by = EXCLUDED.set_by, \
         updated_at = now()",
        zone.name(),
        i16::try_from(month).unwrap_or(1),
        i16::try_from(stalled_days).unwrap_or(14),
        who,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "organization_settings",
        Uuid::nil(),
        "set",
        caller.actor(),
        Some(serde_json::json!({
            "time_zone": zone.name(),
            "fiscal_year_start_month": month,
            "stalled_after_days": stalled_days,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(view(pool).await?)
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/settings/organization", get(show))
        .add("/settings/organization", put(set))
}
