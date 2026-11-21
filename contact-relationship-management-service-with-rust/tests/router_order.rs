//! The order of the router: the service's own layers inside `axa`'s stack (CRM-T217), through the
//! booted app. Outside it, a refusal had no request id, security headers or CORS, and the layers
//! read compressed bodies.
//!
//! These settings are read from the environment when the app boots, so each test sets them for its
//! own length (`ScopedEnv`, CRM-T263). Run with `cargo test --test main router_order -- --ignored`.

use axa::problem::PROBLEM_JSON;
use axa_test::prelude::*;
use axum::http::{Method, StatusCode};
use contact_relationship_management::app::App;
use serial_test::serial;

use crate::scoped_env::ScopedEnv;

const ORIGIN: &str = "https://crm.example.org";

/// What these tests vary, until the guard is dropped. Set before the app boots, in `#[serial]` tests.
fn boot_with(limit: &str, compression: &str) -> ScopedEnv {
    let mut env = ScopedEnv::default();
    env.set("CONTACT_RELATIONSHIP_MANAGEMENT_RATE_LIMIT_PER_MIN", limit);
    env.set("HTTP_COMPRESSION", compression);
    env.set("CORS_ALLOWED_ORIGINS", ORIGIN);
    env
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_refusal_from_the_abuse_guard_carries_the_standard_headers() {
    let _env = boot_with("1", "false");
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
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_preflight_is_answered_even_when_the_client_is_over_its_limit() {
    let _env = boot_with("1", "false");
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
/// document in the clear under `Content-Encoding: gzip`, which it cannot read. Since axa 0.11
/// compression skips a `no-store` response (against BREACH), and every error is `no-store`
/// (CRM-T217), so a client asking for gzip gets the document in the clear, readable, under no
/// `content-encoding` (CRM-T228). It asks twice, without gzip and with it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_compressed_error_is_a_problem_document_compressed_once() {
    let _env = boot_with("0", "true");
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
        assert_eq!(gzip.headers()["cache-control"], "no-store");
        assert!(gzip.maybe_header("content-encoding").is_none());
        assert_eq!(gzip.json::<serde_json::Value>(), document);
    })
    .await;
}

/// "hi" as one zstd frame, from axa's own test of the same refusal.
const ZSTD_HI: [u8; 15] = [
    0x28, 0xb5, 0x2f, 0xfd, 0x04, 0x58, 0x11, 0x00, 0x00, 0x68, 0x69, 0xfa, 0x38, 0x26, 0xea,
];

/// The service builds axa without its `zstd` feature (CRM-T264), so a Zstd request body is
/// refused as unsupported, in a problem document, before any handler reads it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_zstd_request_body_is_refused_as_unsupported() {
    let _env = boot_with("0", "true");
    request::<App, _, _>(|request, _ctx| async move {
        let refused = request
            .post("/api/contacts")
            .add_header("content-type", "application/json")
            .add_header("content-encoding", "zstd")
            .bytes(ZSTD_HI.to_vec().into())
            .await;
        refused.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(refused.headers()["content-type"], PROBLEM_JSON);
    })
    .await;
}
