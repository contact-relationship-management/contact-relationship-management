//! Multi-touch attribution (CRM-R49): the touches a campaign has with the buyer, the three
//! models, the window, what no campaign touched, and what is refused.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::{seed_contact, seed_pipeline};

/// A sent e-mail from `campaign` to `contact`, `days_ago` days before now.
async fn sent(
    db: &sea_orm::DatabaseConnection,
    campaign: &str,
    contact: &str,
    state: &str,
    days_ago: i64,
) {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO email_deliveries (pid, campaign_pid, contact_pid, state, sent_at) \
         VALUES (gen_random_uuid(), $1::uuid, $2::uuid, $3, now() - make_interval(days => $4::int))",
        [campaign.into(), contact.into(), state.into(), (days_ago as i32).into()],
    ))
    .await
    .unwrap();
}

/// What a campaign earned in `answer`, in the currency, or zero.
fn earned(answer: &Value, name: &str, currency: &str) -> i64 {
    answer["campaigns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .and_then(|c| {
            c["credit"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["currency"] == currency)
        })
        .map_or(0, |a| a["amount_minor"].as_i64().unwrap())
}

macro_rules! campaign {
    ($request:expr, $segment:expr, $name:expr) => {
        $request
            .post("/api/campaigns")
            .json(&json!({ "name": $name, "segment_pid": $segment, "subject": "Hi", "body": "Hi" }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string()
    };
}

macro_rules! won_deal {
    ($request:expr, $pipeline:expr, $stages:expr, $payload:expr) => {{
        let mut body = $payload;
        body["pipeline_pid"] = json!($pipeline);
        let deal = $request.post("/api/deals").json(&body).await;
        deal.assert_status_ok();
        let pid = deal.json::<Value>()["pid"].as_str().unwrap().to_string();
        $request
            .post(&format!("/api/deals/{pid}/stage"))
            .json(&json!({ "stage_pid": $stages[2] }))
            .await
            .assert_status_ok();
        pid
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn won_revenue_is_shared_among_the_campaigns_that_touched_the_buyer() {
    request::<App, _, _>(|request, ctx| async move {
        let segment = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let alpha = campaign!(&request, segment, "Alpha");
        let beta = campaign!(&request, segment, "Beta");
        let gamma = campaign!(&request, segment, "Gamma");
        let delta = campaign!(&request, segment, "Delta");
        let epsilon = campaign!(&request, segment, "Epsilon");
        let buyer = seed_contact!(&request, "Attribution Buyer").await;
        // Alpha's e-mail and Beta's reach the buyer in the window; Gamma's long before it. A
        // failed send (Delta) and one that has not happened yet (Epsilon) are not touches.
        sent(&ctx.db, &alpha, &buyer, "sent", 30).await;
        sent(&ctx.db, &beta, &buyer, "sent", 10).await;
        sent(&ctx.db, &gamma, &buyer, "sent", 200).await;
        sent(&ctx.db, &delta, &buyer, "failed", 2).await;
        sent(&ctx.db, &epsilon, &buyer, "sent", -1).await;

        let (pipeline, stages) = seed_pipeline!(&request).await;
        won_deal!(
            &request,
            pipeline,
            stages,
            json!({ "name": "Bought", "amount_minor": 10_001, "currency": "GBP",
                    "primary_contact_pid": buyer, "source_campaign_pid": alpha })
        );
        // A deal nobody touched, a deal with only its recorded source, and one still open.
        won_deal!(
            &request,
            pipeline,
            stages,
            json!({ "name": "Walk-in", "amount_minor": 5_000, "currency": "USD" })
        );
        won_deal!(
            &request,
            pipeline,
            stages,
            json!({ "name": "Sourced", "amount_minor": 3_000, "currency": "GBP",
                    "source_campaign_pid": beta })
        );
        request
            .post("/api/deals")
            .json(
                &json!({ "name": "Open", "pipeline_pid": pipeline, "amount_minor": 99,
                           "currency": "GBP", "primary_contact_pid": buyer }),
            )
            .await
            .assert_status_ok();

        // Linear: 10,001 over three touches (Alpha's e-mail, Beta's, and Alpha as the recorded
        // source) is 3,334 + 3,334 + 3,333, the extra units to the earliest touches; Alpha holds
        // the first and the last. Beta also has the 3,000 deal it is the source of.
        let linear: Value = request.get("/api/attribution").await.json();
        assert_eq!(linear["model"], "linear");
        assert_eq!(linear["window_days"], 90);
        assert_eq!(linear["deals"], 3);
        assert_eq!(earned(&linear, "Alpha", "GBP"), 6_667);
        assert_eq!(earned(&linear, "Beta", "GBP"), 3_334 + 3_000);
        assert_eq!(earned(&linear, "Gamma", "GBP"), 0, "outside the window");
        // Every pound is accounted for, and the untouched deal is kept apart, per currency.
        assert_eq!(
            earned(&linear, "Alpha", "GBP") + earned(&linear, "Beta", "GBP"),
            13_001
        );
        assert_eq!(linear["unattributed_deals"], 1);
        assert_eq!(
            linear["unattributed"],
            json!([{ "currency": "USD", "amount_minor": 5_000 }])
        );
        assert_eq!(linear["truncated"], false);
        // Alpha touched one deal, twice; Beta two deals.
        let alpha_row = linear["campaigns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "Alpha")
            .unwrap();
        assert_eq!(
            (alpha_row["touches"].as_i64(), alpha_row["deals"].as_i64()),
            (Some(2), Some(1))
        );
        assert_eq!(
            linear["campaigns"][0]["name"], "Beta",
            "most deals touched first"
        );

        // First touch: Alpha's e-mail, 30 days ago. Last touch: Alpha as the source, at creation.
        for model in ["first", "last"] {
            let answer: Value = request
                .get(&format!("/api/attribution?model={model}"))
                .await
                .json();
            assert_eq!(earned(&answer, "Alpha", "GBP"), 10_001, "{model}");
            assert_eq!(earned(&answer, "Beta", "GBP"), 3_000, "{model}");
        }

        // A short window drops both e-mails but keeps the recorded source, which is exempt; a long one reaches Gamma, which is
        // then the first touch.
        let short: Value = request
            .get("/api/attribution?window_days=7&model=first")
            .await
            .json();
        assert_eq!(earned(&short, "Alpha", "GBP"), 10_001);
        let long: Value = request
            .get("/api/attribution?window_days=365&model=first")
            .await
            .json();
        assert_eq!(earned(&long, "Gamma", "GBP"), 10_001);
        assert_eq!(earned(&long, "Alpha", "GBP"), 0);

        // The date range is on the day a deal closed: all of them closed today.
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1)).date_naive();
        let none: Value = request
            .get(&format!("/api/attribution?to={yesterday}"))
            .await
            .json();
        assert_eq!(none["deals"], 0);
        assert!(none["campaigns"].as_array().unwrap().is_empty());
        let today = chrono::Utc::now().date_naive();
        let all: Value = request
            .get(&format!("/api/attribution?from={today}&to={today}"))
            .await
            .json();
        assert_eq!(all["deals"], 3);

        // Asking is audited with the model and window, not the money.
        let logged: i64 = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM audit_logs WHERE action = 'attribution_run'",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "n")
            .unwrap();
        assert!(logged >= 6);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn time_decay_favours_the_recent_touch_and_position_splits_the_ends_evenly() {
    request::<App, _, _>(|request, ctx| async move {
        let segment = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let early = campaign!(&request, segment, "Early");
        let middle = campaign!(&request, segment, "Middle");
        let late = campaign!(&request, segment, "Late");
        let buyer = seed_contact!(&request, "Decay Buyer").await;
        sent(&ctx.db, &early, &buyer, "sent", 40).await;
        sent(&ctx.db, &middle, &buyer, "sent", 20).await;
        sent(&ctx.db, &late, &buyer, "sent", 1).await;
        let (pipeline, stages) = seed_pipeline!(&request).await;
        won_deal!(
            &request,
            pipeline,
            stages,
            json!({ "name": "Bought", "amount_minor": 100_000, "currency": "GBP",
                    "primary_contact_pid": buyer })
        );

        let ask = |model: &'static str| {
            let request = &request;
            async move {
                request
                    .get(&format!("/api/attribution?model={model}"))
                    .await
                    .json::<Value>()
            }
        };
        let position = ask("position").await;
        assert_eq!(position["model"], "position");
        assert_eq!(earned(&position, "Early", "GBP"), 40_000, "{position}");
        assert_eq!(earned(&position, "Middle", "GBP"), 20_000, "{position}");
        assert_eq!(earned(&position, "Late", "GBP"), 40_000, "{position}");
        assert_eq!(position["end_percent"], 40, "{position}");

        // A chosen end share: 25% each end leaves the middle half.
        let chosen: Value = request
            .get("/api/attribution?model=position&end_percent=25")
            .await
            .json();
        assert_eq!(earned(&chosen, "Early", "GBP"), 25_000, "{chosen}");
        assert_eq!(earned(&chosen, "Middle", "GBP"), 50_000, "{chosen}");
        assert_eq!(earned(&chosen, "Late", "GBP"), 25_000, "{chosen}");
        assert_eq!(chosen["end_percent"], 25, "{chosen}");
        let linear: Value = request.get("/api/attribution").await.json();
        assert!(linear["end_percent"].is_null(), "{linear}");
        for query in [
            "end_percent=25",
            "model=position&end_percent=0",
            "model=position&end_percent=50",
        ] {
            let r = request.get(&format!("/api/attribution?{query}")).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let refused: Value = r.json();
            assert_eq!(
                refused["errors"][0]["pointer"], "/end_percent",
                "{query}: {refused}"
            );
        }

        let decay = ask("time_decay").await;
        let (e, m, l) = (
            earned(&decay, "Early", "GBP"),
            earned(&decay, "Middle", "GBP"),
            earned(&decay, "Late", "GBP"),
        );
        assert!(l > m && m > e && e > 0, "recent earns more: {decay}");
        assert_eq!(e + m + l, 100_000, "every pound is accounted for: {decay}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_attribution_that_cannot_be_asked_is_refused_naming_the_member() {
    request::<App, _, _>(|request, _ctx| async move {
        for (query, pointer) in [
            ("model=u_shaped", "/model"),
            ("window_days=0", "/window_days"),
            ("window_days=366", "/window_days"),
            ("from=2026-06-02&to=2026-06-01", "/from"),
        ] {
            let r = request.get(&format!("/api/attribution?{query}")).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let answer: Value = r.json();
            assert_eq!(answer["errors"][0]["pointer"], pointer, "{query}: {answer}");
        }
        request
            .get("/api/attribution?window_days=1&model=first&from=2026-06-01&to=2026-06-01")
            .await
            .assert_status_ok();
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn attribution_shows_its_credit_in_one_currency_naming_the_rates_and_what_it_left_out() {
    request::<App, _, _>(|request, _ctx| async move {
        let segment = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let alpha = campaign!(&request, segment, "Alpha");
        let beta = campaign!(&request, segment, "Beta");
        let (pipeline, stages) = seed_pipeline!(&request).await;
        // Alpha's source brings in pounds and dollars, Beta's yen (no rate), and one deal has
        // no touch at all.
        for (name, amount, currency, source) in [
            ("A1", 10_000, "GBP", Some(&alpha)),
            ("A2", 5_000, "USD", Some(&alpha)),
            ("B1", 700, "JPY", Some(&beta)),
            ("W1", 2_000, "USD", None),
        ] {
            let mut body = json!({ "name": name, "amount_minor": amount, "currency": currency });
            if let Some(campaign) = source {
                body["source_campaign_pid"] = json!(campaign);
            }
            won_deal!(&request, pipeline, stages, body);
        }
        request
            .put("/api/fx-rates")
            .json(&json!({ "from": "USD", "to": "GBP", "rate": "0.80",
                           "effective_on": "2026-01-01", "source": "test table" }))
            .await
            .assert_status_ok();

        let answer: Value = request.get("/api/attribution?convert_to=GBP").await.json();
        assert_eq!(answer["convert_to"], "GBP", "{answer}");
        assert_eq!(earned(&answer, "Alpha", "GBP"), 10_000 + 4_000, "{answer}");
        let beta_credit = answer["campaigns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "Beta")
            .unwrap()["credit"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(
            beta_credit, 0,
            "yen has no rate and is not taken at 1.0: {answer}"
        );
        assert_eq!(answer["unattributed"][0]["currency"], "GBP", "{answer}");
        assert_eq!(answer["unattributed"][0]["amount_minor"], 1_600, "{answer}");
        assert_eq!(answer["rates"].as_array().unwrap().len(), 1, "{answer}");
        assert_eq!(answer["rates"][0]["from"], "USD", "{answer}");
        assert_eq!(answer["unconverted"][0]["currency"], "JPY", "{answer}");
        assert_eq!(answer["unconverted"][0]["amount_minor"], 700, "{answer}");

        // Without it nothing changes: per currency, no rates.
        let plain: Value = request.get("/api/attribution").await.json();
        assert!(plain["convert_to"].is_null() && plain["rates"].as_array().unwrap().is_empty());
        assert_eq!(earned(&plain, "Alpha", "USD"), 5_000, "{plain}");

        for (query, pointer) in [
            ("convert_to=gbp", "/convert_to"),
            ("rate_date=2026-01-01", "/rate_date"),
            ("convert_to=GBP&rate_date=2999-01-01", "/rate_date"),
        ] {
            let r = request.get(&format!("/api/attribution?{query}")).await;
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
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn attribution_can_convert_each_deals_share_at_the_rate_of_the_day_it_closed() {
    request::<App, _, _>(|request, ctx| async move {
        let segment = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let alpha = campaign!(&request, segment, "Alpha");
        let beta = campaign!(&request, segment, "Beta");
        let (pipeline, stages) = seed_pipeline!(&request).await;
        // Alpha's dollars close either side of a rate change and once before any rate; Beta's yen
        // never have one; a deal no campaign touched closes after the change.
        for (name, amount, currency, source, closed) in [
            ("A1", 10_000, "USD", Some(&alpha), "2026-02-01"),
            ("A2", 10_000, "USD", Some(&alpha), "2026-07-01"),
            ("A3", 4_000, "USD", Some(&alpha), "2025-12-01"),
            ("B1", 700, "JPY", Some(&beta), "2026-02-01"),
            ("W1", 2_000, "USD", None, "2026-07-01"),
        ] {
            let mut body = json!({ "name": name, "amount_minor": amount, "currency": currency });
            if let Some(campaign) = source {
                body["source_campaign_pid"] = json!(campaign);
            }
            let pid = won_deal!(&request, pipeline, stages, body);
            ctx.db
                .execute_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    format!(
                        "UPDATE deals SET closed_at = '{closed}T09:00:00Z' WHERE pid = $1::uuid"
                    ),
                    [pid.as_str().into()],
                ))
                .await
                .unwrap();
        }
        for (rate, on) in [("0.80", "2026-01-01"), ("0.50", "2026-06-01")] {
            request
                .put("/api/fx-rates")
                .json(&json!({ "from": "USD", "to": "GBP", "rate": rate,
                               "effective_on": on, "source": "test table" }))
                .await
                .assert_status_ok();
        }

        // One date for everything: all 24,000 of Alpha's dollars at today's 0.50.
        let one: Value = request.get("/api/attribution?convert_to=GBP").await.json();
        assert_eq!(earned(&one, "Alpha", "GBP"), 12_000, "{one}");
        assert!(one["rate_basis"].is_null(), "{one}");

        // On the day each closed: 8,000 and 5,000 at their own rates; the deal before any rate and
        // the yen are left out and named; the untouched deal is 2,000 at 0.50.
        let dated: Value = request
            .get("/api/attribution?convert_to=GBP&rate_basis=close_day")
            .await
            .json();
        assert_eq!(earned(&dated, "Alpha", "GBP"), 13_000, "{dated}");
        assert_eq!(dated["unattributed"][0]["amount_minor"], 1_000, "{dated}");
        assert_eq!(dated["rate_basis"], "close_day", "{dated}");
        assert!(dated["rate_date"].is_null(), "{dated}");
        assert_eq!(dated["rates"].as_array().unwrap().len(), 2, "{dated}");
        let left: Vec<(String, i64)> = dated["unconverted"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| {
                (
                    u["currency"].as_str().unwrap().into(),
                    u["amount_minor"].as_i64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            left,
            [("JPY".to_string(), 700), ("USD".to_string(), 4_000)],
            "{dated}"
        );

        for (query, pointer) in [
            ("rate_basis=close_day", "/rate_basis"),
            ("convert_to=GBP&rate_basis=today", "/rate_basis"),
            (
                "convert_to=GBP&rate_basis=close_day&rate_date=2026-03-01",
                "/rate_date",
            ),
        ] {
            let r = request.get(&format!("/api/attribution?{query}")).await;
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

/// A sent e-mail that the provider reported opened `after_seconds` after it was sent.
async fn sent_and_opened(
    db: &sea_orm::DatabaseConnection,
    campaign: &str,
    contact: &str,
    days_ago: i32,
    after_seconds: i32,
) {
    sent_and_reported(db, campaign, contact, days_ago, "opened", after_seconds).await;
}

/// A sent e-mail that the provider reported `event` (`opened` or `clicked`) `after_seconds` after
/// it was sent.
async fn sent_and_reported(
    db: &sea_orm::DatabaseConnection,
    campaign: &str,
    contact: &str,
    days_ago: i32,
    event: &str,
    after_seconds: i32,
) {
    let message = format!("<{campaign}@test>");
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO email_deliveries (pid, campaign_pid, contact_pid, state, sent_at, message_id) \
         VALUES (gen_random_uuid(), $1::uuid, $2::uuid, 'sent', \
                 now() - make_interval(days => $3::int), $4)",
        [
            campaign.into(),
            contact.into(),
            days_ago.into(),
            message.clone().into(),
        ],
    ))
    .await
    .unwrap();
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO email_events (message_id, event, received_at) \
         SELECT message_id, $3, sent_at + make_interval(secs => $2::int) \
           FROM email_deliveries WHERE message_id = $1",
        [message.into(), after_seconds.into(), event.into()],
    ))
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_click_too_soon_after_sending_is_left_out_and_opens_are_not_touched() {
    request::<App, _, _>(|request, ctx| async move {
        let segment = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let alpha = campaign!(&request, segment, "Alpha");
        let beta = campaign!(&request, segment, "Beta");
        let buyer = seed_contact!(&request, "Click Buyer").await;
        // Alpha's link was "clicked" ten seconds after sending (a scanner following it); Beta's
        // an hour after.
        sent_and_reported(&ctx.db, &alpha, &buyer, 20, "clicked", 10).await;
        sent_and_reported(&ctx.db, &beta, &buyer, 10, "clicked", 3_600).await;
        let (pipeline, stages) = seed_pipeline!(&request).await;
        won_deal!(
            &request,
            pipeline,
            stages,
            json!({ "name": "Bought", "amount_minor": 10_000, "currency": "GBP",
                    "primary_contact_pid": buyer })
        );

        let both: Value = request.get("/api/attribution?touches=clicked").await.json();
        assert_eq!(earned(&both, "Alpha", "GBP"), 5_000, "{both}");
        assert_eq!(earned(&both, "Beta", "GBP"), 5_000, "{both}");
        assert!(both["min_click_seconds"].is_null(), "{both}");

        let late: Value = request
            .get("/api/attribution?touches=clicked&min_click_seconds=60")
            .await
            .json();
        assert_eq!(earned(&late, "Alpha", "GBP"), 0, "{late}");
        assert_eq!(earned(&late, "Beta", "GBP"), 10_000, "{late}");
        assert_eq!(late["min_click_seconds"], 60, "{late}");

        // The delay is for clicks: an open ten seconds after sending still counts, and sends do.
        let mixed: Value = request
            .get("/api/attribution?touches=sent,clicked&min_click_seconds=60")
            .await
            .json();
        assert!(earned(&mixed, "Alpha", "GBP") > 0, "{mixed}");
        let gamma = campaign!(&request, segment, "Gamma");
        sent_and_reported(&ctx.db, &gamma, &buyer, 5, "opened", 10).await;
        let opens: Value = request
            .get("/api/attribution?touches=opened,clicked&min_click_seconds=60")
            .await
            .json();
        assert!(earned(&opens, "Gamma", "GBP") > 0, "{opens}");

        for (query, pointer) in [
            ("min_click_seconds=60", "/min_click_seconds"),
            ("touches=opened&min_click_seconds=60", "/min_click_seconds"),
            ("touches=clicked&min_click_seconds=0", "/min_click_seconds"),
            (
                "touches=clicked&min_click_seconds=86401",
                "/min_click_seconds",
            ),
        ] {
            let r = request.get(&format!("/api/attribution?{query}")).await;
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
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_open_too_soon_after_sending_is_left_out_and_the_decay_half_life_can_be_chosen() {
    request::<App, _, _>(|request, ctx| async move {
        let segment = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let alpha = campaign!(&request, segment, "Alpha");
        let beta = campaign!(&request, segment, "Beta");
        let buyer = seed_contact!(&request, "Settings Buyer").await;
        // Alpha's e-mail was "opened" half a minute after it was sent (a mail program loading it
        // ahead of the reader); Beta's two hours after.
        sent_and_opened(&ctx.db, &alpha, &buyer, 20, 30).await;
        sent_and_opened(&ctx.db, &beta, &buyer, 10, 7_200).await;
        let (pipeline, stages) = seed_pipeline!(&request).await;
        won_deal!(
            &request,
            pipeline,
            stages,
            json!({ "name": "Bought", "amount_minor": 10_000, "currency": "GBP",
                    "primary_contact_pid": buyer })
        );

        // Opens alone, both counted: 5,000 each.
        let both: Value = request.get("/api/attribution?touches=opened").await.json();
        assert_eq!(earned(&both, "Alpha", "GBP"), 5_000, "{both}");
        assert_eq!(earned(&both, "Beta", "GBP"), 5_000, "{both}");
        assert!(both["min_open_seconds"].is_null(), "{both}");

        // Opens under a minute left out: Alpha's has no part, and the answer says so.
        let late: Value = request
            .get("/api/attribution?touches=opened&min_open_seconds=60")
            .await
            .json();
        assert_eq!(earned(&late, "Alpha", "GBP"), 0, "{late}");
        assert_eq!(earned(&late, "Beta", "GBP"), 10_000, "{late}");
        assert_eq!(late["min_open_seconds"], 60, "{late}");

        // Clicks and sends are not what the setting is about: with `sent` counted, Alpha still
        // has its send.
        let sends: Value = request
            .get("/api/attribution?touches=sent,opened&min_open_seconds=60")
            .await
            .json();
        assert!(earned(&sends, "Alpha", "GBP") > 0, "{sends}");

        // The half-life. Alpha's e-mail went 20 days ago and Beta's 10: at the default 7 days
        // Beta weighs about 2.7 times Alpha; with 90 days they weigh nearly the same.
        let share = |answer: &Value, name: &str| earned(answer, name, "GBP");
        let quick: Value = request
            .get("/api/attribution?model=time_decay&touches=sent")
            .await
            .json();
        let slow: Value = request
            .get("/api/attribution?model=time_decay&touches=sent&half_life_days=90")
            .await
            .json();
        assert_eq!(quick["half_life_days"], 7, "{quick}");
        assert_eq!(slow["half_life_days"], 90, "{slow}");
        assert_eq!(share(&quick, "Alpha") + share(&quick, "Beta"), 10_000);
        assert_eq!(share(&slow, "Alpha") + share(&slow, "Beta"), 10_000);
        assert!(share(&quick, "Beta") > 7_000, "{quick}");
        assert!((5_000..5_700).contains(&share(&slow, "Beta")), "{slow}");
        let linear: Value = request.get("/api/attribution?touches=sent").await.json();
        assert!(linear["half_life_days"].is_null(), "{linear}");

        for (query, pointer) in [
            ("half_life_days=14", "/half_life_days"),
            ("model=time_decay&half_life_days=3", "/half_life_days"),
            ("model=time_decay&half_life_days=91", "/half_life_days"),
            ("min_open_seconds=60", "/min_open_seconds"),
            ("touches=opened&min_open_seconds=0", "/min_open_seconds"),
            ("touches=opened&min_open_seconds=86401", "/min_open_seconds"),
        ] {
            let r = request.get(&format!("/api/attribution?{query}")).await;
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
