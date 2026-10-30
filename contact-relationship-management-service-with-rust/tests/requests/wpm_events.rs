//! The worker service's hire and leave events (CRM-R92): signed as WPM signs them, applied once,
//! pausing marketing to an employee and lifting the pause when they leave, with the consent they
//! had. No WPM runs here: the requests are made the way its `webhook_deliver` task makes them.

use contact_relationship_management::app::App;
use contact_relationship_management::rules::wpm_events::{TOLERANCE_SECS, sign};
use contact_relationship_management::settings::{self, Settings};
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_contact};

const SECRET: &str = "wpm-secret-0123456789abcdef-0123456789";
const ORG: &str = "organization:0c4f1e2a-0000-4000-8000-0000000000aa";

/// Puts the process-wide settings back when a test ends, even by panic.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        settings::install(Settings::default());
    }
}

fn event(id: &str, kind: &str, person_ref: &str, data: bool) -> String {
    let mut body = json!({
        "id": id,
        "kind": kind,
        "occurred_at": "2026-10-30T12:00:00Z",
        "subject": format!("worker:{}", uuid::Uuid::new_v4()),
    });
    if data {
        body["data"] = json!({
            "person_ref": person_ref,
            "organization_ref": ORG,
            "effective_on": "2026-11-02",
        });
        if kind != "worker_hired" {
            body["data"]["from"] = json!("active");
            body["data"]["to"] = json!("terminated");
        }
    }
    body.to_string()
}

/// Post `body` signed at `at` (Unix seconds) with `secret`; expands to the response.
macro_rules! deliver {
    ($request:expr, $body:expr, $secret:expr, $at:expr) => {{
        let body: String = $body;
        let signature = sign($secret.as_bytes(), $at, body.as_bytes());
        $request
            .post("/api/bridges/wpm/events")
            .add_header("content-type", "application/json")
            .add_header("x-wpm-event", "worker_hired")
            .add_header("x-wpm-delivery", uuid::Uuid::new_v4().to_string())
            .add_header("x-wpm-signature", signature)
            .bytes(body.into())
            .await
    }};
    ($request:expr, $body:expr) => {
        deliver!($request, $body, SECRET, chrono::Utc::now().timestamp())
    };
}

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn without_a_secret_the_route_is_not_there() {
    let _restore = Restore;
    settings::install(Settings::default());
    request::<App, _, _>(|request, _ctx| async move {
        let response = deliver!(&request, event("e0", "worker_hired", &a_person(), true));
        assert_eq!(response.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_bad_missing_or_stale_signature_is_refused_and_changes_nothing() {
    let _restore = Restore;
    settings::update(|s| s.wpm_webhook_secret = Some(SECRET.to_string()));
    request::<App, _, _>(|request, ctx| async move {
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let detail: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        let person = detail["contact"]["person_ref"]
            .as_str()
            .unwrap()
            .to_string();
        let body = event("e-bad", "worker_hired", &person, true);
        let now = chrono::Utc::now().timestamp();

        let unsigned = request
            .post("/api/bridges/wpm/events")
            .add_header("content-type", "application/json")
            .bytes(body.clone().into())
            .await;
        assert_eq!(unsigned.status_code(), 401);
        let wrong = deliver!(
            &request,
            body.clone(),
            "another-secret-0123456789abcdef-xyz",
            now
        );
        assert_eq!(wrong.status_code(), 401);
        let stale = deliver!(&request, body.clone(), SECRET, now - TOLERANCE_SECS - 30);
        assert_eq!(stale.status_code(), 401);
        let future = deliver!(&request, body.clone(), SECRET, now + TOLERANCE_SECS + 30);
        assert_eq!(future.status_code(), 401);
        // A body changed after signing.
        let signature = sign(SECRET.as_bytes(), now, body.as_bytes());
        let changed = request
            .post("/api/bridges/wpm/events")
            .add_header("x-wpm-signature", signature)
            .bytes(body.replace("e-bad", "e-other").into())
            .await;
        assert_eq!(changed.status_code(), 401);

        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM wpm_events").await,
            0
        );
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contact_employments").await,
            0
        );

        // Too large a body is refused before it is read.
        let big = format!(
            r#"{{"id":"e-big","kind":"worker_hired","pad":"{}"}}"#,
            "x".repeat(20_000)
        );
        assert_eq!(deliver!(&request, big).status_code(), 413);
        // Signed but not an event.
        assert_eq!(deliver!(&request, "[1,2]".to_string()).status_code(), 400);
        // A hire whose data is wrong.
        let mut bad = serde_json::from_str::<Value>(&body).unwrap();
        bad["id"] = json!("e-bad-data");
        bad["data"]["person_ref"] = json!(ORG);
        assert_eq!(deliver!(&request, bad.to_string()).status_code(), 422);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM wpm_events").await,
            0
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_hire_pauses_marketing_once_and_a_leave_restores_it_under_the_old_consent() {
    let _restore = Restore;
    settings::update(|s| s.wpm_webhook_secret = Some(SECRET.to_string()));
    request::<App, _, _>(|request, ctx| async move {
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        let person_of = |detail: &Value| {
            detail["contact"]["person_ref"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let ada_detail: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        let ada_person = person_of(&ada_detail);
        assert_eq!(ada_detail["employments"], json!([]));
        let grace_detail: Value = request.get(&format!("/api/contacts/{grace}")).await.json();
        let grace_person = person_of(&grace_detail);

        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let segment_pid = segment["pid"].as_str().unwrap().to_string();
        let preview = || request.get(&format!("/api/segments/{segment_pid}/preview"));
        assert_eq!(preview().await.json::<Value>()["count"], 2);

        // An older WPM sends no data: received, acknowledged, nothing changes.
        let old: Value =
            deliver!(&request, event("e-old", "worker_hired", &ada_person, false)).json();
        assert_eq!(
            old,
            json!({ "result": "ignored", "reason": "no_data", "contacts": 0 })
        );
        // Another kind, known or not.
        let other: Value = deliver!(
            &request,
            event("e-act", "worker_activated", &ada_person, true)
        )
        .json();
        assert_eq!(other["reason"], "not_used");
        let unknown: Value = deliver!(
            &request,
            event("e-new", "worker_promoted", &ada_person, true)
        )
        .json();
        assert_eq!(unknown["result"], "ignored");
        assert_eq!(unknown["reason"], "unknown_kind");
        assert_eq!(preview().await.json::<Value>()["count"], 2);

        // Grace is hired: her marketing pauses, Ada's does not.
        let hired: Value = deliver!(
            &request,
            event("e-hire", "worker_hired", &grace_person, true)
        )
        .json();
        assert_eq!(
            hired,
            json!({ "result": "paused", "reason": null, "contacts": 1 })
        );
        let detail: Value = request.get(&format!("/api/contacts/{grace}")).await.json();
        assert_eq!(
            detail["employments"],
            json!([{ "organization_ref": ORG, "since": "2026-11-02" }])
        );
        assert_eq!(preview().await.json::<Value>()["count"], 1);

        // The same event again (WPM retries with a new signature): nothing changes.
        let again: Value = deliver!(
            &request,
            event("e-hire", "worker_hired", &grace_person, true)
        )
        .json();
        assert_eq!(again["result"], "duplicate");
        // A second hire event for the same employment keeps the first record.
        let second: Value = deliver!(
            &request,
            event("e-hire-2", "worker_hired", &grace_person, true)
        )
        .json();
        assert_eq!(second["result"], "already_paused");
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contact_employments").await,
            1
        );
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE action = 'employment_recorded' \
                 AND snapshot->>'via' = 'wpm' AND snapshot->>'event_id' = 'e-hire'"
            )
            .await,
            1
        );
        // A person the CRM has no contact for: nothing is kept about them.
        let stranger: Value =
            deliver!(&request, event("e-x", "worker_hired", &a_person(), true)).json();
        assert_eq!(stranger["result"], "no_contact");
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contact_employments").await,
            1
        );

        // A campaign skips her, saying why; a nurture enrolment is held, not ended.
        let campaign: Value = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Autumn", "segment_pid": segment_pid }))
            .await
            .json();
        let campaign_pid = campaign["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/campaigns/{campaign_pid}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status_ok();
        let ran: Value = request
            .post(&format!("/api/campaigns/{campaign_pid}/run"))
            .await
            .json();
        assert_eq!(ran["recipients"], 1, "only Ada");
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({
                "name": "Welcome",
                "steps": [{ "delay_hours": 0, "template_ref": "welcome-1" }],
            }))
            .await
            .json();
        let sequence_pid = sequence["pid"].as_str().unwrap().to_string();
        request
            .post(&format!(
                "/api/nurture-sequences/{sequence_pid}/enrollments"
            ))
            .json(&json!({ "contact_pid": grace }))
            .await
            .assert_status_ok();
        let advanced: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(advanced["sent"], 0);
        assert_eq!(advanced["held"], 1);

        // She leaves: the pause lifts and the held step goes, under the consent she had.
        let left: Value = deliver!(
            &request,
            event("e-leave", "worker_terminated", &grace_person, true)
        )
        .json();
        assert_eq!(
            left,
            json!({ "result": "resumed", "reason": null, "contacts": 1 })
        );
        let detail: Value = request.get(&format!("/api/contacts/{grace}")).await.json();
        assert_eq!(detail["employments"], json!([]));
        assert_eq!(detail["contact"]["marketing_consent"], "granted");
        assert_eq!(preview().await.json::<Value>()["count"], 2);
        let advanced: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(advanced["sent"], 1);
        let retired: Value = deliver!(
            &request,
            event("e-retire", "worker_retired", &grace_person, true)
        )
        .json();
        assert_eq!(retired["result"], "not_paused");
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE action = 'employment_ended' \
                 AND snapshot->>'via' = 'wpm'"
            )
            .await,
            1
        );

        // Leaving never grants consent that was not there.
        request
            .post(&format!("/api/contacts/{ada}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "unsubscribe link" }))
            .await
            .assert_status_ok();
        deliver!(
            &request,
            event("e-hire-ada", "worker_hired", &ada_person, true)
        )
        .assert_status_ok();
        deliver!(
            &request,
            event("e-leave-ada", "worker_retired", &ada_person, true)
        )
        .assert_status_ok();
        let detail: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        assert_eq!(detail["contact"]["marketing_consent"], "withdrawn");
        assert_eq!(preview().await.json::<Value>()["count"], 1, "Grace only");

        // Every event id was kept once, with what it did.
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM wpm_events WHERE outcome = 'duplicate'"
            )
            .await,
            0
        );
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM wpm_events").await,
            10
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn erasing_a_contact_deletes_what_wpm_said_of_them() {
    let _restore = Restore;
    settings::update(|s| s.wpm_webhook_secret = Some(SECRET.to_string()));
    request::<App, _, _>(|request, ctx| async move {
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let detail: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        let person = detail["contact"]["person_ref"]
            .as_str()
            .unwrap()
            .to_string();
        deliver!(&request, event("e-erase", "worker_hired", &person, true)).assert_status_ok();
        let export: Value = request
            .get(&format!("/api/contacts/{ada}/subject-access"))
            .await
            .json();
        assert_eq!(export["employments"][0]["organization_ref"], ORG);
        request
            .post(&format!("/api/contacts/{ada}/erase"))
            .await
            .assert_status_ok();
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE action = 'erased' \
                 AND (snapshot->>'employments_deleted')::int = 1"
            )
            .await,
            1
        );
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contact_employments").await,
            0
        );
    })
    .await;
}
