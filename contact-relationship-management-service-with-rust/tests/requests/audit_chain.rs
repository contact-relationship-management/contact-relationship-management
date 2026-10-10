//! Tamper-evident audit chain (CRM-T31): audited writes extend a verified
//! chain; the append-only trigger refuses edits and deletes; and a chain
//! altered behind the trigger's back is reported at the altered row.

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::ConnectionTrait;
use serde_json::Value;
use serial_test::serial;

use super::seed_contact;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn audit_chain_detects_tampering() {
    request::<App, _, _>(|request, ctx| async move {
        seed_contact!(&request, "Chain Subject One").await;
        seed_contact!(&request, "Chain Subject Two").await;

        let verdict: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(verdict["ok"], true, "{verdict}");
        assert!(verdict["rows"].as_u64().unwrap() >= 2);
        assert!(verdict["head"].is_string());

        // The append-only trigger refuses edits and deletes.
        let edit = ctx
            .db
            .execute_unprepared("UPDATE audit_logs SET action = 'tampered'")
            .await;
        assert!(edit.is_err(), "audit edit must be refused");
        let delete = ctx.db.execute_unprepared("DELETE FROM audit_logs").await;
        assert!(delete.is_err(), "audit delete must be refused");

        // Behind the trigger's back (an owner who disables it), the chain
        // still reveals the change.
        ctx.db
            .execute_unprepared("ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_append_only")
            .await
            .unwrap();
        ctx.db
            .execute_unprepared(
                "UPDATE audit_logs SET action = 'tampered' \
                 WHERE id = (SELECT min(id) FROM audit_logs)",
            )
            .await
            .unwrap();
        ctx.db
            .execute_unprepared("ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_append_only")
            .await
            .unwrap();
        let broken: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(broken["ok"], false, "{broken}");
        assert!(broken["first_break"]["id"].is_i64());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn personal_data_reads_are_audited() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = seed_contact!(&request, "Read Audit Subject").await;
        request
            .get(&format!("/api/contacts/{pid}"))
            .await
            .assert_status_ok();
        request.get("/api/contacts").await.assert_status_ok();

        let trail: Value = request.get(&format!("/api/audits/{pid}")).await.json();
        let actions: Vec<&str> = trail
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|row| row["action"].as_str())
            .collect();
        assert!(actions.contains(&"contact_read"), "{actions:?}");

        let recent: Value = request.get("/api/audits/recent").await.json();
        let listed = recent
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["action"] == "contact_list_read");
        assert!(listed, "a list read writes one summary row");

        // Read audits join the chain rather than breaking it.
        let verdict: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(verdict["ok"], true, "{verdict}");
    })
    .await;
}

/// Deal-list and insight reads are audited (CRM-T47 remainder).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn deal_and_insight_reads_are_audited() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/api/deals").await.status_code(), 200);
        assert_eq!(
            request.get("/api/insights/stale-deals").await.status_code(),
            200
        );
        let recent: Value = request.get("/api/audits/recent").await.json();
        let has = |action: &str| {
            recent
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["action"] == action)
        };
        assert!(has("deal_list_read"), "{recent}");
        assert!(has("insight_read"), "{recent}");
    })
    .await;
}
