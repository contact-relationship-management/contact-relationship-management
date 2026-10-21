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
