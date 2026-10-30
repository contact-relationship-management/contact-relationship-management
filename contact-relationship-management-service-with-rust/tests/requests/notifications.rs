//! Task reminders and the notifications inbox (CRM-R32): each fires once, a new reminder time
//! fires again, a record that is gone is suppressed, the inbox is the caller's own, and SLA
//! breaches reach the assignee.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_user, seed_contact};

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

fn an_hour_ago() -> String {
    (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()
}

macro_rules! task {
    ($request:expr, $contact:expr, $assignee:expr, $remind:expr) => {{
        let created: Value = $request
            .post("/api/activities")
            .json(&json!({
                "subject_kind": "contact", "subject_pid": $contact, "kind": "task",
                "summary": "Call back", "assignee_ref": $assignee, "remind_at": $remind,
            }))
            .await
            .json();
        created["pid"].as_str().expect("an activity pid").to_string()
    }};
}

macro_rules! sweep {
    ($request:expr) => {{
        let answer = $request.post("/api/notifications/sweep").await;
        answer.assert_status_ok();
        answer.json::<Value>()
    }};
}

macro_rules! inbox {
    ($request:expr, $who:expr, $query:expr) => {{
        let answer = $request
            .get(&format!("/api/notifications?recipient={}{}", $who, $query))
            .await;
        answer.assert_status_ok();
        answer.json::<Vec<Value>>()
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_reminder_fires_once_fires_again_for_a_new_time_and_is_read_by_its_recipient_only() {
    request::<App, _, _>(|request, ctx| async move {
        let (me, other) = (a_user(), a_user());
        let contact = seed_contact!(&request, "Reminded").await;

        // Not yet due, not a task, and finished tasks make nothing.
        let future = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();
        let _later = task!(request, contact, me, future);
        let done = task!(request, contact, me, an_hour_ago());
        request
            .put(&format!("/api/activities/{done}/done"))
            .await
            .assert_status_ok();
        let note = request
            .post("/api/activities")
            .json(&json!({
                "subject_kind": "contact", "subject_pid": contact, "kind": "note",
                "summary": "n", "assignee_ref": me,
            }))
            .await;
        assert_eq!(note.status_code(), 422, "only a task has an assignee");
        let first = sweep!(request);
        assert_eq!(first, json!({ "created": 0, "suppressed": 0 }));

        // Due: one notification, and only once.
        let due = task!(request, contact, me, an_hour_ago());
        assert_eq!(sweep!(request), json!({ "created": 1, "suppressed": 0 }));
        assert_eq!(sweep!(request), json!({ "created": 0, "suppressed": 0 }));
        let mine = inbox!(request, me, "");
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0]["kind"], "task_reminder");
        assert_eq!(mine[0]["subject_kind"], "contact");
        assert_eq!(mine[0]["subject_pid"], contact);
        assert!(mine[0]["read_at"].is_null());
        assert!(
            !mine[0].to_string().contains("Call back"),
            "no record text in a notification"
        );
        assert!(inbox!(request, other, "").is_empty());

        // The unread count, and reading: not someone else's, then mine.
        let unread: Value = request
            .get(&format!("/api/notifications/unread-count?recipient={me}"))
            .await
            .json();
        assert_eq!(unread, json!({ "unread": 1, "capped": false }));
        let pid = mine[0]["pid"].as_str().unwrap().to_string();
        request
            .put(&format!("/api/notifications/{pid}/read?recipient={other}"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        let read: Value = request
            .put(&format!("/api/notifications/{pid}/read?recipient={me}"))
            .await
            .json();
        assert!(read["read_at"].is_string());
        assert!(inbox!(request, me, "&unread=true").is_empty());
        assert_eq!(inbox!(request, me, "").len(), 1);

        // A new reminder time on the same task fires once more; the same time does not.
        request
            .put(&format!("/api/activities/{due}"))
            .json(&json!({ "remind_at": (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339() }))
            .await
            .assert_status_ok();
        assert_eq!(sweep!(request), json!({ "created": 1, "suppressed": 0 }));
        assert_eq!(sweep!(request), json!({ "created": 0, "suppressed": 0 }));
        let all = inbox!(request, me, "");
        assert_eq!(all.len(), 2);
        let marked: Value = request
            .post(&format!("/api/notifications/read-all?recipient={me}"))
            .await
            .json();
        assert_eq!(marked, json!({ "marked": 1 }));

        // The sweep is audited with counts only; marking read is audited.
        assert!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE action = 'notifications_swept'"
            )
            .await
                >= 2
        );
        assert_eq!(
            count(
                &ctx.db,
                &format!("SELECT count(*) AS n FROM audit_logs WHERE entity = 'notification' AND entity_pid = '{pid}' AND action = 'read'")
            )
            .await,
            1
        );
        // Sign-in is off, so the inbox must say whose it is.
        request
            .get("/api/notifications")
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_gone_record_is_suppressed_an_sla_breach_notifies_the_assignee_and_erasure_clears_them() {
    request::<App, _, _>(|request, ctx| async move {
        let me = a_user();
        let gone = seed_contact!(&request, "Soon gone").await;
        let kept = seed_contact!(&request, "Kept").await;
        task!(request, gone, me, an_hour_ago());
        task!(request, kept, me, an_hour_ago());
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET deleted_at = now() WHERE pid = '{gone}'"
            ))
            .await
            .unwrap();
        assert_eq!(sweep!(request), json!({ "created": 1, "suppressed": 1 }));
        let mine = inbox!(request, me, "");
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0]["subject_pid"], kept);

        // A ticket breach flagged by the SLA sweep reaches its assignee, once per breach.
        let ticket: Value = request
            .post("/api/tickets")
            .json(&json!({ "title": "Down", "assignee_ref": me }))
            .await
            .json();
        let ticket_pid = ticket["pid"].as_str().unwrap().to_string();
        let unassigned: Value = request
            .post("/api/tickets")
            .json(&json!({ "title": "Nobody's" }))
            .await
            .json();
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE tickets SET first_response_breached = TRUE \
                 WHERE pid IN ('{ticket_pid}', '{}')",
                unassigned["pid"].as_str().unwrap()
            ))
            .await
            .unwrap();
        assert_eq!(sweep!(request), json!({ "created": 1, "suppressed": 0 }));
        assert_eq!(sweep!(request), json!({ "created": 0, "suppressed": 0 }));
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE tickets SET resolution_breached = TRUE WHERE pid = '{ticket_pid}'"
            ))
            .await
            .unwrap();
        assert_eq!(sweep!(request), json!({ "created": 1, "suppressed": 0 }));
        let kinds: Vec<String> = inbox!(request, me, "")
            .iter()
            .map(|n| n["kind"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(kinds.iter().filter(|k| *k == "sla_breach").count(), 2);

        // Erasing the contact removes the notifications about it.
        request
            .post(&format!("/api/contacts/{kept}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "test" }))
            .await
            .assert_status_ok();
        let done: Vec<Value> = request
            .get(&format!("/api/activities?subject_pid={kept}"))
            .await
            .json::<Value>()
            .as_array()
            .cloned()
            .unwrap_or_default();
        for activity in done {
            if activity["kind"] == "task" {
                request
                    .put(&format!(
                        "/api/activities/{}/done",
                        activity["pid"].as_str().unwrap()
                    ))
                    .await
                    .assert_status_ok();
            }
        }
        request
            .post(&format!("/api/contacts/{kept}/erase"))
            .await
            .assert_status_ok();
        assert_eq!(
            count(
                &ctx.db,
                &format!("SELECT count(*) AS n FROM notifications WHERE subject_pid = '{kept}'")
            )
            .await,
            0
        );
    })
    .await;
}
