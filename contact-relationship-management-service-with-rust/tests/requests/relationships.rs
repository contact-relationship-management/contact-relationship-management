//! Contacts, accounts and activities: creation, lookup, refusals and the task
//! lifecycle, through the routes.

use axum::http::StatusCode;
use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_contact};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_contact_is_created_listed_and_fetched() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = seed_contact!(&request, "Rel Contact").await;
        let one: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(one["contact"]["display_name"], "Rel Contact");
        assert_eq!(one["truncated"]["activities"], false);
        let all: Vec<Value> = request.get("/api/contacts").await.json();
        assert!(all.iter().any(|c| c["pid"] == pid));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn bad_contacts_and_unknown_ids_are_refused_with_the_right_status() {
    request::<App, _, _>(|request, _ctx| async move {
        let blank: Value = {
            let r = request
                .post("/api/contacts")
                .json(&json!({ "person_ref": a_person(), "display_name": "" }))
                .await;
            r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
            r.json()
        };
        assert_eq!(blank["errors"][0]["pointer"], "/display_name", "{blank}");

        request
            .post("/api/contacts")
            .json(&json!({ "person_ref": "not-a-urn", "display_name": "X" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        request
            .get(&format!("/api/contacts/{}", uuid::Uuid::new_v4()))
            .await
            .assert_status(StatusCode::NOT_FOUND);
        let malformed = request.get("/api/contacts/not-a-uuid").await;
        assert!(
            malformed.status_code().is_client_error(),
            "{}",
            malformed.status_code()
        );
        assert_ne!(malformed.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_account_round_trips() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/accounts")
            .json(&json!({
                "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                "display_name": "Rel Org",
            }))
            .await
            .json();
        let pid = created["pid"].as_str().unwrap();
        let fetched: Value = request.get(&format!("/api/accounts/{pid}")).await.json();
        assert_eq!(fetched["account"]["display_name"], "Rel Org");
        let all: Vec<Value> = request.get("/api/accounts").await.json();
        assert!(all.iter().any(|a| a["pid"] == pid));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn only_a_task_can_be_marked_done() {
    request::<App, _, _>(|request, _ctx| async move {
        let contact = seed_contact!(&request, "Rel Tasker").await;
        let make = |kind: &str| {
            let body = json!({
                "subject_kind": "contact", "subject_pid": contact,
                "kind": kind, "summary": "do it", "due_on": "2030-01-01",
            });
            let request = &request;
            async move { request.post("/api/activities").json(&body).await }
        };
        let task: Value = make("task").await.json();
        let note: Value = make("note").await.json();

        let done = request
            .put(&format!(
                "/api/activities/{}/done",
                task["pid"].as_str().unwrap()
            ))
            .await;
        done.assert_status_ok();
        assert_eq!(done.json::<Value>()["done"], true);

        request
            .put(&format!(
                "/api/activities/{}/done",
                note["pid"].as_str().unwrap()
            ))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        request
            .put(&format!("/api/activities/{}/done", uuid::Uuid::new_v4()))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_contact_is_edited_by_the_members_sent_and_nothing_else() {
    request::<App, _, _>(|request, ctx| async move {
        use sea_orm::ConnectionTrait;
        let created: Value = request
            .post("/api/contacts")
            .json(
                &json!({ "person_ref": a_person(), "display_name": "Ada Lovelace",
                           "given_name": "Ada", "job_title": "Analyst" }),
            )
            .await
            .json();
        let pid = created["pid"].as_str().unwrap().to_string();
        let url = format!("/api/contacts/{pid}");

        // Only what is sent changes; null clears.
        let edited: Value = request
            .put(&url)
            .json(&json!({ "family_name": "Lovelace", "job_title": null }))
            .await
            .json();
        assert_eq!(edited["family_name"], "Lovelace");
        assert_eq!(edited["job_title"], Value::Null);
        assert_eq!(edited["given_name"], "Ada");
        assert_eq!(edited["display_name"], "Ada Lovelace");

        // Refused: nothing to change, a blank name, a bad channel, an unknown member, an unknown
        // account, an unknown contact.
        for body in [
            json!({}),
            json!({ "display_name": " " }),
            json!({ "preferred_channel": "pigeon" }),
            json!({ "person_ref": a_person() }),
            json!({ "account_pid": uuid::Uuid::new_v4() }),
        ] {
            let r = request.put(&url).json(&body).await;
            assert!(
                r.status_code().is_client_error(),
                "{body}: {}",
                r.status_code()
            );
        }
        request
            .put(&format!("/api/contacts/{}", uuid::Uuid::new_v4()))
            .json(&json!({ "given_name": "X" }))
            .await
            .assert_status(StatusCode::NOT_FOUND);

        // The audit row names the field, not the value.
        let row = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT snapshot::text AS d FROM audit_logs WHERE entity_pid = '{pid}' \
                     AND action = 'updated'"
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        let details = row.try_get::<String>("", "d").unwrap();
        assert!(details.contains("family_name"), "{details}");
        assert!(!details.contains("Lovelace"), "{details}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_account_is_edited_by_the_members_sent_and_nothing_else() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/accounts")
            .json(&json!({
                "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                "display_name": "Edit Org", "industry": "Retail",
            }))
            .await
            .json();
        let url = format!("/api/accounts/{}", created["pid"].as_str().unwrap());
        let edited: Value = request
            .put(&url)
            .json(&json!({ "tier": "customer", "industry": null }))
            .await
            .json();
        assert_eq!(edited["tier"], "customer");
        assert_eq!(edited["industry"], Value::Null);
        assert_eq!(edited["display_name"], "Edit Org");
        for body in [
            json!({}),
            json!({ "display_name": "" }),
            json!({ "tier": "nonsense" }),
            json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()) }),
        ] {
            let r = request.put(&url).json(&body).await;
            assert!(r.status_code().is_client_error(), "{body}");
        }
        request
            .put(&format!("/api/accounts/{}", uuid::Uuid::new_v4()))
            .json(&json!({ "tier": "customer" }))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    })
    .await;
}
