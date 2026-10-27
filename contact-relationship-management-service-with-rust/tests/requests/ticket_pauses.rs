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

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_customer_reply_resumes_a_waiting_ticket_and_leaves_the_others_alone() {
    request::<App, _, _>(|request, ctx| async move {
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "normal", "first_response_minutes": 60,
                           "resolution_minutes": 240 }))
            .await
            .assert_status_ok();
        let made: Value = request
            .post("/api/tickets")
            .json(&json!({ "title": "Answered", "priority": "normal" }))
            .await
            .json();
        let ticket = made["pid"].as_str().unwrap().to_string();
        let reply = format!("/api/tickets/{ticket}/customer-reply");
        let detail: Value = request.get(&format!("/api/tickets/{ticket}")).await.json();
        let made = detail["ticket"].clone();

        // Already open: returned as it is, nothing changes.
        let same: Value = request.post(&reply).await.json();
        assert_eq!(same["status"], "open", "{same}");
        assert_eq!(
            same["resolution_due_at"], made["resolution_due_at"],
            "{same}"
        );

        // Waiting for 30 minutes, then the customer answers: open again, the clock running,
        // the deadlines moved by the waiting.
        request
            .post(&format!("/api/tickets/{ticket}/status"))
            .json(&json!({ "to": "pending" }))
            .await
            .assert_status_ok();
        sql(
            &ctx.db,
            "UPDATE tickets SET paused_at = now() - interval '30 minutes' WHERE pid = $1::uuid",
            &ticket,
        )
        .await;
        let answered: Value = request.post(&reply).await.json();
        assert_eq!(answered["status"], "open", "{answered}");
        assert!(answered["paused_at"].is_null(), "{answered}");
        let moved = seconds_between(&made["resolution_due_at"], &answered["resolution_due_at"]);
        assert!(
            (1795..=1805).contains(&moved),
            "resolution moved {moved}s: {answered}"
        );

        // The audit says the customer's reply did it.
        let via: i64 = ctx
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM audit_logs WHERE entity = 'ticket' \
                 AND snapshot->>'via' = 'customer_reply' AND entity_pid = $1::uuid",
                [ticket.clone().into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "n")
            .unwrap();
        assert_eq!(via, 1);

        // A resolved ticket is not reopened by a reply.
        request
            .post(&format!("/api/tickets/{ticket}/status"))
            .json(&json!({ "to": "resolved" }))
            .await
            .assert_status_ok();
        request
            .post(&reply)
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_ticket_left_waiting_past_its_policys_limit_is_flagged_once_and_cleared_on_leaving() {
    request::<App, _, _>(|request, ctx| async move {
        // `normal` allows 60 minutes of waiting; `high` has no limit.
        for (priority, limit) in [("normal", json!(60)), ("high", Value::Null)] {
            request
                .post("/api/sla-policies")
                .json(&json!({ "priority": priority, "first_response_minutes": 60,
                               "resolution_minutes": 240, "max_wait_minutes": limit }))
                .await
                .assert_status_ok();
        }
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "low", "first_response_minutes": 60,
                           "resolution_minutes": 240, "max_wait_minutes": 0 }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

        let open = |priority: &'static str| {
            let request = &request;
            async move {
                let made: Value = request
                    .post("/api/tickets")
                    .json(&json!({ "title": priority, "priority": priority }))
                    .await
                    .json();
                let pid = made["pid"].as_str().unwrap().to_string();
                request
                    .post(&format!("/api/tickets/{pid}/status"))
                    .json(&json!({ "to": "pending" }))
                    .await
                    .assert_status_ok();
                pid
            }
        };
        let (limited, unlimited, recent) = (
            open("normal").await,
            open("high").await,
            open("normal").await,
        );
        for (pid, minutes) in [(&limited, 90), (&unlimited, 90), (&recent, 30)] {
            sql(
                &ctx.db,
                &format!(
                    "UPDATE tickets SET paused_at = now() - interval '{minutes} minutes' \
                     WHERE pid = $1::uuid"
                ),
                pid,
            )
            .await;
        }
        let exceeded = |pid: String| {
            let request = &request;
            async move {
                let detail: Value = request.get(&format!("/api/tickets/{pid}")).await.json();
                detail["ticket"]["wait_limit_exceeded_at"].clone()
            }
        };

        // Only the one past its limit is flagged, and it stays pending with its clock stopped.
        let swept: Value = request.post("/api/sla/sweep").await.json();
        assert_eq!(swept["new_wait_limits_exceeded"], 1, "{swept}");
        assert!(exceeded(limited.clone()).await.is_string());
        assert!(exceeded(unlimited.clone()).await.is_null());
        assert!(exceeded(recent.clone()).await.is_null());
        let detail: Value = request.get(&format!("/api/tickets/{limited}")).await.json();
        assert_eq!(detail["ticket"]["status"], "pending", "{detail}");
        assert!(detail["ticket"]["paused_at"].is_string(), "{detail}");

        // Once: a second sweep finds nothing new.
        let again: Value = request.post("/api/sla/sweep").await.json();
        assert_eq!(again["new_wait_limits_exceeded"], 0, "{again}");

        // The customer answers: the flag goes with the wait.
        request
            .post(&format!("/api/tickets/{limited}/customer-reply"))
            .await
            .assert_status_ok();
        assert!(exceeded(limited).await.is_null());
    })
    .await;
}
