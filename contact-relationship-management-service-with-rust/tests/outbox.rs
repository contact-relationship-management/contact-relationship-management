//! The `outbox` event transport: the event is a row committed with the write.
//!
//! Its own test binary (not part of `tests/mod.rs`) because the transport is
//! process-wide and read once.
//! Run with `cargo test --test outbox -- --ignored`.

use axa::testing::prelude::*;
use contact_relationship_management::streaming::{self, EventTransport};
use contact_relationship_management::{app::App, db, settings};
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
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

/// `sqlx`'s version commits with the mutation's transaction, and its row is the one `SeaORM`'s
/// writes (CRM-D24).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn the_event_shares_a_sqlx_transaction() {
    settings::update(|s| s.event_transport = EventTransport::Outbox);
    request::<App, _, _>(|request, ctx| async move {
        let pid = uuid::Uuid::new_v4();
        let text = pid.to_string();
        let pool = db::pool(&ctx);
        let emit = async |txn: &mut sqlx::PgConnection| {
            streaming::emit_in(
                txn,
                "lead",
                "lead_captured",
                &text,
                "A lead",
                Some("a"),
                None,
            )
            .await
            .unwrap();
        };

        let mut txn = pool.begin().await.unwrap();
        emit(&mut txn).await;
        txn.rollback().await.unwrap();
        let events: Vec<Value> = request.get("/api/events/recent").await.json();
        assert!(!events.iter().any(|e| e["pid"] == text), "{events:?}");

        let mut txn = pool.begin().await.unwrap();
        emit(&mut txn).await;
        txn.commit().await.unwrap();
        let events: Vec<Value> = request.get("/api/events/recent").await.json();
        let ours: Vec<&Value> = events.iter().filter(|e| e["pid"] == text).collect();
        assert_eq!(ours.len(), 1, "{events:?}");
        assert_eq!(ours[0]["kind"], "lead_captured");
        let unpublished: bool = sqlx::query_scalar(
            "SELECT published_at IS NULL FROM event_outbox WHERE entity_pid = $1",
        )
        .bind(pid)
        .fetch_one(pool)
        .await
        .unwrap();
        assert!(unpublished);
    })
    .await;
}
