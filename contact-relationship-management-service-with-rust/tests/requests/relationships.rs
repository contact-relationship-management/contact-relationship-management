//! Contacts, accounts and activities: creation, lookup, refusals and the task
//! lifecycle, through the routes.

use axum::http::StatusCode;
use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_contact};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_contact_is_created_listed_and_fetched() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = seed_contact!(&request, "Rel Contact").await;
        let one: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(one["contact"]["display_name"], "Rel Contact");
        assert_eq!(one["truncated"]["activities"], false);
        let all: Vec<Value> = request.get("/api/contacts").await.json();
        assert!(all.iter().any(|c| c["pid"] == pid));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn bad_contacts_and_unknown_ids_are_refused_with_the_right_status() {
    request::<App, _, _>(|request, _ctx| async move {
        let blank: Value = {
            let r = request
                .post("/api/contacts")
                .json(&json!({ "person_ref": a_person(), "display_name": "" }))
                .await;
            r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
            r.json()
        };
        assert_eq!(blank["errors"][0]["pointer"], "/display_name", "{blank}");

        request
            .post("/api/contacts")
            .json(&json!({ "person_ref": "not-a-urn", "display_name": "X" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        request
            .get(&format!("/api/contacts/{}", uuid::Uuid::new_v4()))
            .await
            .assert_status(StatusCode::NOT_FOUND);
        let malformed = request.get("/api/contacts/not-a-uuid").await;
        assert!(
            malformed.status_code().is_client_error(),
            "{}",
            malformed.status_code()
        );
        assert_ne!(malformed.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_account_round_trips() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/accounts")
            .json(&json!({
                "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                "display_name": "Rel Org",
            }))
            .await
            .json();
        let pid = created["pid"].as_str().unwrap();
        let fetched: Value = request.get(&format!("/api/accounts/{pid}")).await.json();
        assert_eq!(fetched["account"]["display_name"], "Rel Org");
        let all: Vec<Value> = request.get("/api/accounts").await.json();
        assert!(all.iter().any(|a| a["pid"] == pid));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn only_a_task_can_be_marked_done() {
    request::<App, _, _>(|request, _ctx| async move {
        let contact = seed_contact!(&request, "Rel Tasker").await;
        let make = |kind: &str| {
            let body = json!({
                "subject_kind": "contact", "subject_pid": contact,
                "kind": kind, "summary": "do it", "due_on": "2030-01-01",
            });
            let request = &request;
            async move { request.post("/api/activities").json(&body).await }
        };
        let task: Value = make("task").await.json();
        let note: Value = make("note").await.json();

        let done = request
            .put(&format!(
                "/api/activities/{}/done",
                task["pid"].as_str().unwrap()
            ))
            .await;
        done.assert_status_ok();
        assert_eq!(done.json::<Value>()["done"], true);

        request
            .put(&format!(
                "/api/activities/{}/done",
                note["pid"].as_str().unwrap()
            ))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        request
            .put(&format!("/api/activities/{}/done", uuid::Uuid::new_v4()))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    })
    .await;
}
