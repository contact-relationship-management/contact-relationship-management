//! Contact and account import (CRM-R43): the preview, the person and organization lookups, the
//! rows left out, no consent, no workflows, and undo.

use contact_relationship_management::app::App;
use contact_relationship_management::clients::Mode;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;
use crate::scoped_auth::scoped;

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

const CONTACTS: &str = "Name,Email,Job title\n\
    Imp Ortone,imp.ortone@import.test,Buyer\n\
    Imp Ortwo,imp.ortwo@import.test,\n\
    ,nobody@import.test,\n\
    Bad Address,not-an-address,\n\
    Imp Ortone Again,IMP.ORTONE@import.test,\n";

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn contacts_are_previewed_then_imported_with_no_consent_and_undone() {
    request::<App, _, _>(|request, ctx| async move {
        // Someone the file looks like, and a workflow an import must not start.
        let known = seed_contact!(&request, "Imp Ortone").await;
        request
            .post("/api/workflows")
            .json(&json!({ "name": "On create", "trigger": "contact_created",
                           "actions": [{ "type": "create_task", "summary": "Hello" }] }))
            .await
            .assert_status_ok();

        let columns: Value = request
            .post("/api/contact-imports/preview")
            .json(&json!({ "csv": CONTACTS }))
            .await
            .json();
        assert_eq!(columns["rows"], 5);
        assert_eq!(columns["suggested_mapping"]["display_name"], "Name");
        assert_eq!(columns["suggested_mapping"]["email"], "Email");
        assert_eq!(columns["suggested_mapping"]["job_title"], "Job title");

        let mapping = columns["suggested_mapping"].clone();
        let preview: Value = request
            .post("/api/contact-imports/preview")
            .json(&json!({ "csv": CONTACTS, "mapping": mapping }))
            .await
            .json();
        assert_eq!((preview["valid"].as_i64(), preview["invalid"].as_i64()), (Some(3), Some(2)));
        let report = preview["report"].as_array().unwrap();
        assert!(report[2]["reason"].as_str().unwrap().contains("no name"));
        assert!(report[3]["reason"].as_str().unwrap().contains("email"));
        let advice = report[0]["likely_duplicates"].as_array().unwrap();
        assert!(advice.iter().any(|d| d["pid"] == json!(known)), "{advice:?}");
        assert_eq!(count(&ctx.db, "SELECT count(*) AS n FROM contacts").await, 1, "a preview saves nothing");

        // A missing required column, a bad date, a missing note.
        for body in [
            json!({ "csv": CONTACTS, "mapping": { "display_name": "Name" },
                    "source_note": "x", "obtained_on": "2026-01-01" }),
            json!({ "csv": CONTACTS, "mapping": mapping, "source_note": " ", "obtained_on": "2026-01-01" }),
            json!({ "csv": CONTACTS, "mapping": mapping, "source_note": "x", "obtained_on": "2999-01-01" }),
        ] {
            request
                .post("/api/contact-imports")
                .json(&body)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }

        let done: Value = request
            .post("/api/contact-imports")
            .json(&json!({ "csv": CONTACTS, "mapping": mapping, "source_note": "Trade show",
                           "obtained_on": "2026-10-01" }))
            .await
            .json();
        // Three valid rows, but the first and the last are one address: one contact is made for it.
        assert_eq!(done["created_count"], 2, "{done}");
        assert_eq!(done["invalid_count"], 2);
        assert_eq!(done["skipped_count"], 1, "the repeated address");
        assert_eq!(done["status"], "completed");
        let consents = count(
            &ctx.db,
            "SELECT count(*) AS n FROM contacts WHERE display_name LIKE 'Imp Ort%' \
             AND marketing_consent <> 'never' AND id > (SELECT min(id) FROM contacts)",
        )
        .await;
        assert_eq!(consents, 0, "an imported contact has no consent");
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM activities WHERE kind = 'task'").await,
            0,
            "an import starts no workflow"
        );
        let job = count(
            &ctx.db,
            "SELECT count(*) AS n FROM contacts WHERE job_title = 'Buyer'",
        )
        .await;
        assert_eq!(job, 1);

        // Undo: one of the two has been worked since, so it stays.
        let ids = ctx
            .db
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT pid::text AS p FROM contacts WHERE display_name IN ('Imp Ortone', 'Imp Ortwo') \
                 AND id > (SELECT min(id) FROM contacts) ORDER BY id"
                    .to_string(),
            ))
            .await
            .unwrap();
        let first: String = ids[0].try_get("", "p").unwrap();
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "contact", "subject_pid": first,
                           "kind": "note", "summary": "Spoke at the show" }))
            .await
            .assert_status_ok();
        let import = done["pid"].as_str().unwrap();
        let undone: Value = request
            .post(&format!("/api/contact-imports/{import}/undo"))
            .await
            .json();
        assert_eq!((undone["removed"].as_i64(), undone["kept"].as_i64()), (Some(1), Some(1)));
        request
            .post(&format!("/api/contact-imports/{import}/undo"))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let listed: Vec<Value> = request.get("/api/contact-imports").await.json();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["status"], "undone");
        // An account import is not found under the contact route.
        request
            .get(&format!("/api/account-imports/{import}"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_person_service_that_cannot_answer_fails_the_row_not_the_import() {
    let _scope = scoped(|s| s.upstream_mode = Mode::Http);
    request::<App, _, _>(|request, _ctx| async move {
        let done: Value = request
            .post("/api/contact-imports")
            .json(&json!({
                "csv": "Name,Email\nAda Lovelace,ada@import.test\n",
                "mapping": { "display_name": "Name", "email": "Email" },
                "source_note": "Test", "obtained_on": "2026-10-01" }))
            .await
            .json();
        assert_eq!(done["created_count"], 0);
        assert_eq!(done["failed_count"], 1);
        assert_eq!(done["status"], "completed");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn accounts_are_imported_with_tiers_and_duplicates_by_name_and_undone() {
    request::<App, _, _>(|request, ctx| async move {
        request
            .post("/api/accounts")
            .json(&json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Engine Co" }))
            .await
            .assert_status_ok();
        let urn = format!("organization:{}", uuid::Uuid::new_v4());
        let csv = format!(
            "Company,Ref,Tier,Sector\n\
             engine  co,,customer,Machines\n\
             Foundry Ltd,{urn},,Metal\n\
             Foundry Ltd,,,\n\
             Badtier Inc,,vip,\n"
        );
        let columns: Value = request
            .post("/api/account-imports/preview")
            .json(&json!({ "csv": csv }))
            .await
            .json();
        let mapping = json!({ "display_name": "Company", "organization_ref": "Ref",
                              "tier": "Tier", "industry": "Sector" });
        assert_eq!(columns["suggested_mapping"]["industry"], "Sector");
        let preview: Value = request
            .post("/api/account-imports/preview")
            .json(&json!({ "csv": csv, "mapping": mapping }))
            .await
            .json();
        assert_eq!((preview["valid"].as_i64(), preview["invalid"].as_i64()), (Some(3), Some(1)));
        let report = preview["report"].as_array().unwrap();
        assert_eq!(report[0]["likely_duplicates"][0]["kind"], "account");
        assert_eq!(report[2]["likely_duplicates"][0]["kind"], "row");
        assert!(report[3]["reason"].as_str().unwrap().contains("tier"));

        let done: Value = request
            .post("/api/account-imports")
            .json(&json!({ "csv": csv, "mapping": mapping, "source_note": "Register",
                           "obtained_on": "2026-10-01", "skip_likely_duplicates": true }))
            .await
            .json();
        // Engine Co is already here and the second Foundry Ltd repeats the first: both left out.
        assert_eq!(done["created_count"], 1, "{done}");
        assert_eq!(done["skipped_count"], 2);
        assert_eq!(
            count(
                &ctx.db,
                &format!("SELECT count(*) AS n FROM accounts WHERE organization_ref = '{urn}' AND industry = 'Metal'")
            )
            .await,
            1
        );
        // Undo keeps an account a contact has been attached to.
        let pid = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT pid::text AS p FROM accounts WHERE organization_ref = '{urn}'"),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "p")
            .unwrap();
        let contact: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                           "display_name": "Works There", "account_pid": pid }))
            .await
            .json();
        assert!(contact["pid"].is_string());
        let undone: Value = request
            .post(&format!("/api/account-imports/{}/undo", done["pid"].as_str().unwrap()))
            .await
            .json();
        assert_eq!((undone["removed"].as_i64(), undone["kept"].as_i64()), (Some(0), Some(1)));
    })
    .await;
}
