//! Business calendars (CRM-R45): counting an SLA target in working time.
//!
//! A calendar is a time zone, the working windows of each weekday and a list of holidays. A deadline
//! counted on it starts from the moment a ticket opens, skips the time outside the windows and the
//! days on the holiday list, and lands on the instant the target's working minutes have run out.
//! Pure and DB-free. Windows are local times, so they follow the zone's clock changes.

use std::collections::BTreeSet;

use jiff::civil::{Date, Time};
use jiff::tz::{AmbiguousOffset, TimeZone};
use jiff::{SignedDuration, Timestamp};
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
    tz: TimeZone,
    /// Monday first.
    windows: [Vec<(Time, Time)>; 7],
    holidays: BTreeSet<Date>,
}

/// `HH:MM`, read as `chrono`'s `NaiveTime::parse_from_str(text, "%H:%M")` read it, of five
/// bytes: one or two digits of hours and of minutes, each of which may follow a space (`" 9:05"`,
/// `"9: 05"`), from `00:00` to `23:59`.
fn time_of(text: &str) -> Option<Time> {
    (text.len() == 5).then_some(())?;
    let (hour, rest) = digits(text.trim_start())?;
    let (minute, rest) = digits(rest.strip_prefix(':')?.trim_start())?;
    rest.is_empty().then_some(())?;
    Time::new(hour, minute, 0, 0).ok()
}

/// One or two ASCII digits at the start of `text`, and what follows them.
fn digits(text: &str) -> Option<(i8, &str)> {
    let count = text.bytes().take(2).take_while(u8::is_ascii_digit).count();
    let value = text[..count].bytes().fold(0_i8, |value, digit| {
        value * 10 + i8::try_from(digit - b'0').unwrap_or(0)
    });
    (count > 0).then(|| (value, &text[count..]))
}

impl Calendar {
    /// The opening windows on `date`'s weekday, none if the weekday has no slot.
    fn windows_on(&self, date: Date) -> std::slice::Iter<'_, (Time, Time)> {
        self.windows
            .get(weekday_index(date))
            .map_or([].iter(), |day| day.iter())
    }

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
        holidays: &[Date],
    ) -> Result<Self, Vec<(String, String)>> {
        let mut problems = Vec::new();
        let tz = super::periods::named_zone(timezone).unwrap_or_else(|| {
            problems.push((
                "timezone".to_string(),
                format!("{timezone:?} is not a time zone name such as Europe/London"),
            ));
            TimeZone::UTC
        });
        let mut windows: [Vec<(Time, Time)>; 7] = Default::default();
        for (slot, (day, hours)) in windows.iter_mut().zip(week.days()) {
            if hours.len() > MAX_WINDOWS {
                problems.push((
                    format!("week.{day}"),
                    format!("has more than {MAX_WINDOWS} windows"),
                ));
                continue;
            }
            for window in hours {
                match (time_of(&window.start), time_of(&window.end)) {
                    (Some(start), Some(end)) if start < end => slot.push((start, end)),
                    _ => problems.push((
                        format!("week.{day}"),
                        "each window needs a start and a later end, written HH:MM".to_string(),
                    )),
                }
            }
            slot.sort();
            if slot
                .windows(2)
                .any(|pair| matches!(pair, [a, b] if a.1 > b.0))
            {
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
    pub fn add_working_minutes(&self, start: Timestamp, minutes: i64) -> Option<Timestamp> {
        self.add_working_seconds(start, minutes.checked_mul(60)?)
    }

    /// The instant `seconds` of working time after `start` (see [`Self::add_working_minutes`]).
    #[must_use]
    pub fn add_working_seconds(&self, start: Timestamp, seconds: i64) -> Option<Timestamp> {
        let mut remaining = seconds;
        let mut cursor = start;
        let mut date = self.tz.to_datetime(start).date();
        for _ in 0..HORIZON_DAYS {
            if !self.holidays.contains(&date) {
                for (from, to) in self.windows_on(date) {
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
                    let room = closes.duration_since(begin).as_secs();
                    if remaining <= room {
                        return begin.checked_add(SignedDuration::from_secs(remaining)).ok();
                    }
                    remaining -= room;
                }
            }
            date = date.tomorrow().ok()?;
            cursor = cursor.max(self.instant(date, Time::midnight()).unwrap_or(cursor));
        }
        None
    }

    /// The seconds of working time between two instants (zero when `to` is not after `from`).
    /// Counted over at most the search horizon, so a longer span is cut short rather than refused.
    #[must_use]
    pub fn working_seconds_between(&self, from: Timestamp, to: Timestamp) -> i64 {
        let mut total = 0;
        let mut date = self.tz.to_datetime(from).date();
        let last = self.tz.to_datetime(to).date();
        for _ in 0..HORIZON_DAYS {
            if date > last {
                break;
            }
            if !self.holidays.contains(&date) {
                for (opens, closes) in self.windows_on(date) {
                    let (Some(opens), Some(closes)) =
                        (self.instant(date, *opens), self.instant(date, *closes))
                    else {
                        continue;
                    };
                    let (begin, end) = (opens.max(from), closes.min(to));
                    if end > begin {
                        total += end.duration_since(begin).as_secs();
                    }
                }
            }
            let Ok(next) = date.tomorrow() else { break };
            date = next;
        }
        total
    }

    /// The instant a local clock time on `date` names: the earlier of two when the clock goes back
    /// and that hour happens twice, and none when the clock goes forward over it.
    fn instant(&self, date: Date, time: Time) -> Option<Timestamp> {
        let local = self.tz.to_ambiguous_timestamp(date.to_datetime(time));
        match local.offset() {
            AmbiguousOffset::Gap { .. } => None,
            AmbiguousOffset::Unambiguous { .. } | AmbiguousOffset::Fold { .. } => {
                local.earlier().ok()
            }
        }
    }
}

/// Monday 0 to Sunday 6, the index of a day's windows.
fn weekday_index(date: Date) -> usize {
    usize::try_from(date.weekday().to_monday_zero_offset()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // CRM-T259: an hour with no digits is not midnight.
    #[test]
    fn a_window_time_needs_a_digit_before_the_colon() {
        assert_eq!(time_of("  :00"), None);
        assert_eq!(time_of(" 9:05"), Time::new(9, 5, 0, 0).ok());
    }

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
        let holidays: Vec<Date> = holidays.iter().map(|d| d.parse().unwrap()).collect();
        Calendar::build(zone, &week, &holidays).unwrap()
    }

    fn utc(text: &str) -> Timestamp {
        text.parse().unwrap()
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
    fn a_clock_time_twice_is_the_earlier_and_one_skipped_is_none() {
        let c = office("America/New_York", &[]);
        let at = |h, m| Time::new(h, m, 0, 0).unwrap();
        // 01:30 on Sunday 1 November 2026 happens in summer time and again in winter time.
        assert_eq!(
            c.instant(jiff::civil::date(2026, 11, 1), at(1, 30)),
            Some(utc("2026-11-01T05:30:00Z"))
        );
        // 02:30 on Sunday 8 March 2026 does not happen: the clock goes from 02:00 to 03:00.
        assert_eq!(c.instant(jiff::civil::date(2026, 3, 8), at(2, 30)), None);
        assert_eq!(
            c.instant(jiff::civil::date(2026, 3, 8), at(3, 0)),
            Some(utc("2026-03-08T07:00:00Z"))
        );
    }

    #[test]
    fn a_window_reads_as_chrono_read_hh_mm() {
        assert_eq!(time_of("09:05"), Time::new(9, 5, 0, 0).ok());
        assert_eq!(time_of("23:59"), Time::new(23, 59, 0, 0).ok());
        // `chrono` let a field follow a space, so these five-byte forms were accepted.
        for spaced in [" 9:05", "9: 05", "\t9:05"] {
            assert_eq!(time_of(spaced), Time::new(9, 5, 0, 0).ok(), "{spaced}");
        }
        assert_eq!(time_of("  1:2"), Time::new(1, 2, 0, 0).ok());
        for bad in [
            "9:05",
            "09:5",
            "0905",
            "24:00",
            "09:60",
            "+9:05",
            "-9:05",
            "9:05 ",
            "9 :05",
            "09:05:00",
            "٠٩:٠٥",
        ] {
            assert_eq!(time_of(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_bad_calendar_says_what_is_wrong() {
        let bad = |zone: &str, week: &Week, holidays: &[Date]| {
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
        let days: Vec<Date> = (0..=i64::try_from(MAX_HOLIDAYS).unwrap())
            .map(|i| jiff::civil::date(2026, 1, 1) + SignedDuration::from_hours(24 * i))
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
        let days: Vec<Date> = (0..i64::try_from(MAX_HOLIDAYS).unwrap())
            .map(|i| jiff::civil::date(2026, 1, 1) + SignedDuration::from_hours(24 * i))
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
