//! The organization's calendar (CRM-T171): `/api/settings/organization`.
//!
//! One row (`organization_settings`): the IANA time zone that decides "today" in the service and
//! the months of reports, and the month the fiscal year starts. Anyone signed in may read it; only
//! an administrator may change it (the policy derives `Destructive` for the `PUT`), and a change
//! is audited with both values.

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::views::owner_of;
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::periods::{self, Periods, USER_ZONE_HEADER};
use crate::validation::Problems;

/// The organization's calendar as the API shows it.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "OrganizationSettings")]
pub(crate) struct SettingsView {
    /// The IANA time zone of "today" and of report periods (`UTC` until one is set).
    time_zone: String,
    /// The month (1 to 12) the fiscal year starts in (1 until one is set).
    fiscal_year_start_month: u32,
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
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

async fn row(db: &impl ConnectionTrait) -> Result<Option<sea_orm::QueryResult>> {
    Ok(db
        .query_one_raw(stmt(
            "SELECT time_zone, fiscal_year_start_month, updated_at, set_by \
             FROM organization_settings WHERE id = 1",
            [],
        ))
        .await?)
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
    let fallback = Periods::default();
    Ok(Periods {
        zone: periods::zone(&zone).unwrap_or(fallback.zone),
        fiscal_start_month: periods::fiscal_start_month(i64::from(month))
            .unwrap_or(fallback.fiscal_start_month),
    })
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

async fn view(db: &impl ConnectionTrait) -> Result<SettingsView> {
    let found = periods(db).await?;
    let (updated_at, set_by) = match row(db).await? {
        Some(row) => (row.try_get("", "updated_at")?, row.try_get("", "set_by")?),
        None => (chrono::Utc::now(), None),
    };
    Ok(SettingsView {
        time_zone: found.zone.name().to_string(),
        fiscal_year_start_month: found.fiscal_start_month,
        updated_at,
        set_by,
        today: periods::today_in(chrono::Utc::now(), found.zone),
    })
}

/// `GET /api/settings/organization`.
#[debug_handler]
async fn show(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(view(&ctx.db).await?)
}

/// `PUT /api/settings/organization` — an administrator sets the zone and the fiscal year's start.
#[debug_handler]
async fn set(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SettingsPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    let zone = periods::zone(&payload.time_zone)
        .map_err(|why| problems.push_field("time_zone", why))
        .ok();
    let month = periods::fiscal_start_month(payload.fiscal_year_start_month)
        .map_err(|why| problems.push_field("fiscal_year_start_month", why))
        .ok();
    problems.finish()?;
    let (Some(zone), Some(month)) = (zone, month) else {
        return Err(super::unprocessable("the settings cannot be set"));
    };
    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    txn.execute_raw(stmt(
        "INSERT INTO organization_settings (id, time_zone, fiscal_year_start_month, set_by) \
         VALUES (1, $1, $2, $3) ON CONFLICT (id) DO UPDATE SET time_zone = EXCLUDED.time_zone, \
         fiscal_year_start_month = EXCLUDED.fiscal_year_start_month, set_by = EXCLUDED.set_by, \
         updated_at = now()",
        [
            zone.name().into(),
            i16::try_from(month).unwrap_or(1).into(),
            who.into(),
        ],
    ))
    .await?;
    Audit::record(
        &txn,
        "organization_settings",
        Uuid::nil(),
        "set",
        caller.actor(),
        Some(serde_json::json!({
            "time_zone": zone.name(),
            "fiscal_year_start_month": month,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(view(&ctx.db).await?)
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/settings/organization", get(show))
        .add("/settings/organization", put(set))
}
