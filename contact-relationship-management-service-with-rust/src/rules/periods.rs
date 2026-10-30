//! The organization's calendar for "today" and report periods (CRM-T171, CRM-R95).
//!
//! Pure and DB-free. An organization sets its IANA time zone and the month its fiscal year starts
//! (`organization_settings`, migration `000057`); a caller may name its own zone with the
//! `CRM-Time-Zone` request header, which wins for that caller's "today". A report's months, fiscal
//! quarters and fiscal years are the organization's, so a shared report reads the same for
//! everyone. Fiscal labels name the calendar year a fiscal year starts in: with a January start
//! `2026` and `2026-Q1`; with an April start `2026/27` and `2026/27-Q1` (April to June 2026).

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;

/// The request header in which a caller names its own time zone.
pub const USER_ZONE_HEADER: &str = "crm-time-zone";

/// The organization's calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periods {
    /// The organization's time zone.
    pub zone: Tz,
    /// The month (1 to 12) its fiscal year starts in.
    pub fiscal_start_month: u32,
}

impl Default for Periods {
    /// What an organization has until it sets its own: UTC and a January fiscal year, both
    /// shown as such in the settings, so nobody mistakes them for a choice.
    fn default() -> Self {
        Self {
            zone: Tz::UTC,
            fiscal_start_month: 1,
        }
    }
}

/// An IANA time zone name, checked.
///
/// # Errors
///
/// The reason, when it is not one.
pub fn zone(name: &str) -> Result<Tz, String> {
    name.trim()
        .parse::<Tz>()
        .map_err(|_| "is an IANA time zone such as Asia/Tokyo or America/Sao_Paulo".to_string())
}

/// A fiscal year's first month, checked.
///
/// # Errors
///
/// The reason, when it is not 1 to 12.
pub fn fiscal_start_month(month: i64) -> Result<u32, String> {
    u32::try_from(month)
        .ok()
        .filter(|m| (1..=12).contains(m))
        .ok_or_else(|| "is a month from 1 (January) to 12 (December)".to_string())
}

/// The day it is at `now` in `zone`.
#[must_use]
pub fn today_in(now: DateTime<Utc>, zone: Tz) -> NaiveDate {
    now.with_timezone(&zone).date_naive()
}

/// The zone for a caller's "today": the one it named, when it is a real zone, else the
/// organization's.
#[must_use]
#[allow(clippy::trivially_copy_pass_by_ref)] // the calendar is passed by reference everywhere
pub fn caller_zone(named: Option<&str>, periods: &Periods) -> Tz {
    named
        .and_then(|name| zone(name).ok())
        .unwrap_or(periods.zone)
}

/// The calendar year a fiscal year containing `day` starts in, and its quarter (1 to 4).
#[must_use]
pub fn fiscal_position(day: NaiveDate, start_month: u32) -> (i32, u32) {
    let shift = (day.month() + 12 - start_month) % 12; // months since the fiscal year began
    let year = if day.month() >= start_month {
        day.year()
    } else {
        day.year() - 1
    };
    (year, shift / 3 + 1)
}

/// A fiscal year's label: `2026`, or `2026/27` when it does not start in January.
#[must_use]
pub fn fiscal_year_label(start_year: i32, start_month: u32) -> String {
    if start_month == 1 {
        start_year.to_string()
    } else {
        format!("{start_year}/{:02}", (start_year + 1).rem_euclid(100))
    }
}

/// `zone`'s name as a SQL literal. Safe to write into a query: an IANA name holds only letters,
/// digits, `_`, `-`, `+` and `/`, which [`zone`] has already checked it is.
fn zone_literal(zone: Tz) -> String {
    let name = zone.name();
    debug_assert!(
        name.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-+/".contains(c))
    );
    format!("'{name}'")
}

/// SQL for the local time of the timestamp `column` in the organization's zone.
#[must_use]
#[allow(clippy::trivially_copy_pass_by_ref)] // the calendar is passed by reference everywhere
pub fn local(column: &str, periods: &Periods) -> String {
    format!("({column} AT TIME ZONE {})", zone_literal(periods.zone))
}

/// SQL for the month (`YYYY-MM`) of `column` in the organization's zone.
#[must_use]
#[allow(clippy::trivially_copy_pass_by_ref)] // the calendar is passed by reference everywhere
pub fn month_expr(column: &str, periods: &Periods) -> String {
    format!("to_char({}, 'YYYY-MM')", local(column, periods))
}

/// SQL for the fiscal year label of `column` (see [`fiscal_year_label`]).
#[must_use]
#[allow(clippy::trivially_copy_pass_by_ref)] // the calendar is passed by reference everywhere
pub fn fiscal_year_expr(column: &str, periods: &Periods) -> String {
    let shifted = shifted(column, periods);
    if periods.fiscal_start_month == 1 {
        format!("to_char({shifted}, 'YYYY')")
    } else {
        format!(
            "(to_char({shifted}, 'YYYY') || '/' || \
             lpad(((extract(year FROM {shifted})::int + 1) % 100)::text, 2, '0'))"
        )
    }
}

/// SQL for the fiscal quarter label of `column`: the fiscal year, `-Q`, and 1 to 4.
#[must_use]
#[allow(clippy::trivially_copy_pass_by_ref)] // the calendar is passed by reference everywhere
pub fn fiscal_quarter_expr(column: &str, periods: &Periods) -> String {
    format!(
        "({} || '-Q' || extract(quarter FROM {})::int)",
        fiscal_year_expr(column, periods),
        shifted(column, periods)
    )
}

/// The local time moved back by the months before the fiscal year starts, so its calendar year
/// and quarter are the fiscal ones.
#[allow(clippy::trivially_copy_pass_by_ref)] // the calendar is passed by reference everywhere
fn shifted(column: &str, periods: &Periods) -> String {
    let months = periods.fiscal_start_month.saturating_sub(1);
    if months == 0 {
        local(column, periods)
    } else {
        format!("({} - interval '{months} months')", local(column, periods))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn today_is_the_zone_s_day_not_utc_s() {
        // 23:30 UTC on 31 December is already 1 January in Tokyo and still 31 December in Lima.
        let now = Utc.with_ymd_and_hms(2026, 12, 31, 23, 30, 0).unwrap();
        assert_eq!(today_in(now, zone("Asia/Tokyo").unwrap()), day(2027, 1, 1));
        assert_eq!(
            today_in(now, zone("America/Lima").unwrap()),
            day(2026, 12, 31)
        );
        assert_eq!(
            today_in(now, zone("Pacific/Kiritimati").unwrap()),
            day(2027, 1, 1)
        );
    }

    #[test]
    fn a_caller_s_own_zone_wins_when_it_is_real() {
        let org = Periods {
            zone: zone("Europe/London").unwrap(),
            fiscal_start_month: 4,
        };
        assert_eq!(
            caller_zone(Some("Asia/Kolkata"), &org).name(),
            "Asia/Kolkata"
        );
        assert_eq!(
            caller_zone(Some("Mars/Olympus"), &org).name(),
            "Europe/London"
        );
        assert_eq!(caller_zone(None, &org).name(), "Europe/London");
    }

    #[test]
    fn fiscal_years_and_quarters_start_in_any_month() {
        // January (many countries), April (India, Japan, the UK government), July (Australia,
        // Egypt), October (the US federal government).
        assert_eq!(fiscal_position(day(2026, 3, 31), 1), (2026, 1));
        assert_eq!(fiscal_position(day(2026, 3, 31), 4), (2025, 4));
        assert_eq!(fiscal_position(day(2026, 4, 1), 4), (2026, 1));
        assert_eq!(fiscal_position(day(2026, 6, 30), 7), (2025, 4));
        assert_eq!(fiscal_position(day(2026, 7, 1), 7), (2026, 1));
        assert_eq!(fiscal_position(day(2026, 12, 15), 10), (2026, 1));
        assert_eq!(fiscal_position(day(2026, 9, 30), 10), (2025, 4));
        assert_eq!(fiscal_year_label(2026, 1), "2026");
        assert_eq!(fiscal_year_label(2026, 4), "2026/27");
        assert_eq!(fiscal_year_label(2099, 7), "2099/00");
    }

    #[test]
    fn settings_are_checked() {
        assert!(zone("Asia/Tokyo").is_ok());
        assert!(zone("Tokyo").is_err());
        assert!(zone("'; DROP TABLE x; --").is_err());
        assert_eq!(fiscal_start_month(4), Ok(4));
        assert!(fiscal_start_month(0).is_err() && fiscal_start_month(13).is_err());
    }

    #[test]
    fn the_sql_reads_the_zone_and_the_shift() {
        let p = Periods {
            zone: zone("Australia/Sydney").unwrap(),
            fiscal_start_month: 7,
        };
        assert_eq!(
            month_expr("d.created_at", &p),
            "to_char((d.created_at AT TIME ZONE 'Australia/Sydney'), 'YYYY-MM')"
        );
        assert!(fiscal_quarter_expr("d.closed_at", &p).contains("interval '6 months'"));
        let january = Periods::default();
        assert_eq!(
            fiscal_year_expr("k.opened_at", &january),
            "to_char((k.opened_at AT TIME ZONE 'UTC'), 'YYYY')"
        );
    }
}
