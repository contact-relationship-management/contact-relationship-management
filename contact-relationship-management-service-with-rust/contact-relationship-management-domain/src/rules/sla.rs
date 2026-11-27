//! SLA derivation (CRM-R11, CRM-D4): deadlines derive once from the
//! open time + the priority's policy (24×7 clocks in v1); breaches
//! are computed facts that clear only by meeting the metric.

use jiff::SignedDuration;

use crate::time::Timestamp;

use super::calendar::Calendar;

/// One priority's SLA targets, minutes.
#[derive(Debug, Clone, Copy)]
pub struct Targets {
    /// First-response target.
    pub first_response_minutes: i32,
    /// Resolution target.
    pub resolution_minutes: i32,
}

/// The derived deadlines for a ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deadlines {
    /// First response due.
    pub first_response_due_at: Timestamp,
    /// Resolution due.
    pub resolution_due_at: Timestamp,
}

/// Derive the deadlines from the ticket's opened time (re-derived on
/// an audited priority change).
#[must_use]
///
/// `None` when a deadline falls outside the range of dates.
pub fn deadlines(opened_at: Timestamp, targets: Targets) -> Option<Deadlines> {
    Some(Deadlines {
        first_response_due_at: opened_at.checked_add(SignedDuration::from_mins(i64::from(
            targets.first_response_minutes,
        )))?,
        resolution_due_at: opened_at.checked_add(SignedDuration::from_mins(i64::from(
            targets.resolution_minutes,
        )))?,
    })
}

/// Like [`deadlines`], but counted in working time on `calendar` when there is one (CRM-R45): the
/// clock runs only inside the calendar's working windows and not on its holidays. A target the
/// calendar cannot place within its search horizon falls back to round-the-clock, so a ticket
/// never loses its deadline.
#[must_use]
pub fn deadlines_on(
    opened_at: Timestamp,
    targets: Targets,
    calendar: Option<&Calendar>,
) -> Option<Deadlines> {
    deadlines_paused(opened_at, targets, calendar, Paused::default())
}

/// Time a ticket spent waiting for the customer that counted against each deadline, in seconds
/// (in working time on a calendar, in elapsed time without one).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Paused {
    /// Counted against the first-response deadline.
    pub first_response_seconds: i64,
    /// Counted against the resolution deadline.
    pub resolution_seconds: i64,
}

/// Like [`deadlines_on`], with the time spent waiting for the customer added to each target, so a
/// deadline can always be derived again from the open time (CRM-R55).
#[must_use]
pub fn deadlines_paused(
    opened_at: Timestamp,
    targets: Targets,
    calendar: Option<&Calendar>,
    paused: Paused,
) -> Option<Deadlines> {
    let place = |minutes: i32, extra: i64| -> Option<Timestamp> {
        let seconds = i64::from(minutes)
            .checked_mul(60)?
            .checked_add(extra.max(0))?;
        let round_the_clock = opened_at.checked_add(SignedDuration::from_secs(seconds))?;
        Some(
            calendar
                .and_then(|c| c.add_working_seconds(opened_at.0, seconds))
                .map_or(round_the_clock, Timestamp),
        )
    };
    Some(Deadlines {
        first_response_due_at: place(
            targets.first_response_minutes,
            paused.first_response_seconds,
        )?,
        resolution_due_at: place(targets.resolution_minutes, paused.resolution_seconds)?,
    })
}

/// The moment the SLA clock reads: `now`, or the moment the ticket was paused while it waits for
/// the customer (the clock is stopped, so a paused ticket does not run into a deadline).
#[must_use]
pub fn clock_now(now: Timestamp, paused_at: Option<Timestamp>) -> Timestamp {
    paused_at.map_or(now, |paused| paused.min(now))
}

/// How much of a pause from `paused_at` to `resumed_at` counts against one deadline: none when the
/// metric is already met or has no deadline, and a pause only counts up to the deadline, so waiting
/// after a breach never undoes it. Working time on a calendar, elapsed time without one.
#[must_use]
pub fn pause_counted(
    calendar: Option<&Calendar>,
    paused_at: Timestamp,
    resumed_at: Timestamp,
    due: Option<Timestamp>,
    met: bool,
) -> Option<i64> {
    let Some(due) = due.filter(|_| !met) else {
        return Some(0);
    };
    let end = resumed_at.min(due);
    if end <= paused_at {
        return Some(0);
    }
    match calendar {
        Some(calendar) => calendar.working_seconds_between(paused_at.0, end.0),
        None => Some(end.duration_since(paused_at).as_secs()),
    }
}

/// Whether the first-response metric is breached at `now`: due has
/// passed and no response is on record. A recorded response before
/// the deadline can never breach later.
#[must_use]
pub fn first_response_breached(
    now: Timestamp,
    due: Timestamp,
    first_responded_at: Option<Timestamp>,
) -> bool {
    match first_responded_at {
        Some(responded) => responded > due,
        None => now > due,
    }
}

/// Whether the resolution metric is breached at `now`.
#[must_use]
pub fn resolution_breached(now: Timestamp, due: Timestamp, resolved_at: Option<Timestamp>) -> bool {
    match resolved_at {
        Some(resolved) => resolved > due,
        None => now > due,
    }
}

/// What became of one deadline, as attainment counts it (CRM-T315).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Done on or before the deadline.
    Met,
    /// Done after it, or not done and the clock is past it.
    Missed,
    /// Not done and the clock has not reached it (a waiting ticket's clock is stopped).
    Undecided,
}

/// What became of a deadline `due` met by `done_at`, read at the SLA clock `clock` (see
/// [`clock_now`]). Agrees with [`first_response_breached`] and [`resolution_breached`]: `Missed` is
/// exactly what they call breached.
#[must_use]
pub fn outcome(clock: Timestamp, due: Timestamp, done_at: Option<Timestamp>) -> Outcome {
    match done_at {
        Some(done) if done <= due => Outcome::Met,
        Some(_) => Outcome::Missed,
        None if clock > due => Outcome::Missed,
        None => Outcome::Undecided,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Met on the deadline itself, missed a moment after, and undecided while the clock is short of
    /// it; `Missed` is what the breach rules call breached.
    #[test]
    fn a_deadline_is_met_missed_or_not_yet_decided() {
        let due: Timestamp = "2026-11-01T12:00:00Z".parse().unwrap();
        let before: Timestamp = "2026-11-01T11:00:00Z".parse().unwrap();
        let after: Timestamp = "2026-11-01T13:00:00Z".parse().unwrap();
        assert_eq!(outcome(after, due, Some(due)), Outcome::Met);
        assert_eq!(outcome(after, due, Some(before)), Outcome::Met);
        assert_eq!(outcome(after, due, Some(after)), Outcome::Missed);
        assert_eq!(outcome(after, due, None), Outcome::Missed);
        assert_eq!(outcome(before, due, None), Outcome::Undecided);
        assert_eq!(outcome(due, due, None), Outcome::Undecided);
        for (clock, done) in [
            (after, Some(due)),
            (after, Some(after)),
            (after, None),
            (before, None),
        ] {
            assert_eq!(
                outcome(clock, due, done) == Outcome::Missed,
                first_response_breached(clock, due, done),
                "{clock} {done:?}"
            );
            assert_eq!(
                outcome(clock, due, done) == Outcome::Missed,
                resolution_breached(clock, due, done),
            );
        }
    }

    // The rules answer `None` on overflow; most tests are about the figures.
    fn deadlines(opened_at: Timestamp, targets: Targets) -> Deadlines {
        super::deadlines(opened_at, targets).unwrap()
    }
    fn deadlines_on(
        opened_at: Timestamp,
        targets: Targets,
        calendar: Option<&Calendar>,
    ) -> Deadlines {
        super::deadlines_on(opened_at, targets, calendar).unwrap()
    }
    fn deadlines_paused(
        opened_at: Timestamp,
        targets: Targets,
        calendar: Option<&Calendar>,
        paused: Paused,
    ) -> Deadlines {
        super::deadlines_paused(opened_at, targets, calendar, paused).unwrap()
    }
    fn pause_counted(
        calendar: Option<&Calendar>,
        paused_at: Timestamp,
        resumed_at: Timestamp,
        due: Option<Timestamp>,
        met: bool,
    ) -> i64 {
        super::pause_counted(calendar, paused_at, resumed_at, due, met).unwrap()
    }

    #[test]
    fn a_deadline_that_does_not_fit_is_none() {
        let last = Timestamp(jiff::Timestamp::MAX);
        let targets = Targets {
            first_response_minutes: 1,
            resolution_minutes: 1,
        };
        assert!(super::deadlines(last, targets).is_none());
        assert!(super::deadlines_on(last, targets, None).is_none());
        let huge = Paused {
            first_response_seconds: i64::MAX,
            resolution_seconds: 0,
        };
        assert!(super::deadlines_paused(at(9, 0), targets, None, huge).is_none());
    }
    use crate::time::test_operators::test_days as days;

    fn at(h: i8, m: i8) -> Timestamp {
        format!("2026-07-06T{h:02}:{m:02}:00Z").parse().unwrap()
    }

    /// Deadlines add the policy minutes to the open time.
    #[test]
    fn deadline_derivation() {
        let d = deadlines(
            at(9, 0),
            Targets {
                first_response_minutes: 60,
                resolution_minutes: 480,
            },
        );
        assert_eq!(d.first_response_due_at, at(10, 0));
        assert_eq!(d.resolution_due_at, at(17, 0));
    }

    /// On a calendar the target runs in working time only.
    #[test]
    fn deadlines_on_a_calendar_count_working_time() {
        use crate::rules::calendar::{Hours, Week};
        let day = vec![Hours {
            start: "09:00".into(),
            end: "17:00".into(),
        }];
        let week = Week {
            mon: day.clone(),
            tue: day,
            ..Week::default()
        };
        let calendar = Calendar::build("UTC", &week, &[]).unwrap();
        let targets = Targets {
            first_response_minutes: 60,
            resolution_minutes: 540,
        };
        // Opened Monday 16:00: one hour left today, the other eight on Tuesday.
        let opened = at(16, 0);
        let d = deadlines_on(opened, targets, Some(&calendar));
        assert_eq!(d.first_response_due_at, at(17, 0));
        assert_eq!(d.resolution_due_at, at(17, 0) + days(1));
        // With no calendar it is the round-the-clock answer.
        assert_eq!(
            deadlines_on(opened, targets, None),
            deadlines(opened, targets)
        );
    }

    /// Breach truth table: on-time response never breaches (even
    /// later); a late response breaches permanently; no response
    /// breaches once now passes due.
    #[test]
    fn breach_matrix() {
        let due = at(10, 0);
        // No response yet.
        assert!(!first_response_breached(at(9, 59), due, None));
        assert!(first_response_breached(at(10, 1), due, None));
        // Responded on time: never a breach, even read later.
        assert!(!first_response_breached(at(23, 0), due, Some(at(9, 30))));
        // Responded late: breach is a fact, not clearable by waiting.
        assert!(first_response_breached(at(23, 0), due, Some(at(11, 0))));
        // Resolution mirrors it.
        assert!(resolution_breached(at(23, 0), due, None));
        assert!(!resolution_breached(at(23, 0), due, Some(at(9, 0))));
    }

    /// The deadline itself is in time: a breach is strictly after it, and `now` no longer
    /// matters once the work is on record.
    #[test]
    fn work_done_exactly_at_the_deadline_is_not_a_breach() {
        let due = at(10, 0);
        let now = at(12, 0);
        for breached in [first_response_breached, resolution_breached] {
            assert!(!breached(now, due, Some(at(9, 59))));
            assert!(!breached(now, due, Some(due)));
            assert!(breached(now, due, Some(at(10, 1))));
            // With nothing on record the clock decides, and the same rule holds.
            assert!(!breached(at(9, 59), due, None));
            assert!(!breached(due, due, None));
            assert!(breached(at(10, 1), due, None));
        }
    }

    fn office() -> Calendar {
        use crate::rules::calendar::{Hours, Week};
        let day = vec![Hours {
            start: "09:00".into(),
            end: "17:00".into(),
        }];
        let week = Week {
            mon: day.clone(),
            tue: day,
            ..Week::default()
        };
        Calendar::build("UTC", &week, &[]).unwrap()
    }

    /// A paused ticket's clock reads the moment it was paused.
    #[test]
    fn a_paused_clock_stands_still() {
        assert_eq!(clock_now(at(12, 0), None), at(12, 0));
        assert_eq!(clock_now(at(12, 0), Some(at(10, 0))), at(10, 0));
        assert_eq!(clock_now(at(12, 0), Some(at(13, 0))), at(12, 0));
        // So a deadline that falls while it waits is not a breach.
        let due = at(11, 0);
        let clock = clock_now(at(15, 0), Some(at(10, 0)));
        assert!(!resolution_breached(clock, due, None));
    }

    /// Waiting counts only until the deadline, and not at all once the metric is met.
    #[test]
    fn a_pause_counts_up_to_the_deadline_only() {
        let due = Some(at(11, 0));
        // Paused 09:00 to 09:30, deadline 11:00: the whole half hour.
        assert_eq!(pause_counted(None, at(9, 0), at(9, 30), due, false), 1800);
        // Paused 10:30 to 13:00: only the half hour before the deadline.
        assert_eq!(pause_counted(None, at(10, 30), at(13, 0), due, false), 1800);
        // Paused after the deadline (already breached): nothing, so it is not undone.
        assert_eq!(pause_counted(None, at(11, 30), at(13, 0), due, false), 0);
        // Metric met, or no deadline: nothing.
        assert_eq!(pause_counted(None, at(9, 0), at(9, 30), due, true), 0);
        assert_eq!(pause_counted(None, at(9, 0), at(9, 30), None, false), 0);
    }

    /// On a calendar only working time spent waiting counts.
    #[test]
    fn on_a_calendar_only_working_time_spent_waiting_counts() {
        let calendar = office();
        // Paused Monday 16:00, resumed Tuesday 10:00; the deadline is far off. One hour on Monday
        // and one on Tuesday: two working hours, not eighteen elapsed ones.
        let due = Some(at(9, 0) + days(7));
        let resumed = at(10, 0) + days(1);
        assert_eq!(
            pause_counted(Some(&calendar), at(16, 0), resumed, due, false),
            2 * 3600
        );
    }

    /// Deriving again with the waiting added moves the deadline by exactly the waiting, and a
    /// priority change that re-derives keeps it.
    #[test]
    fn a_deadline_moves_by_the_time_spent_waiting() {
        let targets = Targets {
            first_response_minutes: 60,
            resolution_minutes: 240,
        };
        let waited = Paused {
            first_response_seconds: 1800,
            resolution_seconds: 3600,
        };
        let d = deadlines_paused(at(9, 0), targets, None, waited);
        assert_eq!(d.first_response_due_at, at(10, 30));
        assert_eq!(d.resolution_due_at, at(14, 0));
        assert_eq!(
            deadlines_paused(at(9, 0), targets, None, Paused::default()),
            deadlines(at(9, 0), targets)
        );
        // On a calendar the extra is working time too: opened Monday 16:00 with a 60 minute target
        // and 60 minutes waited is two working hours, which runs into Tuesday.
        let calendar = office();
        let d = deadlines_paused(
            at(16, 0),
            targets,
            Some(&calendar),
            Paused {
                first_response_seconds: 3600,
                resolution_seconds: 0,
            },
        );
        assert_eq!(d.first_response_due_at, at(10, 0) + days(1));
    }
}
