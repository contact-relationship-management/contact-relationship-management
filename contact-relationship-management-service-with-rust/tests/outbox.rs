//! The `outbox` event transport: the event is a row committed with the write.
//!
//! Its own test binary (not part of `tests/mod.rs`) because the transport is
//! process-wide and read once.
//! Run with `cargo test --test outbox -- --ignored`.

use contact_relationship_management::{app::App, settings, streaming::EventTransport};
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_event_is_an_outbox_row() {
    settings::update(|s| s.event_transport = EventTransport::Outbox);
    request::<App, _, _>(|request, _ctx| async move {
        let name = format!("Outbox Lead {}", uuid::Uuid::new_v4());
        let created: Value = request
            .post("/api/leads")
            .json(
                &json!({ "display_name": name, "source": "referral", "email": "o@events.example" }),
            )
            .await
            .json();
        let pid = created["pid"].as_str().unwrap();
        let events: Vec<Value> = request.get("/api/events/recent").await.json();
        let ours: Vec<&Value> = events.iter().filter(|e| e["pid"] == pid).collect();
        assert_eq!(ours.len(), 1, "{events:?}");
        assert_eq!(ours[0]["kind"], "lead_captured");
    })
    .await;
}
