//! The organization's calendar (CRM-T171): its time zone for "today" and report months, and its
//! fiscal year for fiscal quarters and years.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_organization_s_zone_and_fiscal_year_shape_today_and_the_reports() {
    request::<App, _, _>(|request, ctx| async move {
        // It starts as UTC and January, the values the service used before.
        let start: Value = request.get("/api/settings/organization").await.json();
        assert_eq!(start["time_zone"], "UTC");
        assert_eq!(start["fiscal_year_start_month"], 1);
        assert!(start["set_by"].is_null());

        // Bad values are refused by member.
        let bad = request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "Tokyo", "fiscal_year_start_month": 13 }))
            .await;
        assert_eq!(bad.status_code(), 422);
        let text = bad.text();
        assert!(text.contains("time_zone") && text.contains("fiscal_year_start_month"), "{text}");

        // An organization on the far side of the date line with an April fiscal year.
        let set: Value = request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "Pacific/Kiritimati", "fiscal_year_start_month": 4 }))
            .await
            .json();
        assert_eq!(set["time_zone"], "Pacific/Kiritimati");
        let kiritimati = chrono::Utc::now()
            .with_timezone(&chrono_tz::Pacific::Kiritimati)
            .date_naive();
        assert_eq!(set["today"], json!(kiritimati));

        // Leads made at a time that is a different day, month and fiscal year in the zone: 23:30
        // UTC on 31 March 2026 is 13:30 on 1 April at UTC+14.
        // Five of them: a smaller group is left out of a report (CRM-R78).
        for name in ["Fiscal one", "Fiscal two", "Fiscal three", "Fiscal four", "Fiscal five"] {
            let lead: Value = request
                .post("/api/leads")
                .json(&json!({ "display_name": name, "source": "web" }))
                .await
                .json();
            ctx.db
                .execute_unprepared(&format!(
                    "UPDATE leads SET created_at = '2026-03-31T23:30:00Z' WHERE pid = '{}'",
                    lead["pid"].as_str().unwrap()
                ))
                .await
                .unwrap();
        }
        let run = |group_by: &str| {
            // Only these leads: nothing else was made at the end of March 2026.
            json!({ "kind": "lead", "group_by": group_by, "from": "2026-03-30", "to": "2026-04-02" })
        };
        for (group_by, label) in [
            ("created_month", "2026-04"),
            ("created_fiscal_year", "2026/27"),
            ("created_fiscal_quarter", "2026/27-Q1"),
        ] {
            let answer: Value = request.post("/api/reports/run").json(&run(group_by)).await.json();
            assert_eq!(answer["time_zone"], "Pacific/Kiritimati");
            assert_eq!(answer["fiscal_year_start_month"], 4);
            let groups: Vec<&str> = answer["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["group"].as_str().unwrap())
                .collect();
            assert_eq!(groups, [label], "{group_by}: {answer}");
        }
        // The from and to days are the zone's days too.
        let mut ranged = run("source");
        ranged["from"] = json!("2026-04-01");
        ranged["to"] = json!("2026-04-01");
        let utc_day = request.post("/api/reports/run").json(&{
            let mut r = run("source");
            r["from"] = json!("2026-03-31");
            r["to"] = json!("2026-03-31");
            r
        }).await.json::<Value>();
        assert_eq!(utc_day["rows"].as_array().unwrap().len(), 0, "31 March is not their day there: {utc_day}");
        let answer: Value = request.post("/api/reports/run").json(&ranged).await.json();
        assert_eq!(answer["rows"][0]["value"], 5, "{answer}");

        // Back in UTC with a January year, the same leads are March and 2026-Q1.
        request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1 }))
            .await
            .assert_status_ok();
        let answer: Value = request
            .post("/api/reports/run")
            .json(&run("created_fiscal_quarter"))
            .await
            .json();
        assert_eq!(answer["rows"][0]["group"], "2026-Q1", "{answer}");
        let answer: Value = request.post("/api/reports/run").json(&run("created_month")).await.json();
        assert_eq!(answer["rows"][0]["group"], "2026-03", "{answer}");

        // The forecast snapshot is dated in the organization's zone.
        request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "Pacific/Kiritimati", "fiscal_year_start_month": 4 }))
            .await
            .assert_status_ok();
        let snap: Value = request.post("/api/forecast/snapshot").await.json();
        assert_eq!(snap["taken_on"], json!(kiritimati), "{snap}");

        // A caller's own zone decides its "today": quota attainment is read on that day.
        let pago = chrono::Utc::now()
            .with_timezone(&chrono_tz::Pacific::Pago_Pago)
            .date_naive();
        let read: Value = request
            .get("/api/quotas/attainment")
            .add_header("crm-time-zone", "Pacific/Pago_Pago")
            .await
            .json();
        assert_eq!(read["at"], json!(pago), "{read}");
        let read: Value = request.get("/api/quotas/attainment").await.json();
        assert_eq!(read["at"], json!(kiritimati), "{read}");
        request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1 }))
            .await
            .assert_status_ok();
    })
    .await;
}

/// One value from a query.
async fn one_date(db: &sea_orm::DatabaseConnection, sql: &str) -> chrono::NaiveDate {
    db.query_one_raw(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        sql,
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "d")
    .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_other_todays_follow_the_organization_or_the_caller() {
    // CRM-T179: the knowledge-base window and its counts, the campaign rate day, a workflow
    // task's due day, and the follow-up list's "overdue".
    let _scope = crate::scoped_auth::scoped(|s| {
        s.public_kb = true;
        s.public_kb_client_limit = 0;
    });
    request::<App, _, _>(|request, ctx| async move {
        request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "Pacific/Kiritimati", "fiscal_year_start_month": 1 }))
            .await
            .assert_status_ok();
        let kiritimati = chrono::Utc::now()
            .with_timezone(&chrono_tz::Pacific::Kiritimati)
            .date_naive();
        // Pago Pago is 25 hours behind Kiritimati: always the day before.
        let pago = chrono::Utc::now()
            .with_timezone(&chrono_tz::Pacific::Pago_Pago)
            .date_naive();
        assert_eq!(pago + chrono::Days::new(1), kiritimati);

        // A knowledge-base view is counted on the organization's day.
        let made: Value = request
            .post("/api/articles")
            .json(&json!({ "title": "Zones", "body": "About time zones", "keywords": "help" }))
            .await
            .json();
        let article = made["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/articles/{article}/status"))
            .json(&json!({ "to": "published" }))
            .await
            .assert_status_ok();
        request
            .get(&format!("/kb/articles/{article}"))
            .await
            .assert_status_ok();
        let counted = one_date(
            &ctx.db,
            &format!("SELECT day AS d FROM kb_daily WHERE article_pid = '{article}'"),
        )
        .await;
        assert_eq!(counted, kiritimati);

        // A ticket opened just after midnight there is on that day, though UTC's date is the day
        // before; the window ends on the organization's today.
        let ticket: Value = request
            .post("/api/tickets")
            .json(&json!({ "title": "Early ticket", "priority": "low" }))
            .await
            .json();
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE tickets SET opened_at = ('{kiritimati} 00:00:01'::timestamp \
                 AT TIME ZONE 'Pacific/Kiritimati') WHERE pid = '{}'",
                ticket["pid"].as_str().unwrap()
            ))
            .await
            .unwrap();
        let report: Value = request
            .get("/api/insights/kb-deflection?days=1")
            .await
            .json();
        assert_eq!(report["to"], json!(kiritimati), "{report}");
        assert_eq!(report["from"], json!(kiritimati), "{report}");
        assert_eq!(report["tickets_opened"], 1, "{report}");
        assert_eq!(report["views"], 1, "{report}");

        // A campaign's converted figures name the organization's day as the rate day.
        let campaign: Value = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Zoned", "cost_minor": 1_000, "currency": "EUR" }))
            .await
            .json();
        let funnel: Value = request
            .get(&format!(
                "/api/campaigns/{}/funnel?convert=true",
                campaign["pid"].as_str().unwrap()
            ))
            .await
            .json();
        assert_eq!(funnel["rate_date"], json!(kiritimati), "{funnel}");

        // A workflow's task is due two of the organization's days on.
        request
            .post("/api/workflows")
            .json(&json!({ "name": "Zoned task", "trigger": "contact_created",
                           "conditions": {},
                           "actions": [{ "type": "create_task", "summary": "Zoned",
                                         "due_in_days": 2 }] }))
            .await
            .assert_status_ok();
        let contact: Value = request
            .post("/api/contacts")
            .json(&json!({ "display_name": "Zo Ned", "person_ref": super::a_person() }))
            .await
            .json();
        let contact = contact["pid"].as_str().unwrap().to_string();
        let due = one_date(
            &ctx.db,
            &format!(
                "SELECT due_on AS d FROM activities WHERE kind = 'task' AND subject_pid = '{contact}'"
            ),
        )
        .await;
        assert_eq!(due, kiritimati + chrono::Days::new(2));

        // A follow-up due on Pago Pago's today is not overdue for a caller there, and a day
        // overdue on the organization's day.
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "contact", "subject_pid": contact,
                           "kind": "call", "summary": "Call back", "due_on": pago }))
            .await
            .assert_status_ok();
        let there: Value = request
            .get("/api/insights/followups")
            .add_header("crm-time-zone", "Pacific/Pago_Pago")
            .await
            .json();
        let summaries = |list: &Value| -> Vec<String> {
            list.as_array()
                .unwrap()
                .iter()
                .map(|f| f["summary"].as_str().unwrap().to_string())
                .collect()
        };
        assert!(!summaries(&there["overdue"]).contains(&"Call back".to_string()), "{there}");
        assert!(summaries(&there["upcoming_30d"]).contains(&"Call back".to_string()), "{there}");
        let here: Value = request.get("/api/insights/followups").await.json();
        let late = here["overdue"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["summary"] == "Call back")
            .unwrap_or_else(|| panic!("{here}"));
        assert_eq!(late["overdue_days"], 1, "{here}");

        request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1 }))
            .await
            .assert_status_ok();
    })
    .await;
}
