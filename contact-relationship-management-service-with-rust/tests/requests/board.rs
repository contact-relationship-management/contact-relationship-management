//! Board edits (CRM-T55): edit a card in place, reorder it within its column,
//! and move a follow-up to another day.

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{seed_contact, seed_pipeline};

/// The `pid`s of `items`, in order.
fn pids(items: &Value) -> Vec<String> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["pid"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A deal is edited in place; reordered within its stage with the order stored
// (the list comes back in that order); a closed deal is refused.
async fn deals_are_edited_and_reordered() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline_pid, stages) = seed_pipeline!(&request).await;
        let mut deal_pids = Vec::new();
        for name in ["Alpha", "Beta", "Gamma"] {
            let created: Value = request
                .post("/api/deals")
                .json(&json!({
                    "name": name, "pipeline_pid": pipeline_pid,
                    "amount_minor": 100_000, "currency": "GBP",
                }))
                .await
                .json();
            deal_pids.push(created["pid"].as_str().unwrap().to_string());
        }
        // Edit name and amount; a blank name and a negative amount are refused.
        let edited: Value = request
            .put(&format!("/api/deals/{}", deal_pids[0]))
            .json(&json!({ "name": "  Alpha Prime ", "amount_minor": 250_000 }))
            .await
            .json();
        assert_eq!(edited["name"], "Alpha Prime");
        assert_eq!(edited["amount_minor"], 250_000);
        for bad in [
            json!({ "name": "  " }),
            json!({ "amount_minor": -1 }),
            json!({}),
        ] {
            let refused = request
                .put(&format!("/api/deals/{}", deal_pids[0]))
                .json(&bad)
                .await;
            assert_eq!(refused.status_code(), 422, "{bad}");
        }
        // Move Alpha to the end of its stage; the list follows.
        request
            .put(&format!("/api/deals/{}/position", deal_pids[0]))
            .json(&json!({ "position": 2 }))
            .await
            .assert_status_ok();
        let listed: Value = request
            .get(&format!("/api/deals?pipeline={pipeline_pid}"))
            .await
            .json();
        let order = pids(&listed);
        assert_eq!(
            order,
            vec![
                deal_pids[1].clone(),
                deal_pids[2].clone(),
                deal_pids[0].clone()
            ]
        );
        // A closed deal can be neither edited nor reordered.
        request
            .post(&format!("/api/deals/{}/stage", deal_pids[1]))
            .json(&json!({ "stage_pid": stages[2] }))
            .await
            .assert_status_ok();
        for call in [
            request
                .put(&format!("/api/deals/{}", deal_pids[1]))
                .json(&json!({ "name": "Nope" })),
            request
                .put(&format!("/api/deals/{}/position", deal_pids[1]))
                .json(&json!({ "position": 0 })),
        ] {
            assert_eq!(call.await.status_code(), 422);
        }
        // An unknown deal is a 404.
        let missing = request
            .put(&format!("/api/deals/{}", uuid::Uuid::new_v4()))
            .json(&json!({ "name": "x" }))
            .await;
        assert_eq!(missing.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Leads and tickets are edited in place and reordered within their status.
async fn leads_and_tickets_are_edited_and_reordered() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut leads = Vec::new();
        for name in ["Lead A", "Lead B", "Lead C"] {
            let created: Value = request
                .post("/api/leads")
                .json(&json!({ "display_name": name, "source": "web" }))
                .await
                .json();
            leads.push(created["pid"].as_str().unwrap().to_string());
        }
        let renamed: Value = request
            .put(&format!("/api/leads/{}", leads[0]))
            .json(&json!({ "display_name": "Lead Z" }))
            .await
            .json();
        assert_eq!(renamed["display_name"], "Lead Z");
        request
            .put(&format!("/api/leads/{}/position", leads[2]))
            .json(&json!({ "position": 0 }))
            .await
            .assert_status_ok();
        let listed: Value = request.get("/api/leads").await.json();
        assert_eq!(pids(&listed)[0], leads[2]);

        let mut tickets = Vec::new();
        for title in ["First", "Second", "Third"] {
            let created: Value = request
                .post("/api/tickets")
                .json(&json!({ "title": title, "priority": "normal" }))
                .await
                .json();
            tickets.push(created["pid"].as_str().unwrap().to_string());
        }
        let retitled: Value = request
            .put(&format!("/api/tickets/{}", tickets[0]))
            .json(&json!({ "title": "First, clarified" }))
            .await
            .json();
        assert_eq!(retitled["title"], "First, clarified");
        request
            .put(&format!("/api/tickets/{}/position", tickets[0]))
            .json(&json!({ "position": 2 }))
            .await
            .assert_status_ok();
        let listed: Value = request.get("/api/tickets").await.json();
        assert_eq!(
            pids(&listed),
            vec![tickets[1].clone(), tickets[2].clone(), tickets[0].clone()]
        );
        let empty = request
            .put(&format!("/api/tickets/{}", tickets[0]))
            .json(&json!({}))
            .await;
        assert_eq!(empty.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A follow-up moves to another day (audited); an activity without a due day,
// or a finished one, cannot be rescheduled; the follow-ups view follows.
async fn a_follow_up_is_rescheduled() {
    request::<App, _, _>(|request, _ctx| async move {
        let contact = seed_contact!(&request, "Follow-up Contact").await;
        let created: Value = request
            .post("/api/activities")
            .json(&json!({
                "subject_kind": "contact", "subject_pid": contact,
                "kind": "task", "summary": "Call back", "due_on": "2030-01-10",
            }))
            .await
            .json();
        let pid = created["pid"].as_str().unwrap().to_string();
        let moved: Value = request
            .put(&format!("/api/activities/{pid}"))
            .json(&json!({ "due_on": "2030-01-17" }))
            .await
            .json();
        assert_eq!(moved["due_on"], "2030-01-17");
        let reworded: Value = request
            .put(&format!("/api/activities/{pid}"))
            .json(&json!({ "summary": "Call back about the renewal" }))
            .await
            .json();
        assert_eq!(reworded["summary"], "Call back about the renewal");
        // The trail records the move with both days, and no summary text.
        let audits: Value = request.get(&format!("/api/audits/{pid}")).await.json();
        let trail = audits.to_string();
        assert!(trail.contains("rescheduled"), "{trail}");
        assert!(
            trail.contains("2030-01-10") && trail.contains("2030-01-17"),
            "{trail}"
        );
        assert!(!trail.contains("renewal"), "no summary text in the trail");

        let no_due: Value = request
            .post("/api/activities")
            .json(&json!({
                "subject_kind": "contact", "subject_pid": contact,
                "kind": "note", "summary": "Met at the conference",
            }))
            .await
            .json();
        let refused = request
            .put(&format!(
                "/api/activities/{}",
                no_due["pid"].as_str().unwrap()
            ))
            .json(&json!({ "due_on": "2030-02-01" }))
            .await;
        assert_eq!(refused.status_code(), 422);

        request
            .put(&format!("/api/activities/{pid}/done"))
            .await
            .assert_status_ok();
        let finished = request
            .put(&format!("/api/activities/{pid}"))
            .json(&json!({ "due_on": "2030-03-01" }))
            .await;
        assert_eq!(finished.status_code(), 422);
    })
    .await;
}
