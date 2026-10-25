//! Saved views (CRM-R35): make, list, replace and delete; the closed sets; unique names.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_view_is_saved_listed_replaced_and_deleted() {
    request::<App, _, _>(|request, ctx| async move {
        let body = json!({
            "kind": "ticket", "name": "My urgent",
            "filter": { "status": ["open", "pending"], "priority": ["urgent", "high"] },
            "sort": { "field": "priority", "direction": "desc" },
            "columns": ["title", "priority"], "shared": false,
        });
        let created: Value = request.post("/api/views").json(&body).await.json();
        let pid = created["pid"].as_str().unwrap().to_string();

        let listed: Vec<Value> = request.get("/api/views?kind=ticket").await.json();
        let mine = listed
            .iter()
            .find(|v| v["pid"] == pid.as_str())
            .expect("listed");
        assert_eq!(mine["name"], "My urgent");
        assert_eq!(mine["mine"], true);
        assert_eq!(mine["filter"]["priority"], json!(["urgent", "high"]));
        assert_eq!(
            mine["sort"],
            json!({ "field": "priority", "direction": "desc" })
        );
        assert_eq!(mine["columns"], json!(["title", "priority"]));
        // Another list's filter does not show it.
        let others: Vec<Value> = request.get("/api/views?kind=lead").await.json();
        assert!(others.iter().all(|v| v["pid"] != pid.as_str()));

        // The name is unique for the owner and list, whatever its case.
        let mut again = body.clone();
        again["name"] = json!("MY URGENT");
        assert_eq!(
            request.post("/api/views").json(&again).await.status_code(),
            422
        );
        again["kind"] = json!("lead");
        again["filter"] = json!({});
        again["sort"] = Value::Null;
        again["columns"] = Value::Null;
        request
            .post("/api/views")
            .json(&again)
            .await
            .assert_status_ok();

        // Replace: shared now; the list cannot change; a clash is refused.
        let mut shared = body.clone();
        shared["shared"] = json!(true);
        shared["filter"] = json!({ "status": ["open"] });
        request
            .put(&format!("/api/views/{pid}"))
            .json(&shared)
            .await
            .assert_status_ok();
        let now: Vec<Value> = request.get("/api/views?kind=ticket").await.json();
        let view = now.iter().find(|v| v["pid"] == pid.as_str()).unwrap();
        assert_eq!(view["shared"], true);
        assert_eq!(view["filter"], json!({ "status": ["open"] }));
        let mut moved = shared.clone();
        moved["kind"] = json!("lead");
        moved["filter"] = json!({});
        assert_eq!(
            request
                .put(&format!("/api/views/{pid}"))
                .json(&moved)
                .await
                .status_code(),
            422
        );
        let mut clash = shared.clone();
        clash["name"] = json!("Second");
        let second: Value = request.post("/api/views").json(&clash).await.json();
        let mut rename = clash.clone();
        rename["name"] = json!("my urgent");
        assert_eq!(
            request
                .put(&format!("/api/views/{}", second["pid"].as_str().unwrap()))
                .json(&rename)
                .await
                .status_code(),
            422
        );

        // The closed sets.
        for (field, value) in [
            ("kind", json!("campaign")),
            ("name", json!("  ")),
            ("filter", json!({ "tier": ["customer"] })),
            ("filter", json!({ "status": ["nonsense"] })),
            ("filter", json!({ "status": [] })),
            ("sort", json!({ "field": "salary", "direction": "asc" })),
            ("sort", json!({ "field": "title", "direction": "sideways" })),
            ("columns", json!(["title", "salary"])),
        ] {
            let mut bad = body.clone();
            bad["name"] = json!("Bad one");
            bad[field] = value.clone();
            assert_eq!(
                request.post("/api/views").json(&bad).await.status_code(),
                422,
                "{field}: {value}"
            );
        }
        assert_eq!(
            request.get("/api/views?kind=campaign").await.status_code(),
            422
        );

        // Deleting, and the audit rows (list and sharing, never the name).
        request
            .delete(&format!("/api/views/{pid}"))
            .await
            .assert_status_ok();
        let after: Vec<Value> = request.get("/api/views").await.json();
        assert!(after.iter().all(|v| v["pid"] != pid.as_str()));
        request
            .delete(&format!("/api/views/{pid}"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        use sea_orm::ConnectionTrait;
        let row = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM audit_logs WHERE entity = 'view' \
                 AND snapshot::text ILIKE '%urgent%'",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "n").unwrap(), 0);
        let row = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT count(DISTINCT action) AS n FROM audit_logs WHERE entity = 'view'",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "n").unwrap(), 3);
    })
    .await;
}
