//! Explainable workflow rules (CRM-R40): each action, the dry run, the schedule, the loop guard,
//! the consent re-check, and the run log that says why a run happened.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use serde_json::{Value, json};
use serial_test::serial;
use sqlx::Row as _;

use super::{a_person, a_user, seed_contact, seed_pipeline};

async fn count(db: &sqlx::PgPool, sql: &str) -> i64 {
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_optional(db)
        .await
        .unwrap()
        .unwrap()
        .try_get("n")
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
                contact_relationship_management::db::pool(&ctx),
                &format!(
                    "SELECT count(*) AS n FROM activities WHERE kind = 'task' AND subject_pid = '{emailer}'"
                )
            )
            .await,
            1
        );
        assert_eq!(
            count(
                contact_relationship_management::db::pool(&ctx),
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
                contact_relationship_management::db::pool(&ctx),
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn notify_set_field_and_webhook_actions_each_do_their_one_thing() {
    request::<App, _, _>(|request, ctx| async move {
        let who = a_user();
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
                contact_relationship_management::db::pool(&ctx),
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
            let db = contact_relationship_management::db::pool(&ctx).clone();
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
        let before = count(contact_relationship_management::db::pool(&ctx), "SELECT count(*) AS n FROM activities").await;
        let result: Value = request
            .post(&format!("/api/workflows/{wf}/dry-run"))
            .await
            .json();
        assert_eq!(result["would_fire"], 2);
        let samples = result["samples"].as_array().unwrap();
        let of = |pid: &str| {
            samples
                .iter()
                .find(|s| s["subject_pid"] == json!(pid))
                .unwrap()["steps"]
                .clone()
        };
        assert_eq!(of(&consenting)[0]["result"], "would_do");
        assert_eq!(of(&nobody)[0]["result"], "would_skip");
        assert!(of(&nobody)[0]["detail"].as_str().unwrap().contains("consent"));
        assert_eq!(count(contact_relationship_management::db::pool(&ctx), "SELECT count(*) AS n FROM activities").await, before);
        assert_eq!(
            count(contact_relationship_management::db::pool(&ctx), "SELECT count(*) AS n FROM nurture_enrollments").await,
            0
        );
        assert!(runs!(&request, wf).is_empty());
        assert_eq!(
            count(
                contact_relationship_management::db::pool(&ctx),
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
        sqlx::raw_sql("UPDATE workflow_runs SET created_at = now() - interval '2 hours'")
            .execute(contact_relationship_management::db::pool(&ctx))
            .await
            .unwrap();
        let later: Value = request.post("/api/workflows/run-due").await.json();
        assert_eq!(later["runs"], 2);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_failing_action_undoes_the_run_and_spares_the_write_that_triggered_it() {
    request::<App, _, _>(|request, ctx| async move {
        let wf = workflow!(
            &request,
            when_created(
                "Two steps",
                json!({}),
                json!([
                    { "type": "create_task", "summary": "First" },
                    { "type": "notify", "recipient_ref": a_user() },
                ])
            )
        );
        // Break the second action by taking its table away for the length of one request.
        sqlx::raw_sql("ALTER TABLE notifications RENAME TO notifications_away")
            .execute(contact_relationship_management::db::pool(&ctx))
            .await
            .unwrap();
        let made = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": a_person(), "display_name": "Fa Il" }))
            .await;
        sqlx::raw_sql("ALTER TABLE notifications_away RENAME TO notifications")
            .execute(contact_relationship_management::db::pool(&ctx))
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
                contact_relationship_management::db::pool(&ctx),
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

/// The steps of one run: `(result, detail)` each.
fn steps(run: &Value) -> Vec<(String, String)> {
    run["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["result"].as_str().unwrap().to_string(),
                s["detail"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

/// A workflow stored before its checks, or written by hand, with an action its trigger cannot use:
/// the engine skips that step with the reason instead of applying it to the wrong record.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn an_action_the_record_cannot_take_is_skipped_with_the_reason() {
    request::<App, _, _>(|request, ctx| async move {
        let pool = contact_relationship_management::db::pool(&ctx);
        let rewrite = |wf: String, actions: Value| async move {
            sqlx::query("UPDATE workflows SET actions = $2 WHERE pid = $1::uuid")
                .bind(wf)
                .bind(actions)
                .execute(pool)
                .await
                .unwrap();
        };
        // A lead workflow given contact actions and an empty pool.
        let on_leads = workflow!(
            &request,
            json!({ "name": "Leads", "trigger": "lead_created", "conditions": {},
                    "actions": [{ "type": "assign_owner", "pool": [a_user()], "strategy": "round_robin" }] })
        );
        rewrite(
            on_leads.clone(),
            json!([
                { "type": "set_field", "field": "job_title", "value": "x" },
                { "type": "enrol_nurture", "sequence_pid": uuid::Uuid::new_v4() },
                { "type": "assign_owner", "pool": [], "strategy": "round_robin", "notify": false },
            ]),
        )
        .await;
        request
            .post("/api/leads")
            .json(&json!({ "display_name": "Hand Written", "source": "web" }))
            .await
            .assert_status_ok();
        let lead_steps = steps(&runs!(&request, on_leads)[0]);
        assert_eq!(
            lead_steps,
            [
                ("skipped".into(), "this action is about a contact, not a lead".into()),
                ("skipped".into(), "this action is about a contact, not a lead".into()),
                ("skipped".into(), "the pool is empty".into()),
            ]
        );

        // A contact workflow given a lead's action.
        let on_contacts = workflow!(
            &request,
            when_created("Contacts", json!({}), json!([{ "type": "notify", "recipient_ref": a_user() }]))
        );
        rewrite(
            on_contacts.clone(),
            json!([{ "type": "assign_owner", "pool": [a_user()], "strategy": "round_robin", "notify": false }]),
        )
        .await;
        contact!(&request, json!({ "display_name": "Not A Lead" }));
        assert_eq!(
            steps(&runs!(&request, on_contacts)[0]),
            [("skipped".into(), "assign_owner is about a lead".into())]
        );
    })
    .await;
}

/// The other fields a workflow may set, a value already there, a lead that already has an owner,
/// and a condition on the contact's account tier.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn channel_and_status_are_set_a_held_value_is_left_and_an_owned_lead_keeps_its_owner() {
    request::<App, _, _>(|request, _ctx| async move {
        let account: Value = request
            .post("/api/accounts")
            .json(&json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Partner Org", "tier": "partner" }))
            .await
            .json();
        let wf = workflow!(
            &request,
            when_created(
                "Partner accounts",
                json!({ "account_tiers": ["partner"] }),
                json!([
                    { "type": "set_field", "field": "status", "value": "inactive" },
                    { "type": "set_field", "field": "preferred_channel", "value": "phone" },
                ])
            )
        );
        // No account, so the tier condition is not met: no run.
        contact!(&request, json!({ "display_name": "No Account" }));
        assert!(runs!(&request, wf).is_empty());
        let pid = contact!(
            &request,
            json!({ "display_name": "Partner Contact", "account_pid": account["pid"], "preferred_channel": "phone" })
        );
        let log = runs!(&request, wf);
        assert_eq!(log.len(), 1);
        let results: Vec<String> = steps(&log[0]).into_iter().map(|(r, _)| r).collect();
        // The channel was already the one asked for, so that step changes nothing.
        assert_eq!(results, ["done", "skipped"], "{log:?}");
        assert_eq!(steps(&log[0])[1].1, "preferred_channel already has that value");
        let detail: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(detail["contact"]["preferred_channel"], "phone");
        assert_eq!(detail["contact"]["status"], "inactive");

        // Two workflows give out new leads: the first gives this one an owner, and the second finds
        // it owned and leaves it.
        let (first_owner, second_pool) = (a_user(), a_user());
        let first = workflow!(
            &request,
            json!({ "name": "First", "trigger": "lead_created", "conditions": {},
                    "actions": [{ "type": "assign_owner", "pool": [first_owner], "strategy": "round_robin" }] })
        );
        let second = workflow!(
            &request,
            json!({ "name": "Second", "trigger": "lead_created", "conditions": {},
                    "actions": [{ "type": "assign_owner", "pool": [second_pool], "strategy": "round_robin" }] })
        );
        let lead: Value = request
            .post("/api/leads")
            .json(&json!({ "display_name": "Owned Lead", "source": "web" }))
            .await
            .json();
        let both = [steps(&runs!(&request, first)[0]), steps(&runs!(&request, second)[0])];
        assert!(
            both.contains(&vec![("skipped".into(), "the lead already has an owner".into())]),
            "{both:?}"
        );
        let all: Vec<Value> = request.get("/api/leads").await.json();
        let kept = all.iter().find(|l| l["pid"] == lead["pid"]).unwrap();
        assert!(kept["owner_ref"] == json!(first_owner) || kept["owner_ref"] == json!(second_pool));
    })
    .await;
}

/// A dry run of a lead workflow says what each step would do without doing it: leave an owned
/// lead alone, name the worker an unowned lead would go to (the least loaded, for `least_open`),
/// and say when the pool is empty; a contact that already holds a value is not set again; and a
/// real run that names a sequence that is gone skips that step with the reason (CRM-T292, round 18).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_dry_run_names_who_a_lead_would_go_to_and_a_gone_sequence_is_skipped_with_the_reason() {
    request::<App, _, _>(|request, ctx| async move {
        let pool = contact_relationship_management::db::pool(&ctx);
        let (first, second) = (a_user(), a_user());

        // Two leads: one owned already, one not.
        let mut leads = Vec::new();
        for name in ["Owned Lead", "Free Lead"] {
            let made: Value = request
                .post("/api/leads")
                .json(&json!({ "display_name": name, "source": "web" }))
                .await
                .json();
            leads.push(made["pid"].as_str().unwrap().to_string());
        }
        sqlx::query("UPDATE leads SET owner_ref = $2 WHERE pid = $1::uuid")
            .bind(&leads[0])
            .bind(&first)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("UPDATE leads SET owner_ref = NULL WHERE pid = $1::uuid")
            .bind(&leads[1])
            .execute(pool)
            .await
            .unwrap();

        let on_leads = workflow!(
            &request,
            json!({ "name": "Spread leads", "trigger": "lead_created", "conditions": {},
                    "actions": [{ "type": "assign_owner", "pool": [first, second],
                                  "strategy": "least_open" }] })
        );
        let dry = |wf: String| {
            let request = &request;
            async move {
                request
                    .post(&format!("/api/workflows/{wf}/dry-run"))
                    .await
                    .json::<Value>()
            }
        };
        let result = dry(on_leads.clone()).await;
        assert_eq!(result["would_fire"], 2, "{result}");
        let samples = result["samples"].as_array().unwrap();
        let steps_of = |pid: &str| {
            samples
                .iter()
                .find(|s| s["subject_pid"] == json!(pid))
                .unwrap_or_else(|| panic!("{pid} not in {result}"))["steps"][0]
                .clone()
        };
        let owned = steps_of(&leads[0]);
        assert_eq!(owned["result"], "would_skip");
        assert_eq!(owned["detail"], "the lead already has an owner");
        let free = steps_of(&leads[1]);
        assert_eq!(free["result"], "would_do");
        assert!(
            free["detail"].as_str().unwrap().contains("(least_open)"),
            "{free}"
        );

        // An empty pool is said so. (The API refuses one, so it is written into the row.)
        sqlx::query("UPDATE workflows SET actions = $2 WHERE pid = $1::uuid")
            .bind(&on_leads)
            .bind(json!([{ "type": "assign_owner", "pool": [], "strategy": "round_robin",
                           "notify": false }]))
            .execute(pool)
            .await
            .unwrap();
        let empty = dry(on_leads).await;
        let empty_steps: Vec<&str> = empty["samples"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s["steps"][0]["detail"].as_str())
            .collect();
        assert!(empty_steps.contains(&"the pool is empty"), "{empty}");

        // A contact that already holds the value is not set again, in a dry run.
        let held = contact!(
            &request,
            json!({ "display_name": "Held", "job_title": "Analyst" })
        );
        let set_same = workflow!(
            &request,
            json!({ "name": "Same title", "trigger": "contact_updated",
                    "actions": [{ "type": "set_field", "field": "job_title", "value": "Analyst" }] })
        );
        let result = dry(set_same).await;
        let held_step = result["samples"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["subject_pid"] == json!(held))
            .unwrap_or_else(|| panic!("{result}"))["steps"][0]
            .clone();
        assert_eq!(held_step["result"], "would_skip", "{held_step}");
        assert!(
            held_step["detail"].as_str().unwrap().contains("already has that value"),
            "{held_step}"
        );

        // A real run that names a sequence that has gone skips that step and says why.
        let gone = workflow!(
            &request,
            when_created(
                "Enrol nobody",
                json!({}),
                json!([{ "type": "create_task", "summary": "placeholder" }])
            )
        );
        sqlx::query("UPDATE workflows SET actions = $2 WHERE pid = $1::uuid")
            .bind(&gone)
            .bind(json!([{ "type": "enrol_nurture", "sequence_pid": uuid::Uuid::new_v4() }]))
            .execute(pool)
            .await
            .unwrap();
        contact!(&request, json!({ "display_name": "Wants A Sequence" }));
        let steps = steps(&runs!(&request, gone)[0]);
        assert_eq!(steps.len(), 1, "{steps:?}");
        assert_eq!(steps[0].0, "skipped");
        assert!(steps[0].1.contains("no longer exists"), "{steps:?}");
    })
    .await;
}
