//! Tags (CRM-R47): the labels, a record's set, the list filter, segments, subject access and
//! erasure.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

macro_rules! set_tags {
    ($request:expr, $kind:expr, $pid:expr, $names:expr) => {{
        let r = $request
            .put(&format!("/api/{}/{}/tags", $kind, $pid))
            .json(&json!({ "tags": $names }))
            .await;
        r.assert_status_ok();
        r.json::<Vec<String>>()
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn tags_are_made_renamed_and_deleted_and_names_are_tidy() {
    request::<App, _, _>(|request, ctx| async move {
        let made = request
            .post("/api/tags")
            .json(&json!({ "name": "  Big   Spender " }))
            .await;
        made.assert_status_ok();
        let pid = made.json::<Value>()["pid"].as_str().unwrap().to_string();
        let listed: Vec<Value> = request.get("/api/tags").await.json();
        assert_eq!(listed[0]["name"], "Big Spender");
        assert_eq!(listed[0]["uses"], 0);
        for body in [
            json!({ "name": "big spender" }),
            json!({ "name": "  " }),
            json!({ "name": "x".repeat(41) }),
        ] {
            request
                .post("/api/tags")
                .json(&body)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        let contact = seed_contact!(&request, "Tag Holder").await;
        let _ = set_tags!(request, "contacts", contact, ["Big Spender"]);
        request
            .put(&format!("/api/tags/{pid}"))
            .json(&json!({ "name": "Whale" }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .get(&format!("/api/contacts/{contact}/tags"))
                .await
                .json::<Vec<String>>(),
            ["Whale"],
            "a record keeps a renamed tag"
        );
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM taggings").await,
            1
        );
        request
            .delete(&format!("/api/tags/{pid}"))
            .await
            .assert_status_ok();
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM taggings").await,
            0
        );
        assert!(
            request
                .get(&format!("/api/contacts/{contact}/tags"))
                .await
                .json::<Vec<String>>()
                .is_empty()
        );
        request
            .delete(&format!("/api/tags/{pid}"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_records_tags_are_set_as_a_whole_and_lists_can_be_narrowed() {
    request::<App, _, _>(|request, _ctx| async move {
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Tagged Org" }),
            )
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let lead: Value = request
            .post("/api/leads")
            .json(&json!({ "display_name": "Tagged Lead", "source": "web" }))
            .await
            .json();
        let lead = lead["pid"].as_str().unwrap().to_string();
        let one = seed_contact!(&request, "Tagged One").await;
        let two = seed_contact!(&request, "Tagged Two").await;

        // A set is replaced whole; repeats in any case are one; new names become tags.
        assert_eq!(
            set_tags!(request, "contacts", one, ["VIP", "press", "vip"]),
            ["press", "VIP"]
        );
        assert_eq!(
            set_tags!(request, "contacts", one, ["Press"]),
            ["press"],
            "the tag keeps its first spelling"
        );
        let _ = set_tags!(request, "contacts", two, ["vip"]);
        let _ = set_tags!(request, "accounts", account, ["VIP"]);
        let _ = set_tags!(request, "leads", lead, ["vip"]);
        assert_eq!(request.get("/api/tags").await.json::<Vec<Value>>().len(), 2);

        // Narrowing: any case, per kind, an unknown tag is an empty list.
        let names = |rows: Vec<Value>, key: &str| -> Vec<String> {
            rows.iter()
                .map(|r| r[key].as_str().unwrap().to_string())
                .collect()
        };
        let contacts: Vec<Value> = request.get("/api/contacts?tag=VIP").await.json();
        assert_eq!(names(contacts, "display_name"), ["Tagged Two"]);
        let accounts: Vec<Value> = request.get("/api/accounts?tag=vip").await.json();
        assert_eq!(names(accounts, "display_name"), ["Tagged Org"]);
        let leads: Vec<Value> = request.get("/api/leads?tag=vip").await.json();
        assert_eq!(names(leads, "display_name"), ["Tagged Lead"]);
        assert!(
            request
                .get("/api/contacts?tag=nobody")
                .await
                .json::<Vec<Value>>()
                .is_empty()
        );
        assert!(
            request
                .get("/api/deals?tag=vip")
                .await
                .json::<Vec<Value>>()
                .is_empty()
        );
        assert!(
            request
                .get("/api/contacts")
                .await
                .json::<Vec<Value>>()
                .len()
                >= 2
        );

        // Refusals: too many, an empty name, an unknown record, a bad filter.
        let many: Vec<String> = (0..21).map(|i| format!("t{i}")).collect();
        for body in [json!({ "tags": many }), json!({ "tags": [""] })] {
            request
                .put(&format!("/api/contacts/{one}/tags"))
                .json(&body)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        request
            .put(&format!("/api/contacts/{}/tags", uuid::Uuid::new_v4()))
            .json(&json!({ "tags": ["x"] }))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        request
            .get("/api/contacts?tag=%20")
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        // Clearing the set.
        assert!(set_tags!(request, "contacts", one, Vec::<String>::new()).is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_segment_can_select_by_tag_and_consent_still_gates_it() {
    request::<App, _, _>(|request, _ctx| async move {
        let tagged = seed_contact!(&request, "Segment Tagged").await;
        let _other = seed_contact!(&request, "Segment Untagged").await;
        // Tagged, but never consented: the consent gate keeps this one out.
        let no_consent: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": super::a_person(), "display_name": "Segment No Consent" }))
            .await
            .json();
        let _ = set_tags!(request, "contacts", tagged, ["Newsletter"]);
        let _ = set_tags!(
            request,
            "contacts",
            no_consent["pid"].as_str().unwrap(),
            ["newsletter"]
        );
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Newsletter people", "filter": { "tags": ["  NEWSLETTER "] } }))
            .await
            .json();
        let preview: Value = request
            .get(&format!(
                "/api/segments/{}/preview",
                segment["pid"].as_str().unwrap()
            ))
            .await
            .json();
        assert_eq!(preview["count"], 1);
        assert_eq!(preview["sample"][0], "Segment Tagged");
        let all: Vec<Value> = request.get("/api/segments").await.json();
        assert_eq!(
            all[0]["filter"]["tags"],
            json!(["NEWSLETTER"]),
            "stored tidy"
        );
        request
            .post("/api/segments")
            .json(&json!({ "name": "Bad", "filter": { "tags": [""] } }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn subject_access_lists_the_tags_and_erasure_removes_them() {
    request::<App, _, _>(|request, ctx| async move {
        let contact = seed_contact!(&request, "Erase My Tags").await;
        let _ = set_tags!(request, "contacts", contact, ["private thing", "other"]);
        let export: Value = request
            .get(&format!("/api/contacts/{contact}/subject-access"))
            .await
            .json();
        assert_eq!(export["tags"], json!(["other", "private thing"]));
        request
            .post(&format!("/api/contacts/{contact}/erase"))
            .await
            .assert_status_ok();
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM taggings").await,
            0
        );
        // The tags themselves stay: they are labels, not the person.
        assert_eq!(count(&ctx.db, "SELECT count(*) AS n FROM tags").await, 2);
    })
    .await;
}
