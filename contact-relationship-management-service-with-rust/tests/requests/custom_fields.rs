//! Typed custom fields (CRM-R41): declaring them, values checked against their kind, the
//! sensitivity that decides sealing, subject access and erasure, and key rotation.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use contact_relationship_management::app::App;
use contact_relationship_management::crypto::{self, Keyring};
use contact_relationship_management::tasks::rotate_field_keys::rotate;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;

async fn texts(db: &sea_orm::DatabaseConnection, sql: &str) -> Vec<String> {
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

macro_rules! define {
    ($request:expr, $entity:expr, $key:expr, $kind:expr, $sensitivity:expr) => {
        define!($request, $entity, $key, $kind, $sensitivity, json!([]))
    };
    ($request:expr, $entity:expr, $key:expr, $kind:expr, $sensitivity:expr, $choices:expr) => {{
        let answer = $request
            .post("/api/custom-fields")
            .json(&json!({ "entity": $entity, "key": $key, "label": $key, "kind": $kind,
                           "sensitivity": $sensitivity, "choices": $choices }))
            .await;
        answer.assert_status_ok();
        answer.json::<Value>()["pid"].as_str().unwrap().to_string()
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn fields_are_declared_and_values_are_checked_against_their_kind() {
    request::<App, _, _>(|request, _ctx| async move {
        define!(&request, "contact", "notes", "text", "business");
        define!(&request, "contact", "seats", "number", "business");
        define!(&request, "contact", "renews_on", "date", "business");
        define!(&request, "contact", "tier", "choice", "business", json!(["gold", "silver"]));
        let contact = seed_contact!(&request, "Custom Subject").await;
        let url = format!("/api/contacts/{contact}/custom-fields");

        let listed: Vec<Value> = request.get("/api/custom-fields?entity=contact").await.json();
        assert_eq!(listed.len(), 4);

        let set: Vec<Value> = request
            .put(&url)
            .json(&json!({ "values": { "notes": "Likes calls", "seats": 12.5,
                                        "renews_on": "2026-12-01", "tier": "gold" } }))
            .await
            .json();
        let by = |key: &str| set.iter().find(|v| v["key"] == key).unwrap()["value"].clone();
        assert_eq!(by("notes"), "Likes calls");
        assert_eq!(by("seats"), json!(12.5));
        assert_eq!(by("renews_on"), "2026-12-01");
        assert_eq!(by("tier"), "gold");

        // Each wrong value is refused and names its field.
        for (key, value) in [
            ("seats", json!("many")),
            ("renews_on", json!("1 Dec")),
            ("tier", json!("bronze")),
            ("notes", json!(5)),
            ("nonsense", json!("x")),
        ] {
            let r = request.put(&url).json(&json!({ "values": { key: value } })).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let body: Value = r.json();
            assert_eq!(body["errors"][0]["pointer"], format!("/{key}"), "{body}");
        }
        let after: Vec<Value> = request.get(&url).await.json();
        assert_eq!(
            after.iter().find(|v| v["key"] == "notes").unwrap()["value"],
            "Likes calls"
        );

        // null clears.
        let cleared: Vec<Value> = request
            .put(&url)
            .json(&json!({ "values": { "seats": null } }))
            .await
            .json();
        assert_eq!(
            cleared.iter().find(|v| v["key"] == "seats").unwrap()["value"],
            Value::Null
        );

        // Refused fields: a repeated key, a bad key, choices on a text field, an empty change.
        for body in [
            json!({ "entity": "contact", "key": "notes", "label": "x", "kind": "text", "sensitivity": "business" }),
            json!({ "entity": "contact", "key": "Bad Key", "label": "x", "kind": "text", "sensitivity": "business" }),
            json!({ "entity": "contact", "key": "plain", "label": "x", "kind": "text", "sensitivity": "business", "choices": ["a"] }),
            json!({ "entity": "contact", "key": "secretive", "label": "x", "kind": "text", "sensitivity": "secret" }),
        ] {
            request
                .post("/api/custom-fields")
                .json(&body)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        request
            .put(&url)
            .json(&json!({ "values": {} }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        request
            .get(&format!("/api/contacts/{}/custom-fields", uuid::Uuid::new_v4()))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn every_kind_of_record_can_carry_fields_and_deleting_a_field_deletes_its_values() {
    request::<App, _, _>(|request, ctx| async move {
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Field Org" }),
            )
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let field = define!(&request, "account", "region", "text", "business");
        request
            .put(&format!("/api/accounts/{account}/custom-fields"))
            .json(&json!({ "values": { "region": "North" } }))
            .await
            .assert_status_ok();
        // A field of another kind of record is not a field of this one.
        define!(&request, "contact", "region", "text", "business");
        let contact = seed_contact!(&request, "Other Kind").await;
        request
            .put(&format!("/api/contacts/{contact}/custom-fields"))
            .json(&json!({ "values": { "region": "South" } }))
            .await
            .assert_status_ok();
        let on_account: Vec<Value> = request
            .get(&format!("/api/accounts/{account}/custom-fields"))
            .await
            .json();
        assert_eq!(on_account.len(), 1);
        assert_eq!(on_account[0]["value"], "North");

        request
            .delete(&format!("/api/custom-fields/{field}"))
            .await
            .assert_status_ok();
        let left = texts(
            &ctx.db,
            "SELECT count(*)::text AS v FROM custom_field_values",
        )
        .await;
        assert_eq!(left, ["1"], "only the contact's value remains");
        assert!(
            request
                .get(&format!("/api/accounts/{account}/custom-fields"))
                .await
                .json::<Vec<Value>>()
                .is_empty()
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_choice_in_use_cannot_be_removed() {
    request::<App, _, _>(|request, _ctx| async move {
        let field = define!(
            &request,
            "contact",
            "stage",
            "choice",
            "business",
            json!(["a", "b"])
        );
        let contact = seed_contact!(&request, "Choice Subject").await;
        request
            .put(&format!("/api/contacts/{contact}/custom-fields"))
            .json(&json!({ "values": { "stage": "a" } }))
            .await
            .assert_status_ok();
        let put = |choices: Value| {
            let request = &request;
            let url = format!("/api/custom-fields/{field}");
            async move {
                request
                    .put(&url)
                    .json(&json!({ "label": "Stage", "choices": choices, "position": 0 }))
                    .await
                    .status_code()
            }
        };
        assert_eq!(put(json!(["b"])).await, 409);
        assert_eq!(put(json!(["a", "c"])).await, 200);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn sensitivity_decides_sealing_subject_access_erasure_and_rotation() {
    request::<App, _, _>(|request, ctx| async move {
        let keyring = |id: &str, byte: u8| {
            Keyring::from_spec(&format!("{id}:{}", STANDARD.encode([byte; 32]))).unwrap()
        };
        crypto::install(Some(keyring("k1", 1)));
        define!(
            &request,
            "contact",
            "account_manager_note",
            "text",
            "business"
        );
        define!(&request, "contact", "hobby", "text", "personal");
        define!(&request, "contact", "diagnosis", "text", "sensitive");
        let contact = seed_contact!(&request, "Sensitive Subject").await;
        let url = format!("/api/contacts/{contact}/custom-fields");
        request
            .put(&url)
            .json(
                &json!({ "values": { "account_manager_note": "VIP", "hobby": "sailing",
                                        "diagnosis": "private words" } }),
            )
            .await
            .assert_status_ok();

        // Only the sensitive one is sealed in the database; the API gives all three back.
        let raw = texts(
            &ctx.db,
            "SELECT v.value AS v FROM custom_field_values v JOIN custom_field_defs d \
             ON d.pid = v.field_pid WHERE d.key = 'diagnosis'",
        )
        .await;
        assert!(raw[0].starts_with("enc:v1:k1:"), "{}", raw[0]);
        let plain = texts(
            &ctx.db,
            "SELECT v.value AS v FROM custom_field_values v JOIN custom_field_defs d \
             ON d.pid = v.field_pid WHERE d.key = 'hobby'",
        )
        .await;
        assert_eq!(plain[0], "sailing");
        let read: Vec<Value> = request.get(&url).await.json();
        assert_eq!(
            read.iter().find(|v| v["key"] == "diagnosis").unwrap()["value"],
            "private words"
        );

        // Rotation re-seals the sensitive one only, and everything still reads.
        crypto::install(Some(keyring("k2", 2)));
        // k1 must still open: keep both keys.
        crypto::install(Some(
            Keyring::from_spec(&format!(
                "k2:{},k1:{}",
                STANDARD.encode([2u8; 32]),
                STANDARD.encode([1u8; 32])
            ))
            .unwrap(),
        ));
        let report = rotate(&ctx.db).await.unwrap();
        assert!(
            report
                .iter()
                .any(|(c, n)| c.starts_with("custom_field_values") && *n == 1),
            "{report:?}"
        );
        let resealed = texts(
            &ctx.db,
            "SELECT v.value AS v FROM custom_field_values v JOIN custom_field_defs d \
             ON d.pid = v.field_pid WHERE d.key = 'diagnosis'",
        )
        .await;
        assert!(resealed[0].starts_with("enc:v1:k2:"), "{}", resealed[0]);
        let plain = texts(
            &ctx.db,
            "SELECT v.value AS v FROM custom_field_values v JOIN custom_field_defs d \
             ON d.pid = v.field_pid WHERE d.key = 'hobby'",
        )
        .await;
        assert_eq!(
            plain[0], "sailing",
            "a plain field is not sealed by rotation"
        );
        let read: Vec<Value> = request.get(&url).await.json();
        assert_eq!(
            read.iter().find(|v| v["key"] == "diagnosis").unwrap()["value"],
            "private words"
        );

        // Subject access lists all three, unmasked.
        let export: Value = request
            .get(&format!("/api/contacts/{contact}/subject-access"))
            .await
            .json();
        let exported: Vec<&str> = export["custom_fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["key"].as_str().unwrap())
            .collect();
        let mut exported = exported;
        exported.sort_unstable();
        assert_eq!(exported, ["account_manager_note", "diagnosis", "hobby"]);

        // Erasure deletes the personal and sensitive values and keeps the business one.
        request
            .post(&format!("/api/contacts/{contact}/erase"))
            .await
            .assert_status_ok();
        let left = texts(
            &ctx.db,
            "SELECT d.key AS v FROM custom_field_values v JOIN custom_field_defs d \
             ON d.pid = v.field_pid",
        )
        .await;
        assert_eq!(left, ["account_manager_note"]);
        crypto::install(None);
    })
    .await;
}
