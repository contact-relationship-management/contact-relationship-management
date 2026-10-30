//! Currency codes and minor units (CRM-T167, CRM-T168; CRM-R50, CRM-R95, CRM-D19): an unknown code
//! is refused wherever a currency is taken, a missing one is refused and never assumed, and a
//! conversion between currencies of different exponents scales by each one's own minor unit.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_pipeline};

/// The first problem's pointer of a `422`.
fn pointer(answer: &Value) -> &str {
    answer["errors"][0]["pointer"].as_str().unwrap_or_default()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unknown_currency_code_is_refused_wherever_a_currency_is_taken() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, _stages) = seed_pipeline!(&request).await;
        for code in ["ZZZ", "XAU", "pounds", "jpy"] {
            // A deal.
            let r = request
                .post("/api/deals")
                .json(&json!({ "name": "D", "pipeline_pid": pipeline,
                               "amount_minor": 100, "currency": code }))
                .await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let answer: Value = r.json();
            assert_eq!(pointer(&answer), "/currency", "{code}: {answer}");
            // A campaign.
            let r = request
                .post("/api/campaigns")
                .json(&json!({ "name": "C", "currency": code }))
                .await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(pointer(&r.json::<Value>()), "/currency", "{code}");
            // A quota.
            let r = request
                .put("/api/quotas")
                .json(
                    &json!({ "owner_ref": a_person(), "period_from": "2026-01-01",
                               "period_to": "2026-03-31", "currency": code,
                               "amount_minor": 1_000 }),
                )
                .await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(pointer(&r.json::<Value>()), "/currency", "{code}");
            // A rate, on either side.
            for (from, to, at) in [(code, "EUR", "/from"), ("EUR", code, "/to")] {
                let r = request
                    .put("/api/fx-rates")
                    .json(&json!({ "from": from, "to": to, "rate": "2",
                                   "effective_on": "2026-01-01", "source": "test table" }))
                    .await;
                r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(pointer(&r.json::<Value>()), at, "{code}");
            }
            // A conversion target.
            let r = request
                .post("/api/reports/run")
                .json(&json!({ "kind": "deal", "group_by": "state",
                               "measure": "sum_amount", "convert_to": code }))
                .await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(pointer(&r.json::<Value>()), "/spec/convert_to", "{code}");
        }
        // The text says what a code is, with more than one example.
        let r = request
            .post("/api/deals")
            .json(&json!({ "name": "D", "pipeline_pid": pipeline,
                           "amount_minor": 100, "currency": "ZZZ" }))
            .await;
        let text = r.json::<Value>()["errors"][0]["detail"].to_string();
        assert!(text.contains("ZZZ is not an ISO 4217"), "{text}");
        // A lead converted into a deal in an unknown code is refused before anything changes.
        let lead: Value = request
            .post("/api/leads")
            .json(&json!({ "display_name": "Prospect", "source": "referral" }))
            .await
            .json();
        let lead_pid = lead["pid"].as_str().unwrap().to_string();
        for to in ["contacted", "qualified"] {
            request
                .post(&format!("/api/leads/{lead_pid}/status"))
                .json(&json!({ "to": to }))
                .await
                .assert_status_ok();
        }
        let r = request
            .post(&format!("/api/leads/{lead_pid}/status"))
            .json(&json!({ "to": "converted", "person_ref": a_person(),
                           "deal": { "name": "D", "pipeline_pid": pipeline,
                                     "amount_minor": 100, "currency": "ZZZ" } }))
            .await;
        r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(pointer(&r.json::<Value>()), "/deal/currency");
        let still: Value = request.get(&format!("/api/leads/{lead_pid}")).await.json();
        assert_eq!(still["lead"]["status"], "qualified", "{still}");
        // Nothing in an unknown code was stored.
        let deals: Vec<Value> = request.get("/api/deals").await.json();
        assert!(deals.is_empty(), "{deals:?}");
        // A code in the table, of any exponent, is taken.
        for code in ["JPY", "KWD", "CLF", "INR", "BRL"] {
            request
                .post("/api/deals")
                .json(&json!({ "name": code, "pipeline_pid": pipeline,
                               "amount_minor": 100, "currency": code }))
                .await
                .assert_status_ok();
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_missing_currency_is_refused_and_never_assumed() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, _stages) = seed_pipeline!(&request).await;
        // A campaign used to become GBP when it named no currency; now it is refused.
        let r = request
            .post("/api/campaigns")
            .json(&json!({ "name": "No currency", "cost_minor": 5_000 }))
            .await;
        r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let answer: Value = r.json();
        assert!(
            answer["errors"][0]["detail"]
                .as_str()
                .unwrap_or_default()
                .contains("missing field `currency`"),
            "{answer}"
        );
        // As is a deal, and a blank code on either.
        let r = request
            .post("/api/deals")
            .json(&json!({ "name": "D", "pipeline_pid": pipeline, "amount_minor": 100 }))
            .await;
        r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let answer: Value = r.json();
        assert!(
            answer.to_string().contains("missing field `currency`"),
            "{answer}"
        );
        for (path, body) in [
            ("/api/campaigns", json!({ "name": "C", "currency": "" })),
            (
                "/api/deals",
                json!({ "name": "D", "pipeline_pid": pipeline, "amount_minor": 1,
                        "currency": "" }),
            ),
        ] {
            let r = request.post(path).json(&body).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(pointer(&r.json::<Value>()), "/currency", "{path}");
        }
        let campaigns: Vec<Value> = request.get("/api/campaigns").await.json();
        assert!(campaigns.is_empty(), "{campaigns:?}");
        // A campaign that names its currency keeps it.
        let made: Value = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Yen", "currency": "JPY", "cost_minor": 5_000 }))
            .await
            .json();
        let listed: Vec<Value> = request.get("/api/campaigns").await.json();
        let row = listed.iter().find(|c| c["pid"] == made["pid"]).unwrap();
        assert_eq!(row["currency"], "JPY", "{row}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_converted_report_scales_by_each_currencys_own_minor_unit() {
    let _plain = super::no_suppression();
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, _stages) = seed_pipeline!(&request).await;
        // USD 100.00, KWD 1.000 and CLF 1.0000, all into yen.
        for (name, amount, currency) in [
            ("A", 10_000, "USD"),
            ("B", 1_000, "KWD"),
            ("C", 10_000, "CLF"),
        ] {
            request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline,
                               "amount_minor": amount, "currency": currency }))
                .await
                .assert_status_ok();
        }
        for (from, rate) in [("USD", "150"), ("KWD", "490"), ("CLF", "5800")] {
            request
                .put("/api/fx-rates")
                .json(&json!({ "from": from, "to": "JPY", "rate": rate,
                               "effective_on": "2026-01-01", "source": "test table" }))
                .await
                .assert_status_ok();
        }
        let answer: Value = request
            .post("/api/reports/run")
            .json(
                &json!({ "kind": "deal", "group_by": "state", "measure": "sum_amount",
                           "convert_to": "JPY", "rate_date": "2026-07-01" }),
            )
            .await
            .json();
        // ¥15,000 + ¥490 + ¥5,800; before CRM-T167 the dollars alone came to 1,500,000.
        assert_eq!(
            answer["rows"],
            json!([{ "group": "open", "currency": "JPY", "value": 15_000 + 490 + 5_800 }]),
            "{answer}"
        );
        // And back: ¥ to dollars through the inverse of the same rate.
        let back: Value = request
            .post("/api/reports/run")
            .json(
                &json!({ "kind": "deal", "group_by": "state", "measure": "sum_amount",
                           "convert_to": "USD", "rate_date": "2026-07-01" }),
            )
            .await
            .json();
        // USD 100.00 stays; KWD has no USD rate and CLF none either: both left out, not at one.
        assert_eq!(back["rows"][0]["value"], 10_000, "{back}");
        assert_eq!(back["unconverted"].as_array().unwrap().len(), 2, "{back}");
    })
    .await;
}
