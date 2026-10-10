//! Field-level encryption (CRM-T41): the database holds ciphertext, every API
//! surface returns plaintext, legacy plaintext still reads, key rotation
//! re-seals, and a retired key fails loudly instead of returning garbage.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use contact_relationship_management_service::app::App;
use contact_relationship_management_service::crypto::{self, Keyring};
use contact_relationship_management_service::tasks::rotate_field_keys::rotate;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;

fn keyring(ids: &[(&str, u8)]) -> Keyring {
    let spec = ids
        .iter()
        .map(|(id, byte)| format!("{id}:{}", STANDARD.encode([*byte; 32])))
        .collect::<Vec<_>>()
        .join(",");
    Keyring::from_spec(&spec).unwrap()
}

async fn stored(db: &sea_orm::DatabaseConnection, sql: &str) -> Vec<String> {
    db.query_all_raw(Statement::from_string(
        DatabaseBackend::Postgres,
        sql.to_string(),
    ))
    .await
    .unwrap()
    .iter()
    .map(|row| row.try_get::<String>("", "v").unwrap())
    .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn fields_are_encrypted_at_rest_and_plaintext_in_the_api() {
    request::<App, _, _>(|request, ctx| async move {
        crypto::install(Some(keyring(&[("k1", 1)])));

        let pid = seed_contact!(&request, "Encrypted Subject").await;
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "contact", "subject_pid": pid,
                           "kind": "note", "summary": "Prefers afternoon calls" }))
            .await
            .assert_status_ok();

        // At rest: ciphertext, never the plaintext.
        let summaries = stored(
            &ctx.db,
            &format!("SELECT summary AS v FROM activities WHERE subject_pid = '{pid}'"),
        )
        .await;
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].starts_with("enc:v1:k1:"), "{}", summaries[0]);
        assert!(!summaries[0].contains("afternoon"));
        let channels = stored(
            &ctx.db,
            &format!("SELECT preferred_channel AS v FROM contacts WHERE pid = '{pid}'"),
        )
        .await;
        assert!(channels[0].starts_with("enc:v1:k1:"), "{}", channels[0]);

        // Through the API and the subject-access export: plaintext.
        let detail: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(detail["activities"][0]["summary"], "Prefers afternoon calls");
        assert!(!detail["contact"]["preferred_channel"].as_str().unwrap().starts_with("enc:"));
        let export: Value = request
            .get(&format!("/api/contacts/{pid}/subject-access"))
            .await
            .json();
        assert_eq!(export["activities"][0]["summary"], "Prefers afternoon calls");

        // Legacy plaintext (a raw write) still reads, and is reported for sealing.
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE activities SET summary = 'legacy plaintext' WHERE subject_pid = '{pid}'"
            ))
            .await
            .unwrap();
        let legacy: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(legacy["activities"][0]["summary"], "legacy plaintext");

        // Rotation: new first key, old key kept for reading; everything is
        // re-sealed under the new key and still reads.
        crypto::install(Some(keyring(&[("k2", 2), ("k1", 1)])));
        let report = rotate(&ctx.db).await.unwrap();
        assert!(report.iter().any(|(_, n)| *n > 0), "{report:?}");
        let after = stored(
            &ctx.db,
            &format!("SELECT summary AS v FROM activities WHERE subject_pid = '{pid}'"),
        )
        .await;
        assert!(after[0].starts_with("enc:v1:k2:"), "{}", after[0]);
        let reread: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(reread["activities"][0]["summary"], "legacy plaintext");
        // Idempotent: a second run changes nothing.
        assert!(rotate(&ctx.db).await.unwrap().iter().all(|(_, n)| *n == 0));

        // Retire k2 without rotating: the data is unreadable, loudly.
        crypto::install(Some(keyring(&[("k3", 3)])));
        let blocked = request.get(&format!("/api/contacts/{pid}")).await;
        assert!(blocked.status_code().is_server_error(), "{}", blocked.status_code());

        // Put the world back for the other tests in this process.
        crypto::install(Some(keyring(&[("k2", 2)])));
        rotate(&ctx.db).await.ok();
        crypto::install(None);
        ctx.db
            .execute_unprepared("UPDATE activities SET summary = 'x' WHERE summary LIKE 'enc:v1:%'")
            .await
            .unwrap();
        ctx.db
            .execute_unprepared("UPDATE contacts SET preferred_channel = 'email' WHERE preferred_channel LIKE 'enc:v1:%'")
            .await
            .unwrap();
    })
    .await;
}
