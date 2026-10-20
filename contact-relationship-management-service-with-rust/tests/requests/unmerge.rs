//! `POST /api/contacts/{pid}/unmerge` (CRM-R26): a merge is undone from its
//! ledger, consent never widens, and what cannot be undone is refused.

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

async fn post(request: &loco_rs::TestServer, path: &str, body: Value) -> (u16, Value) {
    let response = request.post(path).json(&body).await;
    (response.status_code().as_u16(), response.json())
}

async fn merge(request: &loco_rs::TestServer, from: &str, into: &str) -> u16 {
    post(
        request,
        &format!("/api/contacts/{from}/merge"),
        json!({ "into_pid": into, "reason": "same person" }),
    )
    .await
    .0
}

async fn unmerge(request: &loco_rs::TestServer, pid: &str, dry_run: bool) -> (u16, Value) {
    post(
        request,
        &format!("/api/contacts/{pid}/unmerge"),
        json!({ "reason": "wrong person", "dry_run": dry_run }),
    )
    .await
}

async fn activities(request: &loco_rs::TestServer, pid: &str) -> usize {
    let export: Value = request
        .get(&format!("/api/contacts/{pid}/subject-access"))
        .await
        .json();
    export["activities"].as_array().unwrap().len()
}

async fn note(request: &loco_rs::TestServer, pid: &str, summary: &str) {
    request
        .post("/api/activities")
        .json(&json!({ "subject_kind": "contact", "subject_pid": pid,
                        "kind": "note", "summary": summary }))
        .await
        .assert_status_ok();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unmerge_gives_back_what_the_merge_took() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        to_json_schema(&mut doc);
        let survivor = contact(&request, "Ana Ruiz", None).await;
        let merged = contact(&request, "A. Ruiz", Some("VP Sales")).await;
        note(&request, &merged, "on the duplicate").await;
        // Both are in the same group and the same sequence, so the merge drops
        // the duplicate's seat and enrolment.
        let group: Value = request
            .post("/api/groups")
            .json(&json!({ "name": "Standards", "purpose": "x" }))
            .await
            .json();
        let g = group["pid"].as_str().unwrap().to_string();
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Welcome", "steps": [
                { "delay_hours": 0, "template_ref": "w1", "body": "Hello" }] }))
            .await
            .json();
        let seq = sequence["pid"].as_str().unwrap().to_string();
        for pid in [&survivor, &merged] {
            request
                .post(&format!("/api/groups/{g}/members"))
                .json(&json!({ "contact_pid": pid }))
                .await
                .assert_status_ok();
            request
                .post(&format!("/api/contacts/{pid}/consent"))
                .json(&json!({ "action": "granted", "source": "test" }))
                .await
                .assert_status_ok();
            request
                .post(&format!("/api/nurture-sequences/{seq}/enrollments"))
                .json(&json!({ "contact_pid": pid }))
                .await
                .assert_status_ok();
        }
        assert_eq!(merge(&request, &merged, &survivor).await, 200);
        note(&request, &survivor, "after the merge").await;
        assert_eq!(activities(&request, &survivor).await, 2);
        request
            .get(&format!("/api/contacts/{merged}"))
            .await
            .assert_status_not_found();

        // The survivor lists what can be undone.
        let listed: Value = request
            .get(&format!("/api/contacts/{survivor}/merges"))
            .await
            .json();
        check(&doc, "ContactMerges", &listed, "merges");
        assert_eq!(listed["merges"][0]["merged_pid"], merged.as_str());
        assert_eq!(listed["merges"][0]["merged_name"], "A. Ruiz");
        assert_eq!(listed["merges"][0]["undoable"], true);

        // A preview reports and changes nothing.
        let (status, preview) = unmerge(&request, &merged, true).await;
        assert_eq!(status, 200, "{preview}");
        assert_eq!(preview["dry_run"], true);
        assert_eq!(preview["restored"]["activities"], 1);
        assert_eq!(preview["dropped_restored"]["working_group_members"], 1);
        assert_eq!(preview["dropped_restored"]["nurture_enrollments"], 1);
        assert_eq!(preview["fields_cleared"], json!(["job_title"]));
        check(&doc, "ContactUnmerge", &preview, "unmerge preview");
        request
            .get(&format!("/api/contacts/{merged}"))
            .await
            .assert_status_not_found();
        assert_eq!(activities(&request, &survivor).await, 2);

        // The real thing.
        let (status, done) = unmerge(&request, &merged, false).await;
        assert_eq!(status, 200, "{done}");
        assert_eq!(done["restored"], preview["restored"]);
        assert_eq!(done["left_behind"]["activities"], 0);
        check(&doc, "ContactUnmerge", &done, "unmerge");
        let back: Value = request.get(&format!("/api/contacts/{merged}")).await.json();
        assert_eq!(back["contact"]["display_name"], "A. Ruiz");
        assert_eq!(back["contact"]["job_title"], "VP Sales");
        let kept: Value = request
            .get(&format!("/api/contacts/{survivor}"))
            .await
            .json();
        assert!(
            kept["contact"]["job_title"].is_null(),
            "the filled field is cleared"
        );
        assert_eq!(activities(&request, &merged).await, 1, "its note came back");
        assert_eq!(
            activities(&request, &survivor).await,
            1,
            "the note made after the merge stays with the survivor"
        );
        let seats: Value = request.get(&format!("/api/groups/{g}")).await.json();
        let text = seats.to_string();
        assert!(text.contains(&merged) && text.contains(&survivor), "{text}");

        let after: Value = request
            .get(&format!("/api/contacts/{survivor}/merges"))
            .await
            .json();
        assert_eq!(after["merges"].as_array().unwrap().len(), 0, "{after}");

        // Done once; refused the second time.
        assert_eq!(unmerge(&request, &merged, false).await.0, 409);
        // The audit chain still verifies, and the contact can be merged again.
        let verify: Value = request.get("/api/audits/verify").await.json();
        assert_eq!(verify["ok"], true, "{verify}");
        assert_eq!(merge(&request, &merged, &survivor).await, 200);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unmerge_never_widens_consent() {
    request::<App, _, _>(|request, _ctx| async move {
        let survivor = contact(&request, "Ana", None).await;
        let merged = contact(&request, "A.", None).await;
        for pid in [&survivor, &merged] {
            request
                .post(&format!("/api/contacts/{pid}/consent"))
                .json(&json!({ "action": "granted", "source": "test" }))
                .await
                .assert_status_ok();
        }
        assert_eq!(merge(&request, &merged, &survivor).await, 200);
        // The person withdraws after the merge, on the surviving record.
        request
            .post(&format!("/api/contacts/{survivor}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "test" }))
            .await
            .assert_status_ok();
        let (status, done) = unmerge(&request, &merged, false).await;
        assert_eq!(status, 200, "{done}");
        assert_eq!(done["consent"]["merged_before"], "granted");
        assert_eq!(done["consent"]["merged_result"], "withdrawn");
        let back: Value = request.get(&format!("/api/contacts/{merged}")).await.json();
        assert_eq!(
            back["contact"]["marketing_consent"], "withdrawn",
            "the returning contact does not come back more permissive than the person is"
        );
        let history: Value = request
            .get(&format!("/api/contacts/{merged}/consent"))
            .await
            .json();
        assert!(
            history.to_string().contains(&format!("unmerge:{survivor}")),
            "{history}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn what_cannot_be_undone_is_refused() {
    request::<App, _, _>(|request, ctx| async move {
        let a = contact(&request, "Ana", None).await;
        let b = contact(&request, "Ben", None).await;
        // Never merged; and a contact that does not exist.
        assert_eq!(unmerge(&request, &a, false).await.0, 409);
        let nobody = uuid::Uuid::new_v4().to_string();
        assert_eq!(unmerge(&request, &nobody, false).await.0, 404);
        // No reason.
        let missing = request
            .post(&format!("/api/contacts/{a}/unmerge"))
            .json(&json!({ "reason": " " }))
            .await;
        assert_eq!(missing.status_code().as_u16(), 422);

        // A merge made before the undo record existed.
        assert_eq!(merge(&request, &b, &a).await, 200);
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contact_merges SET before = NULL WHERE merged_pid = '{b}'"
            ))
            .await
            .unwrap();
        assert_eq!(unmerge(&request, &b, false).await.0, 422);
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contact_merges SET before = '{{\"version\":1,\"filled\":[]}}' \
                 WHERE merged_pid = '{b}'"
            ))
            .await
            .unwrap();

        // The survivor was itself merged away: unmerge that first.
        let c = contact(&request, "Cy", None).await;
        assert_eq!(merge(&request, &a, &c).await, 200);
        assert_eq!(unmerge(&request, &b, false).await.0, 409);
        assert_eq!(unmerge(&request, &a, false).await.0, 200);
        assert_eq!(unmerge(&request, &b, false).await.0, 200);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn erasing_the_survivor_scrubs_the_merged_duplicate_and_the_ledger() {
    request::<App, _, _>(|request, ctx| async move {
        let survivor = seed_contact!(&request, "Sam Survivor").await;
        let merged = contact(&request, "S. Survivor", Some("Chair")).await;
        assert_eq!(merge(&request, &merged, &survivor).await, 200);
        request
            .post(&format!("/api/contacts/{survivor}/erase"))
            .await
            .assert_status_ok();
        let row = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT c.display_name, c.job_title, c.person_ref, m.merged_person_ref \
                     FROM contacts c JOIN contact_merges m ON m.merged_pid = c.pid \
                     WHERE c.pid = '{merged}'"
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        let tombstone = "person:00000000-0000-0000-0000-000000000000";
        assert_ne!(
            row.try_get::<String>("", "display_name").unwrap(),
            "S. Survivor"
        );
        assert!(
            row.try_get::<Option<String>>("", "job_title")
                .unwrap()
                .is_none()
        );
        assert_eq!(row.try_get::<String>("", "person_ref").unwrap(), tombstone);
        assert_eq!(
            row.try_get::<String>("", "merged_person_ref").unwrap(),
            tombstone
        );
        // And it can no longer be unmerged.
        assert_eq!(unmerge(&request, &merged, false).await.0, 422);
    })
    .await;
}
