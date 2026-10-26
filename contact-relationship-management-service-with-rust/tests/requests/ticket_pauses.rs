//! SLA clocks that pause while a ticket waits for the customer (CRM-R55): the clock stops on
//! `pending`, a paused ticket does not breach, and leaving `pending` moves each deadline by the
//! waiting that counted against it.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

async fn sql(db: &sea_orm::DatabaseConnection, text: &str, pid: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        text,
        [pid.into()],
    ))
    .await
    .unwrap();
}

fn seconds_between(a: &Value, b: &Value) -> i64 {
    let parse = |v: &Value| chrono::DateTime::parse_from_rfc3339(v.as_str().unwrap()).unwrap();
    (parse(b) - parse(a)).num_seconds()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_ticket_waiting_for_the_customer_stops_its_clock_and_gets_the_time_back() {
    request::<App, _, _>(|request, ctx| async move {
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "normal", "first_response_minutes": 60,
                           "resolution_minutes": 240 }))
            .await
            .assert_status_ok();
        let open = |title: &'static str| {
            let request = &request;
            async move {
                let made: Value = request
                    .post("/api/tickets")
                    .json(&json!({ "title": title, "priority": "normal" }))
                    .await
                    .json();
                made["pid"].as_str().unwrap().to_string()
            }
        };
        let get = |pid: String| {
            let request = &request;
            async move {
                let all: Value = request.get("/api/tickets").await.json();
                all["items"]
                    .as_array()
                    .or_else(|| all.as_array())
                    .unwrap()
                    .iter()
                    .find(|t| t["pid"] == pid.as_str())
                    .unwrap()
                    .clone()
            }
        };
        let status = |pid: String, to: &'static str| {
            let request = &request;
            async move {
                let r = request
                    .post(&format!("/api/tickets/{pid}/status"))
                    .json(&json!({ "to": to }))
                    .await;
                r.assert_status_ok();
                r.json::<Value>()
            }
        };

        // A ticket is paused, then its resolution deadline falls while it waits: no breach.
        let waiting = open("Waiting").await;
        let before = get(waiting.clone()).await;
        let paused = status(waiting.clone(), "pending").await;
        assert!(paused["paused_at"].is_string(), "{paused}");
        sql(
            &ctx.db,
            "UPDATE tickets SET resolution_due_at = now() - interval '1 hour', \
             paused_at = now() - interval '2 hours' WHERE pid = $1::uuid",
            &waiting,
        )
        .await;
        assert_eq!(
            get(waiting.clone()).await["live_resolution_breached"],
            false
        );
        // A ticket in the same position that is not paused is breached.
        let running = open("Running").await;
        sql(
            &ctx.db,
            "UPDATE tickets SET resolution_due_at = now() - interval '1 hour' \
             WHERE pid = $1::uuid",
            &running,
        )
        .await;
        assert_eq!(get(running.clone()).await["live_resolution_breached"], true);
        // The sweep flags the running one only.
        let swept: Value = request.post("/api/sla/sweep").await.json();
        assert_eq!(swept["new_breaches"], 1, "{swept}");

        // Resuming gives the waiting back: paused 30 minutes ago, 60 of the 240 had run.
        let ticket = open("Resumed").await;
        let opened = get(ticket.clone()).await;
        status(ticket.clone(), "pending").await;
        sql(
            &ctx.db,
            "UPDATE tickets SET paused_at = now() - interval '30 minutes' WHERE pid = $1::uuid",
            &ticket,
        )
        .await;
        let resumed = status(ticket.clone(), "open").await;
        assert!(resumed["paused_at"].is_null(), "{resumed}");
        let moved = seconds_between(&opened["resolution_due_at"], &resumed["resolution_due_at"]);
        assert!(
            (1795..=1805).contains(&moved),
            "resolution moved {moved}s: {resumed}"
        );
        let moved = seconds_between(
            &opened["first_response_due_at"],
            &resumed["first_response_due_at"],
        );
        assert!(
            (1795..=1805).contains(&moved),
            "first response moved {moved}s: {resumed}"
        );
        assert!(resumed["resolution_paused_seconds"].as_i64().unwrap() >= 1795);

        // A priority change keeps the waiting: the new deadlines are the new targets from the
        // open time plus the same 30 minutes.
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "high", "first_response_minutes": 30,
                           "resolution_minutes": 120 }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/tickets/{ticket}/priority"))
            .json(&json!({ "priority": "high", "reason": "customer escalated" }))
            .await
            .assert_status_ok();
        let changed = get(ticket.clone()).await;
        let from_open = seconds_between(&opened["opened_at"], &changed["resolution_due_at"]);
        assert!(
            (120 * 60 + 1795..=120 * 60 + 1805).contains(&from_open),
            "{from_open}: {changed}"
        );

        // A pause that starts after the deadline has passed is not given back.
        let late = open("Late").await;
        sql(
            &ctx.db,
            "UPDATE tickets SET resolution_due_at = now() - interval '10 minutes' \
             WHERE pid = $1::uuid",
            &late,
        )
        .await;
        let late_before = get(late.clone()).await;
        status(late.clone(), "pending").await;
        sql(
            &ctx.db,
            "UPDATE tickets SET paused_at = now() - interval '5 minutes' WHERE pid = $1::uuid",
            &late,
        )
        .await;
        let late_after = status(late.clone(), "open").await;
        assert_eq!(late_after["resolution_paused_seconds"], 0, "{late_after}");
        // Re-derived from the open time with nothing added, so it is the original deadline.
        let original = seconds_between(&late_before["opened_at"], &late_after["resolution_due_at"]);
        assert_eq!(original, 240 * 60, "{late_after}");
        assert_eq!(before["status"], "open");
    })
    .await;
}
