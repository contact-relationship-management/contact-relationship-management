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
async fn a_ticket_left_waiting_past_its_policy_limit_is_flagged_once_and_cleared_on_leaving() {
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

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_policy_can_resolve_or_escalate_a_ticket_left_waiting_past_its_limit() {
    request::<App, _, _>(|request, ctx| async move {
        let worker = format!("person:{}", uuid::Uuid::new_v4());
        let policy = |priority: &'static str, action: Value, to: Value| {
            let request = &request;
            async move {
                request
                    .post("/api/sla-policies")
                    .json(&json!({ "priority": priority, "first_response_minutes": 60,
                                   "resolution_minutes": 240, "max_wait_minutes": 60,
                                   "wait_limit_action": action, "escalate_to": to }))
                    .await
            }
        };
        policy("normal", json!("resolve"), Value::Null)
            .await
            .assert_status_ok();
        policy("high", json!("escalate"), json!(worker))
            .await
            .assert_status_ok();
        policy("low", json!("flag"), Value::Null)
            .await
            .assert_status_ok();

        // Refused: escalating without a worker, a worker without escalating, an unknown action,
        // an action with no limit to act on.
        for (action, to) in [
            (json!("escalate"), Value::Null),
            (json!("flag"), json!(worker)),
            (json!("delete"), Value::Null),
            (json!("escalate"), json!("contact:x")),
        ] {
            policy("urgent", action, to)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "urgent", "first_response_minutes": 15,
                           "resolution_minutes": 240, "wait_limit_action": "resolve" }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

        let wait = |priority: &'static str| {
            let (request, ctx) = (&request, &ctx);
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
                sql(
                    &ctx.db,
                    "UPDATE tickets SET paused_at = now() - interval '90 minutes' \
                     WHERE pid = $1::uuid",
                    &pid,
                )
                .await;
                pid
            }
        };
        let (resolved, escalated, flagged) =
            (wait("normal").await, wait("high").await, wait("low").await);

        let swept: Value = request.post("/api/sla/sweep").await.json();
        assert_eq!(swept["new_wait_limits_exceeded"], 3, "{swept}");
        assert_eq!(swept["new_wait_limit_resolved"], 1, "{swept}");
        assert_eq!(swept["new_wait_limit_escalated"], 1, "{swept}");
        let ticket = |pid: String| {
            let request = &request;
            async move {
                let detail: Value = request.get(&format!("/api/tickets/{pid}")).await.json();
                detail["ticket"].clone()
            }
        };

        // Resolved: out of `pending`, the clock running again, nothing left flagged.
        let done = ticket(resolved).await;
        assert_eq!(done["status"], "resolved", "{done}");
        assert!(
            done["paused_at"].is_null() && done["resolved_at"].is_string(),
            "{done}"
        );
        // Escalated: still waiting, handed to the worker, flagged.
        let handed = ticket(escalated).await;
        assert_eq!(handed["status"], "pending", "{handed}");
        assert_eq!(handed["assignee_ref"], worker.as_str(), "{handed}");
        assert!(handed["wait_limit_exceeded_at"].is_string(), "{handed}");
        // Flagged only: as before.
        let only = ticket(flagged).await;
        assert_eq!(only["status"], "pending", "{only}");
        assert!(only["assignee_ref"].is_null(), "{only}");
        assert!(only["wait_limit_exceeded_at"].is_string(), "{only}");

        // Once: nothing more happens on the next sweep.
        let again: Value = request.post("/api/sla/sweep").await.json();
        assert_eq!(again["new_wait_limits_exceeded"], 0, "{again}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_policy_closes_a_ticket_left_resolved_for_its_days_and_no_other() {
    request::<App, _, _>(|request, ctx| async move {
        let policy = |priority: &'static str, days: Value| {
            let request = &request;
            async move {
                request
                    .post("/api/sla-policies")
                    .json(&json!({ "priority": priority, "first_response_minutes": 60,
                                   "resolution_minutes": 240, "auto_close_days": days }))
                    .await
            }
        };
        policy("normal", json!(7)).await.assert_status_ok();
        policy("low", Value::Null).await.assert_status_ok();
        for days in [json!(0), json!(366), json!(-1)] {
            policy("urgent", days)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }

        // A ticket resolved `ago` days ago, of the given priority.
        let resolved = |priority: &'static str, ago: i32| {
            let (request, ctx) = (&request, &ctx);
            async move {
                let made: Value = request
                    .post("/api/tickets")
                    .json(&json!({ "title": priority, "priority": priority }))
                    .await
                    .json();
                let pid = made["pid"].as_str().unwrap().to_string();
                request
                    .post(&format!("/api/tickets/{pid}/status"))
                    .json(&json!({ "to": "resolved" }))
                    .await
                    .assert_status_ok();
                sql(
                    &ctx.db,
                    &format!(
                        "UPDATE tickets SET resolved_at = now() - interval '{ago} days' \
                         WHERE pid = $1::uuid"
                    ),
                    &pid,
                )
                .await;
                pid
            }
        };
        let (due, recent, no_days) = (
            resolved("normal", 8).await,
            resolved("normal", 6).await,
            resolved("low", 400).await,
        );
        // Reopened after the days passed: resolved again just now, so not due.
        let reopened = resolved("normal", 30).await;
        request
            .post(&format!("/api/tickets/{reopened}/status"))
            .json(&json!({ "to": "open" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/tickets/{reopened}/status"))
            .json(&json!({ "to": "resolved" }))
            .await
            .assert_status_ok();

        let swept: Value = request.post("/api/sla/sweep").await.json();
        assert_eq!(swept["new_auto_closed"], 1, "{swept}");
        let status = |pid: String| {
            let request = &request;
            async move {
                let detail: Value = request.get(&format!("/api/tickets/{pid}")).await.json();
                detail["ticket"]["status"].clone()
            }
        };
        assert_eq!(status(due.clone()).await, "closed");
        assert_eq!(status(recent).await, "resolved", "six days is under seven");
        assert_eq!(status(no_days).await, "resolved", "no days, no closing");
        assert_eq!(
            status(reopened).await,
            "resolved",
            "the days count from the last resolve"
        );

        // The audit says it was the sweep, not a person.
        let audits: Value = request.get(&format!("/api/audits/{due}")).await.json();
        let closed = audits
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["action"] == "ticket_closed")
            .expect("closed audit row");
        assert_eq!(closed["snapshot"]["via"], "auto_close", "{closed}");

        // Once: a closed ticket is not closed again.
        let again: Value = request.post("/api/sla/sweep").await.json();
        assert_eq!(again["new_auto_closed"], 0, "{again}");
    })
    .await;
}

const INBOUND_SECRET: &str = "inbound-secret-0123456789abcdef-xyz";

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_reply_by_mail_resumes_the_ticket_it_names_when_it_comes_from_the_customer() {
    use contact_relationship_management::{clients, settings, webhook};
    settings::update(|s| s.webhook_secret = Some(INBOUND_SECRET.to_string()));
    request::<App, _, _>(|request, _ctx| async move {
        let person = super::a_person();
        clients::prime_email(&person, "ada@example.test");
        let contact: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": person, "display_name": "Ada" }))
            .await
            .json();
        let contact = contact["pid"].as_str().unwrap().to_string();
        let made = |with_contact: bool| {
            let (request, contact) = (&request, &contact);
            async move {
                let mut body = json!({ "title": "Printer", "priority": "normal" });
                if with_contact {
                    body["contact_pid"] = json!(contact);
                }
                let made: Value = request.post("/api/tickets").json(&body).await.json();
                let pid = made["pid"].as_str().unwrap().to_string();
                request
                    .post(&format!("/api/tickets/{pid}/status"))
                    .json(&json!({ "to": "pending" }))
                    .await
                    .assert_status_ok();
                let detail: Value = request.get(&format!("/api/tickets/{pid}")).await.json();
                (pid, detail["reference"].as_str().unwrap().to_string())
            }
        };
        let (pid, reference) = made(true).await;
        assert!(
            reference.starts_with("T-") && pid.starts_with(&reference[2..]),
            "{reference}"
        );

        let post = |from: &str, subject: &str, signed: bool| {
            let request = &request;
            let body = json!({ "from": from, "subject": subject, "text": "ignored" }).to_string();
            let timestamp = chrono::Utc::now().timestamp().to_string();
            let signature = if signed {
                webhook::sign(INBOUND_SECRET, &timestamp, body.as_bytes())
            } else {
                "sha256=00".to_string()
            };
            async move {
                request
                    .post("/webhooks/inbound-email")
                    .add_header("x-crm-timestamp", timestamp)
                    .add_header("x-crm-signature", signature)
                    .add_header("content-type", "application/json")
                    .bytes(body.into())
                    .await
            }
        };
        let result = |from: &'static str, subject: String| {
            let post = &post;
            async move {
                let r = post(from, &subject, true).await;
                r.assert_status_ok();
                r.json::<Value>()["result"].as_str().unwrap().to_string()
            }
        };
        let status = |pid: String| {
            let request = &request;
            async move {
                let detail: Value = request.get(&format!("/api/tickets/{pid}")).await.json();
                detail["ticket"]["status"].as_str().unwrap().to_string()
            }
        };
        let subject = format!("Re: Printer [{reference}]");

        // Nothing resumes it: no reference, an unknown one, someone else's address.
        assert_eq!(
            result("ada@example.test", "Re: Printer".into()).await,
            "no_reference"
        );
        assert_eq!(
            result("ada@example.test", "Re: [T-ffffffff]".into()).await,
            "unknown_ticket"
        );
        assert_eq!(
            result("mallory@example.test", subject.clone()).await,
            "unverified_sender"
        );
        assert_eq!(status(pid.clone()).await, "pending");

        // The customer, with a display name and different case, resumes it once.
        assert_eq!(
            result("Ada Lovelace <ADA@example.test>", subject.clone()).await,
            "resumed"
        );
        assert_eq!(status(pid.clone()).await, "open");
        assert_eq!(
            result("ada@example.test", subject.clone()).await,
            "already_open"
        );
        let audits: Value = request.get(&format!("/api/audits/{pid}")).await.json();
        let via: Vec<&str> = audits
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| a["snapshot"]["via"].as_str())
            .collect();
        assert_eq!(via, ["inbound_mail"], "{audits}");

        // A reply does not reopen a resolved ticket.
        request
            .post(&format!("/api/tickets/{pid}/status"))
            .json(&json!({ "to": "resolved" }))
            .await
            .assert_status_ok();
        assert_eq!(result("ada@example.test", subject).await, "ignored");
        assert_eq!(status(pid).await, "resolved");

        // A ticket about nobody cannot be verified.
        let (other, other_reference) = made(false).await;
        assert_eq!(
            result("ada@example.test", format!("[{other_reference}]")).await,
            "unverified_sender"
        );
        assert_eq!(status(other).await, "pending");

        // Unsigned, badly signed, or with no secret configured: refused or hidden.
        post("ada@example.test", "x", false)
            .await
            .assert_status(axum::http::StatusCode::UNAUTHORIZED);
        request
            .post("/webhooks/inbound-email")
            .add_header("content-type", "application/json")
            .bytes("{}".into())
            .await
            .assert_status(axum::http::StatusCode::UNAUTHORIZED);
        settings::update(|s| s.webhook_secret = None);
        post("ada@example.test", "x", true)
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}
