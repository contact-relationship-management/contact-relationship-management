//! Links to records in other systems (CRM-R27): added to four kinds of record,
//! checked, moved with a merge and an unmerge, and deleted with an erasure.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;

use super::a_person;
use super::openapi_schemas::schema_check::{check, to_json_schema};
use super::seed_contact;

async fn contact(request: &loco_rs::TestServer, name: &str) -> String {
    let created: Value = request
        .post("/api/contacts")
        .json(&json!({ "person_ref": a_person(), "display_name": name }))
        .await
        .json();
    created["pid"].as_str().unwrap().to_string()
}

async fn add(
    request: &loco_rs::TestServer,
    plural: &str,
    pid: &str,
    system: &str,
    url: &str,
) -> (u16, Value) {
    let response = request
        .post(&format!("/api/{plural}/{pid}/external-links"))
        .json(&json!({ "system": system, "url": url, "label": "In the other CRM" }))
        .await;
    (response.status_code().as_u16(), response.json())
}

async fn links(request: &loco_rs::TestServer, plural: &str, pid: &str) -> Vec<Value> {
    let body: Value = request
        .get(&format!("/api/{plural}/{pid}/external-links"))
        .await
        .json();
    body["links"].as_array().unwrap().clone()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn links_can_be_added_listed_and_removed_on_every_kind_of_record() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        to_json_schema(&mut doc);
        let contact_pid = contact(&request, "Ana").await;
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                            "display_name": "Meridian" }),
            )
            .await
            .json();
        let account_pid = account["pid"].as_str().unwrap().to_string();
        let (pipeline, stages) = super::seed_pipeline!(&request).await;
        let deal: Value = request
            .post("/api/deals")
            .json(&json!({ "name": "Renewal", "pipeline_pid": pipeline,
                            "amount_minor": 1000_i64, "currency": "GBP" }))
            .await
            .json();
        let deal_pid = deal["pid"].as_str().unwrap().to_string();
        let lead: Value = request
            .post("/api/leads")
            .json(&json!({ "display_name": "Lee", "source": "web" }))
            .await
            .json();
        let lead_pid = lead["pid"].as_str().unwrap().to_string();

        for (plural, pid, system, url) in [
            (
                "contacts",
                &contact_pid,
                "hubspot",
                "https://app.hubspot.com/contacts/1/record/0-1/2",
            ),
            (
                "accounts",
                &account_pid,
                "salesforce",
                "https://acme.lightning.force.com/lightning/r/Account/001/view",
            ),
            (
                "deals",
                &deal_pid,
                "attio",
                "https://app.attio.com/acme/deals/9",
            ),
            (
                "leads",
                &lead_pid,
                "other",
                "https://crm.example.org/leads/3",
            ),
        ] {
            let (status, link) = add(&request, plural, pid, system, url).await;
            assert_eq!(status, 200, "{plural}: {link}");
            check(&doc, "ExternalLink", &link, "link");
            assert_eq!(link["system"], system);
            let listed: Value = request
                .get(&format!("/api/{plural}/{pid}/external-links"))
                .await
                .json();
            check(&doc, "ExternalLinks", &listed, "links");
            assert_eq!(listed["links"].as_array().unwrap().len(), 1);
            // The same address twice is a conflict.
            assert_eq!(add(&request, plural, pid, system, url).await.0, 409);
        }

        // Remove one.
        let one = links(&request, "contacts", &contact_pid).await;
        let link_pid = one[0]["pid"].as_str().unwrap();
        request
            .delete(&format!("/api/external-links/{link_pid}"))
            .await
            .assert_status_ok();
        assert_eq!(links(&request, "contacts", &contact_pid).await.len(), 0);
        request
            .delete(&format!("/api/external-links/{link_pid}"))
            .await
            .assert_status_not_found();
        let verify: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(verify["ok"], true, "{verify}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn unsafe_and_misplaced_addresses_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = contact(&request, "Ana").await;
        for (system, url) in [
            ("other", "javascript:alert(1)"),
            ("other", "http://crm.example.org/x"),
            ("other", "https://user:pw@crm.example.org/x"),
            ("hubspot", "https://hubspot.com.evil.example/x"),
            ("attio", "https://app.hubspot.com/x"),
            ("nonsense", "https://crm.example.org/x"),
            ("other", "   "),
        ] {
            assert_eq!(
                add(&request, "contacts", &pid, system, url).await.0,
                422,
                "{system} {url}"
            );
        }
        assert_eq!(links(&request, "contacts", &pid).await.len(), 0);
        // A record that does not exist.
        let nobody = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            add(
                &request,
                "contacts",
                &nobody,
                "other",
                "https://crm.example.org/x"
            )
            .await
            .0,
            404
        );
        assert_eq!(
            add(
                &request,
                "deals",
                &nobody,
                "other",
                "https://crm.example.org/x"
            )
            .await
            .0,
            404
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn links_follow_a_merge_and_come_back_with_an_unmerge() {
    request::<App, _, _>(|request, _ctx| async move {
        let survivor = contact(&request, "Ana Ruiz").await;
        let merged = contact(&request, "A. Ruiz").await;
        assert_eq!(
            add(
                &request,
                "contacts",
                &merged,
                "hubspot",
                "https://app.hubspot.com/contacts/1/record/0-1/2"
            )
            .await
            .0,
            200
        );
        assert_eq!(
            add(
                &request,
                "contacts",
                &survivor,
                "attio",
                "https://app.attio.com/acme/people/1"
            )
            .await
            .0,
            200
        );
        let merge = request
            .post(&format!("/api/contacts/{merged}/merge"))
            .json(&json!({ "into_pid": survivor, "reason": "same person" }))
            .await;
        let done: Value = merge.json();
        assert_eq!(done["moved"]["external_links"], 1, "{done}");
        assert_eq!(links(&request, "contacts", &survivor).await.len(), 2);
        let undo: Value = request
            .post(&format!("/api/contacts/{merged}/unmerge"))
            .json(&json!({ "reason": "wrong person" }))
            .await
            .json();
        assert_eq!(undo["restored"]["external_links"], 1, "{undo}");
        let back = links(&request, "contacts", &merged).await;
        assert_eq!(back.len(), 1);
        assert_eq!(back[0]["system"], "hubspot");
        assert_eq!(links(&request, "contacts", &survivor).await.len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn subject_access_lists_links_and_erasure_deletes_them() {
    request::<App, _, _>(|request, ctx| async move {
        let pid = seed_contact!(&request, "Erin").await;
        assert_eq!(
            add(
                &request,
                "contacts",
                &pid,
                "salesforce",
                "https://acme.my.salesforce.com/003"
            )
            .await
            .0,
            200
        );
        let export: Value = request
            .get(&format!("/api/contacts/{pid}/subject-access"))
            .await
            .json();
        assert_eq!(
            export["external_links"].as_array().unwrap().len(),
            1,
            "{export}"
        );
        request
            .post(&format!("/api/contacts/{pid}/erase"))
            .await
            .assert_status_ok();
        let left = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT COUNT(*) AS n FROM external_links WHERE subject_pid = '{pid}'"),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            left.try_get::<i64>("", "n").unwrap(),
            0,
            "erasure deletes the links"
        );
    })
    .await;
}
