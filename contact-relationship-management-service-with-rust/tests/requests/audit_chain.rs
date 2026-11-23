//! Tamper-evident audit chain (CRM-T31): audited writes extend a verified
//! chain; the append-only trigger refuses edits and deletes; and a chain
//! altered behind the trigger's back is reported at the altered row.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use serde_json::Value;
use serial_test::serial;

use super::seed_contact;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn audit_chain_detects_tampering() {
    request::<App, _, _>(|request, ctx| async move {
        seed_contact!(&request, "Chain Subject One").await;
        seed_contact!(&request, "Chain Subject Two").await;

        let verdict: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(verdict["ok"], true, "{verdict}");
        assert!(verdict["rows"].as_u64().unwrap() >= 2);
        assert!(verdict["head"].is_string());

        // The append-only trigger refuses edits and deletes.
        let edit = sqlx::raw_sql("UPDATE audit_logs SET action = 'tampered'")
            .execute(contact_relationship_management::db::pool(&ctx))
            .await;
        assert!(edit.is_err(), "audit edit must be refused");
        let delete = sqlx::raw_sql("DELETE FROM audit_logs")
            .execute(contact_relationship_management::db::pool(&ctx))
            .await;
        assert!(delete.is_err(), "audit delete must be refused");

        // Behind the trigger's back (an owner who disables it), the chain
        // still reveals the change.
        sqlx::raw_sql("ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_append_only")
            .execute(contact_relationship_management::db::pool(&ctx))
            .await
            .unwrap();
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            "UPDATE audit_logs SET action = 'tampered' \
                 WHERE id = (SELECT min(id) FROM audit_logs)",
        ))
        .execute(contact_relationship_management::db::pool(&ctx))
        .await
        .unwrap();
        sqlx::raw_sql("ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_append_only")
            .execute(contact_relationship_management::db::pool(&ctx))
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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

/// `GET /api/audits?owner=&since=` returns the entries whose snapshot names that owner, and no
/// one else's.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_owner_query_returns_only_that_owners_entries() {
    use contact_relationship_management::models::audit_logs::Model as Audit;
    request::<App, _, _>(|request, ctx| async move {
        let mine = format!("person:{}", uuid::Uuid::new_v4());
        let theirs = format!("person:{}", uuid::Uuid::new_v4());
        let pool = contact_relationship_management::db::pool(&ctx);
        let mut txn = pool.begin().await.unwrap();
        for (owner, action) in [(&mine, "mine"), (&theirs, "theirs"), (&mine, "mine-again")] {
            Audit::record_on(
                &mut txn,
                "contact",
                uuid::Uuid::new_v4(),
                action,
                Some("test"),
                Some(serde_json::json!({ "owner": owner })),
            )
            .await
            .unwrap();
        }
        txn.commit().await.unwrap();

        let found: Vec<Value> = request
            .get("/api/audits")
            .add_query_param("owner", &mine)
            .add_query_param("since", "2000-01-01T00:00:00Z")
            .await
            .json();
        let actions: Vec<&str> = found
            .iter()
            .map(|e| e["action"].as_str().unwrap())
            .collect();
        assert_eq!(actions.len(), 2, "{found:?}");
        assert!(actions.contains(&"mine") && actions.contains(&"mine-again"));
        assert!(
            found
                .iter()
                .all(|e| e["snapshot"]["owner"] == mine.as_str())
        );

        // Entries from before `since` are not returned.
        let none: Vec<Value> = request
            .get("/api/audits")
            .add_query_param("owner", &mine)
            .add_query_param("since", "2999-01-01T00:00:00Z")
            .await
            .json();
        assert!(none.is_empty(), "{none:?}");
    })
    .await;
}
