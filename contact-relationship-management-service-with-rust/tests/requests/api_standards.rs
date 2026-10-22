//! GDS API standards (CRM-T49): unknown request members are refused, and
//! authenticated API responses say they must not be cached.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{a_person, seed_contact};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_misspelt_request_member_is_refused_not_dropped() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/contacts")
            .json(&json!({
                "person_ref": a_person(),
                "display_name": "Typo Test",
                "dispaly_name": "ignored before this change",
            }))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"], "validation");
        assert!(
            body["detail"].as_str().unwrap().contains("dispaly_name"),
            "the refusal should name the member: {body}"
        );
        assert_eq!(body["errors"][0]["pointer"], "/dispaly_name", "{body}");

        // The same body without the stray member still works.
        let ok = request
            .post("/api/contacts")
            .json(&json!({"person_ref": a_person(), "display_name": "Typo Test"}))
            .await;
        ok.assert_status_success();
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn each_validation_problem_names_its_member() {
    request::<App, _, _>(|request, _ctx| async move {
        // Two members are wrong at once; one problem has no single member.
        let response = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": "not-a-urn", "display_name": "  " }))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let body: serde_json::Value = response.json();
        let errors = body["errors"].as_array().expect("errors is a list");
        let pointers: Vec<&str> = errors
            .iter()
            .filter_map(|e| e["pointer"].as_str())
            .collect();
        assert!(pointers.contains(&"/person_ref"), "{body}");
        assert!(pointers.contains(&"/display_name"), "{body}");
        for error in errors {
            assert!(
                error["detail"].as_str().is_some_and(|d| !d.is_empty()),
                "{body}"
            );
        }
        // `description` still joins them, as clients have always read it.
        let joined: Vec<&str> = errors.iter().filter_map(|e| e["detail"].as_str()).collect();
        assert_eq!(body["description"], joined.join("; "));
        assert_eq!(body["detail"], body["description"]);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_problem_about_the_whole_request_has_no_pointer() {
    request::<App, _, _>(|request, _ctx| async move {
        // A pipeline with one stage: the rule is about the list, not a member.
        let response = request
            .post("/api/pipelines")
            .json(&json!({ "name": "Too short", "stages": [
                { "name": "Only", "probability_percent": 10 }
            ]}))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let body: serde_json::Value = response.json();
        let first = &body["errors"][0];
        assert!(first["pointer"].is_null(), "{body}");
        assert!(
            first["detail"].as_str().unwrap().contains("2-20 stages"),
            "{body}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn api_responses_are_not_cacheable() {
    request::<App, _, _>(|request, _ctx| async move {
        seed_contact!(&request, "Cache Test").await;
        let list = request.get("/api/contacts").await;
        list.assert_status_ok();
        assert_eq!(list.headers().get("cache-control").unwrap(), "no-store");

        // Errors carry the policy too.
        let missing = request
            .get(&format!("/api/contacts/{}", uuid::Uuid::new_v4()))
            .await;
        missing.assert_status_not_found();
        assert_eq!(missing.headers().get("cache-control").unwrap(), "no-store");

        // The documentation endpoint is outside /api and is left alone.
        let docs = request.get("/api-docs/openapi.json").await;
        assert!(docs.headers().get("cache-control").is_none());
    })
    .await;
}
