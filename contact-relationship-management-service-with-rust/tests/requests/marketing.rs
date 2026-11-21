//! The consent-gated marketing journey (CRM-R6–R9): consent →
//! segment preview → simulated campaign → nurture advance →
//! unsubscribe exits everything and blocks the next send.

use axa_test::prelude::*;
use axum::http::StatusCode;
use contact_relationship_management::app::App;
use contact_relationship_management::db;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use super::{a_person, seed_contact};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
// Consent gates the segment; the simulated campaign touches only the
// consented; withdrawal exits the nurture enrolment and the next
// advance sends nothing.
async fn consent_gates_campaign_and_nurture() {
    request::<App, _, _>(|request, ctx| async move {
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
        let campaign_basis = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT basis FROM email_deliveries WHERE campaign_pid = $1 AND contact_pid = $2",
        )
        .bind(Uuid::parse_str(&campaign_pid).unwrap())
        .bind(Uuid::parse_str(&consented).unwrap())
        .fetch_one(db::pool(&ctx))
        .await
        .unwrap();
        assert_eq!(campaign_basis["basis"], "granted", "{campaign_basis}");
        assert_eq!(campaign_basis["regime"], "opt_in", "{campaign_basis}");
        assert!(campaign_basis["profile"].is_null(), "{campaign_basis}");
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
        let recorded = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM email_deliveries WHERE contact_pid = $1 \
             AND state = 'sent' AND basis ->> 'basis' = 'granted'",
        )
        .bind(Uuid::parse_str(&consented).unwrap())
        .fetch_one(db::pool(&ctx))
        .await
        .unwrap();
        assert_eq!(recorded, 2, "campaign and nurture both record their basis");
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
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

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_funnel_can_convert_each_deal_at_the_rate_of_the_day_it_closed() {
    request::<App, _, _>(|request, ctx| async move {
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let campaign: Value = request
            .post("/api/campaigns")
            .json(
                &json!({ "name": "Dated", "cost_minor": 50_000, "currency": "GBP",
                           "segment_pid": segment["pid"] }),
            )
            .await
            .json();
        let pid = campaign["pid"].as_str().unwrap().to_string();
        let (pipeline, stages) = super::seed_pipeline!(&request).await;
        // Pounds, dollars either side of a rate change and once before any rate, and yen.
        for (name, amount, currency, closed) in [
            ("G", 100_000, "GBP", None),
            ("D1", 10_000, "USD", Some("2026-02-01")),
            ("D2", 10_000, "USD", Some("2026-07-01")),
            ("D3", 4_000, "USD", Some("2025-12-01")),
            ("Y", 700, "JPY", Some("2026-02-01")),
        ] {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": currency, "source_campaign_pid": pid }),
                )
                .await
                .json();
            let deal_pid = deal["pid"].as_str().unwrap();
            request
                .post(&format!("/api/deals/{deal_pid}/stage"))
                .json(&json!({ "stage_pid": stages[2] }))
                .await
                .assert_status_ok();
            if let Some(day) = closed {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE deals SET closed_at = '{day}T09:00:00Z' WHERE pid = $1::uuid"
                )))
                .bind(deal_pid)
                .execute(contact_relationship_management::db::pool(&ctx))
                .await
                .unwrap();
            }
        }
        for (rate, on) in [("0.80", "2026-01-01"), ("0.50", "2026-06-01")] {
            request
                .put("/api/fx-rates")
                .json(&json!({ "from": "USD", "to": "GBP", "rate": rate,
                               "effective_on": on, "source": "test table" }))
                .await
                .assert_status_ok();
        }

        // One date for all the dollars: 24,000 at today's 0.50.
        let one: Value = request
            .get(&format!("/api/campaigns/{pid}/funnel?convert=true"))
            .await
            .json();
        assert_eq!(one["won_revenue_minor"], 112_000, "{one}");
        assert!(one["rate_basis"].is_null(), "{one}");

        // On the day each closed: 8,000 and 5,000; the dollars before any rate and the yen stay
        // apart, counted; no one date is claimed.
        let dated: Value = request
            .get(&format!(
                "/api/campaigns/{pid}/funnel?convert=true&rate_basis=close_day"
            ))
            .await
            .json();
        assert_eq!(dated["won_revenue_minor"], 113_000, "{dated}");
        assert_eq!(dated["roi"]["numerator"], 63_000, "{dated}");
        assert_eq!(dated["rate_basis"], "close_day", "{dated}");
        assert!(dated["rate_date"].is_null(), "{dated}");
        assert_eq!(dated["rates"].as_array().unwrap().len(), 2, "{dated}");
        assert_eq!(
            dated["won_revenue_other_currencies"],
            json!({ "JPY": 700, "USD": 4_000 }),
            "only what had no rate on its day: {dated}"
        );
        assert_eq!(dated["unconverted"].as_array().unwrap().len(), 2, "{dated}");

        for (query, pointer) in [
            ("rate_basis=close_day", "/rate_basis"),
            ("convert=true&rate_basis=today", "/rate_basis"),
            (
                "convert=true&rate_basis=close_day&rate_date=2026-03-01",
                "/rate_date",
            ),
        ] {
            let r = request
                .get(&format!("/api/campaigns/{pid}/funnel?{query}"))
                .await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let refused: Value = r.json();
            assert_eq!(
                refused["errors"][0]["pointer"], pointer,
                "{query}: {refused}"
            );
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
// CRM-T243: the facts of every contact are read in a few set queries and matched up again by pid,
// so each contact must get its own account's tier and its own tags, never a neighbour's.
async fn a_segment_gives_each_contact_its_own_tier_and_tags() {
    request::<App, _, _>(|request, _ctx| async move {
        let account = |tier: &'static str| {
            let request = &request;
            async move {
                let made: Value = request
                    .post("/api/accounts")
                    .json(&json!({
                        "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                        "display_name": format!("A {tier} account"),
                        "tier": tier,
                    }))
                    .await
                    .json();
                made["pid"].as_str().unwrap().to_string()
            }
        };
        let customer = account("customer").await;
        let prospect = account("prospect").await;
        let contact = |name: &'static str,
                       account: Option<String>,
                       consent: bool,
                       tags: &'static [&'static str]| {
            let request = &request;
            async move {
                let mut body = json!({ "person_ref": a_person(), "display_name": name });
                if let Some(account) = account {
                    body["account_pid"] = json!(account);
                }
                let made: Value = request.post("/api/contacts").json(&body).await.json();
                let pid = made["pid"].as_str().unwrap().to_string();
                if consent {
                    request
                        .post(&format!("/api/contacts/{pid}/consent"))
                        .json(&json!({ "action": "granted", "source": "web form" }))
                        .await
                        .assert_status_ok();
                }
                if !tags.is_empty() {
                    request
                        .put(&format!("/api/contacts/{pid}/tags"))
                        .json(&json!({ "tags": tags }))
                        .await
                        .assert_status_ok();
                }
            }
        };
        // Interleaved, so a lookup that lost the pairing would move a tier or a tag to a
        // neighbour.
        contact("Customer VIP", Some(customer.clone()), true, &["vip"]).await;
        contact("Prospect VIP", Some(prospect.clone()), true, &["vip"]).await;
        contact("Customer plain", Some(customer.clone()), true, &[]).await;
        contact("No account VIP", None, true, &["vip", "trial"]).await;
        contact(
            "Customer VIP unconsented",
            Some(customer.clone()),
            false,
            &["vip"],
        )
        .await;
        contact("Prospect plain", Some(prospect), true, &["trial"]).await;
        let preview = |filter: Value| {
            let request = &request;
            async move {
                let made: Value = request
                    .post("/api/segments")
                    .json(&json!({ "name": "Tiered", "filter": filter }))
                    .await
                    .json();
                let pid = made["pid"].as_str().unwrap().to_string();
                let preview: Value = request
                    .get(&format!("/api/segments/{pid}/preview"))
                    .await
                    .json();
                let mut names: Vec<String> = preview["sample"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_str().unwrap().to_string())
                    .collect();
                names.sort();
                (preview["count"].as_u64().unwrap(), names)
            }
        };
        assert_eq!(
            preview(json!({ "account_tiers": ["customer"], "tags": ["vip"] })).await,
            (1, vec!["Customer VIP".to_string()])
        );
        assert_eq!(
            preview(json!({ "account_tiers": ["customer"] })).await,
            (
                2,
                vec!["Customer VIP".to_string(), "Customer plain".to_string()]
            )
        );
        assert_eq!(
            preview(json!({ "tags": ["trial"] })).await,
            (
                2,
                vec!["No account VIP".to_string(), "Prospect plain".to_string()]
            )
        );
        // The consent gate still holds: five consented, the sixth never.
        assert_eq!(preview(json!({})).await.0, 5);
    })
    .await;
}

/// A campaign is refused for each thing wrong with it, a run needs a segment, and a cancelled one
/// is recorded as a status change and not as a start or a finish (CRM-T292, round 17).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_bad_campaign_is_refused_for_each_fault_and_one_with_no_segment_cannot_run() {
    request::<App, _, _>(|request, ctx| async move {
        let good = json!({ "name": "Autumn", "currency": "GBP" });
        let with = |key: &str, value: Value| {
            let mut body = good.clone();
            body[key] = value;
            body
        };
        let too_long = "x".repeat(20_001);
        for (body, part) in [
            (with("kind", json!("sms")), "kind must be email"),
            (
                with("cost_minor", json!(-1)),
                "cost_minor must be non-negative",
            ),
            (with("subject", json!("two\nlines")), "single line"),
            (with("body", json!(too_long)), "body exceeds"),
            (with("purpose", json!("nonsense")), "purpose"),
        ] {
            let r = request.post("/api/campaigns").json(&body).await;
            r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
            let text = r.text();
            assert!(text.contains(part), "{part}: {text}");
        }
        let mixed = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Both", "currency": "GBP", "subject": "S",
                           "template_pid": uuid::Uuid::new_v4() }))
            .await;
        mixed.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        // One with no segment can be made and scheduled, but not run.
        let made: Value = request.post("/api/campaigns").json(&good).await.json();
        let pid = made["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/campaigns/{pid}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status_ok();
        let run = request.post(&format!("/api/campaigns/{pid}/run")).await;
        run.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        assert!(run.text().contains("no segment"), "{}", run.text());

        // Cancelling it is a status change in the audit trail, not a start or a finish.
        request
            .post(&format!("/api/campaigns/{pid}/status"))
            .json(&json!({ "to": "cancelled" }))
            .await
            .assert_status_ok();
        let actions: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT action FROM audit_logs WHERE entity_pid = '{pid}' ORDER BY id"
        )))
        .fetch_all(contact_relationship_management::db::pool(&ctx))
        .await
        .unwrap();
        assert!(
            actions
                .iter()
                .any(|a| a.contains("cancel") || a.contains("status")),
            "{actions:?}"
        );
        // A cancelled campaign cannot be scheduled again.
        request
            .post(&format!("/api/campaigns/{pid}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}
