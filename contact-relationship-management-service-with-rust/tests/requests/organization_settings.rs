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
