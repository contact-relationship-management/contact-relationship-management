//! Operational insight views — read-only derivations over recorded
//! facts, mirroring the family's honesty rules (per-currency never
//! merged, ratios carry their inputs, derivations disclosed in-band,
//! `as_of` + `ETag` via the dashboards helpers):
//!
//! - **Sales** — stale-deal aging (from `deal_stage_changed` audits),
//!   follow-up calendar + overdue list (activity `due_on`/`done`),
//!   pipeline-hygiene findings (rule-disclosed), the period executive
//!   pack, and the stored forecast-trend series (no interpolation).
//! - **Support** — the SLA breach register + per-assignee workload.
//! - **DPO** — consent coverage + withdrawals + per-source counts
//!   (from `consent_events`) and duplicate-contact hygiene (CRM-local
//!   rows sharing one `person_ref`; identity dedup stays upstream).

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder};
use serde::Deserialize;
use uuid::Uuid;

use super::dashboards::conditional;
use super::record_rejection;
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{
    accounts, activities, audit_logs, consent_events, contacts, deals, forecast_snapshots, leads,
    memberships, partnerships, pipeline_stages, tickets,
};
use crate::models::records;
use crate::rules::engagement as engagement_rules;

/// Load all live rows of one entity (soft-deleted excluded).
macro_rules! live {
    ($module:ident, $db:expr) => {
        <$module::Entity as crate::models::live::FindLive>::find_live()
            .all($db)
            .await?
    };
}

mod compliance;
mod outreach;
mod relationships;
mod rollup;
mod sales;
mod support;

use compliance::{consent_by_account, dpo};
use outreach::{cadence, engagement, funnel, members};
use relationships::{memberships_view, partnerships_register, stakeholders};
use sales::{executive, followups, forecast_trends, pipeline_hygiene, stale_deals};
use support::sla_register;

/// A `?days=` recency threshold (default `default`, clamped 1–365).
fn days_or(query_days: Option<u32>, default: u32) -> i64 {
    i64::from(query_days.unwrap_or(default).clamp(1, 365))
}

/// Query carrying an optional `days` threshold.
#[derive(Debug, Deserialize)]
struct DaysQuery {
    days: Option<u32>,
}

/// A `?from=&to=` window (dates or RFC 3339; default trailing 30 days).
#[derive(Debug, Deserialize)]
struct WindowQuery {
    from: Option<String>,
    to: Option<String>,
}

/// Parse one window boundary.
fn parse_instant(raw: &str, end_of_day: bool) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(instant) = raw.parse::<chrono::DateTime<chrono::Utc>>() {
        return Some(instant);
    }
    let date = raw.parse::<chrono::NaiveDate>().ok()?;
    let time = if end_of_day {
        chrono::NaiveTime::from_hms_opt(23, 59, 59)?
    } else {
        chrono::NaiveTime::MIN
    };
    Some(date.and_time(time).and_utc())
}

/// The insight routes (all read-only GETs).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/insights/stale-deals", get(stale_deals))
        .add("/insights/followups", get(followups))
        .add("/insights/pipeline-hygiene", get(pipeline_hygiene))
        .add("/insights/executive", get(executive))
        .add("/insights/forecast-trends", get(forecast_trends))
        .add("/insights/sla", get(sla_register))
        .add("/insights/dpo", get(dpo))
        .add("/insights/cadence", get(cadence))
        .add("/insights/engagement", get(engagement))
        .add("/insights/funnel", get(funnel))
        .add("/insights/members", get(members))
        .add("/insights/consent-by-account", get(consent_by_account))
        .add("/insights/stakeholders", get(stakeholders))
        .add("/insights/partnerships", get(partnerships_register))
        .add("/insights/memberships", get(memberships_view))
}
