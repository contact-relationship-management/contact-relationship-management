//! Events (in-memory transport): a committed write is visible at
//! `/api/events/recent`, newest first, and a refused write emits nothing.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_committed_write_emits_an_event_and_a_refused_one_does_not() {
    request::<App, _, _>(|request, _ctx| async move {
        let name = format!("Event Lead {}", uuid::Uuid::new_v4());
        let created: Value = request
            .post("/api/leads")
            .json(
                &json!({ "display_name": name, "source": "referral", "email": "e@events.example" }),
            )
            .await
            .json();
        let pid = created["pid"].as_str().unwrap().to_string();

        let events: Vec<Value> = request.get("/api/events/recent").await.json();
        let ours: Vec<&Value> = events.iter().filter(|e| e["pid"] == pid).collect();
        assert_eq!(ours.len(), 1, "one event for the one write: {events:?}");
        assert_eq!(ours[0]["kind"], "lead_captured");
        assert_eq!(ours[0]["name"], name);

        // Newest first: sequence numbers strictly fall down the list.
        let seqs: Vec<u64> = events.iter().map(|e| e["seq"].as_u64().unwrap()).collect();
        assert!(seqs.windows(2).all(|w| w[0] > w[1]), "{seqs:?}");

        // A request the service refuses leaves no event behind.
        let before = events.len();
        request
            .post("/api/leads")
            .json(&json!({ "display_name": "", "source": "referral" }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let after: Vec<Value> = request.get("/api/events/recent").await.json();
        assert_eq!(after.len(), before, "a refused write emits nothing");
    })
    .await;
}
