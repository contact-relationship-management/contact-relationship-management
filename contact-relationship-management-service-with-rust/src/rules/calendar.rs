//! Business calendars (CRM-R45): counting an SLA target in working time.
//!
//! A calendar is a time zone, the working windows of each weekday and a list of holidays. A deadline
//! counted on it starts from the moment a ticket opens, skips the time outside the windows and the
//! days on the holiday list, and lands on the instant the target's working minutes have run out.
//! Pure and DB-free. Windows are local times, so they follow the zone's clock changes.

use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// Most windows in one day.
pub const MAX_WINDOWS: usize = 4;
/// Most holidays in a calendar.
pub const MAX_HOLIDAYS: usize = 400;
/// How far ahead a deadline is searched for, in days (a calendar with some working time finds one
/// well inside this unless the target is years of working time).
const HORIZON_DAYS: i64 = 4000;

/// One working window: local start and end, written `HH:MM`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WorkingWindow")]
#[serde(deny_unknown_fields)]
pub struct Hours {
    /// When work starts, `HH:MM`.
    pub start: String,
    /// When work ends, `HH:MM`, after the start.
    pub end: String,
}

/// The working windows of each weekday. A day with none is a day off.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WorkingWeek")]
#[serde(deny_unknown_fields)]
pub struct Week {
    /// Monday.
    #[serde(default)]
    pub mon: Vec<Hours>,
    /// Tuesday.
    #[serde(default)]
    pub tue: Vec<Hours>,
    /// Wednesday.
    #[serde(default)]
    pub wed: Vec<Hours>,
    /// Thursday.
    #[serde(default)]
    pub thu: Vec<Hours>,
    /// Friday.
    #[serde(default)]
    pub fri: Vec<Hours>,
    /// Saturday.
    #[serde(default)]
    pub sat: Vec<Hours>,
    /// Sunday.
    #[serde(default)]
    pub sun: Vec<Hours>,
}

impl Week {
    fn days(&self) -> [(&'static str, &Vec<Hours>); 7] {
        [
            ("mon", &self.mon),
            ("tue", &self.tue),
            ("wed", &self.wed),
            ("thu", &self.thu),
            ("fri", &self.fri),
            ("sat", &self.sat),
            ("sun", &self.sun),
        ]
    }
}

/// A calendar ready to count on.
#[derive(Debug, Clone)]
pub struct Calendar {
    tz: Tz,
    /// Monday first.
    windows: [Vec<(NaiveTime, NaiveTime)>; 7],
    holidays: BTreeSet<NaiveDate>,
}

fn time_of(text: &str) -> Option<NaiveTime> {
    (text.len() == 5).then_some(())?;
    NaiveTime::parse_from_str(text, "%H:%M").ok()
}

impl Calendar {
    /// Build a calendar from what was stored or sent, or say what is wrong, as `(member, message)`.
    ///
    /// # Errors
    ///
    /// Every problem found: an unknown time zone, a window that is not `HH:MM` to a later `HH:MM`,
    /// windows that overlap or are more than [`MAX_WINDOWS`] in a day, too many holidays, or no
    /// working time at all.
    pub fn build(
        timezone: &str,
        week: &Week,
        holidays: &[NaiveDate],
    ) -> Result<Self, Vec<(String, String)>> {
        let mut problems = Vec::new();
        let tz = timezone.parse::<Tz>().unwrap_or_else(|_| {
            problems.push((
                "timezone".to_string(),
                format!("{timezone:?} is not a time zone name such as Europe/London"),
            ));
            Tz::UTC
        });
        let mut windows: [Vec<(NaiveTime, NaiveTime)>; 7] = Default::default();
        for (index, (day, hours)) in week.days().into_iter().enumerate() {
            if hours.len() > MAX_WINDOWS {
                problems.push((
                    format!("week.{day}"),
                    format!("has more than {MAX_WINDOWS} windows"),
                ));
                continue;
            }
            for window in hours {
                match (time_of(&window.start), time_of(&window.end)) {
                    (Some(start), Some(end)) if start < end => windows[index].push((start, end)),
                    _ => problems.push((
                        format!("week.{day}"),
                        "each window needs a start and a later end, written HH:MM".to_string(),
                    )),
                }
            }
            windows[index].sort();
            if windows[index].windows(2).any(|pair| pair[0].1 > pair[1].0) {
                problems.push((format!("week.{day}"), "windows overlap".to_string()));
            }
        }
        if windows.iter().all(Vec::is_empty) {
            problems.push((
                "week".to_string(),
                "needs at least one working window".to_string(),
            ));
        }
        if holidays.len() > MAX_HOLIDAYS {
            problems.push((
                "holidays".to_string(),
                format!("at most {MAX_HOLIDAYS} holidays"),
            ));
        }
        if problems.is_empty() {
            Ok(Self {
                tz,
                windows,
                holidays: holidays.iter().copied().collect(),
            })
        } else {
            Err(problems)
        }
    }

    /// The instant `minutes` of working time after `start`: time outside the windows, and holidays,
    /// do not count. `None` if the target is longer than the search horizon allows.
    #[must_use]
    pub fn add_working_minutes(
        &self,
        start: DateTime<FixedOffset>,
        minutes: i64,
    ) -> Option<DateTime<FixedOffset>> {
        self.add_working_seconds(start, minutes.checked_mul(60)?)
    }

    /// The instant `seconds` of working time after `start` (see [`Self::add_working_minutes`]).
    #[must_use]
    pub fn add_working_seconds(
        &self,
        start: DateTime<FixedOffset>,
        seconds: i64,
    ) -> Option<DateTime<FixedOffset>> {
        let mut remaining = seconds;
        let mut cursor = start.with_timezone(&Utc);
        let mut date = start.with_timezone(&self.tz).date_naive();
        for _ in 0..HORIZON_DAYS {
            if !self.holidays.contains(&date) {
                for (from, to) in &self.windows[date.weekday().num_days_from_monday() as usize] {
                    // A clock time the zone skips (the hour a clock goes forward) has no instant;
                    // that end of the window is dropped.
                    let (Some(opens), Some(closes)) =
                        (self.instant(date, *from), self.instant(date, *to))
                    else {
                        continue;
                    };
                    let begin = opens.max(cursor);
                    if begin >= closes {
                        continue;
                    }
                    let room = (closes - begin).num_seconds();
                    if remaining <= room {
                        return Some((begin + Duration::seconds(remaining)).fixed_offset());
                    }
                    remaining -= room;
                }
            }
            date = date.succ_opt()?;
            cursor = cursor.max(self.instant(date, NaiveTime::MIN).unwrap_or(cursor));
        }
        None
    }

    /// The seconds of working time between two instants (zero when `to` is not after `from`).
    /// Counted over at most the search horizon, so a longer span is cut short rather than refused.
    #[must_use]
    pub fn working_seconds_between(
        &self,
        from: DateTime<FixedOffset>,
        to: DateTime<FixedOffset>,
    ) -> i64 {
        let (from, to) = (from.with_timezone(&Utc), to.with_timezone(&Utc));
        let mut total = 0;
        let mut date = from.with_timezone(&self.tz).date_naive();
        let last = to.with_timezone(&self.tz).date_naive();
        for _ in 0..HORIZON_DAYS {
            if date > last {
                break;
            }
            if !self.holidays.contains(&date) {
                for (opens, closes) in &self.windows[date.weekday().num_days_from_monday() as usize]
                {
                    let (Some(opens), Some(closes)) =
                        (self.instant(date, *opens), self.instant(date, *closes))
                    else {
                        continue;
                    };
                    let (begin, end) = (opens.max(from), closes.min(to));
                    if end > begin {
                        total += (end - begin).num_seconds();
                    }
                }
            }
            let Some(next) = date.succ_opt() else { break };
            date = next;
        }
        total
    }

    fn instant(&self, date: NaiveDate, time: NaiveTime) -> Option<DateTime<Utc>> {
        self.tz
            .from_local_datetime(&date.and_time(time))
            .earliest()
            .map(|d| d.with_timezone(&Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hours(start: &str, end: &str) -> Hours {
        Hours {
            start: start.to_string(),
            end: end.to_string(),
        }
    }

    fn office(zone: &str, holidays: &[&str]) -> Calendar {
        let day = vec![hours("09:00", "17:00")];
        let week = Week {
            mon: day.clone(),
            tue: day.clone(),
            wed: day.clone(),
            thu: day.clone(),
            fri: day,
            ..Week::default()
        };
        let holidays: Vec<NaiveDate> = holidays.iter().map(|d| d.parse().unwrap()).collect();
        Calendar::build(zone, &week, &holidays).unwrap()
    }

    fn utc(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    #[test]
    fn a_target_inside_the_day_lands_in_the_day() {
        let c = office("UTC", &[]);
        // Monday 6 July 2026.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-06T10:00:00Z"), 90),
            Some(utc("2026-07-06T11:30:00Z"))
        );
        // Exactly to the close of the day.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-06T16:00:00Z"), 60),
            Some(utc("2026-07-06T17:00:00Z"))
        );
    }

    #[test]
    fn time_outside_the_windows_does_not_count() {
        let c = office("UTC", &[]);
        // Before opening: the clock starts at 09:00.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-06T07:00:00Z"), 30),
            Some(utc("2026-07-06T09:30:00Z"))
        );
        // After closing: next morning.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-06T18:00:00Z"), 30),
            Some(utc("2026-07-07T09:30:00Z"))
        );
        // Over a weekend: opened Friday 16:00, two hours = one on Friday, one on Monday.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-10T16:00:00Z"), 120),
            Some(utc("2026-07-13T10:00:00Z"))
        );
        // Opened on a Saturday.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-11T10:00:00Z"), 60),
            Some(utc("2026-07-13T10:00:00Z"))
        );
        // A target of several working days.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-06T09:00:00Z"), 8 * 60 * 3),
            Some(utc("2026-07-08T17:00:00Z"))
        );
    }

    #[test]
    fn a_holiday_is_a_day_off() {
        let c = office("UTC", &["2026-07-13"]);
        assert_eq!(
            c.add_working_minutes(utc("2026-07-10T16:00:00Z"), 120),
            Some(utc("2026-07-14T10:00:00Z"))
        );
    }

    #[test]
    fn a_lunch_break_is_not_counted() {
        let day = vec![hours("09:00", "12:00"), hours("13:00", "17:00")];
        let week = Week {
            mon: day,
            ..Week::default()
        };
        let c = Calendar::build("UTC", &week, &[]).unwrap();
        // 11:00 + 120 minutes: one hour before lunch, one after.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-06T11:00:00Z"), 120),
            Some(utc("2026-07-06T14:00:00Z"))
        );
        // A target longer than the week wraps to the next Monday.
        assert_eq!(
            c.add_working_minutes(utc("2026-07-06T16:00:00Z"), 120),
            Some(utc("2026-07-13T10:00:00Z"))
        );
    }

    #[test]
    fn the_windows_follow_the_zones_clock() {
        // New York changes to summer time on Sunday 8 March 2026. 09:00 local is 14:00Z before,
        // 13:00Z after.
        let c = office("America/New_York", &[]);
        let friday_4pm_est = utc("2026-03-06T21:00:00Z");
        assert_eq!(
            c.add_working_minutes(friday_4pm_est, 120),
            Some(utc("2026-03-09T14:00:00Z"))
        );
    }

    #[test]
    fn a_bad_calendar_says_what_is_wrong() {
        let bad = |zone: &str, week: &Week, holidays: &[NaiveDate]| {
            Calendar::build(zone, week, holidays).unwrap_err()
        };
        let none = Week::default();
        assert!(bad("UTC", &none, &[]).iter().any(|(m, _)| m == "week"));
        assert!(
            bad(
                "Mars/Olympus",
                &Week {
                    mon: vec![hours("09:00", "17:00")],
                    ..Week::default()
                },
                &[]
            )
            .iter()
            .any(|(m, _)| m == "timezone")
        );
        for window in [
            hours("9:00", "17:00"),
            hours("17:00", "09:00"),
            hours("09:00", "09:00"),
            hours("09:00", "25:00"),
        ] {
            let week = Week {
                mon: vec![window],
                ..Week::default()
            };
            assert!(bad("UTC", &week, &[]).iter().any(|(m, _)| m == "week.mon"));
        }
        let overlap = Week {
            mon: vec![hours("09:00", "12:00"), hours("11:00", "13:00")],
            ..Week::default()
        };
        assert!(
            bad("UTC", &overlap, &[])
                .iter()
                .any(|(_, text)| text.contains("overlap"))
        );
        let many = Week {
            mon: (0..=MAX_WINDOWS)
                .map(|i| hours(&format!("{:02}:00", i * 2), &format!("{:02}:30", i * 2)))
                .collect(),
            ..Week::default()
        };
        assert!(!bad("UTC", &many, &[]).is_empty());
        let days: Vec<NaiveDate> = (0..=i64::try_from(MAX_HOLIDAYS).unwrap())
            .map(|i| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + Duration::days(i))
            .collect();
        let one = Week {
            mon: vec![hours("09:00", "17:00")],
            ..Week::default()
        };
        assert!(bad("UTC", &one, &days).iter().any(|(m, _)| m == "holidays"));
    }

    #[test]
    fn the_limits_are_inclusive_and_windows_may_touch() {
        // Exactly the most windows in a day, and as many holidays as allowed, are accepted.
        let four: Vec<Hours> = (0..MAX_WINDOWS)
            .map(|i| hours(&format!("{:02}:00", i * 4), &format!("{:02}:30", i * 4)))
            .collect();
        let week = Week {
            mon: four,
            ..Week::default()
        };
        let days: Vec<NaiveDate> = (0..i64::try_from(MAX_HOLIDAYS).unwrap())
            .map(|i| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + Duration::days(i))
            .collect();
        assert!(Calendar::build("UTC", &week, &days).is_ok());
        // One window ending where the next starts is not an overlap.
        let touching = Week {
            mon: vec![hours("09:00", "12:00"), hours("12:00", "13:00")],
            ..Week::default()
        };
        assert!(Calendar::build("UTC", &touching, &[]).is_ok());
    }

    #[test]
    fn working_time_between_two_instants_skips_nights_weekends_and_holidays() {
        let c = office("UTC", &["2026-07-08"]);
        let between = |a, b| c.working_seconds_between(utc(a), utc(b));
        // Inside one day.
        assert_eq!(
            between("2026-07-06T10:00:00Z", "2026-07-06T11:30:00Z"),
            90 * 60
        );
        // Monday 16:00 to Tuesday 10:00: one hour and one hour.
        assert_eq!(
            between("2026-07-06T16:00:00Z", "2026-07-07T10:00:00Z"),
            2 * 3600
        );
        // Across Wednesday's holiday: Tuesday 16:00 to Thursday 10:00 is 1h + 0 + 1h.
        assert_eq!(
            between("2026-07-07T16:00:00Z", "2026-07-09T10:00:00Z"),
            2 * 3600
        );
        // Friday 16:00 to Monday 10:00 skips the weekend.
        assert_eq!(
            between("2026-07-10T16:00:00Z", "2026-07-13T10:00:00Z"),
            2 * 3600
        );
        // Backwards or empty is zero.
        assert_eq!(between("2026-07-06T11:00:00Z", "2026-07-06T10:00:00Z"), 0);
        assert_eq!(between("2026-07-06T11:00:00Z", "2026-07-06T11:00:00Z"), 0);
    }

    #[test]
    fn adding_and_measuring_working_time_agree() {
        let c = office("Europe/London", &["2026-07-08"]);
        let start = utc("2026-07-06T15:00:00Z");
        for seconds in [60, 3600, 5 * 3600, 20 * 3600, 33 * 3600 + 17] {
            let end = c.add_working_seconds(start, seconds).unwrap();
            assert_eq!(c.working_seconds_between(start, end), seconds, "{seconds}");
        }
    }
}
