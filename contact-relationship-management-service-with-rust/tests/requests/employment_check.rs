//! The employment check against the worker service (CRM-R94, WPM-R208): stub-first, then over
//! HTTP against an in-process stand-in for WPM's `POST /api/bridges/overlap`, which answers only an
//! HR caller, as WPM does. No WPM runs here.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use contact_relationship_management::app::App;
use contact_relationship_management::clients::Mode;
use contact_relationship_management::rules::wpm_events::sign;
use contact_relationship_management::settings::{self, Settings};
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;

const SECRET: &str = "wpm-secret-0123456789abcdef-0123456789";
const ORG: &str = "organization:0c4f1e2a-0000-4000-8000-0000000000aa";
const TOKEN: &str = "hr-token-of-the-organization";

/// Puts the process-wide settings back when a test ends, even by panic.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        settings::install(Settings::default());
    }
}

/// What the stand-in answers, and what it was sent.
#[derive(Default)]
struct Wpm {
    /// The person references that are workers of `ORG` today.
    workers: BTreeSet<String>,
    /// Answer for another organization (a bad answer).
    wrong_org: bool,
    /// Each body received, raw.
    bodies: Vec<String>,
    /// The `Authorization` header of each call.
    credentials: Vec<Option<String>>,
}

type Shared = Arc<Mutex<Wpm>>;

/// A stand-in for WPM's overlap check: HR of the organization only (a bearer token), 1 to 500
/// `person:` references, a yes or no for each distinct one, in the order first seen.
async fn fake_wpm() -> (String, Shared) {
    let shared: Shared = Arc::default();
    let app = Router::new()
        .route(
            "/api/bridges/overlap",
            post(
                |State(shared): State<Shared>, headers: HeaderMap, body: String| async move {
                    let mut wpm = shared.lock().unwrap();
                    let credential = headers
                        .get("authorization")
                        .map(|v| v.to_str().unwrap().to_string());
                    wpm.credentials.push(credential.clone());
                    wpm.bodies.push(body.clone());
                    if credential.as_deref() != Some(&format!("Bearer {TOKEN}")) {
                        return (StatusCode::FORBIDDEN, "the overlap check is for HR")
                            .into_response();
                    }
                    let body: Value = serde_json::from_str(&body).unwrap();
                    let refs: Vec<String> =
                        serde_json::from_value(body["person_refs"].clone()).unwrap();
                    if refs.is_empty() || refs.len() > 500 {
                        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
                    }
                    let mut seen = BTreeSet::new();
                    let results: Vec<Value> = refs
                        .iter()
                        .filter(|r| seen.insert((*r).clone()))
                        .map(|r| json!({ "person_ref": r, "worker": wpm.workers.contains(r) }))
                        .collect();
                    let org = if wpm.wrong_org {
                        "organization:0c4f1e2a-0000-4000-8000-0000000000bb".to_string()
                    } else {
                        body["organization_ref"].as_str().unwrap().to_string()
                    };
                    Json(json!({ "organization_ref": org, "results": results })).into_response()
                },
            ),
        )
        .with_state(shared.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, shared)
}

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

/// A WPM event, signed now.
fn signed(id: &str, kind: &str, person_ref: &str, occurred_at: &str) -> (String, String) {
    let body = json!({
        "id": id, "kind": kind, "occurred_at": occurred_at,
        "subject": format!("worker:{}", uuid::Uuid::new_v4()),
        "data": { "person_ref": person_ref, "organization_ref": ORG, "effective_on": "2026-11-02" },
    })
    .to_string();
    let signature = sign(
        SECRET.as_bytes(),
        chrono::Utc::now().timestamp(),
        body.as_bytes(),
    );
    (body, signature)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_check_marks_the_contacts_wpm_says_are_workers_and_a_no_changes_nothing() {
    let _restore = Restore;
    settings::update(|s| s.wpm_webhook_secret = Some(SECRET.to_string()));
    request::<App, _, _>(|request, ctx| async move {
        let mut people = std::collections::HashMap::new();
        for name in ["Ada Lovelace", "Grace Hopper", "Linus Pauling", "Mary Somerville"] {
            let pid = seed_contact!(&request, name).await;
            let detail: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
            let person = detail["contact"]["person_ref"]
                .as_str()
                .unwrap()
                .to_string();
            people.insert(name, (pid, person));
        }
        let (ada, ada_ref) = people["Ada Lovelace"].clone();
        let (_, grace_ref) = people["Grace Hopper"].clone();
        let (linus, linus_ref) = people["Linus Pauling"].clone();
        let (mary, mary_ref) = people["Mary Somerville"].clone();
        // 520 more contacts, so the check takes two calls; one of them is a worker too.
        ctx.db
            .execute_unprepared(&format!(
                "INSERT INTO contacts (pid, person_ref, display_name, status, preferred_channel, \
                   marketing_consent) \
                 SELECT gen_random_uuid(), 'person:' || gen_random_uuid(), 'Bulk ' || n, status, \
                   preferred_channel, marketing_consent \
                 FROM contacts, generate_series(1, 520) AS n WHERE pid = '{ada}'"
            ))
            .await
            .unwrap();
        let bulk_worker: String = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT person_ref FROM contacts WHERE display_name = 'Bulk 7'",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "person_ref")
            .unwrap();
        // An erased contact is not asked about.
        let gone = seed_contact!(&request, "Erased Person").await;
        request
            .post(&format!("/api/contacts/{gone}/erase"))
            .await
            .assert_status_ok();

        // The stub sends nothing and says what it would ask.
        let stub: Value = request
            .post("/api/bridges/wpm/employment-check")
            .await
            .json();
        assert_eq!(
            stub,
            json!({ "mode": "stub", "organization_ref": null, "asked": 524, "batches": 2,
                    "workers": 0, "recorded": 0, "already_paused": 0, "left_by_event": 0 })
        );

        // Mary is an employee by a hire event; Linus was hired and has left.
        for (id, kind, person, at) in [
            ("m-hire", "worker_hired", &mary_ref, "2026-10-01T09:00:00Z"),
            ("l-hire", "worker_hired", &linus_ref, "2026-09-01T09:00:00Z"),
            ("l-leave", "worker_terminated", &linus_ref, "2026-10-29T09:00:00Z"),
        ] {
            let (body, signature) = signed(id, kind, person, at);
            request
                .post("/api/bridges/wpm/events")
                .add_header("x-wpm-signature", signature)
                .bytes(body.into())
                .await
                .assert_status_ok();
        }

        let (base, wpm) = fake_wpm().await;
        settings::update(|s| {
            s.upstream_mode = Mode::Http;
            s.worker_service_url = Some(base.clone());
        });
        // Without an organization nothing is sent.
        let unset = request.post("/api/bridges/wpm/employment-check").await;
        assert_eq!(unset.status_code(), 502);
        assert!(wpm.lock().unwrap().bodies.is_empty());
        // Without the HR token WPM refuses, and nothing is recorded.
        settings::update(|s| s.wpm_organization_ref = Some(ORG.to_string()));
        wpm.lock().unwrap().workers = BTreeSet::from([
            ada_ref.clone(),
            linus_ref.clone(),
            mary_ref.clone(),
            bulk_worker.clone(),
        ]);
        let refused = request.post("/api/bridges/wpm/employment-check").await;
        assert_eq!(refused.status_code(), 502);
        assert_eq!(wpm.lock().unwrap().credentials, [None]);
        // An answer for another organization records nothing.
        settings::update(|s| s.worker_service_token = Some(TOKEN.to_string()));
        wpm.lock().unwrap().wrong_org = true;
        let wrong = request.post("/api/bridges/wpm/employment-check").await;
        assert_eq!(wrong.status_code(), 502);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contact_employments").await,
            1,
            "only Mary's, from her hire event"
        );
        {
            let mut w = wpm.lock().unwrap();
            w.wrong_org = false;
            w.bodies.clear();
            w.credentials.clear();
        }

        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let segment_pid = segment["pid"].as_str().unwrap().to_string();
        let preview = |pid: &str| {
            let url = format!("/api/segments/{pid}/preview");
            let request = &request;
            async move { request.get(&url).await.json::<Value>()["count"].as_i64().unwrap() }
        };
        let before = preview(&segment_pid).await;
        let done: Value = request
            .post("/api/bridges/wpm/employment-check")
            .await
            .json();
        assert_eq!(
            done,
            json!({ "mode": "http", "organization_ref": ORG, "asked": 524, "batches": 2,
                    "workers": 4, "recorded": 2, "already_paused": 1, "left_by_event": 1 })
        );
        {
            let w = wpm.lock().unwrap();
            let sizes: Vec<usize> = w
                .bodies
                .iter()
                .map(|b| {
                    let body: Value = serde_json::from_str(b).unwrap();
                    let keys: Vec<&String> = body.as_object().unwrap().keys().collect();
                    assert_eq!(keys, ["organization_ref", "person_refs"], "references only");
                    assert_eq!(body["organization_ref"], ORG);
                    body["person_refs"].as_array().unwrap().len()
                })
                .collect();
            assert_eq!(sizes, [500, 24]);
            for body in &w.bodies {
                assert!(!body.contains("Lovelace") && !body.contains("Bulk"), "no names");
                assert!(!body.contains("00000000-0000-0000-0000-000000000000"));
            }
            assert!(
                w.credentials
                    .iter()
                    .all(|c| c.as_deref() == Some(&format!("Bearer {TOKEN}")))
            );
        }
        // Ada is now an employee since today, and out of marketing; Linus, who left, is not.
        let today = chrono::Utc::now().date_naive().to_string();
        let detail: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        assert_eq!(
            detail["employments"],
            json!([{ "organization_ref": ORG, "since": today }])
        );
        let detail: Value = request.get(&format!("/api/contacts/{linus}")).await.json();
        assert_eq!(detail["employments"], json!([]));
        let after = preview(&segment_pid).await;
        assert_eq!(before - after, 2, "Ada and the bulk worker are paused now");
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE action = 'employment_recorded' \
                 AND snapshot->>'via' = 'wpm' AND snapshot->>'source' = 'employment_check' \
                 AND snapshot->>'organization_ref' = 'organization:0c4f1e2a-0000-4000-8000-0000000000aa'"
            )
            .await,
            2
        );
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE action = 'employment_checked' \
                 AND (snapshot->>'asked')::int = 524 AND (snapshot->>'recorded')::int = 2 \
                 AND snapshot::text NOT LIKE '%person:%'"
            )
            .await,
            1
        );
        // The check writes no order: that is for events (CRM-R93).
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM employment_last_events").await,
            2,
            "Mary's and Linus's, from their events"
        );

        // A "no" changes nothing: Mary's employment from her hire event stays, and so do the rest.
        wpm.lock().unwrap().workers.clear();
        let none: Value = request
            .post("/api/bridges/wpm/employment-check")
            .await
            .json();
        assert_eq!(none["workers"], 0);
        assert_eq!(none["recorded"], 0);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contact_employments").await,
            3
        );
        // Asked again with the same workers, nothing new.
        wpm.lock().unwrap().workers =
            BTreeSet::from([ada_ref.clone(), mary_ref.clone(), bulk_worker.clone()]);
        let again: Value = request
            .post("/api/bridges/wpm/employment-check")
            .await
            .json();
        assert_eq!(again["recorded"], 0);
        assert_eq!(again["already_paused"], 3);

        // A leave event ends an employment the check recorded, as it would one from a hire.
        let (body, signature) = signed("a-leave", "worker_retired", &ada_ref, "2026-10-30T09:00:00Z");
        let left: Value = request
            .post("/api/bridges/wpm/events")
            .add_header("x-wpm-signature", signature)
            .bytes(body.into())
            .await
            .json();
        assert_eq!(left["result"], "resumed");
        let _ = (grace_ref, mary);
    })
    .await;
}
