//! The request limit through the booted app: the budget headers, the `429`,
//! and the probe exemption.
//!
//! Its own test binary because the limit is read from the environment when the
//! app boots. Run with `cargo test --test rate_limit -- --ignored`.

use axa::testing::prelude::*;
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use contact_relationship_management::app::App;
use contact_relationship_management::auth;
use contact_relationship_management::settings;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
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

/// `value` for `name`, for the rest of this process. Safe as above: `#[serial]`, and set before boot.
fn set(name: &str, value: &str) {
    unsafe { std::env::set_var(name, value) };
}

/// A client that writes a new first `X-Forwarded-For` entry, and a new made-up token, on every
/// request. `TRUSTED_PROXIES` names the test client as the proxy, so the last entry is what the
/// proxy saw: the real client. The retired `CRM_TRUST_FORWARDED_FOR` is set to show the header is
/// not believed either way; with it, the service used to count each request under its own key.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_forged_address_or_token_does_not_buy_a_fresh_limit() {
    set("CRM_RATE_LIMIT_PER_MIN", "3");
    set("TRUSTED_PROXIES", "127.0.0.1");
    set("CRM_TRUST_FORWARDED_FOR", "1");
    request::<App, _, _>(|request, _ctx| async move {
        let mut answers = Vec::new();
        for attempt in 0..5 {
            let answer = request
                .get("/api/contacts")
                .add_header(
                    "x-forwarded-for",
                    format!("192.0.2.{attempt}, 198.51.100.7"),
                )
                .add_header("authorization", format!("Bearer forged-{attempt}"))
                .await;
            answers.push(answer.status_code());
        }
        assert_eq!(
            answers,
            [
                StatusCode::OK,
                StatusCode::OK,
                StatusCode::OK,
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::TOO_MANY_REQUESTS,
            ],
        );
        // The client the proxy saw differently is someone else.
        let other = request
            .get("/api/contacts")
            .add_header("x-forwarded-for", "198.51.100.8")
            .await;
        other.assert_status_ok();
    })
    .await;
}

/// A throwaway key, for this test binary only.
const SEED: [u8; 32] = [9; 32];

fn kid_and_keys() -> (String, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (kid, keys.to_string())
}

/// A valid token for `subject`. `session` makes one token differ from another of the same subject.
fn token(kid: &str, subject: &str, session: &str) -> String {
    let issued: i64 = 1_700_000_000;
    let payload: Value = json!({
        "sub": subject, "email": "rep@example.com", "name": "Test Rep",
        "iss": "authentication-service", "aud": "main-x-service",
        "exp": issued + 10_000_000_000_i64, "iat": issued,
        "sid": session, "attrs": {},
    });
    let key = Key::<64>::from(SigningKey::from_bytes(&SEED).to_keypair_bytes());
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    let payload = payload.to_string();
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    let signed = builder.try_sign(&private).expect("sign");
    format!("Bearer {signed}")
}

/// Puts sign-in back off when a test ends, even by panic.
struct SignInRequired;

impl Drop for SignInRequired {
    fn drop(&mut self) {
        settings::update(|s| {
            s.require_auth = false;
            s.paseto_keys = None;
        });
        auth::reload_from_settings();
    }
}

/// Sign-in on, with the throwaway key, until the guard is dropped.
fn sign_in_required() -> (String, SignInRequired) {
    let (kid, keys) = kid_and_keys();
    settings::update(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys);
    });
    auth::reload_from_settings();
    (kid, SignInRequired)
}

/// A token that verified is one budget, however many tokens its subject holds, and those behind
/// one address do not share. Browsers arrive through the one BFF, so they must not.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_verified_subject_has_one_budget_of_its_own() {
    set("CRM_RATE_LIMIT_PER_MIN", "2");
    set("CRM_AUTH_FAIL_LIMIT", "0");
    let (kid, _sign_in) = sign_in_required();
    request::<App, _, _>(|request, _ctx| async move {
        for session in ["one", "two"] {
            let answer = request
                .get("/api/contacts")
                .add_header("authorization", token(&kid, "person-a", session))
                .await;
            answer.assert_status_ok();
        }
        let fresh_token = token(&kid, "person-a", "three");
        let refused = request
            .get("/api/contacts")
            .add_header("authorization", fresh_token)
            .await;
        refused.assert_status(StatusCode::TOO_MANY_REQUESTS);
        let other = request
            .get("/api/contacts")
            .add_header("authorization", token(&kid, "person-b", "one"))
            .await;
        other.assert_status_ok();
    })
    .await;
}

/// Made-up tokens lock out their address, and neither buy a way round it nor lock out those who
/// sign in properly from the same address.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn made_up_tokens_lock_out_their_address_and_not_the_signed_in() {
    set("CRM_RATE_LIMIT_PER_MIN", "0");
    set("CRM_AUTH_FAIL_LIMIT", "2");
    let (kid, _sign_in) = sign_in_required();
    request::<App, _, _>(|request, _ctx| async move {
        let mut answers = Vec::new();
        for attempt in 0..3 {
            let answer = request
                .get("/api/contacts")
                .add_header("authorization", format!("Bearer forged-{attempt}"))
                .await;
            answers.push(answer.status_code());
        }
        assert_eq!(
            answers,
            [
                StatusCode::UNAUTHORIZED,
                StatusCode::UNAUTHORIZED,
                StatusCode::TOO_MANY_REQUESTS,
            ],
        );
        let signed_in = request
            .get("/api/contacts")
            .add_header("authorization", token(&kid, "person-a", "one"))
            .await;
        signed_in.assert_status_ok();
    })
    .await;
}

/// `/mcp` calls the router directly, past the stack that resolves the address, so the address has
/// to go with the call or every agent would land in one bucket.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_tool_call_is_counted_against_the_client_that_made_it() {
    set("CRM_RATE_LIMIT_PER_MIN", "1");
    set("CRM_AUTH_FAIL_LIMIT", "0");
    set("TRUSTED_PROXIES", "127.0.0.1");
    request::<App, _, _>(|request, _ctx| async move {
        let call = |client: &'static str| {
            request
                .post("/mcp")
                .add_header("x-forwarded-for", client)
                .json(&json!({
                    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                    "params": { "name": "get_contact", "arguments": {
                        "pid": "00000000-0000-4000-8000-000000000000",
                    } },
                }))
        };
        let spent = call("198.51.100.1").await.json::<Value>().to_string();
        assert!(spent.contains("404") || spent.contains("not"), "{spent}");
        let over = call("198.51.100.1").await.json::<Value>().to_string();
        assert!(over.contains("429"), "{over}");
        let other = call("198.51.100.2").await.json::<Value>().to_string();
        assert!(!other.contains("429"), "{other}");
    })
    .await;
}
