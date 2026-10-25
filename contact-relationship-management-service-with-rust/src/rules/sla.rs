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
    let round_the_clock = deadlines(opened_at, targets);
    let Some(calendar) = calendar else {
        return round_the_clock;
    };
    Deadlines {
        first_response_due_at: calendar
            .add_working_minutes(opened_at, i64::from(targets.first_response_minutes))
            .unwrap_or(round_the_clock.first_response_due_at),
        resolution_due_at: calendar
            .add_working_minutes(opened_at, i64::from(targets.resolution_minutes))
            .unwrap_or(round_the_clock.resolution_due_at),
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
}
