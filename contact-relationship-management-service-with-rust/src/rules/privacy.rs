//! Pure rules for subject rights & retention (CRM-R20 / CRM-D14):
//! when erasure is allowed, how the retention horizon is read, and
//! which tables the sweep covers. No I/O.

/// A contact is erasable when it holds no **live commercial
/// engagement**: no open deal naming it primary contact, no open
/// support ticket, and no active nurture enrolment.
///
/// This deliberately does **not** gate on `Contact::status`
/// (`active`/`inactive`), unlike WPM's analogous `erasable(status)`
/// over the employee's employment status. WPM's `status` genuinely
/// transitions through an offboarding lifecycle and is the real
/// lawful-basis signal; CRM's `Contact::status` is set once at
/// creation (always `"active"`) and no endpoint in this crate ever
/// transitions it, so gating on it would refuse erasure forever. The
/// signals below — an open deal, an open ticket, an active nurture
/// enrolment — are the ones that actually change over a contact's
/// lifetime, so they are the real lawful-basis check: each represents
/// live business activity that still needs the contact's identity.
#[must_use]
pub fn erasable(has_open_deal: bool, has_open_ticket: bool, has_active_nurture: bool) -> bool {
    !has_open_deal && !has_open_ticket && !has_active_nurture
}

/// Default retention horizon (days) when `CRM_RETENTION_DAYS` is unset.
pub const RETENTION_DEFAULT_DAYS: i64 = 365;

/// Horizon floor: a sweep that could run at 0 days would silently turn
/// every soft-delete into a hard-delete (CRM-D14, mirroring WPM-D22).
pub const RETENTION_FLOOR_DAYS: i64 = 30;

/// Parse the retention horizon from the raw env value: unset / blank /
/// junk ⇒ the default; anything below the floor is clamped up to it.
#[must_use]
pub fn retention_days(raw: Option<&str>) -> i64 {
    raw.and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(RETENTION_DEFAULT_DAYS)
        .max(RETENTION_FLOOR_DAYS)
}

/// Default horizon (days) for the event outbox and webhook deliveries, when `CRM_EVENT_RETENTION_DAYS`
/// is unset (CRM-R81). They are working records of delivery, not evidence.
pub const EVENT_DEFAULT_DAYS: i64 = 30;
/// Floor for it: a week, so a receiver that was down for a few days can still be retried and
/// investigated before the record of the attempt is gone.
pub const EVENT_FLOOR_DAYS: i64 = 7;

/// Default horizon (days) for email deliveries and events, when `CRM_MAIL_RETENTION_DAYS` is unset.
pub const MAIL_DEFAULT_DAYS: i64 = 730;
/// Floor for it. An open or click is a touch for attribution, which looks back at most 365 days
/// from a deal's close, so events must outlive that window with room to spare.
pub const MAIL_FLOOR_DAYS: i64 = 400;

fn horizon(raw: Option<&str>, default: i64, floor: i64) -> i64 {
    raw.and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(default)
        .max(floor)
}

/// The horizon for the event outbox and webhook deliveries: unset, blank or junk is the default,
/// and anything below the floor is raised to it.
#[must_use]
pub fn event_retention_days(raw: Option<&str>) -> i64 {
    horizon(raw, EVENT_DEFAULT_DAYS, EVENT_FLOOR_DAYS)
}

/// The horizon for email deliveries and events, read the same way.
#[must_use]
pub fn mail_retention_days(raw: Option<&str>) -> i64 {
    horizon(raw, MAIL_DEFAULT_DAYS, MAIL_FLOOR_DAYS)
}

/// Which of the two horizons a log table is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Horizon {
    /// `CRM_EVENT_RETENTION_DAYS`.
    Event,
    /// `CRM_MAIL_RETENTION_DAYS`.
    Mail,
}

/// A delivery or event table the sweep prunes, and what makes a row **finished with** (CRM-R81).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRule {
    /// The table.
    pub table: &'static str,
    /// Which horizon applies.
    pub horizon: Horizon,
    /// The rows that may go once they are older than the horizon; `{days}` is the horizon. A row
    /// that something may still act on (a pending delivery, an unpublished event, a delivery of a
    /// running campaign) is never matched.
    pub condition: String,
}

/// The tables pruned by age rather than by a soft-delete: delivery and event logs. `max_attempts`
/// is how many sends a failed delivery gets, after which it is final.
#[must_use]
pub fn log_rules(max_attempts: i32) -> Vec<LogRule> {
    vec![
        LogRule {
            table: "email_deliveries",
            horizon: Horizon::Mail,
            // Sent, or failed for good, a long time ago, and of a campaign that cannot run again
            // (or of no campaign: a nurture step). A running campaign may still send or retry.
            condition: format!(
                "(state = 'sent' OR (state = 'failed' AND attempts >= {max_attempts})) \
                 AND COALESCE(sent_at, updated_at) < now() - interval '{{days}} days' \
                 AND NOT EXISTS (SELECT 1 FROM campaigns c WHERE c.pid = email_deliveries.campaign_pid \
                                 AND c.status NOT IN ('completed', 'cancelled'))"
            ),
        },
        LogRule {
            table: "email_events",
            horizon: Horizon::Mail,
            condition: "received_at < now() - interval '{days} days'".to_string(),
        },
        LogRule {
            table: "event_outbox",
            horizon: Horizon::Event,
            // Only an event that has been published and handed to the webhooks: an unpublished one
            // is still owed to a subscriber.
            condition: "published_at IS NOT NULL AND webhook_fanned_out_at IS NOT NULL \
                        AND published_at < now() - interval '{days} days'"
                .to_string(),
        },
        LogRule {
            table: "webhook_deliveries",
            horizon: Horizon::Event,
            condition: "status <> 'pending' AND updated_at < now() - interval '{days} days'"
                .to_string(),
        },
        // The ids of the worker service's events (CRM-R92): needed only while WPM may still retry
        // one, which it stops doing within two days, and the floor is seven.
        LogRule {
            table: "wpm_events",
            horizon: Horizon::Event,
            condition: "received_at < now() - interval '{days} days'".to_string(),
        },
    ]
}

/// Every table with a `deleted_at` column, for the retention sweep.
/// Kept in one place so a new soft-deleting table is added here (the
/// sweep test counts this list against the entity modules). Unlike
/// WPM's 41-table list this is **not** scoped to one subject — it is
/// every soft-deleting table in the CRM schema, including account-only
/// tables (`memberships`, `partnerships`) that carry no `contact_pid`
/// at all. `consent_events` and `working_group_members` are excluded
/// on purpose: the former is append-only compliance evidence with no
/// `deleted_at` column (CRM-D6), the latter is a plain roster join row
/// with no soft-delete of its own.
pub const SOFT_DELETED_TABLES: &[&str] = &[
    "accounts",
    "activities",
    "articles",
    "campaigns",
    "contacts",
    "deals",
    "forecast_snapshots",
    "leads",
    "memberships",
    "nurture_enrollments",
    "nurture_sequences",
    "nurture_steps",
    "partnerships",
    "pipeline_stages",
    "pipelines",
    "segments",
    "sla_policies",
    "tickets",
    "working_groups",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Erasure requires no live engagement in any of the three forms;
    /// any one of them refuses it.
    #[test]
    fn erasure_requires_no_live_engagement() {
        assert!(erasable(false, false, false));
        assert!(!erasable(true, false, false), "open deal blocks erasure");
        assert!(!erasable(false, true, false), "open ticket blocks erasure");
        assert!(
            !erasable(false, false, true),
            "active nurture enrolment blocks erasure"
        );
        assert!(!erasable(true, true, true));
    }

    /// The horizon: default on unset/junk, floor-clamped, and a sane
    /// value passes through.
    #[test]
    fn retention_horizon_defaults_and_floors() {
        assert_eq!(retention_days(None), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("")), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("junk")), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("730")), 730);
        assert_eq!(
            retention_days(Some("0")),
            RETENTION_FLOOR_DAYS,
            "0 would hard-delete"
        );
        assert_eq!(retention_days(Some("-5")), RETENTION_FLOOR_DAYS);
        assert_eq!(retention_days(Some("30")), 30);
    }

    /// The sweep list is sorted and duplicate-free (each table swept
    /// exactly once), and covers the known soft-deleting tables —
    /// while excluding the two tables that structurally cannot be
    /// swept this way.
    #[test]
    fn sweep_table_list_is_sound() {
        let mut sorted = SOFT_DELETED_TABLES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, SOFT_DELETED_TABLES, "sorted and unique");
        assert_eq!(SOFT_DELETED_TABLES.len(), 19);
        for table in ["contacts", "accounts", "deals", "tickets", "leads"] {
            assert!(SOFT_DELETED_TABLES.contains(&table));
        }
        for table in ["consent_events", "working_group_members"] {
            assert!(
                !SOFT_DELETED_TABLES.contains(&table),
                "{table} has no deleted_at column"
            );
        }
    }

    #[test]
    fn the_log_horizons_default_and_never_go_below_their_floors() {
        assert_eq!(event_retention_days(None), EVENT_DEFAULT_DAYS);
        assert_eq!(event_retention_days(Some("junk")), EVENT_DEFAULT_DAYS);
        assert_eq!(event_retention_days(Some("90")), 90);
        assert_eq!(event_retention_days(Some("1")), EVENT_FLOOR_DAYS);
        assert_eq!(mail_retention_days(None), MAIL_DEFAULT_DAYS);
        assert_eq!(mail_retention_days(Some(" 1000 ")), 1000);
        assert_eq!(mail_retention_days(Some("30")), MAIL_FLOOR_DAYS);
        assert_eq!(mail_retention_days(Some("-1")), MAIL_FLOOR_DAYS);
    }

    #[test]
    fn the_mail_floor_outlives_the_longest_attribution_window() {
        // Attribution looks back at most 365 days from a deal's close.
        // Checked when the code is built, not when the test runs.
        const {
            assert!(MAIL_FLOOR_DAYS > 365);
            assert!(MAIL_DEFAULT_DAYS >= MAIL_FLOOR_DAYS);
            assert!(EVENT_DEFAULT_DAYS >= EVENT_FLOOR_DAYS);
        }
    }

    #[test]
    fn each_log_rule_names_its_table_and_uses_the_horizon_and_never_matches_pending_work() {
        let rules = log_rules(5);
        let tables: Vec<&str> = rules.iter().map(|r| r.table).collect();
        assert_eq!(
            tables,
            [
                "email_deliveries",
                "email_events",
                "event_outbox",
                "webhook_deliveries",
                "wpm_events"
            ]
        );
        for rule in &rules {
            assert!(rule.condition.contains("{days}"), "{}", rule.table);
        }
        let by = |t: &str| {
            rules
                .iter()
                .find(|r| r.table == t)
                .unwrap()
                .condition
                .clone()
        };
        // A delivery that may still be sent, retried or claimed is not matched.
        assert!(by("email_deliveries").contains("state = 'sent'"));
        assert!(by("email_deliveries").contains("attempts >= 5"));
        assert!(by("email_deliveries").contains("NOT IN ('completed', 'cancelled')"));
        assert!(!by("email_deliveries").contains("'pending'"));
        // An event not yet published and fanned out is still owed.
        assert!(by("event_outbox").contains("published_at IS NOT NULL"));
        assert!(by("event_outbox").contains("webhook_fanned_out_at IS NOT NULL"));
        assert!(by("webhook_deliveries").contains("status <> 'pending'"));
        assert_eq!(rules[0].horizon, Horizon::Mail);
        assert_eq!(rules[2].horizon, Horizon::Event);
    }
}
