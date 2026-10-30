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

use super::{a_person, a_user, seed_pipeline};

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

/// The worker fields WPM's list lets a caller name (WPM-R216, `rules::worker_fields::LIST_FIELDS`).
const WPM_LIST_FIELDS: [&str; 20] = [
    "pid",
    "person_ref",
    "upstream_worker_ref",
    "organization_ref",
    "worker_number",
    "display_name",
    "status",
    "employment_type",
    "fte_percent",
    "department",
    "job_title",
    "location",
    "manager_pid",
    "salary_minor",
    "salary_currency",
    "hired_on",
    "terminated_on",
    "engagement_ends_on",
    "created_at",
    "updated_at",
];

/// A stand-in WPM that lists `workers` 200 at a time, as the real one pages by `offset`. As WPM
/// does since WPM-R216, `fields` keeps only the named fields of each row and an unknown name is
/// `422`; with `lean: false` it is a WPM from before that, which refuses `fields` outright.
async fn fake_wpm_with(workers: Vec<Value>, lean: bool) -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route(
            "/api/workers",
            get(
                move |State((seen, workers)): State<(Seen, Arc<Vec<Value>>)>,
                      Query(q): Query<HashMap<String, String>>| async move {
                    seen.lock().unwrap().push(json!(q));
                    let fields: Option<Vec<String>> = q
                        .get("fields")
                        .map(|f| f.split(',').map(|n| n.trim().to_string()).collect());
                    if let Some(names) = &fields
                        && (!lean || names.iter().any(|n| !WPM_LIST_FIELDS.contains(&n.as_str())))
                    {
                        return (
                            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                            Json(json!({})),
                        );
                    }
                    let offset: usize = q["offset"].parse().unwrap();
                    let limit: usize = q["limit"].parse().unwrap();
                    let page = workers
                        .iter()
                        .skip(offset)
                        .take(limit)
                        .map(|w| match &fields {
                            None => w.clone(),
                            Some(names) => Value::Object(
                                w.as_object()
                                    .unwrap()
                                    .iter()
                                    .filter(|(k, _)| names.contains(k))
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                            ),
                        })
                        .collect::<Vec<_>>();
                    (axum::http::StatusCode::OK, Json(json!(page)))
                },
            ),
        )
        .with_state((seen.clone(), Arc::new(workers)));
    (serve(app).await, seen)
}

/// A stand-in WPM that supports `fields` (WPM-R216).
async fn fake_wpm(workers: Vec<Value>) -> (String, Seen) {
    fake_wpm_with(workers, true).await
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

/// A worker as WPM holds one: the whole record, pay included (CRM-D17). The stand-in sends only the
/// fields asked for, as WPM does since WPM-R216.
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
            worker(
                Some(&format!("worker:{}", uuid::Uuid::new_v4())),
                Some("Support"),
            ),
            worker(Some("person:nope"), None),
        ]);
        // A team set by hand with a legacy `worker:` member: stored as `person:`, same id (CRM-D18).
        let legacy = a_user();
        let set: Value = request
            .put("/api/quotas/teams")
            .json(
                &json!({ "name": "Legacy", "members": [legacy.replacen("person:", "worker:", 1)] }),
            )
            .await
            .json();
        assert_eq!(set["members"], json!([legacy]), "{set}");
        // A team an earlier import set: the next import replaces its members.
        let old_member = a_user();
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
        // Nothing but the references reaches the answer: no pay, name or job from WPM's record.
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
        // Only the three fields the import reads were asked for (WPM-R216), on every page.
        assert!(
            asked
                .iter()
                .all(|q| q["fields"] == "pid,department,person_ref"),
            "{asked:?}"
        );
        let listed: Value = request.get("/api/quotas/teams").await.json();
        let all: Vec<&str> = listed
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(all, ["Legacy", "Sales", "Support"], "{listed}");
        // The hand-set team keeps its member; the imported one no longer holds the old one.
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

/// WPM-R216: a WPM from before the lean list answers `fields` with `422`. The import then fails with
/// that status and changes no team; it never falls back to asking for the whole record.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_worker_service_without_the_lean_list_fails_the_import_and_changes_nothing() {
    request::<App, _, _>(|request, _ctx| async move {
        let _restore = Restore;
        let kept = a_user();
        request
            .put("/api/quotas/teams")
            .json(&json!({ "name": "Sales", "members": [kept] }))
            .await
            .assert_status_ok();
        let (base, seen) =
            fake_wpm_with(vec![worker(Some(&a_person()), Some("Sales"))], false).await;
        settings::update(|s| {
            s.upstream_mode = Mode::Http;
            s.worker_service_url = Some(base.clone());
        });
        let failed = request.post("/api/bridges/wpm/teams-import").await;
        failed.assert_status(axum::http::StatusCode::BAD_GATEWAY);
        assert!(failed.text().contains("422"), "{}", failed.text());
        // One call, with `fields`, and no retry without it.
        let asked = seen.lock().unwrap().clone();
        assert_eq!(asked.len(), 1, "{asked:?}");
        assert_eq!(asked[0]["fields"], "pid,department,person_ref");
        let listed: Value = request.get("/api/quotas/teams").await.json();
        let sales = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "Sales")
            .unwrap_or_else(|| panic!("{listed}"))
            .clone();
        assert_eq!(sales["members"], json!([kept]), "{listed}");
    })
    .await;
}

/// CRM-D18: a team the WPM import sets names its members by `person_ref`, and quotas and deal
/// owners name the same people by `person:`, so the team's roll-up (CRM-R67) meets their quotas.
/// A quota and a deal owner given in the legacy `worker:` form are stored as `person:` and count.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_imported_teams_roll_up_meets_the_quotas_set_for_the_same_people() {
    request::<App, _, _>(|request, _ctx| async move {
        let _restore = Restore;
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let (ada, grace, idle) = (a_person(), a_person(), a_person());
        let legacy = |person: &str| person.replacen("person:", "worker:", 1);
        for (owner, name, amount) in [
            (ada.clone(), "Ada's", 30_000),
            (legacy(&grace), "Grace's", 20_000),
        ] {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": "GBP", "owner_ref": owner }),
                )
                .await
                .json();
            request
                .post(&format!(
                    "/api/deals/{}/stage",
                    deal["pid"].as_str().unwrap()
                ))
                .json(&json!({ "stage_pid": stages[2] }))
                .await
                .assert_status_ok();
        }
        let today = chrono::Utc::now().date_naive();
        let (from, to) = (
            today - chrono::Duration::days(10),
            today + chrono::Duration::days(80),
        );
        for owner in [ada.clone(), legacy(&grace)] {
            let quota: Value = request
                .put("/api/quotas")
                .json(
                    &json!({ "owner_ref": owner, "period_from": from, "period_to": to,
                               "currency": "GBP", "amount_minor": 100_000 }),
                )
                .await
                .json();
            assert!(
                quota["owner_ref"]
                    .as_str()
                    .is_some_and(|o| o.starts_with("person:")),
                "{quota}"
            );
            // Who set it is the caller as a person; with sign-in off, the all-zero one.
            assert_eq!(
                quota["set_by"], "person:00000000-0000-0000-0000-000000000000",
                "{quota}"
            );
        }
        // Filtering by the legacy form finds the same quota.
        let listed: Value = request
            .get(&format!("/api/quotas?owner_ref={}", legacy(&grace)))
            .await
            .json();
        assert_eq!(listed.as_array().unwrap().len(), 1, "{listed}");
        assert_eq!(listed[0]["owner_ref"], grace.as_str(), "{listed}");

        let (base, _seen) = fake_wpm(vec![
            worker(Some(&ada), Some("Sales")),
            worker(Some(&grace), Some("Sales")),
            worker(Some(&idle), Some("Sales")),
        ])
        .await;
        settings::update(|s| {
            s.upstream_mode = Mode::Http;
            s.worker_service_url = Some(base.clone());
        });
        let done: Value = request.post("/api/bridges/wpm/teams-import").await.json();
        let mut members = vec![ada.clone(), grace.clone(), idle.clone()];
        members.sort();
        assert_eq!(done["teams"][0]["members"], json!(members), "{done}");

        // Grace's deal was given an owner in the legacy form; it counts, so it is hers as a person.
        let report: Value = request.get("/api/quotas/team-attainment").await.json();
        let row = report["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "Sales")
            .unwrap_or_else(|| panic!("{report}"))
            .clone();
        let gbp = &row["currencies"].as_array().unwrap()[0];
        assert_eq!(gbp["quota_minor"], 200_000, "{row}");
        assert_eq!(gbp["revenue_minor"], 50_000, "{row}");
        assert_eq!(gbp["workers"], 2, "{row}");
        assert_eq!(row["without_quota"], json!([idle]), "{row}");

        // The imported team can be saved again by hand, as it is.
        request
            .put("/api/quotas/teams")
            .json(&json!({ "name": "Sales", "members": members }))
            .await
            .assert_status_ok();
        // Naming one person in both forms is naming them twice.
        request
            .put("/api/quotas/teams")
            .json(&json!({ "name": "Sales", "members": [ada.clone(), legacy(&ada)] }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
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
