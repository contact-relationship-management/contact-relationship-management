//! Request-level test suites: the sales journey in [`sales`], the
//! consent-gated marketing journey in [`marketing`], and the support
//! journey + contract pins in [`support`].

mod api_standards;
mod attachments;
mod attribution;
mod audit_chain;
mod board;
mod bridges;
mod business_calendars;
mod consent_purposes;
mod custom_fields;
mod duplicates;
mod email_delivery;
mod email_templates;
mod enforcement;
mod engagement;
mod external_links;
mod field_encryption;
mod fx_rates;
mod idempotency;
mod insight_rollups;
mod insights;
mod inventory;
mod kb;
mod lead_assignment;
mod lead_imports;
mod log_retention;
mod marketing;
mod mcp;
mod merge;
mod notifications;
mod openapi_schemas;
mod pagination;
mod privacy;
mod quotas;
mod record_imports;
mod relationships;
mod reports;
mod sales;
mod search;
mod streaming;
mod support;
mod tags;
mod ticket_pauses;
mod unmerge;
mod views;
mod web_forms;
mod webhooks;
mod workflows;

/// A fresh synthetic `person:` URN.
/// Switches small-cell suppression (CRM-R78) off for a test that checks a report's arithmetic on
/// a few records, and puts it back when the test ends, even by panic.
pub struct NoSuppression;

/// Puts the default small-cell size back when a test that set its own ends.
pub fn no_suppression_restore() -> NoSuppression {
    NoSuppression
}

pub fn no_suppression() -> NoSuppression {
    contact_relationship_management::settings::update(|s| s.min_cell_size = 0);
    NoSuppression
}

impl Drop for NoSuppression {
    fn drop(&mut self) {
        contact_relationship_management::settings::update(|s| {
            s.min_cell_size =
                contact_relationship_management::settings::Settings::default().min_cell_size;
        });
    }
}

pub fn a_person() -> String {
    format!("person:{}", uuid::Uuid::new_v4())
}

/// A fresh synthetic `worker:` URN.
pub fn a_worker() -> String {
    format!("worker:{}", uuid::Uuid::new_v4())
}

/// Create the demo pipeline (3 open stages + Won + Lost). Expands to
/// `(pipeline_pid, stage_pids)`, await it like a function call.
///
/// A macro, not a function: the `request` handed to each test by
/// loco's `request()` helper is a `loco_rs`-internal-pinned
/// `axum_test::TestServer` — a type this crate cannot name directly
/// without pulling in that exact same `axum-test` version as a direct
/// dependency (defeating the point of tracking a newer one for the
/// dev-dependency's own sake). Expanding inline sidesteps naming the
/// type at all; the actual value's type is inferred at each call site.
macro_rules! seed_pipeline {
    ($request:expr) => {
        async {
            let request = $request;
            let created: serde_json::Value = request
                .post("/api/pipelines")
                .json(&serde_json::json!({
                    "name": "Test Pipeline",
                    "stages": [
                        { "name": "Qualification", "probability_percent": 10 },
                        { "name": "Proposal", "probability_percent": 50 },
                        { "name": "Won", "probability_percent": 100, "is_won": true },
                        { "name": "Lost", "probability_percent": 0, "is_lost": true },
                    ],
                }))
                .await
                .json();
            let stages: Vec<String> = created["stage_pids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap().to_string())
                .collect();
            (created["pid"].as_str().unwrap().to_string(), stages)
        }
    };
}
pub(crate) use seed_pipeline;

/// Create a consented contact. Expands to the pid. Same macro
/// rationale as [`seed_pipeline`].
macro_rules! seed_contact {
    ($request:expr, $name:expr) => {
        async {
            let request = $request;
            let name = $name;
            let created: serde_json::Value = request
                .post("/api/contacts")
                .json(&serde_json::json!({ "person_ref": $crate::requests::a_person(), "display_name": name }))
                .await
                .json();
            let pid = created["pid"].as_str().unwrap().to_string();
            request
                .post(&format!("/api/contacts/{pid}/consent"))
                .json(&serde_json::json!({ "action": "granted", "source": "web form" }))
                .await
                .assert_status_ok();
            pid
        }
    };
}
pub(crate) use seed_contact;
