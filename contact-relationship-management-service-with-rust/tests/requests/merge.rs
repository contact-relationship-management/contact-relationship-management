//! `POST /api/contacts/{pid}/merge` (CRM-R24): rows move, consent never
//! widens, the refusals hold, and a failure part-way changes nothing.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;

use super::a_person;
use super::openapi_schemas::schema_check::{check, to_json_schema};
use super::seed_contact;

async fn contact(request: &loco_rs::TestServer, name: &str, title: Option<&str>) -> String {
    let created: Value = request
        .post("/api/contacts")
        .json(&json!({ "person_ref": a_person(), "display_name": name, "job_title": title }))
        .await
        .json();
    created["pid"].as_str().unwrap().to_string()
}

async fn consent(request: &loco_rs::TestServer, pid: &str, action: Option<&str>) {
    if let Some(action) = action {
        request
            .post(&format!("/api/contacts/{pid}/consent"))
            .json(&json!({ "action": action, "source": "test" }))
            .await
            .assert_status_ok();
    }
}

async fn merge(
    request: &loco_rs::TestServer,
    from: &str,
    into: &str,
    dry_run: bool,
) -> (u16, Value) {
    let response = request
        .post(&format!("/api/contacts/{from}/merge"))
        .json(&json!({ "into_pid": into, "reason": "same person", "dry_run": dry_run }))
        .await;
    (response.status_code().as_u16(), response.json())
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_merge_moves_everything_and_is_recorded() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        to_json_schema(&mut doc);
        let survivor = contact(&request, "Ana Ruiz", None).await;
        let merged = contact(&request, "A. Ruiz", Some("VP Sales")).await;
        // Both are in the same nurture sequence: the survivor's enrolment stays.
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Welcome", "steps": [
                { "delay_hours": 0, "template_ref": "welcome-1", "body": "Hello" }] }))
            .await
            .json();
        let seq = sequence["pid"].as_str().unwrap().to_string();
        for pid in [&survivor, &merged] {
            consent(&request, pid, Some("granted")).await;
            request
                .post(&format!("/api/nurture-sequences/{seq}/enrollments"))
                .json(&json!({ "contact_pid": pid }))
                .await
                .assert_status_ok();
        }
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "contact", "subject_pid": merged,
                            "kind": "note", "summary": "Met at the conference" }))
            .await
            .assert_status_ok();
        let (pipeline, _) = super::seed_pipeline!(&request).await;
        request
            .post("/api/deals")
            .json(&json!({ "name": "Renewal", "pipeline_pid": pipeline,
                            "amount_minor": 1000_i64, "currency": "GBP",
                            "primary_contact_pid": merged }))
            .await
            .assert_status_ok();
        let group: Value = request
            .post("/api/groups")
            .json(&json!({ "name": "Standards", "purpose": "x" }))
            .await
            .json();
        let g = group["pid"].as_str().unwrap().to_string();
        for pid in [&survivor, &merged] {
            request
                .post(&format!("/api/groups/{g}/members"))
                .json(&json!({ "contact_pid": pid }))
                .await
                .assert_status_ok();
        }

        // A dry run reports and changes nothing.
        let (status, preview) = merge(&request, &merged, &survivor, true).await;
        assert_eq!(status, 200, "{preview}");
        assert_eq!(preview["dry_run"], true);
        assert_eq!(preview["moved"]["activities"], 1);
        assert_eq!(preview["moved"]["deals"], 1);
        assert_eq!(preview["dropped"]["working_group_members"], 1);
        assert_eq!(preview["dropped"]["nurture_enrollments"], 1);
        assert_eq!(preview["moved"]["nurture_enrollments"], 0);
        check(&doc, "ContactMerge", &preview, "merge preview");
        assert_eq!(preview["filled"], json!(["job_title"]));
        request
            .get(&format!("/api/contacts/{merged}"))
            .await
            .assert_status_ok();

        // The real merge.
        let (status, done) = merge(&request, &merged, &survivor, false).await;
        assert_eq!(status, 200, "{done}");
        assert_eq!(done["moved"], preview["moved"]);
        assert_eq!(done["dropped"], preview["dropped"]);
        check(&doc, "ContactMerge", &done, "merge");
        request
            .get(&format!("/api/contacts/{merged}"))
            .await
            .assert_status_not_found();
        let kept: Value = request
            .get(&format!("/api/contacts/{survivor}"))
            .await
            .json();
        assert_eq!(
            kept["contact"]["job_title"], "VP Sales",
            "an empty field is filled"
        );
        assert_eq!(
            kept["contact"]["display_name"], "Ana Ruiz",
            "the survivor's name stays"
        );

        // The survivor's subject-access export covers the merged-in data.
        let export: Value = request
            .get(&format!("/api/contacts/{survivor}/subject-access"))
            .await
            .json();
        assert_eq!(export["activities"].as_array().unwrap().len(), 1);
        assert_eq!(
            export["deals_as_primary_contact"].as_array().unwrap().len(),
            1
        );

        // One audit entry names both contacts and the counts, not values.
        let audits: Value = request.get(&format!("/api/audits/{survivor}")).await.json();
        let entry = audits
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["action"] == "contacts_merged")
            .expect("the merge is audited");
        let text = entry.to_string();
        assert!(text.contains(&merged), "{text}");
        assert!(!text.contains("Met at the conference"), "{text}");
        assert!(!text.contains("VP Sales"), "{text}");

        // The audit chain still verifies.
        let verdict: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(verdict["ok"], true, "{verdict}");

        // Merging again is a conflict, not a second merge.
        let (status, _) = merge(&request, &merged, &survivor, false).await;
        assert_eq!(status, 409);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn consent_never_widens() {
    request::<App, _, _>(|request, _ctx| async move {
        // (survivor, merged, expected). `None` leaves a new contact at "never".
        let cases = [
            (Some("granted"), Some("withdrawn"), "withdrawn"),
            (Some("withdrawn"), Some("granted"), "withdrawn"),
            (None, Some("granted"), "never"),
            (Some("granted"), None, "never"),
            (Some("granted"), Some("granted"), "granted"),
            (None, None, "never"),
        ];
        for (survivor_consent, merged_consent, expected) in cases {
            let survivor = contact(&request, "Survivor", None).await;
            let merged = contact(&request, "Merged", None).await;
            consent(&request, &survivor, survivor_consent).await;
            consent(&request, &merged, merged_consent).await;
            let (status, done) = merge(&request, &merged, &survivor, false).await;
            assert_eq!(status, 200, "{done}");
            assert_eq!(
                done["consent"]["result"], expected,
                "{survivor_consent:?} + {merged_consent:?}"
            );
            let kept: Value = request
                .get(&format!("/api/contacts/{survivor}"))
                .await
                .json();
            assert_eq!(kept["contact"]["marketing_consent"], expected);
            // Both histories are kept, with their original sources.
            let history: Value = request
                .get(&format!("/api/contacts/{survivor}/consent"))
                .await
                .json();
            let expected_events = usize::from(survivor_consent.is_some())
                + usize::from(merged_consent.is_some())
                + usize::from(
                    survivor_consent.is_some_and(|s| s != expected)
                        || survivor_consent.is_none() && expected != "never",
                );
            assert_eq!(
                history.as_array().unwrap().len(),
                expected_events,
                "{history}"
            );
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn refusals() {
    request::<App, _, _>(|request, _ctx| async move {
        let a = contact(&request, "Ana", None).await;
        let b = contact(&request, "Ben", None).await;
        let erased = seed_contact!(&request, "Erin").await;
        request
            .post(&format!("/api/contacts/{erased}/erase"))
            .await
            .assert_status_ok();

        // The same contact twice.
        assert_eq!(merge(&request, &a, &a, false).await.0, 422);
        // An erased contact, in either direction.
        assert_eq!(merge(&request, &erased, &a, false).await.0, 422);
        assert_eq!(merge(&request, &a, &erased, false).await.0, 422);
        // A contact that does not exist.
        let nobody = uuid::Uuid::new_v4().to_string();
        assert_eq!(merge(&request, &nobody, &a, false).await.0, 404);
        assert_eq!(merge(&request, &a, &nobody, false).await.0, 404);
        // No reason.
        let missing = request
            .post(&format!("/api/contacts/{a}/merge"))
            .json(&json!({ "into_pid": b, "reason": "  " }))
            .await;
        assert_eq!(missing.status_code().as_u16(), 422);
        // None of that changed anything.
        request
            .get(&format!("/api/contacts/{a}"))
            .await
            .assert_status_ok();
        request
            .get(&format!("/api/contacts/{b}"))
            .await
            .assert_status_ok();
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_failure_part_way_leaves_both_contacts_unchanged() {
    request::<App, _, _>(|request, ctx| async move {
        let survivor = contact(&request, "Ana Ruiz", None).await;
        let merged = contact(&request, "A. Ruiz", Some("VP Sales")).await;
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "contact", "subject_pid": merged,
                            "kind": "note", "summary": "kept" }))
            .await
            .assert_status_ok();
        // A ledger row for the merged contact already exists, so the ledger
        // insert at the end of the merge fails, after every row has moved.
        ctx.db
            .execute_unprepared(&format!(
                "INSERT INTO contact_merges (pid, survivor_pid, merged_pid, merged_person_ref, moved) \
                 VALUES (gen_random_uuid(), gen_random_uuid(), '{merged}', 'person:x', '{{}}')"
            ))
            .await
            .unwrap();
        let (status, _) = merge(&request, &merged, &survivor, false).await;
        assert!(status >= 500, "{status}");

        request.get(&format!("/api/contacts/{merged}")).await.assert_status_ok();
        let kept: Value = request.get(&format!("/api/contacts/{survivor}")).await.json();
        assert!(kept["contact"]["job_title"].is_null(), "the survivor was not filled");
        let export: Value = request
            .get(&format!("/api/contacts/{merged}/subject-access"))
            .await
            .json();
        assert_eq!(export["activities"].as_array().unwrap().len(), 1, "the activity did not move");
    })
    .await;
}
