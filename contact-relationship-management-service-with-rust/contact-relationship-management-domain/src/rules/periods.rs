//! The organization's calendar for "today" and report periods (CRM-T171, CRM-R95).
//!
//! Pure and DB-free. An organization sets its IANA time zone and the month its fiscal year starts
//! (`organization_settings`, migration `000057`); a caller may name its own zone with the
//! `CRM-Time-Zone` request header, which wins for that caller's "today". A report's months, fiscal
//! quarters and fiscal years are the organization's, so a shared report reads the same for
//! everyone. Fiscal labels name the calendar year a fiscal year starts in: with a January start
//! `2026` and `2026-Q1`; with an April start `2026/27` and `2026/27-Q1` (April to June 2026).

use jiff::civil::Date;
use jiff::tz::TimeZone;

/// The request header in which a caller names its own time zone.
pub const USER_ZONE_HEADER: &str = "crm-time-zone";

/// The organization's calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Periods {
    /// The organization's time zone, always one read by its IANA name (see [`zone`]).
    pub zone: TimeZone,
    /// The month (1 to 12) its fiscal year starts in.
    pub fiscal_start_month: u32,
}

impl Default for Periods {
    /// What an organization has until it sets its own: UTC and a January fiscal year, both
    /// shown as such in the settings, so nobody mistakes them for a choice.
    fn default() -> Self {
        Self {
            zone: TimeZone::UTC,
            fiscal_start_month: 1,
        }
    }
}

/// An IANA time zone name, checked.
///
/// # Errors
///
/// The reason, when it is not one.
pub fn zone(name: &str) -> Result<TimeZone, String> {
    named_zone(name.trim())
        .ok_or_else(|| "is an IANA time zone such as Asia/Tokyo or America/Sao_Paulo".to_string())
}

/// The zone of an IANA name, written exactly as the database writes it. `jiff` looks a name up
/// regardless of case; the name is also written into SQL and stored, so `asia/tokyo` is refused,
/// as `chrono-tz` refused it.
pub(crate) fn named_zone(name: &str) -> Option<TimeZone> {
    TimeZone::get(name)
        .ok()
        .filter(|zone| zone.iana_name() == Some(name))
}

/// A zone's IANA name; `UTC` for one without a name, which [`zone`] never gives.
#[must_use]
pub fn zone_name(zone: &TimeZone) -> &str {
    zone.iana_name().unwrap_or("UTC")
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

/// The day it is at `now` in `zone`. Takes and gives the stored types, as every caller has one
/// and binds or sends the other.
#[must_use]
pub fn today_in(now: crate::time::Timestamp, zone: &TimeZone) -> crate::time::Date {
    crate::time::Date(zone.to_datetime(now.0).date())
}

/// The organization's day of an event at `at`: the day a deal closed on, for its close-day rate
/// and for a range of close days (CRM-T181), is the organization's, not UTC's.
#[must_use]
pub fn day_of(at: crate::time::Timestamp, periods: &Periods) -> crate::time::Date {
    today_in(at, &periods.zone)
}

/// The zone for a caller's "today": the one it named, when it is a real zone, else the
/// organization's.
#[must_use]
pub fn caller_zone(named: Option<&str>, periods: &Periods) -> TimeZone {
    named
        .and_then(|name| zone(name).ok())
        .unwrap_or_else(|| periods.zone.clone())
}

/// The calendar year a fiscal year containing `day` starts in, and its quarter (1 to 4).
#[must_use]
pub fn fiscal_position(day: Date, start_month: u32) -> Option<(i32, u32)> {
    let month = u32::from(day.month().unsigned_abs()); // 1 to 12
    // Months since the fiscal year began.
    let shift = month
        .checked_add(12)?
        .checked_sub(start_month)?
        .checked_rem(12)?;
    let year = if month >= start_month {
        i32::from(day.year())
    } else {
        i32::from(day.year()).checked_sub(1)?
    };
    Some((year, shift.checked_div(3)?.checked_add(1)?))
}

/// A fiscal year's label: `2026`, or `2026/27` when it does not start in January.
#[must_use]
pub fn fiscal_year_label(start_year: i32, start_month: u32) -> Option<String> {
    if start_month == 1 {
        Some(start_year.to_string())
    } else {
        let next = start_year.checked_add(1)?.checked_rem_euclid(100)?;
        Some(format!("{start_year}/{next:02}"))
    }
}

/// `zone`'s name as a SQL literal. Safe to write into a query: an IANA name holds only letters,
/// digits, `_`, `-`, `+` and `/`, which [`zone`] has already checked it is.
fn zone_literal(zone: &TimeZone) -> String {
    let name = zone_name(zone);
    debug_assert!(
        name.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-+/".contains(c))
    );
    format!("'{name}'")
}

/// SQL for the local time of the timestamp `column` in the organization's zone.
#[must_use]
pub fn local(column: &str, periods: &Periods) -> String {
    format!("({column} AT TIME ZONE {})", zone_literal(&periods.zone))
}

/// SQL for the day of the timestamp `column` in the organization's zone (see [`day_of`]).
#[must_use]
pub fn local_day(column: &str, periods: &Periods) -> String {
    format!("{}::date", local(column, periods))
}

/// SQL for the month (`YYYY-MM`) of `column` in the organization's zone.
#[must_use]
pub fn month_expr(column: &str, periods: &Periods) -> String {
    format!("to_char({}, 'YYYY-MM')", local(column, periods))
}

/// SQL for the fiscal year label of `column` (see [`fiscal_year_label`]).
#[must_use]
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
pub fn fiscal_quarter_expr(column: &str, periods: &Periods) -> String {
    format!(
        "({} || '-Q' || extract(quarter FROM {})::int)",
        fiscal_year_expr(column, periods),
        shifted(column, periods)
    )
}

/// The local time moved back by the months before the fiscal year starts, so its calendar year
/// and quarter are the fiscal ones.
fn shifted(column: &str, periods: &Periods) -> String {
    // PERIOD-1: January starts the fiscal year no earlier than the calendar one, so no months.
    let months = periods.fiscal_start_month.checked_sub(1).unwrap_or(0);
    if months == 0 {
        local(column, periods)
    } else {
        format!("({} - interval '{months} months')", local(column, periods))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fiscal_position(day: Date, start_month: u32) -> (i32, u32) {
        super::fiscal_position(day, start_month).unwrap()
    }
    fn fiscal_year_label(start_year: i32, start_month: u32) -> String {
        super::fiscal_year_label(start_year, start_month).unwrap()
    }

    fn day(y: i16, m: i8, d: i8) -> Date {
        Date::new(y, m, d).unwrap()
    }

    fn utc(y: i16, mo: i8, d: i8, h: i8, mi: i8) -> crate::time::Timestamp {
        crate::time::Date(day(y, mo, d))
            .at_utc(jiff::civil::time(h, mi, 0, 0))
            .unwrap()
    }

    #[test]
    fn today_is_the_zone_s_day_not_utc_s() {
        // 23:30 UTC on 31 December is already 1 January in Tokyo and still 31 December in Lima.
        let now = utc(2026, 12, 31, 23, 30);
        assert_eq!(
            today_in(now, &zone("Asia/Tokyo").unwrap()).0,
            day(2027, 1, 1)
        );
        assert_eq!(
            today_in(now, &zone("America/Lima").unwrap()).0,
            day(2026, 12, 31)
        );
        assert_eq!(
            today_in(now, &zone("Pacific/Kiritimati").unwrap()).0,
            day(2027, 1, 1)
        );
    }

    #[test]
    fn an_event_s_day_is_the_organization_s() {
        // A deal closed at 10:30 UTC on 30 June is closed on 1 July in Kiritimati (UTC+14) and on
        // 29 June in Pago Pago (UTC-11): three days for one moment. The SQL reads the same zone.
        let at = utc(2026, 6, 30, 10, 30);
        let kiritimati = Periods {
            zone: zone("Pacific/Kiritimati").unwrap(),
            fiscal_start_month: 1,
        };
        assert_eq!(day_of(at, &kiritimati).0, day(2026, 7, 1));
        let pago = Periods {
            zone: zone("Pacific/Pago_Pago").unwrap(),
            fiscal_start_month: 1,
        };
        assert_eq!(day_of(at, &pago).0, day(2026, 6, 29));
        assert_eq!(day_of(at, &Periods::default()).0, day(2026, 6, 30));
        assert_eq!(
            local_day("d.closed_at", &kiritimati),
            "(d.closed_at AT TIME ZONE 'Pacific/Kiritimati')::date"
        );
    }

    #[test]
    fn a_caller_s_own_zone_wins_when_it_is_real() {
        let org = Periods {
            zone: zone("Europe/London").unwrap(),
            fiscal_start_month: 4,
        };
        assert_eq!(
            zone_name(&caller_zone(Some("Asia/Kolkata"), &org)),
            "Asia/Kolkata"
        );
        assert_eq!(
            zone_name(&caller_zone(Some("Mars/Olympus"), &org)),
            "Europe/London"
        );
        assert_eq!(zone_name(&caller_zone(None, &org)), "Europe/London");
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
        // A name is written as the database writes it, as `chrono-tz` required.
        assert!(zone("asia/tokyo").is_err() && zone("utc").is_err());
        assert_eq!(zone_name(&zone(" UTC ").unwrap()), "UTC");
        assert_eq!(zone_name(&zone("US/Pacific").unwrap()), "US/Pacific");
        assert_eq!(zone_name(&Periods::default().zone), "UTC");
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
