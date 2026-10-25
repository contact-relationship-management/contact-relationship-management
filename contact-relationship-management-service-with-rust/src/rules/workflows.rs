//! Workflow rules (CRM-R40): the triggers, the conditions, the closed set of actions and the loop
//! guard. Pure and DB-free; [`crate::workflows`] runs them.
//!
//! A workflow never holds code or a URL. It is a trigger, conditions written in the segment filter
//! language, and a short list of actions from a closed set, so what it will do can be read, dry-run
//! and explained.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::segment::Filter;
use super::tokens;

/// The record changes and the clock that can start a workflow.
pub const TRIGGERS: &[&str] = &[
    "contact_created",
    "contact_updated",
    "deal_stage_changed",
    "schedule",
    "lead_created",
];
/// How `assign_owner` chooses among its workers.
pub const ASSIGN_STRATEGIES: &[&str] = &["round_robin", "least_open"];
/// Most workers in one `assign_owner` pool.
pub const MAX_POOL: usize = 20;
/// The contact fields `set_field` may change.
pub const SET_FIELDS: &[&str] = &["job_title", "owner_ref", "preferred_channel", "status"];
/// Most actions in one workflow.
pub const MAX_ACTIONS: usize = 5;
/// Longest name, task summary or event name.
pub const MAX_TEXT: usize = 120;
/// Shortest and longest schedule interval, in minutes (five minutes to thirty days).
pub const EVERY_MINUTES: std::ops::RangeInclusive<i32> = 5..=43_200;
/// How many workflows may fire from one another before the chain is cut.
pub const MAX_DEPTH: i32 = 3;
/// Most runs one workflow may make in a minute before it is held back.
pub const MAX_RUNS_PER_MINUTE: i64 = 60;
/// Most contacts one scheduled workflow acts on in one tick (the rest wait for the next).
pub const SCHEDULE_BATCH: usize = 50;
/// Most contacts one scheduled workflow looks at in one tick.
pub const SCHEDULE_SCAN: usize = 500;

/// What a workflow looks at: the segment filter language, with the same clauses. There is no
/// consent clause, because consent is checked inside each action that leads to a send.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowConditions")]
#[serde(deny_unknown_fields)]
pub struct Conditions {
    /// Contact statuses to include (empty = any).
    #[serde(default)]
    pub statuses: Vec<String>,
    /// Preferred channels to include (empty = any).
    #[serde(default)]
    pub channels: Vec<String>,
    /// Account tiers to include (empty = any; a contact with no account matches only when empty).
    #[serde(default)]
    pub account_tiers: Vec<String>,
    /// For a `lead_created` workflow only: the lead sources to include (empty = any).
    #[serde(default)]
    pub sources: Vec<String>,
}

impl Conditions {
    /// The same clauses as a segment filter.
    #[must_use]
    pub fn filter(&self) -> Filter {
        Filter {
            statuses: self.statuses.clone(),
            channels: self.channels.clone(),
            account_tiers: self.account_tiers.clone(),
            tags: Vec::new(),
        }
    }

    /// The conditions in words, for the run log.
    #[must_use]
    pub fn describe(&self) -> String {
        let parts: Vec<String> = [
            ("status", &self.statuses),
            ("channel", &self.channels),
            ("account tier", &self.account_tiers),
            ("lead source", &self.sources),
        ]
        .into_iter()
        .filter(|(_, values)| !values.is_empty())
        .map(|(name, values)| format!("{name} is {}", values.join(" or ")))
        .collect();
        if parts.is_empty() {
            "no conditions".to_string()
        } else {
            parts.join(" and ")
        }
    }
}

/// One thing a workflow does. The set is closed.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowAction")]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    /// Make a task on the contact.
    CreateTask {
        /// What the task says.
        summary: String,
        /// Who it is for (a `worker:` URN).
        #[serde(default)]
        assignee_ref: Option<String>,
        /// Due this many days from now.
        #[serde(default)]
        due_in_days: Option<u32>,
    },
    /// Put a notification in a worker's inbox.
    Notify {
        /// The worker (a `worker:` URN).
        recipient_ref: String,
    },
    /// Set one field of the contact.
    SetField {
        /// One of [`SET_FIELDS`].
        field: String,
        /// The new value; `null` clears `job_title` or `owner_ref`.
        #[serde(default)]
        value: Option<String>,
    },
    /// Enrol the contact in a nurture sequence, if they have consented to its purpose right now.
    EnrolNurture {
        /// The sequence.
        sequence_pid: Uuid,
    },
    /// Emit a `workflow.<event>` event to the webhook subscriptions that ask for it.
    SendWebhook {
        /// A short name: lower-case letters, digits and underscores.
        event: String,
    },
    /// Give a new lead an owner from a pool of workers (`lead_created` only).
    AssignOwner {
        /// The workers to choose among (`worker:` URNs).
        pool: Vec<String>,
        /// One of [`ASSIGN_STRATEGIES`]: `round_robin` takes the pool in turn, `least_open` the
        /// worker with the fewest open leads and deals.
        #[serde(default = "default_strategy")]
        strategy: String,
        /// Also put a notification in the chosen worker's inbox.
        #[serde(default)]
        notify: bool,
    },
}

fn default_strategy() -> String {
    "round_robin".to_string()
}

impl Action {
    /// What the action does, in words, for the dry run and the run log. It holds no contact data.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::CreateTask { summary, .. } => format!("create a task: {summary}"),
            Self::Notify { recipient_ref } => format!("notify {recipient_ref}"),
            Self::SetField { field, value } => match value {
                Some(v) => format!("set {field} to {v}"),
                None => format!("clear {field}"),
            },
            Self::EnrolNurture { sequence_pid } => {
                format!("enrol in nurture sequence {sequence_pid}")
            }
            Self::SendWebhook { event } => format!("send the webhook event workflow.{event}"),
            Self::AssignOwner { pool, strategy, .. } => {
                format!(
                    "assign the lead to one of {} workers ({strategy})",
                    pool.len()
                )
            }
        }
    }

    /// Whether the action can lead to mail being sent, so that consent must be re-checked inside it.
    #[must_use]
    pub fn leads_to_send(&self) -> bool {
        matches!(self, Self::EnrolNurture { .. })
    }
}

/// Whether `event` is a name a webhook pattern can match.
fn is_event_name(event: &str) -> bool {
    !event.is_empty()
        && event.len() <= 40
        && event
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// What would stop this workflow being saved, as `(field, message)` pairs. References (`worker:`
/// URNs, the sequence) are checked by the caller, which can reach the database.
#[must_use]
pub fn check(
    trigger: &str,
    every_minutes: Option<i32>,
    conditions: &Conditions,
    actions: &[Action],
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut add = |field: &str, message: String| out.push((field.to_string(), message));
    if !tokens::is_token(TRIGGERS, trigger) {
        add("trigger", format!("must be one of {}", TRIGGERS.join(", ")));
    }
    match (trigger == "schedule", every_minutes) {
        (true, None) => add("every_minutes", "is required for a schedule".to_string()),
        (true, Some(n)) if !EVERY_MINUTES.contains(&n) => add(
            "every_minutes",
            format!(
                "must be between {} and {}",
                EVERY_MINUTES.start(),
                EVERY_MINUTES.end()
            ),
        ),
        (false, Some(_)) => add("every_minutes", "is only for a schedule".to_string()),
        _ => {}
    }
    let for_leads = trigger == "lead_created";
    for (field, set, values) in [
        (
            "conditions.statuses",
            tokens::CONTACT_STATUSES,
            &conditions.statuses,
        ),
        (
            "conditions.channels",
            tokens::CHANNELS,
            &conditions.channels,
        ),
        (
            "conditions.account_tiers",
            tokens::ACCOUNT_TIERS,
            &conditions.account_tiers,
        ),
        (
            "conditions.sources",
            tokens::LEAD_SOURCES,
            &conditions.sources,
        ),
    ] {
        if let Some(bad) = values.iter().find(|v| !tokens::is_token(set, v)) {
            add(field, format!("{bad} is not one of {}", set.join(", ")));
        }
        // A lead is judged by its source; a contact by status, channel and tier. A clause that
        // cannot apply to the trigger is refused, so it cannot be silently ignored.
        let lead_clause = field == "conditions.sources";
        if !values.is_empty() && lead_clause != for_leads {
            add(
                field,
                if lead_clause {
                    "only a lead_created workflow can look at the lead source".to_string()
                } else {
                    "a lead_created workflow can only look at the lead source".to_string()
                },
            );
        }
    }
    if actions.is_empty() || actions.len() > MAX_ACTIONS {
        add("actions", format!("needs one to {MAX_ACTIONS} actions"));
    }
    for (i, action) in actions.iter().enumerate() {
        for message in action_problems(action, for_leads) {
            out.push((format!("actions[{i}]"), message));
        }
    }
    out
}

/// What is wrong with one action, for a workflow whose trigger is about leads (`for_leads`) or not.
fn action_problems(action: &Action, for_leads: bool) -> Vec<String> {
    let mut out = Vec::new();
    match action {
        Action::CreateTask { summary, .. } => {
            if summary.trim().is_empty() || summary.chars().count() > MAX_TEXT {
                out.push(format!("summary must be 1 to {MAX_TEXT} characters"));
            }
        }
        Action::SetField { .. } | Action::EnrolNurture { .. } if for_leads => {
            out.push("this action is about a contact, not a lead".to_string());
        }
        Action::SetField { field, value } => {
            out.extend(set_field_problem(field, value.as_deref()));
        }
        Action::AssignOwner { pool, strategy, .. } => {
            if !for_leads {
                out.push("assign_owner is only for a lead_created workflow".to_string());
            }
            if pool.is_empty() || pool.len() > MAX_POOL {
                out.push(format!("pool needs 1 to {MAX_POOL} workers"));
            }
            let mut seen = std::collections::BTreeSet::new();
            if pool.iter().any(|w| !seen.insert(w.as_str())) {
                out.push("pool names a worker twice".to_string());
            }
            if !tokens::is_token(ASSIGN_STRATEGIES, strategy) {
                out.push(format!(
                    "strategy must be one of {}",
                    ASSIGN_STRATEGIES.join(", ")
                ));
            }
        }
        Action::SendWebhook { event } if !is_event_name(event) => {
            out.push("event must be 1 to 40 lower-case letters, digits or underscores".to_string());
        }
        _ => {}
    }
    out
}

fn set_field_problem(field: &str, value: Option<&str>) -> Option<String> {
    if !tokens::is_token(SET_FIELDS, field) {
        return Some(format!("field must be one of {}", SET_FIELDS.join(", ")));
    }
    let set = match field {
        "preferred_channel" => Some(tokens::CHANNELS),
        "status" => Some(tokens::CONTACT_STATUSES),
        _ => None,
    };
    match (set, value) {
        (Some(_), None) => Some(format!("{field} cannot be cleared")),
        (Some(set), Some(v)) if !tokens::is_token(set, v) => {
            Some(format!("{v} is not one of {}", set.join(", ")))
        }
        (None, Some(v)) if v.chars().count() > MAX_TEXT => {
            Some(format!("value is longer than {MAX_TEXT} characters"))
        }
        _ => None,
    }
}

/// The index in the pool of the worker to give the next lead.
///
/// `round_robin` takes the pool in turn: `cursor` is how many leads this workflow has assigned
/// before. `least_open` takes the worker with the fewest open leads and deals (`loads`, in pool
/// order), the first of those on a tie. An empty pool has no answer.
#[must_use]
pub fn pick(strategy: &str, pool_len: usize, cursor: i64, loads: &[i64]) -> Option<usize> {
    if pool_len == 0 {
        return None;
    }
    if strategy == "least_open" {
        return loads
            .iter()
            .take(pool_len)
            .enumerate()
            .min_by_key(|(i, load)| (**load, *i))
            .map(|(i, _)| i);
    }
    let len = i64::try_from(pool_len).ok()?;
    usize::try_from(cursor.rem_euclid(len)).ok()
}

/// Whether the loop guard holds a run back: the chain is too deep, or the workflow has already run
/// too often in the last minute. `None` lets it run; otherwise the reason, for the run log.
#[must_use]
pub fn guard(depth: i32, runs_in_last_minute: i64) -> Option<&'static str> {
    if depth >= MAX_DEPTH {
        Some("loop guard: workflows fired from one another too many times in a row")
    } else if runs_in_last_minute >= MAX_RUNS_PER_MINUTE {
        Some("loop guard: this workflow has run too often in the last minute")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(summary: &str) -> Action {
        Action::CreateTask {
            summary: summary.to_string(),
            assignee_ref: None,
            due_in_days: None,
        }
    }

    #[test]
    fn a_good_workflow_has_no_problems() {
        let found = check(
            "contact_created",
            None,
            &Conditions::default(),
            &[task("Say hello")],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn the_trigger_and_the_schedule_must_agree() {
        let c = Conditions::default();
        let a = [task("x")];
        assert!(!check("nonsense", None, &c, &a).is_empty());
        assert!(!check("schedule", None, &c, &a).is_empty());
        assert!(!check("schedule", Some(1), &c, &a).is_empty());
        assert!(check("schedule", Some(60), &c, &a).is_empty());
        assert!(!check("contact_updated", Some(60), &c, &a).is_empty());
    }

    #[test]
    fn conditions_use_the_closed_vocabularies() {
        let c = Conditions {
            statuses: vec!["active".into()],
            channels: vec!["pigeon".into()],
            ..Conditions::default()
        };
        let found = check("contact_created", None, &c, &[task("x")]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "conditions.channels");
    }

    #[test]
    fn actions_are_counted_and_checked() {
        let c = Conditions::default();
        assert!(!check("contact_created", None, &c, &[]).is_empty());
        let many: Vec<Action> = (0..=MAX_ACTIONS).map(|_| task("x")).collect();
        assert!(!check("contact_created", None, &c, &many).is_empty());
        assert!(!check("contact_created", None, &c, &[task("  ")]).is_empty());
        let bad_field = Action::SetField {
            field: "display_name".into(),
            value: Some("x".into()),
        };
        assert!(!check("contact_created", None, &c, &[bad_field]).is_empty());
        let bad_channel = Action::SetField {
            field: "preferred_channel".into(),
            value: Some("pigeon".into()),
        };
        assert!(!check("contact_created", None, &c, &[bad_channel]).is_empty());
        let clear_status = Action::SetField {
            field: "status".into(),
            value: None,
        };
        assert!(!check("contact_created", None, &c, &[clear_status]).is_empty());
        let clear_title = Action::SetField {
            field: "job_title".into(),
            value: None,
        };
        assert!(check("contact_created", None, &c, &[clear_title]).is_empty());
        let hook = Action::SendWebhook {
            event: "Has Spaces".into(),
        };
        assert!(!check("contact_created", None, &c, &[hook]).is_empty());
    }

    #[test]
    fn an_action_outside_the_closed_set_does_not_parse() {
        let unknown: Result<Action, _> =
            serde_json::from_str(r#"{"type":"http_request","url":"https://example.test"}"#);
        assert!(unknown.is_err());
        let extra: Result<Action, _> =
            serde_json::from_str(r#"{"type":"notify","recipient_ref":"x","url":"y"}"#);
        assert!(extra.is_err());
        let ok: Action =
            serde_json::from_str(r#"{"type":"send_webhook","event":"hot_lead"}"#).unwrap();
        assert_eq!(ok.describe(), "send the webhook event workflow.hot_lead");
    }

    #[test]
    fn conditions_reject_an_unknown_clause() {
        let typo: Result<Conditions, _> = serde_json::from_str(r#"{"account_tier":["customer"]}"#);
        assert!(typo.is_err());
    }

    fn assign(pool: &[&str], strategy: &str) -> Action {
        Action::AssignOwner {
            pool: pool.iter().map(ToString::to_string).collect(),
            strategy: strategy.to_string(),
            notify: false,
        }
    }

    #[test]
    fn a_lead_workflow_looks_at_the_source_and_acts_on_the_lead() {
        let c = Conditions {
            sources: vec!["web".into()],
            ..Conditions::default()
        };
        let a = [assign(&["worker:a", "worker:b"], "round_robin")];
        assert!(check("lead_created", None, &c, &a).is_empty());
        // The source only makes sense for a lead, and the contact clauses only for a contact.
        assert!(!check("contact_created", None, &c, &[task("x")]).is_empty());
        let contact_clause = Conditions {
            statuses: vec!["active".into()],
            ..Conditions::default()
        };
        assert!(!check("lead_created", None, &contact_clause, &a).is_empty());
        let bad_source = Conditions {
            sources: vec!["carrier_pigeon".into()],
            ..Conditions::default()
        };
        assert!(!check("lead_created", None, &bad_source, &a).is_empty());
        // assign_owner is for leads only; set_field and enrol_nurture are for contacts only.
        assert!(!check("contact_created", None, &Conditions::default(), &a).is_empty());
        let set = Action::SetField {
            field: "job_title".into(),
            value: Some("x".into()),
        };
        assert!(!check("lead_created", None, &Conditions::default(), &[set]).is_empty());
        let enrol = Action::EnrolNurture {
            sequence_pid: Uuid::nil(),
        };
        assert!(!check("lead_created", None, &Conditions::default(), &[enrol]).is_empty());
        assert!(
            check(
                "lead_created",
                None,
                &Conditions::default(),
                &[task("Call")]
            )
            .is_empty()
        );
    }

    #[test]
    fn the_pool_and_the_strategy_are_checked() {
        let none = Conditions::default();
        assert!(!check("lead_created", None, &none, &[assign(&[], "round_robin")]).is_empty());
        assert!(
            !check(
                "lead_created",
                None,
                &none,
                &[assign(&["worker:a", "worker:a"], "round_robin")]
            )
            .is_empty()
        );
        assert!(
            !check(
                "lead_created",
                None,
                &none,
                &[assign(&["worker:a"], "random")]
            )
            .is_empty()
        );
        let big: Vec<String> = (0..=MAX_POOL).map(|i| format!("worker:{i}")).collect();
        let refs: Vec<&str> = big.iter().map(String::as_str).collect();
        assert!(!check("lead_created", None, &none, &[assign(&refs, "least_open")]).is_empty());
    }

    #[test]
    fn the_next_owner_is_picked_in_turn_or_by_load() {
        assert_eq!(pick("round_robin", 3, 0, &[]), Some(0));
        assert_eq!(pick("round_robin", 3, 4, &[]), Some(1));
        assert_eq!(pick("round_robin", 3, 5, &[]), Some(2));
        assert_eq!(pick("round_robin", 0, 5, &[]), None);
        assert_eq!(pick("least_open", 3, 99, &[4, 1, 7]), Some(1));
        assert_eq!(
            pick("least_open", 3, 0, &[2, 2, 9]),
            Some(0),
            "the first on a tie"
        );
        assert_eq!(
            pick("least_open", 2, 0, &[5, 5, 0]),
            Some(0),
            "only the pool counts"
        );
    }

    #[test]
    fn only_enrolment_leads_to_a_send() {
        assert!(
            Action::EnrolNurture {
                sequence_pid: Uuid::nil()
            }
            .leads_to_send()
        );
        assert!(!task("x").leads_to_send());
    }

    #[test]
    fn the_loop_guard_cuts_depth_and_rate() {
        assert_eq!(guard(0, 0), None);
        assert_eq!(guard(MAX_DEPTH - 1, MAX_RUNS_PER_MINUTE - 1), None);
        assert!(guard(MAX_DEPTH, 0).is_some());
        assert!(guard(0, MAX_RUNS_PER_MINUTE).is_some());
    }

    #[test]
    fn conditions_read_in_words() {
        assert_eq!(Conditions::default().describe(), "no conditions");
        let c = Conditions {
            statuses: vec!["active".into()],
            channels: vec!["email".into(), "phone".into()],
            ..Conditions::default()
        };
        assert_eq!(
            c.describe(),
            "status is active and channel is email or phone"
        );
    }
}
