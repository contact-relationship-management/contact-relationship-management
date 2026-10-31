//! The sales journey (CRM-R3–R5): capture → score → convert →
//! pipeline → won, with the forecast reflecting every move, plus the
//! unknown-pid 404 pins.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_pipeline};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Lead: scored on capture with breakdown; the state machine refuses a
// skip to converted; conversion creates the contact + deal in-tx;
// the deal moves through stages to Won; the forecast follows.
async fn sales_journey_end_to_end() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline_pid, stage_pids) = seed_pipeline!(&request).await;
        // Capture a referral lead with a corporate domain: 20+10 = 30.
        let captured: Value = request
            .post("/api/leads")
            .json(&json!({
                "display_name": "Test Prospect",
                "source": "referral",
                "email": "prospect@initech.example",
            }))
            .await
            .json();
        let lead_pid = captured["pid"].as_str().unwrap().to_string();
        assert_eq!(captured["score"]["score"], 30);
        assert_eq!(captured["score"]["label"], "cold");
        let rules = captured["score"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 8, "every rule listed in the breakdown");
        // Skip to converted is refused by the machine.
        let skip = request
            .post(&format!("/api/leads/{lead_pid}/status"))
            .json(&json!({ "to": "converted" }))
            .await;
        assert_eq!(skip.status_code(), 422);
        // Walk: contacted → qualified → converted (creates contact + deal).
        for to in ["contacted", "qualified"] {
            request
                .post(&format!("/api/leads/{lead_pid}/status"))
                .json(&json!({ "to": to }))
                .await
                .assert_status_ok();
        }
        let converted: Value = request
            .post(&format!("/api/leads/{lead_pid}/status"))
            .json(&json!({
                "to": "converted",
                "person_ref": a_person(),
                "deal": {
                    "name": "Prospect Deal", "pipeline_pid": pipeline_pid,
                    "amount_minor": 1_000_000, "currency": "GBP",
                },
            }))
            .await
            .json();
        let deal_pid = converted["deal_pid"]
            .as_str()
            .expect("deal opened")
            .to_string();
        assert!(converted["contact_pid"].is_string(), "contact created");
        // Forecast at the first stage (10%): 100,000.
        let forecast: Value = request.get("/api/forecast").await.json();
        assert_eq!(forecast["totals_minor"]["GBP"], 100_000);
        // Move to Proposal (50%): 500,000.
        request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[1] }))
            .await
            .assert_status_ok();
        let forecast: Value = request.get("/api/forecast").await.json();
        assert_eq!(forecast["totals_minor"]["GBP"], 500_000);
        // Lost without a reason is refused; Won closes the deal.
        let lost_no_reason = request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[3] }))
            .await;
        assert_eq!(lost_no_reason.status_code(), 422);
        request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[2] }))
            .await
            .assert_status_ok();
        // Closed deals leave the forecast; further moves are refused.
        let forecast: Value = request.get("/api/forecast").await.json();
        assert!(forecast["totals_minor"].as_object().unwrap().is_empty());
        let move_closed = request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[1] }))
            .await;
        assert_eq!(move_closed.status_code(), 422, "closed deals are immutable");
        // Reasoned reopen returns it to an open stage.
        request
            .post(&format!("/api/deals/{deal_pid}/reopen"))
            .json(&json!({ "reason": "signature fell through" }))
            .await
            .assert_status_ok();
        let forecast: Value = request.get("/api/forecast").await.json();
        assert_eq!(
            forecast["totals_minor"]["GBP"], 500_000,
            "reopened at Proposal"
        );
        // The sales dashboard reports the honest win rate parts.
        let dashboard: Value = request.get("/api/dashboards/sales").await.json();
        assert_eq!(dashboard["win_rate"]["denominator"], 0);
        assert!(
            dashboard["win_rate"]["value"].is_null(),
            "0/0 is null, not 0%"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Unknown-pid reads are honest 404s (family lesson); a stage from a
// different pipeline is refused.
async fn contracts_404_and_pipeline_membership() {
    request::<App, _, _>(|request, _ctx| async move {
        let ghost = uuid::Uuid::new_v4();
        for path in [
            format!("/api/contacts/{ghost}"),
            format!("/api/leads/{ghost}"),
            format!("/api/tickets/{ghost}"),
            "/api/contacts/not-a-uuid".to_string(),
        ] {
            assert_eq!(request.get(&path).await.status_code(), 404, "{path}");
        }
        // A stage from another pipeline is refused on a move.
        let (pipeline_a, _stages_a) = seed_pipeline!(&request).await;
        let (_pipeline_b, stages_b) = seed_pipeline!(&request).await;
        let deal: Value = request
            .post("/api/deals")
            .json(&json!({
                "name": "Cross-pipeline", "pipeline_pid": pipeline_a,
                "amount_minor": 100, "currency": "GBP",
            }))
            .await
            .json();
        let cross = request
            .post(&format!(
                "/api/deals/{}/stage",
                deal["pid"].as_str().unwrap()
            ))
            .json(&json!({ "stage_pid": stages_b[1] }))
            .await;
        assert_eq!(
            cross.status_code(),
            422,
            "stage from another pipeline refused"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// CRM-R4, CRM-D20: an open deal is stalled after the organization's N days with no stage move
// and no activity on it; a move or an activity that has happened clears it; a closed deal never is.
async fn a_deal_with_no_move_or_activity_for_n_days_is_stalled() {
    use sea_orm::ConnectionTrait;
    request::<App, _, _>(|request, ctx| async move {
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let mut pids = Vec::new();
        for name in ["Stalled quiet", "Stalled called", "Stalled planned", "Stalled moved", "Stalled won"] {
            let deal: Value = request
                .post("/api/deals")
                .json(&json!({
                    "name": name, "pipeline_pid": pipeline,
                    "amount_minor": 100, "currency": "GBP",
                }))
                .await
                .json();
            pids.push(deal["pid"].as_str().unwrap().to_string());
        }
        let listed = |all: &Value, pid: &str| -> Value {
            all.as_array()
                .unwrap()
                .iter()
                .find(|d| d["pid"] == pid)
                .cloned()
                .unwrap_or_else(|| panic!("{pid} listed"))
        };
        // Just made: nothing is stalled, and the last touch is the creation.
        let all: Value = request.get(&format!("/api/deals?pipeline={pipeline}")).await.json();
        for pid in &pids {
            let deal = listed(&all, pid);
            assert_eq!(deal["stalled"], false, "{deal}");
            assert_eq!(deal["last_touched_at"], deal["stage_moved_at"], "{deal}");
        }
        // Twenty days without a move, for all five.
        for pid in &pids {
            ctx.db
                .execute_unprepared(&format!(
                    "UPDATE deals SET stage_moved_at = now() - interval '20 days' WHERE pid = '{pid}'"
                ))
                .await
                .unwrap();
        }
        // An activity five days ago on the second, a planned one next week on the third; the
        // first stays quiet.
        let five_days_ago = (chrono::Utc::now() - chrono::Duration::days(5)).to_rfc3339();
        let next_week = (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339();
        for (pid, at) in [(&pids[1], &five_days_ago), (&pids[2], &next_week)] {
            request
                .post("/api/activities")
                .json(&json!({
                    "subject_kind": "deal", "subject_pid": pid, "kind": "call",
                    "summary": "Spoke about the order", "occurred_at": at,
                }))
                .await
                .assert_status_success();
        }
        // The fourth moves stage; the fifth is won.
        request
            .post(&format!("/api/deals/{}/stage", pids[3]))
            .json(&json!({ "stage_pid": stages[1] }))
            .await
            .assert_status_ok();
        let won_stage = &stages[2];
        let won = request
            .post(&format!("/api/deals/{}/stage", pids[4]))
            .json(&json!({ "stage_pid": won_stage }))
            .await;
        assert_eq!(won.status_code(), 200, "{}", won.text());
        let all: Value = request.get(&format!("/api/deals?pipeline={pipeline}")).await.json();
        let flags: Vec<bool> = pids
            .iter()
            .map(|pid| listed(&all, pid)["stalled"].as_bool().unwrap())
            .collect();
        assert_eq!(flags, [true, false, true, false, false], "{all}");
        let called = listed(&all, &pids[1]);
        let touched = chrono::DateTime::parse_from_rfc3339(called["last_touched_at"].as_str().unwrap())
            .unwrap();
        let expected = chrono::DateTime::parse_from_rfc3339(&five_days_ago).unwrap();
        assert!((touched - expected).num_seconds().abs() <= 1, "{called}");

        // N is the organization's: at 30 days the quiet deal is not stalled; at 5 the one
        // called five days ago is.
        let settings: Value = request.get("/api/settings/organization").await.json();
        assert_eq!(settings["stalled_after_days"], 14);
        for (bad, why) in [(0, "0"), (366, "366")] {
            let refused = request
                .put("/api/settings/organization")
                .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1, "stalled_after_days": bad }))
                .await;
            assert_eq!(refused.status_code(), 422, "{why}");
            assert!(refused.text().contains("stalled_after_days"), "{why}");
        }
        let set: Value = request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1, "stalled_after_days": 30 }))
            .await
            .json();
        assert_eq!(set["stalled_after_days"], 30);
        // A PUT without it keeps it.
        let kept: Value = request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1 }))
            .await
            .json();
        assert_eq!(kept["stalled_after_days"], 30);
        let all: Value = request.get(&format!("/api/deals?pipeline={pipeline}")).await.json();
        assert_eq!(listed(&all, &pids[0])["stalled"], false, "{all}");
        request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1, "stalled_after_days": 5 }))
            .await
            .assert_status_ok();
        let all: Value = request.get(&format!("/api/deals?pipeline={pipeline}")).await.json();
        assert_eq!(listed(&all, &pids[1])["stalled"], true, "{all}");
        // The change is audited with the value.
        let audited = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM audit_logs WHERE entity = 'organization_settings' \
                 AND (snapshot ->> 'stalled_after_days') = '5'",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "n")
            .unwrap();
        assert_eq!(audited, 1);
        // Put the starting value back for the other tests.
        request
            .put("/api/settings/organization")
            .json(&json!({ "time_zone": "UTC", "fiscal_year_start_month": 1, "stalled_after_days": 14 }))
            .await
            .assert_status_ok();
    })
    .await;
}
