//! Idempotency keys (CRM-T52): a retried `POST` with the same
//! `Idempotency-Key` replays the first response instead of creating a second
//! record; a reused key with different content, a bad key, and a failed first
//! attempt each behave as documented.

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::a_person;

fn contact(name: &str, person: &str) -> Value {
    json!({ "person_ref": person, "display_name": name })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_retry_replays_the_first_response() {
    request::<App, _, _>(|request, _ctx| async move {
        let person = a_person();
        let key = format!("retry-{}", uuid::Uuid::new_v4());
        let first = request
            .post("/api/contacts")
            .add_header("idempotency-key", key.clone())
            .json(&contact("Idem One", &person))
            .await;
        first.assert_status_ok();
        assert!(first.headers().get("idempotent-replayed").is_none());
        let first: Value = first.json();

        let second = request
            .post("/api/contacts")
            .add_header("idempotency-key", key.clone())
            .json(&contact("Idem One", &person))
            .await;
        second.assert_status_ok();
        assert_eq!(second.headers().get("idempotent-replayed").unwrap(), "true");
        let second: Value = second.json();
        assert_eq!(first["pid"], second["pid"], "same record, not a new one");

        let all: Value = request.get("/api/contacts").await.json();
        let named = all
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["display_name"] == "Idem One")
            .count();
        assert_eq!(named, 1, "the retry must not create a second contact");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn without_a_key_nothing_changes() {
    request::<App, _, _>(|request, _ctx| async move {
        for _ in 0..2 {
            request
                .post("/api/contacts")
                .json(&contact("No Key", &a_person()))
                .await
                .assert_status_ok();
        }
        let all: Value = request.get("/api/contacts").await.json();
        let named = all
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["display_name"] == "No Key")
            .count();
        assert_eq!(named, 2);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_reused_key_with_different_content_and_bad_keys_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let key = format!("reuse-{}", uuid::Uuid::new_v4());
        request
            .post("/api/contacts")
            .add_header("idempotency-key", key.clone())
            .json(&contact("Original", &a_person()))
            .await
            .assert_status_ok();
        let clash = request
            .post("/api/contacts")
            .add_header("idempotency-key", key)
            .json(&contact("Different", &a_person()))
            .await;
        assert_eq!(clash.status_code(), 422);

        for bad in ["has space", &"x".repeat(256)] {
            let refused = request
                .post("/api/contacts")
                .add_header("idempotency-key", bad.to_string())
                .json(&contact("Bad Key", &a_person()))
                .await;
            assert_eq!(refused.status_code(), 422, "{bad}");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_failed_first_attempt_is_forgotten_so_the_retry_runs() {
    request::<App, _, _>(|request, _ctx| async move {
        let key = format!("failed-{}", uuid::Uuid::new_v4());
        // Invalid (no display name): refused, and the key must not stick.
        let bad = request
            .post("/api/contacts")
            .add_header("idempotency-key", key.clone())
            .json(&json!({ "person_ref": a_person(), "display_name": "" }))
            .await;
        assert_ne!(bad.status_code(), 200);
        let good = request
            .post("/api/contacts")
            .add_header("idempotency-key", key)
            .json(&contact("Second Try", &a_person()))
            .await;
        good.assert_status_ok();
        assert!(good.headers().get("idempotent-replayed").is_none());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn two_simultaneous_identical_requests_create_one_record() {
    request::<App, _, _>(|request, _ctx| async move {
        let person = a_person();
        let key = format!("race-{}", uuid::Uuid::new_v4());
        let send = || {
            request
                .post("/api/contacts")
                .add_header("idempotency-key", key.clone())
                .json(&contact("Race", &person))
        };
        let (a, b) = tokio::join!(send(), send());
        for status in [a.status_code().as_u16(), b.status_code().as_u16()] {
            assert!(status == 200 || status == 409, "unexpected {status}");
        }
        assert!(
            a.status_code() == 200 || b.status_code() == 200,
            "one of them must win"
        );
        let all: Value = request.get("/api/contacts").await.json();
        let named = all
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["display_name"] == "Race")
            .count();
        assert_eq!(named, 1);
    })
    .await;
}
