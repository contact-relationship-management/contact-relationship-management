//! The order of the router: the service's own layers inside `axa`'s stack (CRM-T217), through the
//! booted app. Outside it, a refusal had no request id, security headers or CORS, and the layers
//! read compressed bodies.
//!
//! Its own test binary because these settings are read from the environment when the app boots.
//! Run with `cargo test --test router_order -- --ignored`.

use axa::problem::PROBLEM_JSON;
use axa::testing::prelude::*;
use axum::http::{Method, StatusCode};
use contact_relationship_management::app::App;
use serial_test::serial;

const ORIGIN: &str = "https://crm.example.org";

/// What these tests vary. Safe: `#[serial]`, and set before the app boots.
fn boot_with(limit: &str, compression: &str) {
    unsafe {
        std::env::set_var("CRM_RATE_LIMIT_PER_MIN", limit);
        std::env::set_var("HTTP_COMPRESSION", compression);
        std::env::set_var("CORS_ALLOWED_ORIGINS", ORIGIN);
    }
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_refusal_from_the_abuse_guard_carries_the_standard_headers() {
    boot_with("1", "false");
    request::<App, _, _>(|request, _ctx| async move {
        request.get("/api/contacts").await.assert_status_ok();
        let refused = request
            .get("/api/contacts")
            .add_header("origin", ORIGIN)
            .await;
        refused.assert_status(StatusCode::TOO_MANY_REQUESTS);
        let headers = refused.headers();
        assert!(headers.contains_key("x-request-id"));
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(headers["x-frame-options"], "DENY");
        assert_eq!(headers["referrer-policy"], "no-referrer");
        assert_eq!(headers["access-control-allow-origin"], ORIGIN);
        assert_eq!(headers["content-type"], PROBLEM_JSON);
        assert_eq!(headers["cache-control"], "no-store");
        assert!(headers.contains_key("retry-after"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_preflight_is_answered_even_when_the_client_is_over_its_limit() {
    boot_with("1", "false");
    request::<App, _, _>(|request, _ctx| async move {
        request.get("/api/contacts").await.assert_status_ok();
        let preflight = request
            .method(Method::OPTIONS, "/api/contacts")
            .add_header("origin", ORIGIN)
            .add_header("access-control-request-method", "POST")
            .add_header("access-control-request-headers", "authorization")
            .await;
        preflight.assert_status_ok();
        let headers = preflight.headers();
        assert_eq!(headers["access-control-allow-origin"], ORIGIN);
        assert!(
            headers["access-control-allow-methods"]
                .to_str()
                .unwrap()
                .contains("POST")
        );
        assert_eq!(headers["access-control-allow-headers"], "authorization");
        assert!(headers.contains_key("x-request-id"));

        let foreign = request
            .method(Method::OPTIONS, "/api/contacts")
            .add_header("origin", "https://evil.example")
            .add_header("access-control-request-method", "POST")
            .await;
        assert!(
            foreign
                .maybe_header("access-control-allow-origin")
                .is_none()
        );
    })
    .await;
}

/// A real error from a controller, long enough to compress. Outside compression `problem_mw` read
/// gzip as if it were the error: the client lost the error's code and text, and got the rewritten
/// document in the clear under `Content-Encoding: gzip`, which it cannot read. The test has no
/// decoder, so it asks twice: the document in the clear when not asking for gzip, and bytes that
/// are really gzip, under the problem media type, when asking.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_compressed_error_is_a_problem_document_compressed_once() {
    boot_with("0", "true");
    request::<App, _, _>(|request, _ctx| async move {
        let path = "/api/contacts/00000000-0000-4000-8000-000000000000";
        let refused =
            |encoding: &'static str| request.get(path).add_header("accept-encoding", encoding);
        let plain = refused("identity").await;
        plain.assert_status(StatusCode::NOT_FOUND);
        assert_eq!(plain.headers()["content-type"], PROBLEM_JSON);
        assert!(plain.maybe_header("content-encoding").is_none());
        let document = plain.json::<serde_json::Value>();
        assert_eq!(document["error"], "not_found");
        assert_eq!(document["instance"], path);

        let gzip = refused("gzip").await;
        gzip.assert_status(StatusCode::NOT_FOUND);
        assert_eq!(gzip.headers()["content-type"], PROBLEM_JSON);
        assert_eq!(gzip.headers()["content-encoding"], "gzip");
        assert_eq!(gzip.as_bytes()[..2], [0x1f, 0x8b], "not gzip");
    })
    .await;
}
