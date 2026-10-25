//! The report builder (CRM-R48): counts and per-currency sums grouped by a field, filters and a
//! date range, what is refused, and saved reports.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{seed_contact, seed_pipeline};

macro_rules! run {
    ($request:expr, $spec:expr) => {{
        let r = $request.post("/api/reports/run").json(&$spec).await;
        r.assert_status_ok();
        r.json::<Value>()
    }};
}

fn rows(result: &Value) -> Vec<(String, i64)> {
    result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["group"].as_str().unwrap().to_string(),
                r["value"].as_i64().unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn records_are_counted_by_a_field_a_tag_and_a_date_range() {
    request::<App, _, _>(|request, _ctx| async move {
        let one = seed_contact!(&request, "Report One").await;
        let two = seed_contact!(&request, "Report Two").await;
        // A third has never consented.
        request
            .post("/api/contacts")
            .json(&json!({ "person_ref": super::a_person(), "display_name": "Report Three" }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/contacts/{one}/tags"))
            .json(&json!({ "tags": ["vip", "press"] }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/contacts/{two}/tags"))
            .json(&json!({ "tags": ["vip"] }))
            .await
            .assert_status_ok();

        let by_consent = run!(
            &request,
            json!({ "kind": "contact", "group_by": "marketing_consent" })
        );
        assert_eq!(
            rows(&by_consent),
            [("granted".to_string(), 2), ("never".to_string(), 1)]
        );
        assert!(by_consent["as_of"].is_string());
        assert_eq!(by_consent["truncated"], false);

        // A record with two tags is in both groups, and one with none in "(no tag)".
        let by_tag = run!(&request, json!({ "kind": "contact", "group_by": "tag" }));
        assert_eq!(
            rows(&by_tag),
            [
                ("vip".to_string(), 2),
                ("(no tag)".to_string(), 1),
                ("press".to_string(), 1)
            ]
        );

        // A filter on a field, and on a tag (any case).
        let granted_vip = run!(
            &request,
            json!({ "kind": "contact", "group_by": "status",
                    "filter": { "marketing_consent": ["granted"], "tag": ["VIP"] } })
        );
        assert_eq!(rows(&granted_vip), [("active".to_string(), 2)]);
        let only_press = run!(
            &request,
            json!({ "kind": "contact", "group_by": "status", "filter": { "tag": ["press"] } })
        );
        assert_eq!(rows(&only_press), [("active".to_string(), 1)]);

        // A date range: today is in, last year is out.
        let today = chrono::Utc::now().date_naive();
        let in_range = run!(
            &request,
            json!({ "kind": "contact", "group_by": "status", "from": today, "to": today })
        );
        assert_eq!(rows(&in_range), [("active".to_string(), 3)]);
        let last_year = today - chrono::Duration::days(400);
        let out = run!(
            &request,
            json!({ "kind": "contact", "group_by": "status", "to": last_year })
        );
        assert!(rows(&out).is_empty());

        // Months.
        let by_month = run!(
            &request,
            json!({ "kind": "contact", "group_by": "created_month" })
        );
        assert_eq!(rows(&by_month), [(today.format("%Y-%m").to_string(), 3)]);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn deal_amounts_are_summed_per_currency_and_never_added_across_them() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let deal = |name: &'static str, amount: i64, currency: &'static str| {
            let request = &request;
            let pipeline = pipeline.clone();
            async move {
                request
                    .post("/api/deals")
                    .json(&json!({ "name": name, "pipeline_pid": pipeline,
                                   "amount_minor": amount, "currency": currency }))
                    .await
                    .json::<Value>()["pid"]
                    .as_str()
                    .unwrap()
                    .to_string()
            }
        };
        deal("A", 10_000, "GBP").await;
        deal("B", 25_000, "GBP").await;
        let usd = deal("C", 5_000, "USD").await;
        request
            .post(&format!("/api/deals/{usd}/stage"))
            .json(&json!({ "stage_pid": stages[1] }))
            .await
            .assert_status_ok();

        let sums = run!(
            &request,
            json!({ "kind": "deal", "group_by": "state", "measure": "sum_amount" })
        );
        let lines: Vec<(String, String, i64)> = sums["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r["group"].as_str().unwrap().to_string(),
                    r["currency"].as_str().unwrap().to_string(),
                    r["value"].as_i64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            lines,
            [
                ("open".to_string(), "GBP".to_string(), 35_000),
                ("open".to_string(), "USD".to_string(), 5_000)
            ]
        );
        let by_stage = run!(&request, json!({ "kind": "deal", "group_by": "stage" }));
        assert_eq!(
            rows(&by_stage),
            [
                ("Qualification".to_string(), 2),
                ("Proposal".to_string(), 1)
            ]
        );
        let usd_only = run!(
            &request,
            json!({ "kind": "deal", "group_by": "currency", "filter": { "currency": ["USD"] } })
        );
        assert_eq!(rows(&usd_only), [("USD".to_string(), 1)]);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_report_that_cannot_be_asked_is_refused_naming_the_member() {
    request::<App, _, _>(|request, _ctx| async move {
        for (body, pointer) in [
            (
                json!({ "kind": "nonsense", "group_by": "status" }),
                "/spec/kind",
            ),
            (
                json!({ "kind": "lead", "group_by": "score" }),
                "/spec/group_by",
            ),
            (
                json!({ "kind": "ticket", "group_by": "tag" }),
                "/spec/group_by",
            ),
            (
                json!({ "kind": "lead", "group_by": "source", "measure": "sum_amount" }),
                "/spec/measure",
            ),
            (
                json!({ "kind": "lead", "group_by": "source", "filter": { "source": ["pigeon"] } }),
                "/spec/filter",
            ),
        ] {
            let r = request.post("/api/reports/run").json(&body).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let answer: Value = r.json();
            assert_eq!(answer["errors"][0]["pointer"], pointer, "{answer}");
        }
        // A request member that does not exist.
        request
            .post("/api/reports/run")
            .json(&json!({ "kind": "lead", "group_by": "source", "sql": "select 1" }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_report_is_saved_run_again_changed_and_deleted() {
    request::<App, _, _>(|request, _ctx| async move {
        request
            .post("/api/leads")
            .json(&json!({ "display_name": "Report Lead", "source": "web" }))
            .await
            .assert_status_ok();
        let spec = json!({ "kind": "lead", "group_by": "source" });
        let made = request
            .post("/api/reports")
            .json(&json!({ "name": "Leads by source", "spec": spec, "shared": false }))
            .await;
        made.assert_status_ok();
        let pid = made.json::<Value>()["pid"].as_str().unwrap().to_string();
        request
            .post("/api/reports")
            .json(&json!({ "name": "LEADS BY SOURCE", "spec": spec }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        request
            .post("/api/reports")
            .json(&json!({ "name": "Bad", "spec": { "kind": "lead", "group_by": "nope" } }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let listed: Vec<Value> = request.get("/api/reports").await.json();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["mine"], true);
        let ran: Value = request.get(&format!("/api/reports/{pid}/run")).await.json();
        assert_eq!(rows(&ran), [("web".to_string(), 1)]);
        request
            .put(&format!("/api/reports/{pid}"))
            .json(&json!({ "name": "Leads by status",
                           "spec": { "kind": "lead", "group_by": "status" } }))
            .await
            .assert_status_ok();
        let again: Value = request.get(&format!("/api/reports/{pid}/run")).await.json();
        assert_eq!(rows(&again), [("new".to_string(), 1)]);
        request
            .delete(&format!("/api/reports/{pid}"))
            .await
            .assert_status_ok();
        request
            .get(&format!("/api/reports/{pid}/run"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}
