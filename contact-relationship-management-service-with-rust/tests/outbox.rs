//! The `outbox` event transport: the event is a row committed with the write.
//!
//! The transport is a process-wide setting, so each test sets it for its own length
//! (`scoped_auth::scoped`, CRM-T263) and the tests after it get the default back.
//! Run with `cargo test --test main outbox -- --ignored`.

use axa_test::prelude::*;
use contact_relationship_management::streaming::{self, EventTransport};
use contact_relationship_management::{app::App, db};
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_event_is_an_outbox_row() {
    let _outbox = crate::scoped_auth::scoped(|s| s.event_transport = EventTransport::Outbox);
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_event_shares_a_sqlx_transaction() {
    let _outbox = crate::scoped_auth::scoped(|s| s.event_transport = EventTransport::Outbox);
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

/// The relay's poll claims the rows nothing has published, and its ack stamps them, so the next
/// poll no longer returns them (the Phase-3 relay's two calls, `Model::unpublished` and
/// `Model::mark_published`).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_relay_poll_claims_unpublished_rows_and_the_ack_marks_them() {
    use contact_relationship_management::models::event_outbox::Model;
    let _outbox = crate::scoped_auth::scoped(|s| s.event_transport = EventTransport::Outbox);
    request::<App, _, _>(|_request, ctx| async move {
        let pid = uuid::Uuid::new_v4();
        let pool = db::pool(&ctx);
        let mut txn = pool.begin().await.unwrap();
        streaming::emit_in(
            &mut txn,
            "lead",
            "lead_captured",
            &pid.to_string(),
            "A lead",
            Some("a"),
            None,
        )
        .await
        .unwrap();
        txn.commit().await.unwrap();

        let mut txn = pool.begin().await.unwrap();
        let polled = Model::unpublished(&mut txn, 100_000).await.unwrap();
        let ours: Vec<i32> = polled
            .iter()
            .filter(|row| row.entity_pid == pid)
            .map(|row| row.id)
            .collect();
        assert_eq!(ours.len(), 1, "the new row is polled once");
        assert_eq!(Model::mark_published(&mut txn, &ours).await.unwrap(), 1);
        txn.commit().await.unwrap();

        let mut txn = pool.begin().await.unwrap();
        let again = Model::unpublished(&mut txn, 100_000).await.unwrap();
        assert!(
            !again.iter().any(|row| row.entity_pid == pid),
            "an acknowledged row is not polled again"
        );
        txn.commit().await.unwrap();
        let published: bool = sqlx::query_scalar(
            "SELECT published_at IS NOT NULL FROM event_outbox WHERE entity_pid = $1",
        )
        .bind(pid)
        .fetch_one(pool)
        .await
        .unwrap();
        assert!(published);
    })
    .await;
}
