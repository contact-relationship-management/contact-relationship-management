//! The bridges to PPM and WPM (CRM-R72, CRM-R73): stub-first, then over HTTP against an in-process
//! stand-in for each service, since neither is available to run here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use contact_relationship_management::app::App;
use contact_relationship_management::clients::Mode;
use contact_relationship_management::settings::{self, Settings};
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_worker, seed_pipeline};

/// Puts the process-wide settings back when a test ends, even by panic.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        settings::install(Settings::default());
    }
}

type Seen = Arc<Mutex<Vec<Value>>>;

/// A stand-in PPM that records each idea it is sent and answers a fresh id.
async fn fake_ppm() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route(
            "/api/ideas",
            post(
                |State(seen): State<Seen>, Json(body): Json<Value>| async move {
                    seen.lock().unwrap().push(body);
                    Json(json!({ "pid": uuid::Uuid::new_v4().to_string() }))
                },
            ),
        )
        .with_state(seen.clone());
    (serve(app).await, seen)
}

/// A stand-in WPM that lists `workers` 200 at a time, as the real one pages by `offset`.
async fn fake_wpm(workers: Vec<Value>) -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route(
            "/api/workers",
            get(
                |State((seen, workers)): State<(Seen, Arc<Vec<Value>>)>,
                 Query(q): Query<HashMap<String, String>>| async move {
                    seen.lock().unwrap().push(json!(q));
                    let offset: usize = q["offset"].parse().unwrap();
                    let limit: usize = q["limit"].parse().unwrap();
                    Json(
                        workers
                            .iter()
                            .skip(offset)
                            .take(limit)
                            .cloned()
                            .collect::<Vec<_>>(),
                    )
                },
            ),
        )
        .with_state((seen.clone(), Arc::new(workers)));
    (serve(app).await, seen)
}

/// A stand-in that records the `Authorization` header of every call it gets.
async fn fake_that_records_credentials() -> (String, Arc<Mutex<Vec<Option<String>>>>) {
    use axum::http::HeaderMap;
    let seen: Arc<Mutex<Vec<Option<String>>>> = Arc::default();
    let record = |State(seen): State<Arc<Mutex<Vec<Option<String>>>>>, headers: HeaderMap| async move {
        seen.lock().unwrap().push(
            headers
                .get("authorization")
                .map(|v| v.to_str().unwrap().to_string()),
        );
    };
    let app = Router::new()
        .route(
            "/api/ideas",
            post(move |s: State<_>, h: HeaderMap| async move {
                record(s, h).await;
                Json(json!({ "pid": uuid::Uuid::new_v4().to_string() }))
            }),
        )
        .route(
            "/api/workers",
            get(move |s: State<_>, h: HeaderMap| async move {
                record(s, h).await;
                Json(json!([]))
            }),
        )
        .with_state(seen.clone());
    (serve(app).await, seen)
}

async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    url
}

/// A worker as WPM's list serialises one: the whole record, pay included (CRM-D17).
fn worker(person_ref: Option<&str>, department: Option<&str>) -> Value {
    json!({ "pid": uuid::Uuid::new_v4().to_string(), "person_ref": person_ref,
            "display_name": "Someone", "status": "active", "job_title": "Rep",
            "department": department, "salary_minor": 9_999_999, "salary_currency": "EUR" })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_won_deal_opens_one_delivery_idea_in_ppm_and_only_over_http() {
    request::<App, _, _>(|request, ctx| async move {
        let _restore = Restore;
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let deal = |win: bool| {
            let (request, pipeline, stage) = (&request, pipeline.clone(), stages[2].clone());
            async move {
                let made: Value = request
                    .post("/api/deals")
                    .json(&json!({ "name": "Fleet renewal", "pipeline_pid": pipeline,
                                   "amount_minor": 88_000, "currency": "GBP" }))
                    .await
                    .json();
                let pid = made["pid"].as_str().unwrap().to_string();
                if win {
                    request
                        .post(&format!("/api/deals/{pid}/stage"))
                        .json(&json!({ "stage_pid": stage }))
                        .await
                        .assert_status_ok();
                }
                pid
            }
        };
        let (won, open) = (deal(true).await, deal(false).await);
        let url = |pid: &str| format!("/api/bridges/ppm/deals/{pid}");
        let recorded = || {
            let db = &ctx.db;
            async move {
                use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
                db.query_one_raw(Statement::from_string(
                    DatabaseBackend::Postgres,
                    "SELECT count(*) AS n FROM deal_deliveries",
                ))
                .await
                .unwrap()
                .unwrap()
                .try_get::<i64>("", "n")
                .unwrap()
            }
        };

        // The stub sends nothing and records nothing, and says what it would have sent.
        let dry: Value = request.post(&url(&won)).await.json();
        assert_eq!(
            (dry["mode"].as_str(), dry["sent"].as_bool()),
            (Some("stub"), Some(false))
        );
        assert_eq!(dry["idea"]["title"], "Deliver: Fleet renewal", "{dry}");
        assert!(dry["ppm_idea_pid"].is_null(), "{dry}");
        assert_eq!(recorded().await, 0);

        // Only a won deal; an unknown one is 404.
        request
            .post(&url(&open))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        request
            .post(&url(&uuid::Uuid::new_v4().to_string()))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);

        // An unreachable PPM is a 502 and nothing is recorded.
        settings::update(|s| {
            s.upstream_mode = Mode::Http;
            s.ppm_service_url = Some("http://127.0.0.1:1".to_string());
        });
        request
            .post(&url(&won))
            .await
            .assert_status(axum::http::StatusCode::BAD_GATEWAY);
        assert_eq!(recorded().await, 0);

        // Over HTTP the idea is sent once, with the deal's name and amount and no more.
        let (base, seen) = fake_ppm().await;
        settings::update(|s| s.ppm_service_url = Some(base.clone()));
        let sent: Value = request.post(&url(&won)).await.json();
        assert_eq!(
            (sent["mode"].as_str(), sent["sent"].as_bool()),
            (Some("http"), Some(true))
        );
        let idea = sent["ppm_idea_pid"].as_str().unwrap().to_string();
        assert_eq!(recorded().await, 1);
        let ideas = seen.lock().unwrap().clone();
        assert_eq!(ideas.len(), 1, "{ideas:?}");
        assert_eq!(ideas[0]["title"], "Deliver: Fleet renewal");
        assert!(
            ideas[0]["pitch"]
                .as_str()
                .unwrap()
                .contains("88000 minor units of GBP")
        );
        assert_eq!(ideas[0]["tags"], json!(["crm", "won-deal"]));

        // Asking again answers the same idea and sends nothing.
        let again: Value = request.post(&url(&won)).await.json();
        assert_eq!(again["already_opened"], true, "{again}");
        assert_eq!(again["ppm_idea_pid"], idea.as_str(), "{again}");
        assert_eq!(seen.lock().unwrap().len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_worker_service_gives_a_team_per_department_and_leaves_the_rest_alone() {
    request::<App, _, _>(|request, _ctx| async move {
        let _restore = Restore;
        let (a, b, c, late) = (a_person(), a_person(), a_person(), a_person());
        // 200 in one big department fill the first page, so the rest are only read from the second.
        let mut workers: Vec<Value> = (0..200)
            .map(|_| worker(Some(&a_person()), Some("Bulk")))
            .collect();
        workers.extend([
            worker(Some(&a), Some("Sales")),
            worker(Some(&b), Some("Sales")),
            worker(Some(&c), Some("Support")),
            worker(Some(&a_person()), None),
            worker(Some(&late), Some("sales")),
            // No person, a worker reference and a malformed one: counted, never a member.
            worker(None, Some("Support")),
            worker(Some(&a_worker()), Some("Support")),
            worker(Some("person:nope"), None),
        ]);
        let legacy = a_worker();
        request
            .put("/api/quotas/teams")
            .json(&json!({ "name": "Legacy", "members": [legacy] }))
            .await
            .assert_status_ok();
        // A team an earlier import set by `worker:` reference: the next import replaces its members.
        let old_member = a_worker();
        request
            .put("/api/quotas/teams")
            .json(&json!({ "name": "Support", "members": [old_member] }))
            .await
            .assert_status_ok();

        // The stub reads nothing.
        let dry: Value = request.post("/api/bridges/wpm/teams-import").await.json();
        assert_eq!(dry["mode"], "stub", "{dry}");
        assert_eq!(dry["teams"].as_array().unwrap().len(), 0);

        let (base, seen) = fake_wpm(workers).await;
        settings::update(|s| {
            s.upstream_mode = Mode::Http;
            s.worker_service_url = Some(base.clone());
        });
        let done: Value = request.post("/api/bridges/wpm/teams-import").await.json();
        assert_eq!(done["mode"], "http", "{done}");
        let names: Vec<&str> = done["teams"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["Sales", "Support"], "{done}");
        let mut sales = vec![a.clone(), b.clone()];
        sales.sort();
        assert_eq!(done["teams"][0]["members"], json!(sales), "{done}");
        assert_eq!(done["teams"][1]["members"], json!([c]), "{done}");
        assert_eq!(done["without_department"], 1, "{done}");
        assert_eq!(done["without_person_ref"], 3, "{done}");
        // Nothing but the references is kept: no pay, name or job from WPM's record.
        let text = done.to_string();
        for leaked in ["9999999", "Someone", "\"Rep\"", "EUR"] {
            assert!(!text.contains(leaked), "{leaked} in {done}");
        }
        // Skipped, each with its reason: 200 is too many for a team, and "sales" is "Sales" in
        // another case.
        let skipped: Vec<String> = done["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["reason"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(skipped.len(), 2, "{done}");
        assert!(
            skipped.iter().any(|r| r.contains("more than 50")),
            "{skipped:?}"
        );
        assert!(
            skipped.iter().any(|r| r.contains("another case")),
            "{skipped:?}"
        );

        // Both pages were read, only active workers were asked for, and a team nobody named stays.
        let asked = seen.lock().unwrap().clone();
        assert_eq!(asked.len(), 2, "{asked:?}");
        assert_eq!(asked[1]["offset"], "200");
        assert!(asked.iter().all(|q| q["status"] == "active"));
        let listed: Value = request.get("/api/quotas/teams").await.json();
        let all: Vec<&str> = listed
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(all, ["Legacy", "Sales", "Support"], "{listed}");
        // The hand-set team keeps its `worker:` member; the imported one no longer holds the old one.
        let members = |name: &str| {
            listed
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["name"] == name)
                .unwrap()["members"]
                .clone()
        };
        assert_eq!(members("Legacy"), json!([legacy]));
        assert_eq!(members("Support"), json!([c]));

        // Run again: the same teams, replaced and not added.
        request
            .post("/api/bridges/wpm/teams-import")
            .await
            .assert_status_ok();
        let again: Value = request.get("/api/quotas/teams").await.json();
        assert_eq!(again.as_array().unwrap().len(), 3, "{again}");

        // An unreachable worker service is a 502 and changes nothing.
        settings::update(|s| s.worker_service_url = Some("http://127.0.0.1:1".to_string()));
        request
            .post("/api/bridges/wpm/teams-import")
            .await
            .assert_status(axum::http::StatusCode::BAD_GATEWAY);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn each_bridge_sends_its_own_token_when_it_has_one_and_none_when_it_has_not() {
    let _restore = Restore;
    let (base, seen) = fake_that_records_credentials().await;
    settings::update(|s| {
        s.ppm_service_url = Some(base.clone());
        s.worker_service_url = Some(base.clone());
    });

    // No tokens: nothing is attached.
    contact_relationship_management::clients::open_delivery_idea("t", "p", &["crm"])
        .await
        .unwrap();
    contact_relationship_management::clients::active_workers()
        .await
        .unwrap();
    assert_eq!(*seen.lock().unwrap(), [None, None]);
    seen.lock().unwrap().clear();

    // Each service gets the token meant for it, and not the other's.
    settings::update(|s| {
        s.ppm_service_token = Some("ppm-token".to_string());
        s.worker_service_token = Some("wpm-token".to_string());
    });
    contact_relationship_management::clients::open_delivery_idea("t", "p", &["crm"])
        .await
        .unwrap();
    contact_relationship_management::clients::active_workers()
        .await
        .unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        [
            Some("Bearer ppm-token".to_string()),
            Some("Bearer wpm-token".to_string())
        ]
    );

    // A token is not sent in the clear to another machine: the call fails and sends nothing.
    seen.lock().unwrap().clear();
    settings::update(|s| s.ppm_service_url = Some("http://ppm.example.com".to_string()));
    let why = contact_relationship_management::clients::open_delivery_idea("t", "p", &[])
        .await
        .unwrap_err();
    assert!(!why.contains("ppm-token"), "{why}");
    assert!(seen.lock().unwrap().is_empty());
}
