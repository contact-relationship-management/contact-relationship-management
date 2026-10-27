//! Exchange rates and converted reports (CRM-R50): setting and replacing rates, what is refused,
//! which rate a date picks, and what a report says it used and could not convert.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_pipeline;

macro_rules! set_rate {
    ($request:expr, $from:expr, $to:expr, $rate:expr, $on:expr) => {{
        let r = $request
            .put("/api/fx-rates")
            .json(&json!({ "from": $from, "to": $to, "rate": $rate, "effective_on": $on,
                           "source": "test table" }))
            .await;
        r.assert_status_ok();
        r.json::<Value>()
    }};
}

async fn audited(db: &sea_orm::DatabaseConnection, action: &str) -> i64 {
    db.query_one_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT count(*) AS n FROM audit_logs WHERE entity = 'fx_rate' AND action = $1",
        [action.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn rates_are_set_replaced_listed_deleted_and_audited() {
    request::<App, _, _>(|request, ctx| async move {
        let made = set_rate!(&request, "USD", "GBP", "0.8", "2026-01-01");
        assert_eq!(made["rate"], "0.8");
        assert_eq!(made["from"], "USD");
        assert_eq!(made["source"], "test table");
        // The same pair and day replaces; another day is another rate.
        let again = set_rate!(&request, "USD", "GBP", "0.8100", "2026-01-01");
        assert_eq!(again["pid"], made["pid"]);
        assert_eq!(again["rate"], "0.81", "trailing zeros are not kept");
        let _ = set_rate!(&request, "USD", "GBP", "0.9", "2026-06-01");
        let listed: Vec<Value> = request.get("/api/fx-rates").await.json();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0]["effective_on"], "2026-06-01", "newest first");

        request
            .delete(&format!("/api/fx-rates/{}", made["pid"].as_str().unwrap()))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/fx-rates/{}", made["pid"].as_str().unwrap()))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        let left: Vec<Value> = request.get("/api/fx-rates").await.json();
        assert_eq!(left.len(), 1);
        assert_eq!(audited(&ctx.db, "set").await, 3);
        assert_eq!(audited(&ctx.db, "deleted").await, 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_rate_that_cannot_be_recorded_is_refused_naming_the_member() {
    request::<App, _, _>(|request, _ctx| async move {
        let good = json!({ "from": "USD", "to": "GBP", "rate": "0.8",
                           "effective_on": "2026-01-01", "source": "test table" });
        for (change, pointer) in [
            (json!({ "from": "usd" }), "/from"),
            (json!({ "to": "GB" }), "/to"),
            (json!({ "to": "USD" }), "/to"),
            (json!({ "rate": "0" }), "/rate"),
            (json!({ "rate": "-1" }), "/rate"),
            (json!({ "rate": "1.0000000001" }), "/rate"),
            (json!({ "rate": "1000000" }), "/rate"),
            (json!({ "rate": "abc" }), "/rate"),
            (json!({ "source": "  " }), "/source"),
        ] {
            let mut body = good.clone();
            for (k, v) in change.as_object().unwrap() {
                body[k] = v.clone();
            }
            let r = request.put("/api/fx-rates").json(&body).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let answer: Value = r.json();
            assert_eq!(
                answer["errors"][0]["pointer"], pointer,
                "{change}: {answer}"
            );
        }
        // A member that does not exist.
        let mut extra = good.clone();
        extra["currency_pair"] = json!("USDGBP");
        request
            .put("/api/fx-rates")
            .json(&extra)
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        request
            .put("/api/fx-rates")
            .json(&good)
            .await
            .assert_status_ok();
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_report_shows_its_sums_in_one_currency_naming_the_rates_and_what_it_could_not_convert() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, _stages) = seed_pipeline!(&request).await;
        for (name, amount, currency) in [
            ("A", 10_000, "GBP"),
            ("B", 5_000, "USD"),
            ("C", 2_000, "EUR"),
            ("D", 700, "JPY"),
        ] {
            request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline,
                               "amount_minor": amount, "currency": currency }))
                .await
                .assert_status_ok();
        }
        // USD to GBP changed in June; the EUR rate is only recorded the other way round.
        let _ = set_rate!(&request, "USD", "GBP", "0.80", "2026-01-01");
        let _ = set_rate!(&request, "USD", "GBP", "0.90", "2026-06-01");
        let _ = set_rate!(&request, "GBP", "EUR", "1.25", "2026-01-01");

        let run = |extra: Value| {
            let request = &request;
            async move {
                let mut spec = json!({ "kind": "deal", "group_by": "state",
                                       "measure": "sum_amount", "convert_to": "GBP" });
                for (k, v) in extra.as_object().unwrap() {
                    spec[k] = v.clone();
                }
                request.post("/api/reports/run").json(&spec).await
            }
        };

        // On 1 July: USD at 0.90 is 4,500; EUR is the inverse of 1.25, 2,000 / 1.25 = 1,600.
        let r = run(json!({ "rate_date": "2026-07-01" })).await;
        r.assert_status_ok();
        let answer: Value = r.json();
        assert_eq!(answer["rate_date"], "2026-07-01");
        assert_eq!(
            answer["rows"],
            json!([{ "group": "open", "currency": "GBP", "value": 10_000 + 4_500 + 1_600 }])
        );
        let rates = answer["rates"].as_array().unwrap();
        assert_eq!(rates.len(), 2);
        let usd = rates.iter().find(|r| r["from"] == "USD").unwrap();
        assert_eq!(
            (
                usd["rate"].as_str(),
                usd["effective_on"].as_str(),
                usd["inverted"].as_bool()
            ),
            (Some("0.9"), Some("2026-06-01"), Some(false))
        );
        let eur = rates.iter().find(|r| r["from"] == "EUR").unwrap();
        assert_eq!(
            (eur["rate"].as_str(), eur["inverted"].as_bool()),
            (Some("1.25"), Some(true))
        );
        assert_eq!(eur["source"], "test table");
        // The yen have no rate: left out and counted, not added at 1.0.
        assert_eq!(
            answer["unconverted"],
            json!([{ "currency": "JPY", "groups": 1, "amount_minor": 700 }])
        );

        // The earlier date picks the earlier USD rate, and no EUR rate is yet in force.
        let early: Value = run(json!({ "rate_date": "2026-03-01" })).await.json();
        assert_eq!(early["rows"][0]["value"], 10_000 + 4_000 + 1_600);
        // Before any rate, only the pounds convert, and both other currencies are left out.
        let none: Value = run(json!({ "rate_date": "2025-12-31" })).await.json();
        assert_eq!(none["rows"][0]["value"], 10_000);
        assert_eq!(none["unconverted"].as_array().unwrap().len(), 3);

        // With no date, today is used; a report in another currency is just another conversion.
        let today = chrono::Utc::now().date_naive().to_string();
        let now: Value = run(json!({})).await.json();
        assert_eq!(now["rate_date"], today);
        let usd_view: Value = run(json!({ "convert_to": "USD", "rate_date": "2026-07-01" }))
            .await
            .json();
        // 10,000 GBP back to USD is the inverse of 0.90; the pounds are not rounded on the way.
        assert_eq!(usd_view["rows"][0]["currency"], "USD");
        assert_eq!(usd_view["rows"][0]["value"], 5_000 + 11_111);

        // Without converting nothing changes: per currency rows, and no rates listed.
        let plain: Value = request
            .post("/api/reports/run")
            .json(&json!({ "kind": "deal", "group_by": "state", "measure": "sum_amount" }))
            .await
            .json();
        assert_eq!(plain["rows"].as_array().unwrap().len(), 4);
        assert!(plain["rates"].as_array().unwrap().is_empty());
        assert!(plain["rate_date"].is_null());

        // Refusals: a future day, a count, a lower-case currency, a date for nothing.
        let tomorrow = (chrono::Utc::now() + chrono::Duration::days(1))
            .date_naive()
            .to_string();
        for (extra, pointer) in [
            (json!({ "rate_date": tomorrow }), "/spec/rate_date"),
            (json!({ "measure": "count" }), "/spec/convert_to"),
            (json!({ "convert_to": "gbp" }), "/spec/convert_to"),
        ] {
            let r = run(extra.clone()).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let answer: Value = r.json();
            assert_eq!(answer["errors"][0]["pointer"], pointer, "{extra}: {answer}");
        }
        let r = request
            .post("/api/reports/run")
            .json(
                &json!({ "kind": "deal", "group_by": "state", "measure": "sum_amount",
                           "rate_date": "2026-07-01" }),
            )
            .await;
        r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(r.json::<Value>()["errors"][0]["pointer"], "/spec/rate_date");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_saved_report_keeps_its_conversion() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, _stages) = seed_pipeline!(&request).await;
        request
            .post("/api/deals")
            .json(&json!({ "name": "U", "pipeline_pid": pipeline,
                           "amount_minor": 1_000, "currency": "USD" }))
            .await
            .assert_status_ok();
        let _ = set_rate!(&request, "USD", "GBP", "0.5", "2026-01-01");
        let saved = request
            .post("/api/reports")
            .json(&json!({ "name": "In pounds",
                           "spec": { "kind": "deal", "group_by": "state", "measure": "sum_amount",
                                     "convert_to": "GBP", "rate_date": "2026-02-01" } }))
            .await;
        saved.assert_status_ok();
        let pid = saved.json::<Value>()["pid"].as_str().unwrap().to_string();
        let ran: Value = request.get(&format!("/api/reports/{pid}/run")).await.json();
        assert_eq!(ran["rows"][0]["value"], 500);
        assert_eq!(ran["spec"]["convert_to"], "GBP");
        assert_eq!(ran["rate_date"], "2026-02-01");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_forecast_adds_a_total_in_one_currency_and_names_what_it_left_out() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, _stages) = seed_pipeline!(&request).await;
        for (name, amount, currency) in [
            ("A", 1_000_000, "GBP"),
            ("B", 500_000, "USD"),
            ("C", 70_000, "JPY"),
        ] {
            request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline,
                               "amount_minor": amount, "currency": currency }))
                .await
                .assert_status_ok();
        }
        let _ = set_rate!(&request, "USD", "GBP", "0.80", "2026-01-01");

        let plain: Value = request.get("/api/forecast").await.json();
        let gbp = plain["totals_minor"]["GBP"].as_i64().unwrap();
        let usd = plain["totals_minor"]["USD"].as_i64().unwrap();
        assert!(
            gbp > 0 && usd > 0,
            "the first stage must weigh something: {plain}"
        );
        assert!(
            plain["converted_minor"].is_null() && plain["rates"].as_array().unwrap().is_empty()
        );

        let done: Value = request.get("/api/forecast?convert_to=GBP").await.json();
        // Per-currency totals are still there; the converted total adds the dollars at 0.80.
        assert_eq!(done["totals_minor"], plain["totals_minor"], "{done}");
        assert_eq!(done["converted_minor"], gbp + usd * 80 / 100, "{done}");
        assert_eq!(done["rates"][0]["from"], "USD", "{done}");
        assert_eq!(done["unconverted"][0]["currency"], "JPY", "{done}");

        for query in [
            "convert_to=gbp",
            "rate_date=2026-01-01",
            "convert_to=GBP&rate_date=2999-01-01",
        ] {
            let r = request.get(&format!("/api/forecast?{query}")).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_accounts_clv_adds_a_total_in_one_currency_and_names_what_it_left_out() {
    request::<App, _, _>(|request, _ctx| async move {
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Mixed Money Ltd" }),
            )
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let (pipeline, stages) = seed_pipeline!(&request).await;
        for (name, amount, currency) in
            [("A", 10_000, "GBP"), ("B", 5_000, "USD"), ("C", 700, "JPY")]
        {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": currency, "account_pid": account }),
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
        let _ = set_rate!(&request, "USD", "GBP", "0.80", "2026-01-01");

        let plain: Value = request
            .get(&format!("/api/accounts/{account}/clv"))
            .await
            .json();
        assert_eq!(plain["clv_minor"]["USD"], 5_000, "{plain}");
        assert!(
            plain["converted_minor"].is_null() && plain["rates"].as_array().unwrap().is_empty()
        );

        let done: Value = request
            .get(&format!("/api/accounts/{account}/clv?convert_to=GBP"))
            .await
            .json();
        assert_eq!(done["clv_minor"], plain["clv_minor"], "{done}");
        assert_eq!(done["converted_minor"], 14_000, "{done}");
        assert_eq!(done["rates"][0]["from"], "USD", "{done}");
        assert_eq!(done["unconverted"][0]["currency"], "JPY", "{done}");
        assert_eq!(done["unconverted"][0]["amount_minor"], 700, "{done}");

        for query in [
            "convert_to=gbp",
            "rate_date=2026-01-01",
            "convert_to=GBP&rate_date=2999-01-01",
        ] {
            request
                .get(&format!("/api/accounts/{account}/clv?{query}"))
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_accounts_clv_shows_its_margin_and_a_discounted_look_ahead_per_currency() {
    request::<App, _, _>(|request, _ctx| async move {
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Margin Ltd" }),
            )
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let (pipeline, stages) = seed_pipeline!(&request).await;
        for (name, amount, currency) in [("A", 10_000, "GBP"), ("B", 1_000, "USD")] {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": currency, "account_pid": account }),
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
        let url = format!("/api/accounts/{account}/clv");

        let plain: Value = request.get(&url).await.json();
        assert!(
            plain["margin_minor"].is_null() && plain["projection"].is_null(),
            "{plain}"
        );

        let margin: Value = request.get(&format!("{url}?margin_bps=2500")).await.json();
        assert_eq!(margin["margin_minor"]["GBP"], 2_500, "{margin}");
        assert_eq!(margin["margin_minor"]["USD"], 250, "{margin}");
        assert_eq!(margin["clv_minor"], plain["clv_minor"], "{margin}");

        // 50% retention over 3 years, no discount: 5000 + 2500 + 1250 on a 10000 year.
        let ahead: Value = request
            .get(&format!(
                "{url}?margin_bps=10000&retention_bps=5000&years=3"
            ))
            .await
            .json();
        assert_eq!(
            ahead["projection"]["annual_base_minor"]["GBP"], 10_000,
            "{ahead}"
        );
        assert_eq!(
            ahead["projection"]["projected_minor"]["GBP"], 8_750,
            "{ahead}"
        );
        assert_eq!(ahead["projection"]["total_minor"]["GBP"], 18_750, "{ahead}");
        assert_eq!(
            ahead["projection"]["projected_minor"]["USD"], 875,
            "{ahead}"
        );

        for query in [
            "margin_bps=10001",
            "years=3",
            "retention_bps=5000",
            "retention_bps=5000&years=21",
            "retention_bps=5000&years=0",
            "discount_bps=500",
        ] {
            request
                .get(&format!("{url}?{query}"))
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_accounts_clv_can_take_its_retention_from_the_year_before() {
    request::<App, _, _>(|request, ctx| async move {
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "History Ltd" }),
            )
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let (pipeline, stages) = seed_pipeline!(&request).await;
        // GBP: 8000 won 400 days ago, 6000 won now. USD: only 1000 won now.
        for (name, amount, currency, days_ago) in [
            ("old", 8_000, "GBP", 400),
            ("new", 6_000, "GBP", 0),
            ("usd", 1_000, "USD", 0),
        ] {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": currency, "account_pid": account }),
                )
                .await
                .json();
            let pid = deal["pid"].as_str().unwrap();
            request
                .post(&format!("/api/deals/{pid}/stage"))
                .json(&json!({ "stage_pid": stages[2] }))
                .await
                .assert_status_ok();
            ctx.db
                .execute_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE deals SET closed_at = now() - make_interval(days => $1) WHERE name = $2",
                    [days_ago.into(), name.into()],
                ))
                .await
                .unwrap();
        }
        let url = format!("/api/accounts/{account}/clv");
        // 75% retention, 2 years, no discount: 4500 + 3375 on a 6000 year.
        let done: Value = request
            .get(&format!("{url}?retention_from=history&years=2"))
            .await
            .json();
        let projection = &done["projection"];
        assert!(projection["retention_bps"].is_null(), "{done}");
        assert_eq!(projection["retention_history_bps"]["GBP"], 7_500, "{done}");
        assert!(projection["retention_history_bps"]["USD"].is_null(), "{done}");
        assert_eq!(projection["annual_base_minor"]["GBP"], 6_000, "{done}");
        assert_eq!(projection["projected_minor"]["GBP"], 7_875, "{done}");
        assert_eq!(projection["total_minor"]["GBP"], 21_875, "{done}");
        // No year before for USD: nothing projected, and what was won still counts.
        assert_eq!(projection["projected_minor"]["USD"], 0, "{done}");
        assert_eq!(projection["total_minor"]["USD"], 1_000, "{done}");

        for query in [
            "retention_from=history",
            "retention_from=forecast&years=2",
            "retention_from=history&retention_bps=5000&years=2",
        ] {
            request
                .get(&format!("{url}?{query}"))
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_accounts_clv_margin_and_look_ahead_are_converted_once_per_currency_at_the_same_rates() {
    request::<App, _, _>(|request, _ctx| async move {
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Converted Ltd" }),
            )
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let (pipeline, stages) = seed_pipeline!(&request).await;
        for (name, amount, currency) in
            [("A", 10_000, "GBP"), ("B", 5_000, "USD"), ("C", 700, "JPY")]
        {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": currency, "account_pid": account }),
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
        let _ = set_rate!(&request, "USD", "GBP", "0.80", "2026-01-01");
        let url = format!("/api/accounts/{account}/clv");

        // Without `convert_to` nothing is converted.
        let plain: Value = request
            .get(&format!("{url}?margin_bps=5000&retention_bps=5000&years=2"))
            .await
            .json();
        assert!(plain["margin_converted_minor"].is_null(), "{plain}");
        assert!(plain["projection"]["converted"].is_null(), "{plain}");

        // Margin 50%: GBP 5000 + USD 2500 (2000). Look-ahead at 50% over 2 years: GBP 3750 and
        // USD 1875 (1500) projected; the JPY is left out, as it is for the CLV.
        let done: Value = request
            .get(&format!(
                "{url}?convert_to=GBP&margin_bps=5000&retention_bps=5000&years=2"
            ))
            .await
            .json();
        assert_eq!(done["converted_minor"], 14_000, "{done}");
        assert_eq!(done["margin_converted_minor"], 7_000, "{done}");
        let converted = &done["projection"]["converted"];
        assert_eq!(converted["annual_base_minor"], 7_000, "{done}");
        assert_eq!(converted["projected_minor"], 5_250, "{done}");
        assert_eq!(converted["total_minor"], 12_250, "{done}");
        assert_eq!(done["unconverted"].as_array().unwrap().len(), 1, "{done}");
        assert_eq!(done["unconverted"][0]["amount_minor"], 700, "{done}");
        // Each currency's own figures are still shown, unconverted.
        assert_eq!(done["projection"]["total_minor"]["USD"], 4_375, "{done}");

        // Convert the margin alone; no look-ahead was asked for.
        let margin: Value = request
            .get(&format!("{url}?convert_to=GBP&margin_bps=5000"))
            .await
            .json();
        assert_eq!(margin["margin_converted_minor"], 7_000, "{margin}");
        assert!(margin["projection"].is_null(), "{margin}");
    })
    .await;
}
