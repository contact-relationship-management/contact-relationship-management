//! The request limit through the booted app: the budget headers, the `429`,
//! and the probe exemption.
//!
//! Its own test binary because the limit is read from the environment when the
//! app boots. Run with `cargo test --test rate_limit -- --ignored`.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_limit_answers_429_with_the_budget_and_spares_probes() {
    // Safe here: the test binary is single-threaded at this point and nothing
    // else reads the variable before the app boots.
    unsafe { std::env::set_var("CRM_RATE_LIMIT_PER_MIN", "3") };
    request::<App, _, _>(|request, _ctx| async move {
        for remaining in ["2", "1", "0"] {
            let ok = request.get("/api/contacts").await;
            ok.assert_status_ok();
            assert_eq!(ok.headers()["ratelimit-limit"], "3");
            assert_eq!(ok.headers()["ratelimit-remaining"], remaining);
        }
        let refused = request.get("/api/contacts").await;
        refused.assert_status(axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert!(refused.headers().contains_key("retry-after"));
        assert_eq!(refused.headers()["cache-control"], "no-store");

        for _ in 0..5 {
            request.get("/_health").await.assert_status_ok();
        }
    })
    .await;
}
