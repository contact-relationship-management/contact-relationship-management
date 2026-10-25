//! Lead assignment (CRM-R44): `assign_owner` on a `lead_created` workflow, in turn or by load,
//! the conditions on a lead's source, the run log, and what is left alone.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::a_worker;

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

macro_rules! workflow {
    ($request:expr, $body:expr) => {{
        let answer = $request.post("/api/workflows").json(&$body).await;
        answer.assert_status_ok();
        answer.json::<Value>()["pid"].as_str().unwrap().to_string()
    }};
}

macro_rules! lead {
    ($request:expr, $name:expr, $source:expr) => {{
        let made: Value = $request
            .post("/api/leads")
            .json(&json!({ "display_name": $name, "source": $source }))
            .await
            .json();
        made["pid"].as_str().unwrap().to_string()
    }};
}

macro_rules! owner_of {
    ($request:expr, $lead:expr) => {{
        let all: Vec<Value> = $request.get("/api/leads").await.json();
        all.iter().find(|l| l["pid"] == $lead).expect("the lead")["owner_ref"].clone()
    }};
}

fn assigning(name: &str, pool: &[&str], strategy: &str, conditions: Value) -> Value {
    json!({ "name": name, "trigger": "lead_created", "conditions": conditions,
            "actions": [{ "type": "assign_owner", "pool": pool, "strategy": strategy, "notify": true }] })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn leads_are_given_out_in_turn_and_the_worker_is_told() {
    request::<App, _, _>(|request, ctx| async move {
        let (a, b) = (a_worker(), a_worker());
        let wf = workflow!(
            &request,
            assigning("Round robin", &[&a, &b], "round_robin", json!({}))
        );
        let first = lead!(&request, "First Lead", "web");
        let second = lead!(&request, "Second Lead", "web");
        let third = lead!(&request, "Third Lead", "web");
        assert_eq!(owner_of!(request, first), json!(a));
        assert_eq!(owner_of!(request, second), json!(b));
        assert_eq!(owner_of!(request, third), json!(a));

        let log: Vec<Value> = request
            .get(&format!("/api/workflows/{wf}/runs"))
            .await
            .json();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0]["subject_kind"], "lead");
        assert!(
            log[0]["steps"][0]["detail"]
                .as_str()
                .unwrap()
                .contains("assigned to")
        );
        // The worker has a notification about the lead, and the audit trail names the workflow.
        let inbox: Vec<Value> = request
            .get(&format!("/api/notifications?recipient={a}"))
            .await
            .json();
        assert_eq!(inbox.len(), 2);
        assert_eq!(inbox[0]["subject_kind"], "lead");
        assert_eq!(
            count(
                &ctx.db,
                &format!(
                    "SELECT count(*) AS n FROM audit_logs WHERE entity = 'lead' \
                          AND action = 'assigned' AND actor = 'workflow:{wf}'"
                )
            )
            .await,
            3
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_least_loaded_worker_gets_the_next_lead() {
    request::<App, _, _>(|request, _ctx| async move {
        let (a, b) = (a_worker(), a_worker());
        workflow!(
            &request,
            assigning("By load", &[&a, &b], "least_open", json!({}))
        );
        let one = lead!(&request, "Load One", "web");
        let two = lead!(&request, "Load Two", "web");
        let three = lead!(&request, "Load Three", "web");
        // A tie goes to the first; then the other has fewer.
        assert_eq!(owner_of!(request, one), json!(a));
        assert_eq!(owner_of!(request, two), json!(b));
        assert_eq!(owner_of!(request, three), json!(a));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn only_leads_that_meet_the_conditions_are_assigned_and_imports_are_not() {
    request::<App, _, _>(|request, _ctx| async move {
        let a = a_worker();
        workflow!(
            &request,
            assigning(
                "Web only",
                &[&a],
                "round_robin",
                json!({ "sources": ["web"] })
            )
        );
        let web = lead!(&request, "From Web", "web");
        let referral = lead!(&request, "From Referral", "referral");
        assert_eq!(owner_of!(request, web), json!(a));
        assert_eq!(owner_of!(request, referral), Value::Null);

        // A list import starts no workflow.
        let done: Value = request
            .post("/api/lead-imports")
            .json(
                &json!({ "csv": "Name\nImported One\n", "mapping": { "display_name": "Name" },
                           "source_note": "Show", "obtained_on": "2026-10-01" }),
            )
            .await
            .json();
        assert_eq!(done["created_count"], 1);
        let all: Vec<Value> = request.get("/api/leads").await.json();
        let imported = all
            .iter()
            .find(|l| l["display_name"] == "Imported One")
            .unwrap();
        assert_eq!(imported["owner_ref"], Value::Null);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_lead_workflow_is_refused_where_it_cannot_apply_and_a_dry_run_shows_the_turn() {
    request::<App, _, _>(|request, _ctx| async move {
        let a = a_worker();
        for body in [
            // assign_owner on a contact trigger; a contact clause on a lead trigger; a source on a
            // contact trigger; a contact-only action on a lead trigger; a pool that is not workers.
            json!({ "name": "A", "trigger": "contact_created",
                    "actions": [{ "type": "assign_owner", "pool": [a], "strategy": "round_robin" }] }),
            json!({ "name": "B", "trigger": "lead_created", "conditions": { "statuses": ["active"] },
                    "actions": [{ "type": "assign_owner", "pool": [a], "strategy": "round_robin" }] }),
            json!({ "name": "C", "trigger": "contact_created", "conditions": { "sources": ["web"] },
                    "actions": [{ "type": "create_task", "summary": "x" }] }),
            json!({ "name": "D", "trigger": "lead_created",
                    "actions": [{ "type": "set_field", "field": "job_title", "value": "x" }] }),
            json!({ "name": "E", "trigger": "lead_created",
                    "actions": [{ "type": "assign_owner", "pool": ["not-a-urn"], "strategy": "round_robin" }] }),
            json!({ "name": "F", "trigger": "lead_created",
                    "actions": [{ "type": "assign_owner", "pool": [], "strategy": "round_robin" }] }),
        ] {
            let r = request.post("/api/workflows").json(&body).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }

        // Leads that exist before the workflow do; the dry run shows who would get the next one.
        let waiting = lead!(&request, "Waiting Lead", "web");
        let b = a_worker();
        let wf = workflow!(&request, assigning("Dry", &[&a, &b], "round_robin", json!({})));
        let result: Value = request
            .post(&format!("/api/workflows/{wf}/dry-run"))
            .await
            .json();
        assert_eq!(result["would_fire"], 1);
        let sample = &result["samples"][0];
        assert_eq!(sample["subject_kind"], "lead");
        assert_eq!(sample["subject_pid"], json!(waiting));
        assert!(sample["steps"][0]["detail"].as_str().unwrap().contains(&a));
        assert_eq!(owner_of!(request, waiting), Value::Null, "a dry run assigns nothing");
        assert_eq!(
            request.get(&format!("/api/workflows/{wf}/runs")).await.json::<Vec<Value>>().len(),
            0
        );
    })
    .await;
}
