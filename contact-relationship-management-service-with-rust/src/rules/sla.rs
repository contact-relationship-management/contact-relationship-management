//! SLA derivation (CRM-R11, CRM-D4): deadlines derive once from the
//! open time + the priority's policy (24×7 clocks in v1); breaches
//! are computed facts that clear only by meeting the metric.

use chrono::{DateTime, Duration, FixedOffset};

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
    pub first_response_due_at: DateTime<FixedOffset>,
    /// Resolution due.
    pub resolution_due_at: DateTime<FixedOffset>,
}

/// Derive the deadlines from the ticket's opened time (re-derived on
/// an audited priority change).
#[must_use]
pub fn deadlines(opened_at: DateTime<FixedOffset>, targets: Targets) -> Deadlines {
    Deadlines {
        first_response_due_at: opened_at
            + Duration::minutes(i64::from(targets.first_response_minutes)),
        resolution_due_at: opened_at + Duration::minutes(i64::from(targets.resolution_minutes)),
    }
}

/// Like [`deadlines`], but counted in working time on `calendar` when there is one (CRM-R45): the
/// clock runs only inside the calendar's working windows and not on its holidays. A target the
/// calendar cannot place within its search horizon falls back to round-the-clock, so a ticket
/// never loses its deadline.
#[must_use]
pub fn deadlines_on(
    opened_at: DateTime<FixedOffset>,
    targets: Targets,
    calendar: Option<&Calendar>,
) -> Deadlines {
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
    opened_at: DateTime<FixedOffset>,
    targets: Targets,
    calendar: Option<&Calendar>,
    paused: Paused,
) -> Deadlines {
    let place = |minutes: i32, extra: i64| {
        let seconds = i64::from(minutes) * 60 + extra.max(0);
        let round_the_clock = opened_at + Duration::seconds(seconds);
        calendar
            .and_then(|c| c.add_working_seconds(opened_at, seconds))
            .unwrap_or(round_the_clock)
    };
    Deadlines {
        first_response_due_at: place(
            targets.first_response_minutes,
            paused.first_response_seconds,
        ),
        resolution_due_at: place(targets.resolution_minutes, paused.resolution_seconds),
    }
}

/// The moment the SLA clock reads: `now`, or the moment the ticket was paused while it waits for
/// the customer (the clock is stopped, so a paused ticket does not run into a deadline).
#[must_use]
pub fn clock_now(
    now: DateTime<FixedOffset>,
    paused_at: Option<DateTime<FixedOffset>>,
) -> DateTime<FixedOffset> {
    paused_at.map_or(now, |paused| paused.min(now))
}

/// How much of a pause from `paused_at` to `resumed_at` counts against one deadline: none when the
/// metric is already met or has no deadline, and a pause only counts up to the deadline, so waiting
/// after a breach never undoes it. Working time on a calendar, elapsed time without one.
#[must_use]
pub fn pause_counted(
    calendar: Option<&Calendar>,
    paused_at: DateTime<FixedOffset>,
    resumed_at: DateTime<FixedOffset>,
    due: Option<DateTime<FixedOffset>>,
    met: bool,
) -> i64 {
    let Some(due) = due.filter(|_| !met) else {
        return 0;
    };
    let end = resumed_at.min(due);
    if end <= paused_at {
        return 0;
    }
    match calendar {
        Some(calendar) => calendar.working_seconds_between(paused_at, end),
        None => (end - paused_at).num_seconds(),
    }
}

/// Whether the first-response metric is breached at `now`: due has
/// passed and no response is on record. A recorded response before
/// the deadline can never breach later.
#[must_use]
pub fn first_response_breached(
    now: DateTime<FixedOffset>,
    due: DateTime<FixedOffset>,
    first_responded_at: Option<DateTime<FixedOffset>>,
) -> bool {
    match first_responded_at {
        Some(responded) => responded > due,
        None => now > due,
    }
}

/// Whether the resolution metric is breached at `now`.
#[must_use]
pub fn resolution_breached(
    now: DateTime<FixedOffset>,
    due: DateTime<FixedOffset>,
    resolved_at: Option<DateTime<FixedOffset>>,
) -> bool {
    match resolved_at {
        Some(resolved) => resolved > due,
        None => now > due,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 6, h, m, 0)
            .unwrap()
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
        assert_eq!(d.resolution_due_at, at(17, 0) + Duration::days(1));
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
        let due = Some(at(9, 0) + Duration::days(7));
        let resumed = at(10, 0) + Duration::days(1);
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
        assert_eq!(d.first_response_due_at, at(10, 0) + Duration::days(1));
    }
}
