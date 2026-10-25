//! Business calendars (CRM-R45): SLA targets counted in working time, holidays, a calendar changed
//! or removed, and what is left as it was.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;

fn office(name: &str) -> Value {
    let day = json!([{ "start": "09:00", "end": "17:00" }]);
    json!({ "name": name, "timezone": "UTC",
            "week": { "mon": day, "tue": day, "wed": day, "thu": day, "fri": day },
            "holidays": ["2026-07-13"] })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn calendars_are_made_checked_and_not_deleted_while_used() {
    request::<App, _, _>(|request, _ctx| async move {
        let made = request
            .post("/api/business-calendars")
            .json(&office("Head office"))
            .await;
        made.assert_status_ok();
        let pid = made.json::<Value>()["pid"].as_str().unwrap().to_string();
        let shown: Value = request
            .get(&format!("/api/business-calendars/{pid}"))
            .await
            .json();
        assert_eq!(shown["timezone"], "UTC");
        assert_eq!(shown["week"]["mon"][0]["start"], "09:00");
        assert_eq!(shown["week"]["sat"], json!([]));
        assert_eq!(shown["holidays"], json!(["2026-07-13"]));
        assert_eq!(shown["in_use"], 0);

        // Refused: a repeated name, an unknown zone, a bad window, no working time, overlap.
        let mut bad_zone = office("Zone");
        bad_zone["timezone"] = json!("Mars/Olympus");
        let mut bad_window = office("Window");
        bad_window["week"]["mon"] = json!([{ "start": "17:00", "end": "09:00" }]);
        let mut empty = office("Empty");
        empty["week"] = json!({});
        let mut overlap = office("Overlap");
        overlap["week"]["tue"] =
            json!([{ "start": "09:00", "end": "12:00" }, { "start": "11:00", "end": "13:00" }]);
        for body in [office("HEAD OFFICE"), bad_zone, bad_window, empty, overlap] {
            request
                .post("/api/business-calendars")
                .json(&body)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }

        // A policy uses it: it cannot be deleted, and a policy cannot name a calendar that is not there.
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "high", "first_response_minutes": 60,
                           "resolution_minutes": 600, "calendar_pid": uuid::Uuid::new_v4() }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "high", "first_response_minutes": 60,
                           "resolution_minutes": 600, "calendar_pid": pid }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .get(&format!("/api/business-calendars/{pid}"))
                .await
                .json::<Value>()["in_use"],
            1
        );
        request
            .delete(&format!("/api/business-calendars/{pid}"))
            .await
            .assert_status(axum::http::StatusCode::CONFLICT);
        let policies: Vec<Value> = request.get("/api/sla-policies").await.json();
        assert_eq!(policies[0]["calendar_pid"], json!(pid));
        // Let it go, and the calendar can be deleted.
        let policy = policies[0]["pid"].as_str().unwrap();
        let freed: Value = request
            .put(&format!("/api/sla-policies/{policy}/calendar"))
            .json(&json!({ "calendar_pid": null }))
            .await
            .json();
        assert_eq!(freed["calendar_pid"], Value::Null);
        request
            .delete(&format!("/api/business-calendars/{pid}"))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/sla-policies/{policy}/calendar"))
            .json(&json!({ "calendar_pid": uuid::Uuid::new_v4() }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_ticket_under_a_calendar_has_working_time_deadlines() {
    request::<App, _, _>(|request, ctx| async move {
        let calendar: Value = request
            .post("/api/business-calendars")
            .json(&office("Support hours"))
            .await
            .json();
        let policy = |priority: &'static str, calendar: Option<&str>| {
            json!({ "priority": priority, "first_response_minutes": 60,
                    "resolution_minutes": 600, "calendar_pid": calendar })
        };
        request
            .post("/api/sla-policies")
            .json(&policy("high", calendar["pid"].as_str()))
            .await
            .assert_status_ok();
        // Round the clock for `normal`, as before.
        request
            .post("/api/sla-policies")
            .json(&policy("normal", None))
            .await
            .assert_status_ok();

        let ticket: Value = request
            .post("/api/tickets")
            .json(&json!({ "title": "Printer on fire", "priority": "normal" }))
            .await
            .json();
        let pid = ticket["pid"].as_str().unwrap().to_string();
        // Pretend it was opened on a Friday at 16:00 UTC (10 July 2026), then raise its priority:
        // the deadlines are derived again from that moment, on the calendar of `high`.
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE tickets SET opened_at = '2026-07-10T16:00:00Z' WHERE pid = '{pid}'"
            ))
            .await
            .unwrap();
        request
            .put(&format!("/api/tickets/{pid}/priority"))
            .json(&json!({ "priority": "high", "reason": "escalated" }))
            .await
            .assert_status_ok();
        let detail: Value = request.get(&format!("/api/tickets/{pid}")).await.json();
        let at = |field: &str| {
            chrono::DateTime::parse_from_rfc3339(detail["ticket"][field].as_str().unwrap())
                .unwrap()
                .to_utc()
        };
        // One hour of first response fits in Friday's last hour.
        assert_eq!(
            at("first_response_due_at").to_rfc3339(),
            "2026-07-10T17:00:00+00:00"
        );
        // Ten working hours: one on Friday, Monday the 13th is a holiday, eight on Tuesday, and
        // the last one on Wednesday morning.
        assert_eq!(
            at("resolution_due_at").to_rfc3339(),
            "2026-07-15T10:00:00+00:00"
        );

        // Back to round-the-clock: sixty minutes and ten hours, straight on.
        request
            .put(&format!("/api/tickets/{pid}/priority"))
            .json(&json!({ "priority": "normal", "reason": "de-escalated" }))
            .await
            .assert_status_ok();
        let detail: Value = request.get(&format!("/api/tickets/{pid}")).await.json();
        assert_eq!(
            detail["ticket"]["first_response_due_at"]
                .as_str()
                .unwrap()
                .get(..16),
            Some("2026-07-10T17:00")
        );
        let normal_resolution = chrono::DateTime::parse_from_rfc3339(
            detail["ticket"]["resolution_due_at"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(
            normal_resolution.to_utc().to_rfc3339(),
            "2026-07-11T02:00:00+00:00"
        );
    })
    .await;
}
