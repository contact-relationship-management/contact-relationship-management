//! The public knowledge base and its deflection report (CRM-R57): off until it is turned on,
//! published articles only, counted without identifying anyone, rate limited per client.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use crate::scoped_auth::scoped;

macro_rules! article {
    ($request:expr, $title:expr, $body:expr, $to:expr) => {{
        let made: Value = $request
            .post("/api/articles")
            .json(&json!({ "title": $title, "body": $body, "keywords": "help" }))
            .await
            .json();
        let pid = made["pid"].as_str().unwrap().to_string();
        for to in $to {
            $request
                .post(&format!("/api/articles/{pid}/status"))
                .json(&json!({ "to": to }))
                .await
                .assert_status_ok();
        }
        pid
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn nothing_is_public_until_the_knowledge_base_is_turned_on() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = article!(&request, "Hidden", "Not for the public yet", ["published"]);
        for path in ["/kb/articles".to_string(), format!("/kb/articles/{pid}")] {
            request
                .get(&path)
                .await
                .assert_status(axum::http::StatusCode::NOT_FOUND);
        }
        request
            .post(&format!("/kb/articles/{pid}/feedback"))
            .json(&json!({ "helpful": true }))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn published_articles_are_public_and_what_happens_is_counted_without_naming_anyone() {
    let _scope = scoped(|s| {
        s.public_kb = true;
        s.public_kb_client_limit = 0;
    });
    request::<App, _, _>(|request, _ctx| async move {
        let reset = article!(
            &request,
            "Reset your password",
            "Open Settings, choose Security, then Reset password. ".repeat(10),
            ["published"]
        );
        let draft = article!(
            &request,
            "Draft article",
            "Still being written",
            [] as [&str; 0]
        );
        let archived = article!(
            &request,
            "Old article",
            "Out of date",
            ["published", "archived"]
        );

        // The list holds the published article only, with the start of its text.
        let all: Value = request.get("/kb/articles").await.json();
        let all = all.as_array().unwrap();
        assert_eq!(all.len(), 1, "{all:?}");
        assert_eq!(all[0]["title"], "Reset your password");
        assert!(all[0]["snippet"].as_str().unwrap().ends_with('…'));
        assert!(all[0].get("body").is_none());

        // A search finds it by its text, and one that finds nothing is counted as such.
        let found: Value = request.get("/kb/articles?q=security").await.json();
        assert_eq!(found.as_array().unwrap().len(), 1);
        let missed: Value = request.get("/kb/articles?q=zzzz").await.json();
        assert!(missed.as_array().unwrap().is_empty());

        // One article reads in full; a draft, an archived and a missing one are simply not found.
        let one = request.get(&format!("/kb/articles/{reset}")).await;
        one.assert_status_ok();
        assert!(
            one.json::<Value>()["body"]
                .as_str()
                .unwrap()
                .contains("Security")
        );
        for hidden in [
            draft.as_str(),
            archived.as_str(),
            "00000000-0000-0000-0000-000000000001",
        ] {
            request
                .get(&format!("/kb/articles/{hidden}"))
                .await
                .assert_status(axum::http::StatusCode::NOT_FOUND);
        }
        request
            .get("/kb/articles/not-a-uuid")
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);

        // Votes: two helped, one did not; none for a draft; an unknown member is refused.
        for helpful in [true, true, false] {
            request
                .post(&format!("/kb/articles/{reset}/feedback"))
                .json(&json!({ "helpful": helpful }))
                .await
                .assert_status(axum::http::StatusCode::ACCEPTED);
        }
        request
            .post(&format!("/kb/articles/{draft}/feedback"))
            .json(&json!({ "helpful": true }))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        request
            .post(&format!("/kb/articles/{reset}/feedback"))
            .json(&json!({ "helpful": true, "email": "someone@example.test" }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

        // One ticket was raised.
        request
            .post("/api/tickets")
            .json(&json!({ "title": "Still stuck", "priority": "normal" }))
            .await
            .assert_status_ok();

        let report: Value = request.get("/api/insights/kb-deflection").await.json();
        assert_eq!(report["views"], 1, "{report}");
        assert_eq!(report["searches"], 2, "{report}");
        assert_eq!(report["searches_without_result"], 1, "{report}");
        assert_eq!(report["helpful_yes"], 2, "{report}");
        assert_eq!(report["helpful_no"], 1, "{report}");
        assert_eq!(report["tickets_opened"], 1, "{report}");
        assert_eq!(report["deflection"]["numerator"], 2, "{report}");
        assert_eq!(report["deflection"]["denominator"], 3, "{report}");
        assert_eq!(
            report["top_articles"][0]["title"], "Reset your password",
            "{report}"
        );
        assert_eq!(report["top_articles"][0]["helpful_yes"], 2, "{report}");
        assert_eq!(
            report["top_articles"].as_array().unwrap().len(),
            1,
            "{report}"
        );

        for days in ["0", "366", "-1"] {
            let refused = request
                .get(&format!("/api/insights/kb-deflection?days={days}"))
                .await;
            refused.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(refused.json::<Value>()["errors"][0]["pointer"], "/days");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_client_over_its_limit_is_told_to_wait() {
    let _scope = scoped(|s| {
        s.public_kb = true;
        s.public_kb_client_limit = 3;
    });
    request::<App, _, _>(|request, _ctx| async move {
        for _ in 0..3 {
            request.get("/kb/articles").await.assert_status_ok();
        }
        let over = request.get("/kb/articles").await;
        over.assert_status(axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert!(over.headers().get("retry-after").is_some());
    })
    .await;
}
