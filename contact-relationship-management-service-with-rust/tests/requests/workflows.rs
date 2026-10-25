//! Explainable workflow rules (CRM-R40): each action, the dry run, the schedule, the loop guard,
//! the consent re-check, and the run log that says why a run happened.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_worker, seed_contact, seed_pipeline};

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

macro_rules! runs {
    ($request:expr, $pid:expr) => {{
        let answer = $request.get(&format!("/api/workflows/{}/runs", $pid)).await;
        answer.assert_status_ok();
        answer.json::<Vec<Value>>()
    }};
}

macro_rules! contact {
    ($request:expr, $body:expr) => {{
        let mut body = $body;
        body["person_ref"] = json!(a_person());
        let answer = $request.post("/api/contacts").json(&body).await;
        answer.assert_status_ok();
        answer.json::<Value>()["pid"].as_str().unwrap().to_string()
    }};
}

fn when_created(name: &str, conditions: Value, actions: Value) -> Value {
    json!({ "name": name, "trigger": "contact_created", "conditions": conditions,
            "actions": actions })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_task_is_made_when_conditions_are_met_and_the_run_says_why() {
    request::<App, _, _>(|request, ctx| async move {
        let wf = workflow!(
            &request,
            when_created(
                "Greet emailers",
                json!({ "channels": ["email"] }),
                json!([{ "type": "create_task", "summary": "Say hello", "due_in_days": 2 }])
            )
        );
        let emailer = contact!(&request, json!({ "display_name": "Em Ailer" }));
        contact!(
            &request,
            json!({ "display_name": "Fo Ner", "preferred_channel": "phone" })
        );
        // Only the contact who meets the condition got a task, and one run is logged.
        assert_eq!(
            count(
                &ctx.db,
                &format!(
                    "SELECT count(*) AS n FROM activities WHERE kind = 'task' AND subject_pid = '{emailer}'"
                )
            )
            .await,
            1
        );
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM activities WHERE kind = 'task'"
            )
            .await,
            1
        );
        let log = runs!(&request, wf);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0]["outcome"], "done");
        assert_eq!(log[0]["subject_pid"], json!(emailer));
        let reason = log[0]["reason"].as_str().unwrap();
        assert!(reason.contains("contact created") && reason.contains("channel is email"), "{reason}");
        assert_eq!(log[0]["steps"][0]["result"], "done");
        // The run is audited.
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE entity = 'workflow_run' AND action = 'fired'"
            )
            .await,
            1
        );
        // A disabled workflow does nothing.
        let mut off = when_created("Greet emailers", json!({}), json!([{ "type": "create_task", "summary": "x" }]));
        off["enabled"] = json!(false);
        request.put(&format!("/api/workflows/{wf}")).json(&off).await.assert_status_ok();
        contact!(&request, json!({ "display_name": "Ne Xt" }));
        assert_eq!(runs!(&request, wf).len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn notify_set_field_and_webhook_actions_each_do_their_one_thing() {
    request::<App, _, _>(|request, ctx| async move {
        let who = a_worker();
        let wf = workflow!(
            &request,
            when_created(
                "Triage",
                json!({}),
                json!([
                    { "type": "notify", "recipient_ref": who },
                    { "type": "set_field", "field": "job_title", "value": "Prospect" },
                    { "type": "send_webhook", "event": "new_contact" },
                ])
            )
        );
        let pid = contact!(&request, json!({ "display_name": "Tri Age" }));
        let log = runs!(&request, wf);
        let results: Vec<&str> = log[0]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["result"].as_str().unwrap())
            .collect();
        assert_eq!(results, ["done", "done", "done"], "{log:?}");
        let inbox: Vec<Value> = request
            .get(&format!("/api/notifications?recipient={who}"))
            .await
            .json();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0]["kind"], "workflow");
        assert_eq!(inbox[0]["subject_pid"], json!(pid));
        let detail: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(detail["contact"]["job_title"], "Prospect");
        let events: Vec<Value> = request.get("/api/events/recent").await.json();
        assert!(
            events.iter().any(|e| e["kind"] == "new_contact"),
            "{events:?}"
        );
        // The field change was made by the workflow, and says so in the audit trail.
        assert_eq!(
            count(
                &ctx.db,
                &format!(
                    "SELECT count(*) AS n FROM audit_logs WHERE entity_pid = '{pid}' \
                     AND action = 'updated' AND actor = 'workflow:{wf}'"
                )
            )
            .await,
            1
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn enrolment_rechecks_consent_inside_the_action() {
    request::<App, _, _>(|request, ctx| async move {
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(
                &json!({ "name": "Drip", "steps": [{ "delay_hours": 1, "template_ref": "t-1" }] }),
            )
            .await
            .json();
        let sequence = sequence["pid"].as_str().unwrap().to_string();
        let wf = workflow!(
            &request,
            json!({ "name": "Enrol on change", "trigger": "contact_updated",
                    "actions": [{ "type": "enrol_nurture", "sequence_pid": sequence }] })
        );
        let nobody = contact!(&request, json!({ "display_name": "No Consent" }));
        let consenting = seed_contact!(&request, "Has Consent").await;
        for pid in [&nobody, &consenting] {
            request
                .put(&format!("/api/contacts/{pid}"))
                .json(&json!({ "job_title": "x" }))
                .await
                .assert_status_ok();
        }
        // The contact who never consented was not enrolled, and the run says why.
        let enrolments = |pid: String| {
            let db = ctx.db.clone();
            async move {
                count(
                    &db,
                    &format!(
                        "SELECT count(*) AS n FROM nurture_enrollments WHERE contact_pid = '{pid}'"
                    ),
                )
                .await
            }
        };
        assert_eq!(enrolments(nobody.clone()).await, 0);
        assert_eq!(enrolments(consenting.clone()).await, 1);
        let log = runs!(&request, wf);
        let for_nobody = log
            .iter()
            .find(|r| r["subject_pid"] == json!(nobody))
            .unwrap();
        assert_eq!(for_nobody["steps"][0]["result"], "skipped");
        assert!(
            for_nobody["steps"][0]["detail"]
                .as_str()
                .unwrap()
                .contains("consent")
        );
        // Withdrawn since the last time: still not enrolled; consented and already enrolled: skipped.
        request
            .put(&format!("/api/contacts/{consenting}"))
            .json(&json!({ "job_title": "y" }))
            .await
            .assert_status_ok();
        assert_eq!(enrolments(consenting.clone()).await, 1);
        request
            .post(&format!("/api/contacts/{nobody}/consent"))
            .json(&json!({ "action": "granted", "source": "web form" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/contacts/{nobody}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "web form" }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/contacts/{nobody}"))
            .json(&json!({ "job_title": "z" }))
            .await
            .assert_status_ok();
        assert_eq!(enrolments(nobody).await, 0);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_loop_of_workflows_is_cut_and_a_busy_one_is_held_back() {
    request::<App, _, _>(|request, _ctx| async move {
        // Two workflows that undo each other: A sets the title to A, B sets it to B.
        let on_update = |name: &str, title: &str| {
            json!({ "name": name, "trigger": "contact_updated",
                    "actions": [{ "type": "set_field", "field": "job_title", "value": title }] })
        };
        let a = workflow!(&request, on_update("Make it A", "A"));
        let b = workflow!(&request, on_update("Make it B", "B"));
        let pid = contact!(&request, json!({ "display_name": "Lo Op" }));
        request
            .put(&format!("/api/contacts/{pid}"))
            .json(&json!({ "family_name": "Loop" }))
            .await
            .assert_status_ok();
        let all: Vec<Value> = runs!(&request, a)
            .into_iter()
            .chain(runs!(&request, b))
            .collect();
        let blocked: Vec<&Value> = all.iter().filter(|r| r["outcome"] == "blocked").collect();
        assert_eq!(
            blocked.len(),
            2,
            "each workflow notes that the loop was cut once: {all:?}"
        );
        assert!(
            blocked[0]["reason"]
                .as_str()
                .unwrap()
                .contains("loop guard")
        );
        assert!(all.iter().all(|r| r["depth"].as_i64().unwrap() <= 3));

        // A workflow that runs too often in a minute is held back, with one note.
        let busy = workflow!(
            &request,
            when_created(
                "Busy",
                json!({}),
                json!([{ "type": "create_task", "summary": "x" }])
            )
        );
        for i in 0..62 {
            contact!(&request, json!({ "display_name": format!("Busy {i}") }));
        }
        let log = runs!(&request, busy);
        let done = log.iter().filter(|r| r["outcome"] == "done").count();
        let held: Vec<&Value> = log.iter().filter(|r| r["outcome"] == "blocked").collect();
        assert_eq!(done, 60);
        assert_eq!(held.len(), 1);
        assert!(held[0]["reason"].as_str().unwrap().contains("too often"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_dry_run_changes_and_logs_nothing() {
    request::<App, _, _>(|request, ctx| async move {
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Drip", "steps": [{ "delay_hours": 1, "template_ref": "t-1" }] }))
            .await
            .json();
        let sequence = sequence["pid"].as_str().unwrap().to_string();
        let consenting = seed_contact!(&request, "Dry Consent").await;
        let nobody = contact!(&request, json!({ "display_name": "Dry Never" }));
        let wf = workflow!(
            &request,
            json!({ "name": "Try me", "trigger": "contact_updated",
                    "actions": [
                        { "type": "enrol_nurture", "sequence_pid": sequence },
                        { "type": "create_task", "summary": "Follow up" },
                    ] })
        );
        let before = count(&ctx.db, "SELECT count(*) AS n FROM activities").await;
        let result: Value = request
            .post(&format!("/api/workflows/{wf}/dry-run"))
            .await
            .json();
        assert_eq!(result["would_fire"], 2);
        let samples = result["samples"].as_array().unwrap();
        let of = |pid: &str| {
            samples
                .iter()
                .find(|s| s["contact_pid"] == json!(pid))
                .unwrap()["steps"]
                .clone()
        };
        assert_eq!(of(&consenting)[0]["result"], "would_do");
        assert_eq!(of(&nobody)[0]["result"], "would_skip");
        assert!(of(&nobody)[0]["detail"].as_str().unwrap().contains("consent"));
        assert_eq!(count(&ctx.db, "SELECT count(*) AS n FROM activities").await, before);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM nurture_enrollments").await,
            0
        );
        assert!(runs!(&request, wf).is_empty());
        assert_eq!(
            count(
                &ctx.db,
                &format!("SELECT count(*) AS n FROM audit_logs WHERE entity_pid = '{wf}' AND action = 'dry_run'")
            )
            .await,
            1
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_schedule_runs_once_per_contact_per_interval() {
    request::<App, _, _>(|request, ctx| async move {
        contact!(&request, json!({ "display_name": "Sched One" }));
        contact!(&request, json!({ "display_name": "Sched Two" }));
        let wf = workflow!(
            &request,
            json!({ "name": "Check in", "trigger": "schedule", "every_minutes": 60,
                    "actions": [{ "type": "create_task", "summary": "Check in" }] })
        );
        let first: Value = request.post("/api/workflows/run-due").await.json();
        assert_eq!(first["runs"], 2);
        let again: Value = request.post("/api/workflows/run-due").await.json();
        assert_eq!(again["runs"], 0, "inside the interval");
        assert_eq!(runs!(&request, wf).len(), 2);
        // Once the interval has passed they run again.
        ctx.db
            .execute_unprepared("UPDATE workflow_runs SET created_at = now() - interval '2 hours'")
            .await
            .unwrap();
        let later: Value = request.post("/api/workflows/run-due").await.json();
        assert_eq!(later["runs"], 2);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_deal_stage_change_starts_a_workflow_for_the_primary_contact() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let pid = contact!(&request, json!({ "display_name": "Deal Holder" }));
        let wf = workflow!(
            &request,
            json!({ "name": "On stage", "trigger": "deal_stage_changed",
                    "actions": [{ "type": "create_task", "summary": "Prepare proposal" }] })
        );
        let deal: Value = request
            .post("/api/deals")
            .json(
                &json!({ "name": "Big", "pipeline_pid": pipeline, "amount_minor": 100,
                           "currency": "GBP", "primary_contact_pid": pid }),
            )
            .await
            .json();
        let deal = deal["pid"].as_str().unwrap();
        request
            .post(&format!("/api/deals/{deal}/stage"))
            .json(&json!({ "stage_pid": stages[1] }))
            .await
            .assert_status_ok();
        let log = runs!(&request, wf);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0]["subject_kind"], "deal");
        assert_eq!(log[0]["subject_pid"], json!(deal));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_workflow_outside_the_closed_set_is_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let bad = |body: Value| {
            let request = &request;
            async move {
                let r = request.post("/api/workflows").json(&body).await;
                assert!(r.status_code().is_client_error(), "{body}: {}", r.status_code());
            }
        };
        let ok = json!([{ "type": "create_task", "summary": "x" }]);
        bad(json!({ "name": "A", "trigger": "contact_created",
                    "actions": [{ "type": "http_request", "url": "https://example.test" }] })).await;
        bad(json!({ "name": "A", "trigger": "contact_created", "actions": [] })).await;
        bad(json!({ "name": "A", "trigger": "nonsense", "actions": ok })).await;
        bad(json!({ "name": "A", "trigger": "schedule", "actions": ok })).await;
        bad(json!({ "name": "A", "trigger": "contact_created",
                    "conditions": { "account_tier": ["customer"] }, "actions": ok })).await;
        bad(json!({ "name": "A", "trigger": "contact_created",
                    "actions": [{ "type": "notify", "recipient_ref": "not-a-urn" }] })).await;
        bad(json!({ "name": "A", "trigger": "contact_created",
                    "actions": [{ "type": "enrol_nurture", "sequence_pid": uuid::Uuid::new_v4() }] })).await;
        // A name is taken once, ignoring case.
        workflow!(&request, json!({ "name": "Unique", "trigger": "contact_created", "actions": ok }));
        bad(json!({ "name": "UNIQUE", "trigger": "contact_created", "actions": ok })).await;
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_failing_action_undoes_the_run_and_spares_the_write_that_triggered_it() {
    request::<App, _, _>(|request, ctx| async move {
        let wf = workflow!(
            &request,
            when_created(
                "Two steps",
                json!({}),
                json!([
                    { "type": "create_task", "summary": "First" },
                    { "type": "notify", "recipient_ref": a_worker() },
                ])
            )
        );
        // Break the second action by taking its table away for the length of one request.
        ctx.db
            .execute_unprepared("ALTER TABLE notifications RENAME TO notifications_away")
            .await
            .unwrap();
        let made = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": a_person(), "display_name": "Fa Il" }))
            .await;
        ctx.db
            .execute_unprepared("ALTER TABLE notifications_away RENAME TO notifications")
            .await
            .unwrap();
        made.assert_status_ok();
        let pid = made.json::<Value>()["pid"].as_str().unwrap().to_string();
        // The contact exists; the first action's task did not survive; the log says so.
        request
            .get(&format!("/api/contacts/{pid}"))
            .await
            .assert_status_ok();
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM activities WHERE kind = 'task'"
            )
            .await,
            0
        );
        let log = runs!(&request, wf);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0]["outcome"], "failed");
        assert_eq!(log[0]["steps"][0]["result"], "rolled_back");
        assert_eq!(log[0]["steps"][1]["result"], "failed");
    })
    .await;
}
