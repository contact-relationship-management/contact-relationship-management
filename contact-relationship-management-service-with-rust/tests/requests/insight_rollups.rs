//! The activity-based insight views (cadence, engagement, members, stakeholders,
//! pipeline hygiene) over a small estate with known touch dates. They pin what
//! each view reports, so that how it is computed can change without the answers
//! changing (CRM-T80).

use chrono::{Duration, Utc};
use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_worker, seed_pipeline};

fn ago(days: i64) -> String {
    (Utc::now() - Duration::days(days)).to_rfc3339()
}

fn find<'a>(rows: &'a Value, pid: &str) -> Option<&'a Value> {
    rows.as_array()?.iter().find(|r| r["pid"] == pid)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one estate, five views
async fn the_activity_views_report_the_touches_they_were_given() {
    request::<App, _, _>(|request, _ctx| async move {
        let account = |name: &'static str| {
            let request = &request;
            async move {
                let made: Value = request
                    .post("/api/accounts")
                    .json(&json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                                   "display_name": name }))
                    .await
                    .json();
                made["pid"].as_str().unwrap().to_string()
            }
        };
        let contact = |name: &'static str, account_pid: Option<String>| {
            let request = &request;
            async move {
                let made: Value = request
                    .post("/api/contacts")
                    .json(&json!({ "person_ref": a_person(), "display_name": name,
                                   "account_pid": account_pid }))
                    .await
                    .json();
                made["pid"].as_str().unwrap().to_string()
            }
        };
        let activity = |subject_kind: &'static str,
                        subject_pid: String,
                        kind: &'static str,
                        days: i64,
                        actor: Option<String>,
                        sentiment: Option<&'static str>,
                        due_on: Option<&'static str>| {
            let request = &request;
            async move {
                request
                    .post("/api/activities")
                    .json(&json!({ "subject_kind": subject_kind, "subject_pid": subject_pid,
                                   "kind": kind, "summary": "rollup fixture",
                                   "occurred_at": ago(days), "actor_ref": actor,
                                   "sentiment": sentiment, "due_on": due_on }))
                    .await
                    .assert_status_ok();
            }
        };

        // The engagement view counts everything in its window, so read it before
        // and after and compare.
        let before: Value = request.get("/api/insights/engagement?days=90").await.json();

        let acc_a = account("Rollup A").await;
        let acc_b = account("Rollup B").await;
        let c1 = contact("Rollup One", Some(acc_a.clone())).await;
        let c2 = contact("Rollup Two", Some(acc_a.clone())).await;
        let c3 = contact("Rollup Three", Some(acc_b.clone())).await;
        let c4 = contact("Rollup Four", None).await;
        let (w1, w2) = (a_worker(), a_worker());

        activity("contact", c1.clone(), "call", 10, Some(w1.clone()), Some("positive"), None).await;
        activity("contact", c1.clone(), "meeting", 3, Some(w1.clone()), None, Some("2099-01-01")).await;
        activity("contact", c2.clone(), "email", 40, Some(w2.clone()), None, None).await;
        activity("account", acc_a.clone(), "note", 5, Some(w1.clone()), Some("neutral"), None).await;
        activity("contact", c3.clone(), "call", 100, None, None, None).await;
        activity("contact", c3.clone(), "email", 1, None, None, Some("2099-02-02")).await;

        // A deal with a 20-day-old touch and one with a fresh touch, and two leads.
        let (pipeline_pid, _stages) = seed_pipeline!(&request).await;
        let mut deals = Vec::new();
        for name in ["Rollup stale deal", "Rollup fresh deal", "Rollup silent deal"] {
            let made: Value = request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline_pid,
                               "amount_minor": 1000_i64, "currency": "GBP",
                               "expected_close_on": "2099-12-31" }))
                .await
                .json();
            deals.push(made["pid"].as_str().unwrap().to_string());
        }
        activity("deal", deals[0].clone(), "call", 20, None, None, None).await;
        activity("deal", deals[1].clone(), "call", 1, None, None, None).await;

        // ── Cadence (30 days): who has gone quiet, and who has a next step.
        let cadence: Value = request.get("/api/insights/cadence?days=30").await.json();
        let c2_row = find(&cadence["untouched_contacts"], &c2).expect("c2 is 40 days quiet");
        assert_eq!(c2_row["days_since_touch"], 40);
        assert_eq!(c2_row["has_next_touch"], false);
        let c3_row = find(&cadence["untouched_contacts"], &c3);
        assert!(c3_row.is_none(), "c3 was touched yesterday");
        assert!(find(&cadence["untouched_contacts"], &c1).is_none(), "c1 was touched 3 days ago");
        assert!(find(&cadence["untouched_contacts"], &c4).is_none(), "c4 ages from creation: today");
        // Account A: its own note (5), c1 (3) and c2 (40): the latest touch is 3 days.
        assert!(find(&cadence["untouched_accounts"], &acc_a).is_none());
        assert!(find(&cadence["untouched_accounts"], &acc_b).is_none(), "c3 was touched 1 day ago");
        // The threshold is at least one day, so a touch 3 days ago is quiet and one 1 day ago is not.
        let tight: Value = request.get("/api/insights/cadence?days=1").await.json();
        assert_eq!(find(&tight["untouched_contacts"], &c1).unwrap()["has_next_touch"], true);
        assert_eq!(find(&tight["untouched_contacts"], &c1).unwrap()["days_since_touch"], 3);
        assert!(find(&tight["untouched_contacts"], &c3).is_none(), "c3 was touched 1 day ago");
        assert_eq!(find(&tight["untouched_accounts"], &acc_a).unwrap()["days_since_touch"], 3);
        assert!(find(&tight["untouched_accounts"], &acc_b).is_none(), "its contact c3: 1 day");

        // ── Members: last touch, contacts and open follow-ups per account.
        let members: Value = request.get("/api/insights/members?days=30").await.json();
        let a_row = find(&members["accounts"], &acc_a).unwrap();
        assert_eq!(a_row["contacts"], 2);
        assert_eq!(a_row["days_since_touch"], 3);
        assert_eq!(a_row["open_followups"], 1, "only c1's meeting has a due date");
        assert_eq!(a_row["silent"], false);
        let b_row = find(&members["accounts"], &acc_b).unwrap();
        assert_eq!(b_row["days_since_touch"], 1);
        assert_eq!(b_row["open_followups"], 1, "c3's email");

        // ── Stakeholders: days since a contact's own last touch.
        for (pid, role) in [(&c1, "customer"), (&c2, "partner"), (&c4, "customer")] {
            request
                .put(&format!("/api/contacts/{pid}/stakeholder"))
                .json(&json!({ "role": role, "influence": 3, "interest": 4 }))
                .await
                .assert_status_ok();
        }
        let stakeholders: Value = request.get("/api/insights/stakeholders").await.json();
        let days_of = |pid: &str| -> i64 {
            ["customer", "partner"]
                .iter()
                .find_map(|role| find(&stakeholders["by_role"][role], pid))
                .unwrap_or_else(|| panic!("{pid} is declared"))["days_since_touch"]
                .as_i64()
                .unwrap()
        };
        assert_eq!(days_of(&c1), 3, "the latest of its two touches");
        assert_eq!(days_of(&c2), 40);
        assert_eq!(days_of(&c4), 0, "never touched: ages from creation");

        // ── Pipeline hygiene (14 days): the deal touched 20 days ago and the one
        // never touched are flagged; yesterday's is not.
        let hygiene: Value = request.get("/api/insights/pipeline-hygiene?days=14").await.json();
        let stale_deals: Vec<&str> = hygiene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["rule"] == "open_deal_without_recent_activity")
            .filter_map(|f| f["deal"]["pid"].as_str())
            .collect();
        assert!(stale_deals.contains(&deals[0].as_str()), "20 days old");
        assert!(stale_deals.contains(&deals[2].as_str()), "never touched");
        assert!(!stale_deals.contains(&deals[1].as_str()), "touched yesterday");

        // ── Engagement (90 days): what the six fixture touches added. The 100-day
        // one is outside the window and the deal touches count as well.
        let after: Value = request.get("/api/insights/engagement?days=90").await.json();
        let delta = |section: &str, key: &str| {
            after[section][key].as_i64().unwrap_or(0) - before[section][key].as_i64().unwrap_or(0)
        };
        let total = after["touches"].as_i64().unwrap() - before["touches"].as_i64().unwrap();
        assert_eq!(total, 7, "all but the 100-day touch");
        assert_eq!(delta("per_kind", "call"), 3, "c1, and the two deal calls");
        assert_eq!(delta("per_kind", "meeting"), 1);
        assert_eq!(delta("per_kind", "email"), 2, "c2 and c3");
        assert_eq!(delta("per_kind", "note"), 1);
        assert_eq!(delta("per_kind", "task"), 0);
        assert_eq!(delta("sentiment", "positive"), 1);
        assert_eq!(delta("sentiment", "neutral"), 1);
        let month = |days: i64| (Utc::now() - Duration::days(days)).format("%Y-%m").to_string();
        let w1_this_month = after["per_recorder_month"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(k, _)| k.starts_with(w1.as_str()))
            .map(|(_, v)| v.as_i64().unwrap())
            .sum::<i64>();
        assert_eq!(w1_this_month, 3, "w1 recorded three touches across 10, 3 and 5 days ago");
        let w2_key = format!("{w2} {}", month(40));
        assert_eq!(after["per_recorder_month"][&w2_key], 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_consent_views_count_the_events_they_were_given() {
    request::<App, _, _>(|request, _ctx| async move {
        let before: Value = request.get("/api/insights/dpo?days=30").await.json();
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Consent rollup" }),
            )
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let mut contacts = Vec::new();
        for name in ["Consent One", "Consent Two", "Consent Three"] {
            let made: Value = request
                .post("/api/contacts")
                .json(&json!({ "person_ref": a_person(), "display_name": name,
                               "account_pid": account }))
                .await
                .json();
            contacts.push(made["pid"].as_str().unwrap().to_string());
        }
        let source = format!("rollup-{}", uuid::Uuid::new_v4());

        // Two grants and a withdrawal for the first contact, a withdrawal for the
        // second, nothing for the third.
        for (contact, action) in [
            (&contacts[0], "granted"),
            (&contacts[0], "withdrawn"),
            (&contacts[1], "granted"),
            (&contacts[1], "withdrawn"),
        ] {
            request
                .post(&format!("/api/contacts/{contact}/consent"))
                .json(&json!({ "action": action, "source": source }))
                .await
                .assert_status_ok();
        }

        let after: Value = request.get("/api/insights/dpo?days=30").await.json();
        assert_eq!(after["consent_events_by_source"][&source], 4);
        assert_eq!(
            after["withdrawals_in_window"].as_i64().unwrap()
                - before["withdrawals_in_window"].as_i64().unwrap(),
            2
        );
        assert_eq!(
            after["contacts"].as_i64().unwrap() - before["contacts"].as_i64().unwrap(),
            3
        );

        let by_account: Value = request
            .get("/api/insights/consent-by-account?days=30")
            .await
            .json();
        let row = find(&by_account["accounts"], &account).expect("the account is listed");
        assert_eq!(row["withdrawals_in_window"], 2);
        assert_eq!(row["consent_coverage"]["withdrawn"], 2);
        assert_eq!(row["consent_coverage"]["never"], 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_contact_page_cuts_its_lists_at_200_and_says_so() {
    request::<App, _, _>(|request, _ctx| async move {
        let made: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": a_person(), "display_name": "Busy Contact" }))
            .await
            .json();
        let pid = made["pid"].as_str().unwrap().to_string();

        let detail: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        assert_eq!(
            detail["truncated"],
            json!({ "activities": false, "deals": false, "tickets": false })
        );

        // 201 activities, one a day further back than the last: the oldest is cut.
        for day in 0..=200 {
            request
                .post("/api/activities")
                .json(&json!({ "subject_kind": "contact", "subject_pid": pid,
                               "kind": "note", "summary": format!("touch {day}"),
                               "occurred_at": ago(day) }))
                .await
                .assert_status_ok();
        }
        let detail: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
        let activities = detail["activities"].as_array().unwrap();
        assert_eq!(activities.len(), 200);
        assert_eq!(detail["truncated"]["activities"], true);
        assert_eq!(detail["truncated"]["deals"], false);
        assert!(
            activities.iter().all(|a| a["summary"] != "touch 200"),
            "the oldest touch is the one that is cut"
        );
        assert_eq!(activities[0]["summary"], "touch 0", "newest first");
    })
    .await;
}
