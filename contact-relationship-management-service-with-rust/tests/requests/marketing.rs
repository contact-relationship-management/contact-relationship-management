//! The consent-gated marketing journey (CRM-R6–R9): consent →
//! segment preview → simulated campaign → nurture advance →
//! unsubscribe exits everything and blocks the next send.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_contact};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Consent gates the segment; the simulated campaign touches only the
// consented; withdrawal exits the nurture enrolment and the next
// advance sends nothing.
async fn consent_gates_campaign_and_nurture() {
    request::<App, _, _>(|request, _ctx| async move {
        let consented = seed_contact!(&request, "Consented Contact").await;
        // A second contact who never consented.
        let never: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": a_person(), "display_name": "Never Contact" }))
            .await
            .json();
        let _never_pid = never["pid"].as_str().unwrap();
        // Segment (empty filter = everyone consented).
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let segment_pid = segment["pid"].as_str().unwrap().to_string();
        let preview: Value = request
            .get(&format!("/api/segments/{segment_pid}/preview"))
            .await
            .json();
        assert_eq!(preview["count"], 1, "only the consented contact matches");
        // Campaign: draft → scheduled → run (simulated).
        let campaign: Value = request
            .post("/api/campaigns")
            .json(&json!({
                "name": "Summer Push", "cost_minor": 50_000, "currency": "GBP",
                "segment_pid": segment_pid,
            }))
            .await
            .json();
        let campaign_pid = campaign["pid"].as_str().unwrap().to_string();
        // Run from draft is refused; schedule first.
        assert_eq!(
            request
                .post(&format!("/api/campaigns/{campaign_pid}/run"))
                .await
                .status_code(),
            422
        );
        request
            .post(&format!("/api/campaigns/{campaign_pid}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status_ok();
        let ran: Value = request
            .post(&format!("/api/campaigns/{campaign_pid}/run"))
            .await
            .json();
        assert_eq!(ran["recipients"], 1);
        assert_eq!(ran["status"], "completed");
        // Funnel/ROI: no attributed revenue yet ⇒ negative ROI with
        // honest parts.
        let funnel: Value = request
            .get(&format!("/api/campaigns/{campaign_pid}/funnel"))
            .await
            .json();
        assert_eq!(funnel["won_revenue_minor"], 0);
        assert_eq!(funnel["roi"]["denominator"], 50_000);
        // Nurture: enrol the consented contact; step 0 due immediately.
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({
                "name": "Welcome Drip",
                "steps": [
                    { "delay_hours": 0, "template_ref": "welcome-1" },
                    { "delay_hours": 0, "template_ref": "welcome-2" },
                ],
            }))
            .await
            .json();
        let sequence_pid = sequence["pid"].as_str().unwrap().to_string();
        request
            .post(&format!(
                "/api/nurture-sequences/{sequence_pid}/enrollments"
            ))
            .json(&json!({ "contact_pid": consented }))
            .await
            .assert_status_ok();
        // First advance sends step 0.
        let advanced: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(advanced["sent"], 1);
        // Withdraw consent: the enrolment exits; the next advance
        // sends nothing (send-time re-check).
        request
            .post(&format!("/api/contacts/{consented}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "unsubscribe link" }))
            .await
            .assert_status_ok();
        let advanced: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(advanced["sent"], 0, "withdrawn contact gets nothing");
        // The segment no longer matches them either.
        let preview: Value = request
            .get(&format!("/api/segments/{segment_pid}/preview"))
            .await
            .json();
        assert_eq!(preview["count"], 0);
        // The consent history holds both events, append-only.
        let history: Value = request
            .get(&format!("/api/contacts/{consented}/consent"))
            .await
            .json();
        let actions: Vec<&str> = history
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["action"].as_str())
            .collect();
        assert_eq!(actions, vec!["granted", "withdrawn"]);
        // An enrolment for a non-consented contact is refused outright.
        let refused = request
            .post(&format!(
                "/api/nurture-sequences/{sequence_pid}/enrollments"
            ))
            .json(&json!({ "contact_pid": consented }))
            .await;
        assert_eq!(refused.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_funnel_can_count_revenue_won_in_other_currencies_at_the_rate_in_force() {
    request::<App, _, _>(|request, _ctx| async move {
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let campaign: Value = request
            .post("/api/campaigns")
            .json(
                &json!({ "name": "Converted", "cost_minor": 50_000, "currency": "GBP",
                           "segment_pid": segment["pid"] }),
            )
            .await
            .json();
        let pid = campaign["pid"].as_str().unwrap().to_string();
        let (pipeline, stages) = super::seed_pipeline!(&request).await;
        for (name, amount, currency) in [
            ("G", 100_000, "GBP"),
            ("D", 20_000, "USD"),
            ("Y", 700, "JPY"),
        ] {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": currency, "source_campaign_pid": pid }),
                )
                .await
                .json();
            request
                .post(&format!(
                    "/api/deals/{}/stage",
                    deal["pid"].as_str().unwrap()
                ))
                .json(&json!({ "stage_pid": stages[2] }))
                .await
                .assert_status_ok();
        }
        request
            .put("/api/fx-rates")
            .json(&json!({ "from": "USD", "to": "GBP", "rate": "0.80",
                           "effective_on": "2026-01-01", "source": "test table" }))
            .await
            .assert_status_ok();

        // As before: the campaign's own currency only, the others listed apart.
        let plain: Value = request
            .get(&format!("/api/campaigns/{pid}/funnel"))
            .await
            .json();
        assert_eq!(plain["won_revenue_minor"], 100_000, "{plain}");
        assert_eq!(
            plain["won_revenue_other_currencies"]["USD"], 20_000,
            "{plain}"
        );
        assert!(plain["rate_date"].is_null() && plain["rates"].as_array().unwrap().is_empty());

        // Converted: the dollars count at 0.80; the yen have no rate and stay apart, counted.
        let done: Value = request
            .get(&format!("/api/campaigns/{pid}/funnel?convert=true"))
            .await
            .json();
        assert_eq!(done["won_revenue_minor"], 116_000, "{done}");
        assert_eq!(done["roi"]["numerator"], 66_000, "{done}");
        assert_eq!(done["won_revenue_other_currencies"]["JPY"], 700, "{done}");
        assert!(
            done["won_revenue_other_currencies"]["USD"].is_null(),
            "{done}"
        );
        assert_eq!(done["rates"][0]["from"], "USD", "{done}");
        assert_eq!(done["unconverted"][0]["currency"], "JPY", "{done}");

        for query in ["rate_date=2026-01-01", "convert=true&rate_date=2999-01-01"] {
            let r = request
                .get(&format!("/api/campaigns/{pid}/funnel?{query}"))
                .await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let refused: Value = r.json();
            assert_eq!(
                refused["errors"][0]["pointer"], "/rate_date",
                "{query}: {refused}"
            );
        }
    })
    .await;
}
