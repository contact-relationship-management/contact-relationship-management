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
async fn an_attribution_that_cannot_be_asked_is_refused_naming_the_member() {
    request::<App, _, _>(|request, _ctx| async move {
        for (query, pointer) in [
            ("model=time_decay", "/model"),
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
