//! `GET /api/contacts/{pid}/duplicates` (CRM-R23): other contacts that are
//! probably the same person, best first, with the reasons.

use contact_relationship_management::app::App;
use contact_relationship_management::clients;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::a_person;
use super::openapi_schemas::schema_check::{check, to_json_schema};

async fn account(request: &loco_rs::TestServer, name: &str) -> String {
    let created: Value = request
        .post("/api/accounts")
        .json(&json!({
            "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
            "display_name": name,
        }))
        .await
        .json();
    created["pid"].as_str().unwrap().to_string()
}

async fn contact(
    request: &loco_rs::TestServer,
    person_ref: &str,
    name: &str,
    title: Option<&str>,
    account_pid: Option<&str>,
) -> String {
    let created: Value = request
        .post("/api/contacts")
        .json(&json!({
            "person_ref": person_ref,
            "display_name": name,
            "job_title": title,
            "account_pid": account_pid,
        }))
        .await
        .json();
    created["pid"].as_str().unwrap().to_string()
}

fn pids(body: &Value) -> Vec<&str> {
    body["duplicates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["contact"]["pid"].as_str().unwrap())
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn finds_likely_duplicates_best_first_with_reasons() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        to_json_schema(&mut doc);

        let acme = account(&request, "Acme Widgets Inc.").await;
        let globex = account(&request, "Globex").await;
        let person = a_person();
        let target = contact(
            &request,
            &person,
            "Robert Smithson",
            Some("VP Sales"),
            Some(&acme),
        )
        .await;
        // Another wrapper over the same person record: certain.
        let twin = contact(&request, &person, "R. Smithson", None, None).await;
        // The same person written differently, at the same employer.
        let nickname = contact(
            &request,
            &a_person(),
            "Bob Smithson",
            Some("Vice President, Sales"),
            Some(&acme),
        )
        .await;
        // The same name and nothing else known: worth a look, not a match.
        let namesake = contact(&request, &a_person(), "Robert Smithson", None, None).await;
        // The same name at another employer: evidence against, so not offered.
        let elsewhere = contact(
            &request,
            &a_person(),
            "Robert Smithson",
            None,
            Some(&globex),
        )
        .await;
        // Nothing like it.
        let stranger = contact(
            &request,
            &a_person(),
            "Ben Okafor",
            Some("Engineer"),
            Some(&globex),
        )
        .await;

        let response = request
            .get(&format!("/api/contacts/{target}/duplicates"))
            .await;
        response.assert_status_ok();
        let body: Value = response.json();
        check(&doc, "ContactDuplicates", &body, "duplicates");

        assert_eq!(body["contact_pid"], target);
        assert_eq!(body["truncated"], false);
        // Certain first, then the strong match, then the one to look at.
        let found = pids(&body);
        assert_eq!(
            found,
            vec![twin.as_str(), nickname.as_str(), namesake.as_str()],
            "{body}"
        );
        assert!(!found.contains(&elsewhere.as_str()) && !found.contains(&stranger.as_str()));
        assert!(!found.contains(&target.as_str()), "never its own duplicate");

        let certain = &body["duplicates"][0];
        assert_eq!(certain["deterministic"], true);
        assert_eq!(certain["probability"], 1.0);
        let strong = &body["duplicates"][1];
        assert_eq!(strong["verdict"], "match");
        assert!(strong["probability"].as_f64().unwrap() > 0.9);
        assert_eq!(strong["email_checked"], false);
        assert_eq!(body["duplicates"][2]["verdict"], "review");
        // The reasons name fields, never values.
        let fields: Vec<&str> = strong["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["field"].as_str().unwrap())
            .collect();
        assert!(fields.contains(&"organization") && fields.contains(&"given_name"));

        // `limit` caps the answer.
        let limited: Value = request
            .get(&format!("/api/contacts/{target}/duplicates?limit=1"))
            .await
            .json();
        assert_eq!(limited["duplicates"].as_array().unwrap().len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_shared_email_address_rescues_a_changed_surname() {
    request::<App, _, _>(|request, _ctx| async move {
        let (a, b, c) = (a_person(), a_person(), a_person());
        let target = contact(&request, &a, "Ana Ruiz", None, None).await;
        let married = contact(&request, &b, "Ana Berg", None, None).await;
        let namesake = contact(&request, &c, "Ana Ruiz", None, None).await;

        // Without addresses the new surname rules her out.
        let before: Value = request
            .get(&format!("/api/contacts/{target}/duplicates"))
            .await
            .json();
        assert!(!pids(&before).contains(&married.as_str()), "{before}");

        clients::prime_email(&a, "ana@example.test");
        clients::prime_email(&b, "ana@example.test");
        clients::prime_email(&c, "someone.else@example.test");
        let body: Value = request
            .get(&format!("/api/contacts/{target}/duplicates"))
            .await
            .json();
        let found = body["duplicates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["contact"]["pid"] == married)
            .cloned()
            .expect("the shared mailbox brings her back");
        assert_eq!(found["email_checked"], true);
        // Same first name, a new surname and one mailbox: worth a look.
        assert_eq!(found["verdict"], "review", "{found}");
        let email = found["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["field"] == "email")
            .unwrap();
        assert_eq!(email["outcome"], "agree");
        // The same name with a different mailbox is checked and counts against.
        if let Some(other) = body["duplicates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["contact"]["pid"] == namesake)
        {
            assert_eq!(other["email_checked"], true);
            assert_ne!(other["verdict"], "match", "{other}");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn unknown_contact_is_not_found_and_reads_are_audited() {
    request::<App, _, _>(|request, _ctx| async move {
        request
            .get(&format!(
                "/api/contacts/{}/duplicates",
                uuid::Uuid::new_v4()
            ))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);

        let target = contact(&request, &a_person(), "Audit Subject", None, None).await;
        request
            .get(&format!("/api/contacts/{target}/duplicates"))
            .await
            .assert_status_ok();
        let trail: Value = request.get(&format!("/api/audits/{target}")).await.json();
        let row = trail
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["action"] == "contact_duplicates_read")
            .unwrap_or_else(|| panic!("no duplicates read in the trail: {trail}"));
        // The entry counts what was compared; it holds no contact data.
        assert!(row.to_string().contains("compared"));
        let verdict: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(verdict["ok"], true, "{verdict}");
    })
    .await;
}
