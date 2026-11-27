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

use axa::prelude::*;
use axum::http::HeaderMap;
use serde::Deserialize;
use uuid::Uuid;

use super::dashboards::conditional;
use super::record_rejection;
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{
    accounts, activities, contacts, deals, forecast_snapshots, leads, memberships, partnerships,
    pipeline_stages, tickets,
};
use crate::rules::engagement as engagement_rules;

/// Load all live rows of one entity (soft-deleted excluded), through its
/// `sqlx` finder in [`crate::models::rows`].
macro_rules! live {
    ($module:ident, $ctx:expr) => {
        crate::models::rows::$module(crate::db::pool($ctx)).await?
    };
}

pub(crate) mod compliance;
pub(crate) mod outreach;
pub(crate) mod relationships;
mod rollup;
pub(crate) mod sales;
pub(crate) mod support;

use compliance::{consent_by_account, dpo};
use outreach::{cadence, engagement, funnel, members};
use relationships::{memberships_view, partnerships_register, stakeholders};
use sales::{executive, followups, forecast_trends, pipeline_hygiene, stale_deals};
use support::{sla_attainment, sla_register, stuck_tickets};

/// A `?days=` recency threshold: `default` when absent, a `422` when outside 1 to 365.
fn days_or(query_days: Option<u32>, default: u32) -> Result<i64> {
    match query_days {
        None => Ok(i64::from(default)),
        Some(n) => crate::rules::bounds::within(n, 1..=365)
            .map(i64::from)
            .ok_or_else(|| crate::arith::out_of_range("days", 1, 365)),
    }
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

/// One window boundary as asked: `None` when none was given. A bound that is not a date or an
/// RFC 3339 instant is refused with a `422` naming it, never read as the default.
///
/// # Errors
///
/// `422` "{name} must be a date or an RFC 3339 instant".
fn window_bound(
    raw: Option<&str>,
    name: &str,
    end_of_day: bool,
) -> Result<Option<crate::time::Timestamp>> {
    raw.map(|raw| {
        parse_instant(raw, end_of_day).ok_or_else(|| {
            super::unprocessable(&format!("{name} must be a date or an RFC 3339 instant"))
        })
    })
    .transpose()
}

/// Parse one window boundary.
fn parse_instant(raw: &str, end_of_day: bool) -> Option<crate::time::Timestamp> {
    if let Ok(instant) = raw.parse::<crate::time::Timestamp>() {
        return Some(instant);
    }
    let date = raw.parse::<crate::time::Date>().ok()?;
    let time = if end_of_day {
        jiff::civil::time(23, 59, 59, 0)
    } else {
        jiff::civil::Time::midnight()
    };
    date.at_utc(time)
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
        .add("/insights/stuck-tickets", get(stuck_tickets))
        .add("/insights/sla-attainment", get(sla_attainment))
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

#[cfg(test)]
mod tests {
    use super::days_or;

    /// A window outside 1 to 365 is an error that names the figure, never the nearest bound.
    #[test]
    fn a_window_outside_one_to_365_days_is_refused() {
        assert_eq!(days_or(None, 90).unwrap(), 90);
        assert_eq!(days_or(Some(1), 90).unwrap(), 1);
        assert_eq!(days_or(Some(365), 90).unwrap(), 365);
        for bad in [0, 366, u32::MAX] {
            let refused = days_or(Some(bad), 90).unwrap_err().to_string();
            assert!(refused.contains("days must be from 1 to 365"), "{refused}");
            assert!(
                !refused.contains(&bad.to_string()),
                "the value is not echoed"
            );
        }
    }
}
