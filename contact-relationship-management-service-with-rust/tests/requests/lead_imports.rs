//! Lead import (CRM-R34): mapping, a bad row, duplicate advice, commit in batches, undo, no
//! consent, and the audit rows.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::a_person;

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

const FILE: &str = "Full Name,E-mail,Organisation,Notes\n\
    Ada Lovelace,ada@engine.test,Analytical Engines,met at stand 4\n\
    \"Hopper, Grace\",grace@navy.test,US Navy,\n\
    ,nobody@x.test,,no name\n\
    Alan Turing,not-an-address,Bletchley,\n\
    Margaret Hamilton,margaret@apollo.test,MIT\n\
    Ada Lovelace,ada2@engine.test,Analytical Engines,again\n";

fn mapping() -> Value {
    json!({ "display_name": "Full Name", "email": "E-mail", "company": "Organisation" })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_preview_maps_columns_reports_each_row_and_gives_duplicate_advice_only() {
    request::<App, _, _>(|request, ctx| async move {
        // Someone already known: a contact and an earlier lead.
        request
            .post("/api/contacts")
            .json(&json!({ "person_ref": a_person(), "display_name": "Grace Hopper" }))
            .await
            .assert_status_ok();
        request
            .post("/api/leads")
            .json(&json!({ "display_name": "Alan Turing", "source": "web" }))
            .await
            .assert_status_ok();
        let leads_before = count(&ctx.db, "SELECT count(*) AS n FROM leads").await;

        // Step one: columns and a suggestion, no report.
        let first: Value = request
            .post("/api/lead-imports/preview")
            .json(&json!({ "csv": FILE }))
            .await
            .json();
        assert_eq!(
            first["columns"],
            json!(["Full Name", "E-mail", "Organisation", "Notes"])
        );
        assert_eq!(first["rows"], 6);
        assert_eq!(first["suggested_mapping"], mapping());
        assert!(first["report"].is_null());

        // Step two: the report.
        let second: Value = request
            .post("/api/lead-imports/preview")
            .json(&json!({ "csv": FILE, "mapping": mapping() }))
            .await
            .json();
        let report = second["report"].as_array().unwrap();
        assert_eq!(report.len(), 6);
        assert_eq!(report[0]["status"], "valid");
        assert_eq!(report[1]["name"], "Hopper, Grace");
        assert_eq!(report[2]["status"], "invalid");
        assert!(report[2]["reason"].as_str().unwrap().contains("no name"));
        assert_eq!(report[3]["status"], "invalid", "a bad address");
        assert!(report[3]["reason"].as_str().unwrap().contains("email"));
        assert_eq!(report[4]["status"], "invalid", "a short row");
        assert!(report[4]["reason"].as_str().unwrap().contains("columns"));
        assert_eq!(second["valid"], 3);
        assert_eq!(second["invalid"], 3);
        // Advice: the surname-first row matches the contact; the repeated name matches row 1.
        let grace = report[1]["likely_duplicates"].as_array().unwrap();
        assert!(grace.iter().any(|d| d["kind"] == "contact"), "{grace:?}");
        let again = report[5]["likely_duplicates"].as_array().unwrap();
        assert!(
            again.iter().any(|d| d["kind"] == "row" && d["row"] == 1),
            "{again:?}"
        );
        // Nothing was saved.
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads").await,
            leads_before
        );

        // Refusals: no name column, an unknown column, an empty file.
        for body in [
            json!({ "csv": FILE, "mapping": { "email": "E-mail" } }),
            json!({ "csv": FILE, "mapping": { "display_name": "Nope" } }),
            json!({ "csv": "" }),
        ] {
            assert_eq!(
                request
                    .post("/api/lead-imports/preview")
                    .json(&body)
                    .await
                    .status_code(),
                422
            );
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_commit_saves_valid_rows_without_consent_and_can_be_undone() {
    request::<App, _, _>(|request, ctx| async move {
        let today = chrono::Utc::now().date_naive().to_string();
        let body = |skip: bool| {
            json!({ "csv": FILE, "mapping": mapping(), "source_note": "Trade show 2026",
                    "obtained_on": today, "skip_likely_duplicates": skip })
        };
        // The source and the date are required, and the date is not in the future.
        for bad in [
            json!({ "csv": FILE, "mapping": mapping(), "obtained_on": today }),
            json!({ "csv": FILE, "mapping": mapping(), "source_note": "x", "obtained_on": "2999-01-01" }),
            json!({ "csv": FILE, "mapping": mapping(), "source_note": " ", "obtained_on": today }),
        ] {
            assert_eq!(
                request.post("/api/lead-imports").json(&bad).await.status_code(),
                422
            );
        }

        let done: Value = request.post("/api/lead-imports").json(&body(false)).await.json();
        assert_eq!(done["status"], "completed");
        assert_eq!(done["row_count"], 6);
        assert_eq!(done["invalid_count"], 3);
        assert_eq!(done["created_count"], 3);
        let import = done["pid"].as_str().unwrap().to_string();
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads WHERE source = 'import' AND deleted_at IS NULL").await,
            3
        );
        // Domains only: no address, no contact, no consent of any kind.
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads WHERE source = 'import' AND email IS NOT NULL").await,
            0
        );
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads WHERE source = 'import' AND email_domain = 'engine.test'").await,
            2
        );
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads WHERE source = 'import' AND contact_pid IS NOT NULL").await,
            0
        );
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM consent_events WHERE source ILIKE '%import%'").await,
            0
        );

        // The audit rows: committed (with the source and date, no names), one per lead.
        let committed = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT snapshot::text AS s FROM audit_logs WHERE entity_pid = '{import}' AND action = 'lead_import_committed'"),
            ))
            .await
            .unwrap()
            .expect("an audit row");
        let snapshot: String = committed.try_get("", "s").unwrap();
        assert!(snapshot.contains("Trade show 2026") && snapshot.contains(&today));
        assert!(!snapshot.contains("Lovelace") && !snapshot.contains("engine.test"));
        let chain: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(chain["ok"], true, "{chain}");

        // The record is listed and readable.
        let listed: Vec<Value> = request.get("/api/lead-imports").await.json();
        assert!(listed.iter().any(|r| r["pid"] == import.as_str()));
        let one: Value = request.get(&format!("/api/lead-imports/{import}")).await.json();
        assert_eq!(one["source_note"], "Trade show 2026");

        // One lead has been worked; undo removes the others and keeps that one.
        let kept: String = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT pid::text AS p FROM leads WHERE source = 'import' ORDER BY id LIMIT 1",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "p")
            .unwrap();
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "lead", "subject_pid": kept, "kind": "call", "summary": "rang" }))
            .await
            .assert_status_ok();
        let undone: Value = request
            .post(&format!("/api/lead-imports/{import}/undo"))
            .await
            .json();
        assert_eq!(undone, json!({ "removed": 2, "kept": 1 }));
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads WHERE source = 'import' AND deleted_at IS NULL").await,
            1
        );
        let again = request.post(&format!("/api/lead-imports/{import}/undo")).await;
        assert_eq!(again.status_code(), 422, "undone once");
        let one: Value = request.get(&format!("/api/lead-imports/{import}")).await.json();
        assert_eq!(one["status"], "undone");
        assert_eq!(one["undone_count"], 2);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM audit_logs WHERE action = 'import_undone'").await,
            2
        );
        request
            .post(&format!("/api/lead-imports/{}/undo", uuid::Uuid::new_v4()))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn likely_duplicates_can_be_left_out_and_a_long_file_goes_in_batches() {
    request::<App, _, _>(|request, ctx| async move {
        let today = chrono::Utc::now().date_naive().to_string();
        let skipped: Value = request
            .post("/api/lead-imports")
            .json(
                &json!({ "csv": FILE, "mapping": mapping(), "source_note": "again",
                           "obtained_on": today, "skip_likely_duplicates": true }),
            )
            .await
            .json();
        // Row 6 repeats row 1, so it is left out; the others have no advice.
        assert_eq!(skipped["skipped_count"], 1);
        assert_eq!(skipped["created_count"], 2);

        // 250 rows are saved in three batches and all are recorded for undo.
        let mut csv = String::from("Name\n");
        for n in 0..250 {
            csv.push_str(&format!("Batch Person{n:03} Zxq{n}\n"));
        }
        let big: Value = request
            .post("/api/lead-imports")
            .json(&json!({ "csv": csv, "mapping": { "display_name": "Name" },
                           "source_note": "big", "obtained_on": today }))
            .await
            .json();
        assert_eq!(big["created_count"], 250);
        let undone: Value = request
            .post(&format!(
                "/api/lead-imports/{}/undo",
                big["pid"].as_str().unwrap()
            ))
            .await
            .json();
        assert_eq!(undone["removed"], 250);
        let too_many = format!("Name\n{}", "x y\n".repeat(501));
        assert_eq!(
            request
                .post("/api/lead-imports/preview")
                .json(&json!({ "csv": too_many }))
                .await
                .status_code(),
            422
        );
        assert!(count(&ctx.db, "SELECT count(*) AS n FROM lead_imports").await >= 2);
    })
    .await;
}
