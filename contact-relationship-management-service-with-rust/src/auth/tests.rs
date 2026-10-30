//! DB-free, fully in-process pins for token verification and the blanket
//! `enforce` decision. A throwaway Ed25519 key mints PASETO tokens and a
//! matching key set, so the whole verification path (valid / missing /
//! non-bearer / expired / tampered / empty-keys) and the on/off/public-path
//! enforcement matrix are exercised without the auth service or a database.

use super::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use sha2::{Digest, Sha256};

/// Issuer the test tokens and verifier agree on.
const ISSUER: &str = "authentication-service";
/// Audience the test tokens and verifier agree on.
const AUDIENCE: &str = "main-x-service";
/// A throwaway Ed25519 seed, used only to mint test tokens and a
/// matching key set in-process. Not a secret — never used in production.
const SEED: [u8; 32] = [
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
];

/// Build a key set + matching `kid` from the test public key, the same
/// way the auth service publishes it.
fn test_keys_and_kid() -> (serde_json::Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = serde_json::json!({
        "keys": [{
            "kty": "OKP", "crv": "Ed25519", "use": "sig",
            "kid": kid, "x": URL_SAFE_NO_PAD.encode(public),
        }]
    });
    (keys, kid)
}

/// Mint a signed PASETO `v4.public` token for the test identity with
/// `kid` in the footer and `exp` set `exp_offset_secs` from a fixed
/// `iat` (negative offsets produce an already-expired token).
fn sign(kid: &str, exp_offset_secs: i64) -> String {
    sign_with_attrs(kid, exp_offset_secs, &[])
}

/// Like [`sign`], with the given ABAC subject attributes minted into
/// the token's `attrs` claim (e.g. `&[("access", &["write"])]`).
fn sign_with_attrs(kid: &str, exp_offset_secs: i64, attrs: &[(&str, &[&str])]) -> String {
    let iat: i64 = 1_700_000_000;
    let claims = Claims {
        sub: "11111111-1111-1111-1111-111111111111".into(),
        email: "alice@example.com".into(),
        name: "Alice".into(),
        iss: ISSUER.into(),
        aud: AUDIENCE.into(),
        exp: iat + exp_offset_secs,
        iat,
        nbf: None,
        sid: "test-sid".into(),
        scope: Vec::new(),
        roles: Vec::new(),
        attrs: attrs
            .iter()
            .map(|(key, values)| {
                (
                    (*key).to_string(),
                    values.iter().map(ToString::to_string).collect(),
                )
            })
            .collect(),
    };
    let keypair = SigningKey::from_bytes(&SEED).to_keypair_bytes();
    let key = Key::<64>::from(keypair);
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let payload = serde_json::to_string(&claims).expect("serialize claims");
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    builder.try_sign(&private).expect("sign")
}

/// Wrap a token in a `HeaderMap` with `Authorization: Bearer <token>`.
fn bearer(token: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    h
}

/// A well-formed, in-date, correctly-signed token verifies and yields
/// the expected claims.
#[test]
fn valid_token_yields_claims() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let token = sign(&kid, 10_000_000_000);
    let claims = bearer_claims(&bearer(&token), &verifier).expect("valid token verifies");
    assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
    assert_eq!(claims.email, "alice@example.com");
}

/// No `Authorization` header ⇒ `401`.
#[test]
fn missing_header_is_401() {
    let (keys, _) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let err = bearer_claims(&HeaderMap::new(), &verifier).unwrap_err();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

/// A non-bearer scheme (e.g. `Basic`) ⇒ `401`.
#[test]
fn non_bearer_header_is_401() {
    let (keys, _) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let mut h = HeaderMap::new();
    h.insert(AUTHORIZATION, "Basic abc123".parse().unwrap());
    assert_eq!(
        bearer_claims(&h, &verifier).unwrap_err().0,
        StatusCode::UNAUTHORIZED
    );
}

/// A token whose `exp` is in the past ⇒ `401`.
#[test]
fn expired_token_is_401() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let token = sign(&kid, -60);
    assert_eq!(
        bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
        StatusCode::UNAUTHORIZED
    );
}

/// Flipping a token character breaks PASETO verification ⇒ `401`.
#[test]
fn tampered_token_is_401() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let mut token = sign(&kid, 10_000_000_000);
    let last = token.pop().unwrap();
    token.push(if last == 'a' { 'b' } else { 'a' });
    assert_eq!(
        bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
        StatusCode::UNAUTHORIZED
    );
}

/// A no-key verifier (the boot fallback) rejects even a valid token.
#[test]
fn empty_verifier_rejects_everything() {
    let verifier = empty_verifier(ISSUER, AUDIENCE);
    let (_, kid) = test_keys_and_kid();
    let token = sign(&kid, 10_000_000_000);
    assert_eq!(
        bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
        StatusCode::UNAUTHORIZED
    );
}

/// `parse_bool` accepts the documented truthy set and rejects the
/// rest (including empty, `0`, and junk).
#[test]
fn parse_bool_truthy_and_falsy() {
    for t in ["1", "true", "TRUE", "Yes", "on", " on ", "ON"] {
        assert!(parse_bool(t), "{t:?} should parse true");
    }
    for f in ["", " ", "0", "false", "no", "off", "junk", "2"] {
        assert!(!parse_bool(f), "{f:?} should parse false");
    }
}

/// The default policy the enforcement tests share (what an
/// unconfigured service uses).
fn policy() -> Policy {
    Policy::default_policy()
}

/// Enforcement off ⇒ a protected path passes with no token — for
/// reads and mutations alike (no authn and no authz when the flag
/// is off; today's default behaviour stays intact).
#[test]
fn enforce_off_allows_without_token() {
    let (keys, _) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    for method in [Method::GET, Method::POST, Method::DELETE] {
        assert!(
            enforce(
                false,
                &method,
                "/api/wards",
                &HeaderMap::new(),
                &verifier,
                &policy
            )
            .is_ok(),
            "{method} should pass with enforcement off"
        );
    }
}

/// SEC-G8 — the default-off **exposure pin**. With `CRM_REQUIRE_AUTH`
/// off (the shipped default), the most sensitive reads — the audit
/// trail, patient locate, and a single stay's PII — are **open without
/// a token**. This is by design
/// (see `agents/share/security.md` §4), but it means **activation is a
/// tracked release gate**: a deployment exposed to untrusted callers
/// MUST set the flag before it is reachable. This test documents that
/// exposure explicitly so flipping the default cannot happen silently.
#[test]
fn default_off_exposes_sensitive_reads_activation_is_a_release_gate() {
    let (keys, _) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    let no_token = HeaderMap::new();
    for path in [
        "/api/stays/0c4f1e2a-0000-4000-8000-000000000001", // a stay's PII
        "/api/locate/person:0c4f1e2a-0000-4000-8000-000000000001", // patient location
        "/api/audits/recent",                              // system-wide audit
        "/api/whiteboard/0c4f1e2a-0000-4000-8000-000000000002", // ward board
    ] {
        assert!(
            enforce(false, &Method::GET, path, &no_token, &verifier, &policy).is_ok(),
            "SEC-G8: with the flag OFF, {path} is open without a token (activation is the gate)"
        );
    }
}

#[test]
fn marking_your_own_notifications_read_is_a_read_but_the_sweep_is_an_admin_action() {
    assert_eq!(
        derive_action(&Method::PUT, "/api/notifications/abc/read"),
        Action::Read
    );
    assert_eq!(
        derive_action(&Method::POST, "/api/notifications/read-all"),
        Action::Read
    );
    assert_eq!(
        derive_action(&Method::POST, "/api/notifications/sweep"),
        Action::Destructive
    );
    assert_eq!(
        derive_action(&Method::PUT, "/api/activities/abc/read"),
        Action::Write
    );
}

#[test]
fn your_own_saved_views_are_not_data_writes() {
    for method in [Method::POST, Method::PUT, Method::DELETE] {
        let path = if method == Method::POST {
            "/api/views"
        } else {
            "/api/views/abc"
        };
        assert_eq!(derive_action(&method, path), Action::Read, "{method}");
    }
    assert_eq!(derive_action(&Method::POST, "/api/viewsx"), Action::Write);
}

/// Enforcement on ⇒ the public paths (health/ping, `OpenAPI`,
/// Swagger UI, Prometheus metrics) still pass without a token.
#[test]
fn enforce_on_allows_public_paths() {
    let (keys, _) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    for path in [
        "/_health",
        "/_ping",
        "/api-docs/openapi.json",
        "/swagger-ui",
        "/swagger-ui/index.html",
        "/metrics.prom",
        "/unsubscribe/sometoken",
        "/forms/newsletter",
        "/kb/articles",
        "/api/bridges/wpm/events",
    ] {
        assert!(
            enforce(
                true,
                &Method::GET,
                path,
                &HeaderMap::new(),
                &verifier,
                &policy
            )
            .is_ok(),
            "{path} should be public"
        );
    }
}

/// Only the worker service's event route is public under `/api/bridges` (CRM-R92): the rest of the
/// prefix, and a path that only starts like it, still need a token.
#[test]
fn only_the_wpm_event_route_is_public_under_bridges() {
    let (keys, _) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    for path in [
        "/api/bridges/wpm/teams-import",
        "/api/bridges/wpm/events/x",
        "/api/bridges/wpm/eventsx",
        "/api/bridges/ppm/deals/x",
    ] {
        let got = enforce(
            true,
            &Method::POST,
            path,
            &HeaderMap::new(),
            &verifier,
            &policy,
        );
        assert_eq!(got.unwrap_err().0, StatusCode::UNAUTHORIZED, "{path}");
    }
}

/// Enforcement on, protected path, no token ⇒ `401`.
#[test]
fn enforce_on_protected_without_token_is_401() {
    let (keys, _) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let err = enforce(
        true,
        &Method::GET,
        "/api/wards",
        &HeaderMap::new(),
        &verifier,
        &policy(),
    )
    .unwrap_err();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

/// Enforcement on, protected path, valid token ⇒ a read passes.
#[test]
fn enforce_on_protected_with_valid_token_is_ok() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let token = sign(&kid, 10_000_000_000);
    assert!(
        enforce(
            true,
            &Method::GET,
            "/api/wards",
            &bearer(&token),
            &verifier,
            &policy()
        )
        .is_ok()
    );
}

/// Enforcement on, protected path, expired token ⇒ `401`.
#[test]
fn enforce_on_protected_with_expired_token_is_401() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let token = sign(&kid, -60);
    let err = enforce(
        true,
        &Method::GET,
        "/api/wards",
        &bearer(&token),
        &verifier,
        &policy(),
    )
    .unwrap_err();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

/// Enforcement on, protected path, tampered token ⇒ `401`.
#[test]
fn enforce_on_protected_with_tampered_token_is_401() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let mut token = sign(&kid, 10_000_000_000);
    let last = token.pop().unwrap();
    token.push(if last == 'a' { 'b' } else { 'a' });
    let err = enforce(
        true,
        &Method::GET,
        "/api/wards",
        &bearer(&token),
        &verifier,
        &policy(),
    )
    .unwrap_err();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

/// Action derivation (`authorization-attributes.md` §2): safe
/// methods read; DELETE deletes; the crate's destructive named
/// POSTs (merge / deduplicate / import) are destructive, not
/// write; every other POST/PUT/PATCH writes.
#[test]
fn break_glass_routes_are_classified() {
    assert_eq!(
        derive_action(&Method::POST, "/api/break-glass"),
        Action::Read
    );
    assert_eq!(
        derive_action(&Method::POST, "/api/break-glass/"),
        Action::Read
    );
    assert_eq!(
        derive_action(&Method::GET, "/api/break-glass"),
        Action::Destructive
    );
    assert_eq!(
        derive_action(
            &Method::POST,
            "/api/break-glass/0c4f1e2a-0000-4000-8000-000000000001/review"
        ),
        Action::Destructive
    );
}

#[test]
fn privileged_reads_derive_the_admin_tier() {
    for path in [
        "/api/audits/recent",
        "/api/audits/verify",
        "/api/audits",
        "/api/audits/0c4f1e2a-0000-4000-8000-000000000001",
        "/api/events/recent",
        "/api/retention",
        "/api/retention/", // trailing slash normalised
    ] {
        assert_eq!(
            derive_action(&Method::GET, path),
            Action::Destructive,
            "{path}"
        );
    }
    // Segment-wise match: lookalikes stay ordinary reads.
    for path in ["/api/auditsfoo", "/api/contacts", "/api/insights/dpo"] {
        assert_eq!(derive_action(&Method::GET, path), Action::Read, "{path}");
    }
}

#[test]
fn derive_action_matrix() {
    for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
        assert_eq!(derive_action(&method, "/api/wards"), Action::Read);
    }
    assert_eq!(
        derive_action(&Method::DELETE, "/api/wards/1"),
        Action::Delete
    );
    for path in [
        "/api/wards/merge",
        "/api/wards/deduplicate",
        "/api/wards/import",
        "/api/contacts/1/erase",
        "/api/retention/sweep",
    ] {
        assert_eq!(derive_action(&Method::POST, path), Action::Destructive);
    }
    assert_eq!(derive_action(&Method::POST, "/api/wards"), Action::Write);
    assert_eq!(
        derive_action(&Method::POST, "/api/wards/check-duplicates"),
        Action::Write
    );
    assert_eq!(derive_action(&Method::PUT, "/api/wards/1"), Action::Write);
    assert_eq!(derive_action(&Method::PATCH, "/api/wards/1"), Action::Write);
    // Webhook subscriptions decide where change notifications go (CRM-R28): every method is the
    // admin/service tier, and a lookalike prefix is not caught.
    for method in [Method::GET, Method::POST, Method::PUT, Method::DELETE] {
        for path in [
            "/api/webhook-subscriptions",
            "/api/webhook-subscriptions/",
            "/api/webhook-subscriptions/relay",
            "/api/webhook-subscriptions/abc/deliveries",
            "/api/webhook-subscriptions/abc/rotate-secret",
        ] {
            assert_eq!(
                derive_action(&method, path),
                Action::Destructive,
                "{method} {path}"
            );
        }
    }
    assert_eq!(
        derive_action(&Method::POST, "/api/webhook-subscriptionsfoo"),
        Action::Write
    );
    // GET on a destructive-suffixed path is still a read — only
    // POST consults the suffix list.
    assert_eq!(
        derive_action(&Method::GET, "/api/wards/merge"),
        Action::Read
    );
    assert_eq!(
        derive_action(&Method::GET, "/api/contacts/1/erase"),
        Action::Read
    );
    // SEC-G6: a trailing slash must NOT downgrade a destructive POST to
    // Write (a non-admin `access=write` caller must not reach merge).
    for path in [
        "/api/wards/merge/",
        "/api/wards/merge//",
        "/api/wards/deduplicate/",
        "/api/wards/import/",
        "/api/contacts/1/erase/",
        "/api/retention/sweep/",
    ] {
        assert_eq!(
            derive_action(&Method::POST, path),
            Action::Destructive,
            "{path} must stay Destructive"
        );
    }
}

/// ABAC default policy, empty `attrs` ⇒ GET allowed, POST `403`
/// (default allow-read / deny-mutation).
#[test]
fn abac_empty_attrs_reads_but_cannot_write() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    let token = sign_with_attrs(&kid, 10_000_000_000, &[]);
    assert!(
        enforce(
            true,
            &Method::GET,
            "/api/wards",
            &bearer(&token),
            &verifier,
            &policy
        )
        .is_ok()
    );
    let err = enforce(
        true,
        &Method::POST,
        "/api/wards",
        &bearer(&token),
        &verifier,
        &policy,
    )
    .unwrap_err();
    assert_eq!(err.0, StatusCode::FORBIDDEN);
}

/// ABAC `access=write` ⇒ POST/PUT allowed; DELETE and the merge
/// POST still `403` (write is not destructive).
#[test]
fn abac_access_write_writes_but_not_destructive() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    let token = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["write"])]);
    for method in [Method::POST, Method::PUT] {
        assert!(
            enforce(
                true,
                &method,
                "/api/wards",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok(),
            "{method} should be allowed for access=write"
        );
    }
    let delete = enforce(
        true,
        &Method::DELETE,
        "/api/wards/1",
        &bearer(&token),
        &verifier,
        &policy,
    )
    .unwrap_err();
    assert_eq!(delete.0, StatusCode::FORBIDDEN);
    let merge = enforce(
        true,
        &Method::POST,
        "/api/wards/merge",
        &bearer(&token),
        &verifier,
        &policy,
    )
    .unwrap_err();
    assert_eq!(merge.0, StatusCode::FORBIDDEN);
}

/// ABAC `access=admin` ⇒ DELETE and the destructive named POSTs
/// are allowed (destructive covers delete).
#[test]
fn abac_access_admin_allows_destructive() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    let token = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["admin"])]);
    assert!(
        enforce(
            true,
            &Method::DELETE,
            "/api/wards/1",
            &bearer(&token),
            &verifier,
            &policy
        )
        .is_ok()
    );
    for path in ["/api/wards/merge", "/api/wards/deduplicate"] {
        assert!(
            enforce(
                true,
                &Method::POST,
                path,
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok(),
            "{path} should be allowed for access=admin"
        );
    }
}

/// ABAC `svc=true` (machine peer) ⇒ everything is allowed.
#[test]
fn abac_svc_true_allows_everything() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    let token = sign_with_attrs(&kid, 10_000_000_000, &[("svc", &["true"])]);
    for (method, path) in [
        (Method::GET, "/api/wards"),
        (Method::POST, "/api/wards"),
        (Method::PUT, "/api/wards/1"),
        (Method::DELETE, "/api/wards/1"),
        (Method::POST, "/api/wards/merge"),
        (Method::POST, "/api/wards/deduplicate"),
    ] {
        assert!(
            enforce(true, &method, path, &bearer(&token), &verifier, &policy).is_ok(),
            "{method} {path} should be allowed for svc=true"
        );
    }
}

/// A configured deny rule ahead of an allow rule wins
/// (first-match-wins pin, through the guard).
#[test]
fn abac_configured_deny_beats_later_allow() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = Policy::from_json(
        r#"{ "rules": [
            { "effect": "deny",  "actions": ["write"], "when": { "purpose": ["research"] } },
            { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
        ] }"#,
    )
    .expect("policy parses");
    let denied = sign_with_attrs(
        &kid,
        10_000_000_000,
        &[("access", &["write"]), ("purpose", &["research"])],
    );
    let err = enforce(
        true,
        &Method::POST,
        "/api/wards",
        &bearer(&denied),
        &verifier,
        &policy,
    )
    .unwrap_err();
    assert_eq!(err.0, StatusCode::FORBIDDEN);
    let allowed = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["write"])]);
    assert!(
        enforce(
            true,
            &Method::POST,
            "/api/wards",
            &bearer(&allowed),
            &verifier,
            &policy
        )
        .is_ok()
    );
}

/// 401 vs 403: missing/bad credential is `401`; a valid credential
/// the policy denies is `403` with the deciding-rule reason.
#[test]
fn abac_401_versus_403_distinction() {
    let (keys, kid) = test_keys_and_kid();
    let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let policy = policy();
    let no_token = enforce(
        true,
        &Method::POST,
        "/api/wards",
        &HeaderMap::new(),
        &verifier,
        &policy,
    )
    .unwrap_err();
    assert_eq!(no_token.0, StatusCode::UNAUTHORIZED);
    let token = sign_with_attrs(&kid, 10_000_000_000, &[]);
    let denied = enforce(
        true,
        &Method::POST,
        "/api/wards",
        &bearer(&token),
        &verifier,
        &policy,
    )
    .unwrap_err();
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    assert_eq!(denied.1, "default deny");
}

/// `policy_from_env` never breaks boot: bad policy JSON falls back
/// to the built-in default policy (the pure fallback path
/// `policy_from_env` takes on parse failure).
#[test]
fn policy_bad_json_falls_back_to_default() {
    assert!(Policy::from_json("{ not json").is_err());
    assert_eq!(
        Policy::from_json("{ not json").unwrap_or_else(|_| Policy::default_policy()),
        Policy::default_policy()
    );
}

/// Serve `keys` as the key-set JSON from a local ephemeral-port HTTP
/// listener (the auth service's `/.well-known/paseto-keys` stand-in)
/// and return the URL to fetch it from.
async fn serve_keys(keys: serde_json::Value) -> String {
    let app = axum::Router::new().route(
        "/.well-known/paseto-keys",
        axum::routing::get(move || {
            let keys = keys.clone();
            async move { axum::Json(keys) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve key set");
    });
    format!("http://{addr}/.well-known/paseto-keys")
}

/// Boot-time fetch happy path: `fetch_or` against a local listener
/// serving a valid key set builds the verifier from the **fetched**
/// keys — a token signed by the served key verifies even though the
/// fallback verifier has no keys (the fetched key set wins).
#[tokio::test]
async fn fetch_or_fetched_key_set_wins() {
    let (keys, kid) = test_keys_and_kid();
    let url = serve_keys(keys).await;
    let verifier = fetch_or(&url, ISSUER, AUDIENCE, empty_verifier(ISSUER, AUDIENCE)).await;
    assert_eq!(verifier.key_count(), 1);
    let token = sign(&kid, 10_000_000_000);
    let claims = bearer_claims(&bearer(&token), &verifier).expect("token signed by fetched key");
    assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
}

/// Boot-time fetch fallback: a fast-failing URL (nothing listens on
/// port 1) makes `fetch_or` return the env-style fallback verifier —
/// no panic, and tokens signed by the fallback key set still verify,
/// so the service always boots.
#[tokio::test]
async fn fetch_or_unreachable_url_falls_back() {
    let (keys, kid) = test_keys_and_kid();
    let fallback = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
    let verifier = fetch_or("http://127.0.0.1:1/", ISSUER, AUDIENCE, fallback).await;
    assert_eq!(verifier.key_count(), 1);
    let token = sign(&kid, 10_000_000_000);
    assert!(bearer_claims(&bearer(&token), &verifier).is_ok());
}

/// Build `Claims` with the given subject attributes for the
/// record-level decision tests (no signing needed).
fn claims_with_attrs(attrs: &[(&str, &[&str])]) -> Claims {
    Claims {
        sub: "11111111-1111-1111-1111-111111111111".into(),
        email: "alice@example.com".into(),
        name: "Alice".into(),
        iss: ISSUER.into(),
        aud: AUDIENCE.into(),
        exp: 2_000_000_000,
        iat: 1_900_000_000,
        nbf: None,
        sid: "test-sid".into(),
        scope: Vec::new(),
        roles: Vec::new(),
        attrs: attrs
            .iter()
            .map(|(key, values)| {
                (
                    (*key).to_string(),
                    values.iter().map(ToString::to_string).collect(),
                )
            })
            .collect(),
    }
}

/// The environment-attribute derivation flags working vs after
/// hours from a UTC hour (`authorization-attributes.md` §10).
#[test]
fn env_attrs_at_flags_after_hours() {
    for hour in 8..18 {
        let env = env_attrs_at(hour);
        assert_eq!(
            env["after_hours"],
            vec!["false".to_string()],
            "{hour}:00 is working hours"
        );
        assert_eq!(env["hour"], vec![hour.to_string()]);
    }
    for hour in [0, 7, 18, 22, 23] {
        assert_eq!(
            env_attrs_at(hour)["after_hours"],
            vec!["true".to_string()],
            "{hour}:00 is after hours"
        );
    }
}

/// A deal model for the resource-attribute tests.
fn a_deal(
    owner: Option<uuid::Uuid>,
    closed: bool,
    won: bool,
) -> crate::models::_entities::deals::Model {
    crate::models::_entities::deals::Model {
        created_at: chrono::Utc::now().into(),
        updated_at: chrono::Utc::now().into(),
        id: 1,
        pid: uuid::Uuid::new_v4(),
        account_pid: None,
        primary_contact_pid: None,
        owner_ref: owner.map(|o| format!("worker:{o}")),
        pipeline_pid: uuid::Uuid::new_v4(),
        stage_pid: uuid::Uuid::new_v4(),
        name: "Big Deal".to_string(),
        amount_minor: 5_000_000,
        currency: "GBP".to_string(),
        expected_close_on: None,
        kanban_position: 0,
        source_campaign_pid: None,
        closed_at: closed.then(|| chrono::Utc::now().into()),
        won,
        lost_reason: None,
        deleted_at: None,
    }
}

/// The deal resource attributes map owner + open/won/lost status
/// (`authorization-attributes.md` §9).
#[test]
fn deal_resource_attrs_map_owner_and_status() {
    let owner = uuid::Uuid::new_v4();
    let attrs = deal_resource_attrs(&a_deal(Some(owner), false, false));
    assert_eq!(attrs["owner"], vec![owner.to_string()]);
    assert_eq!(attrs["status"], vec!["open".to_string()]);
    assert_eq!(
        deal_resource_attrs(&a_deal(None, true, true))["status"],
        vec!["won".to_string()]
    );
    assert_eq!(
        deal_resource_attrs(&a_deal(None, true, false))["status"],
        vec!["lost".to_string()]
    );
    assert!(!deal_resource_attrs(&a_deal(None, false, false)).contains_key("owner"));
}

/// The attributes drive rep ownership through the shared engine:
/// a `$sub` owner-rule reads the own deal unmasked; anyone else
/// falls through to the masked-read rule.
#[test]
fn deal_attrs_drive_ownership_and_masking() {
    let me = uuid::Uuid::new_v4();
    let policy = Policy::from_json(
        r#"{ "rules": [
            { "effect": "allow", "actions": ["read"], "when": { "resource.owner": ["$sub"] } },
            { "effect": "allow", "actions": ["read"], "when": {}, "obligations": ["mask"] }
        ] }"#,
    )
    .expect("policy parses");
    let mut own = claims_with_attrs(&[]);
    own.sub = me.to_string();
    let self_read = policy.evaluate_with_resource(
        &own,
        Action::Read,
        ENTITY,
        &deal_resource_attrs(&a_deal(Some(me), false, false)),
    );
    assert!(self_read.allowed);
    assert!(self_read.obligations.is_empty());
    let other = claims_with_attrs(&[]);
    let masked_read = policy.evaluate_with_resource(
        &other,
        Action::Read,
        ENTITY,
        &deal_resource_attrs(&a_deal(Some(me), false, false)),
    );
    assert!(masked_read.allowed, "falls through to the masked-read rule");
    assert!(masked_read.requires("mask"));
}

/// `mask_json` nulls the named amount keys and leaves structure —
/// never a fake zero.
#[test]
fn mask_json_nulls_amounts_keeps_structure() {
    let masked = mask_json(
        serde_json::json!({ "name": "Big Deal", "amount_minor": 5_000_000, "currency": "GBP" }),
        &["amount_minor"],
    );
    assert_eq!(masked["name"], "Big Deal");
    assert!(masked["amount_minor"].is_null());
    assert_eq!(masked["currency"], "GBP");
    // Non-object values pass through untouched.
    let passthrough = mask_json(serde_json::json!([1, 2]), &["amount_minor"]);
    assert_eq!(passthrough, serde_json::json!([1, 2]));
}

/// `mask_text_json` replaces the named text keys with [`MASKED`]
/// (never `null`, unlike `mask_json`'s amount handling) and leaves
/// every other field untouched.
#[test]
fn mask_text_json_redacts_text_keeps_structure() {
    let masked = mask_text_json(
        serde_json::json!({ "display_name": "Alice", "preferred_channel": "phone" }),
        &["preferred_channel"],
    );
    assert_eq!(masked["display_name"], "Alice");
    assert_eq!(masked["preferred_channel"], MASKED);
    // Non-object values pass through untouched.
    let passthrough = mask_text_json(serde_json::json!([1, 2]), &["preferred_channel"]);
    assert_eq!(passthrough, serde_json::json!([1, 2]));
}

/// Evaluate the **shipped reference policy** for one persona:
/// `(allowed, masked)` for `action` against `resource`/`env`.
fn reference_decision(
    attrs: &[(&str, &[&str])],
    action: Action,
    resource: &BTreeMap<String, Vec<String>>,
    env: &BTreeMap<String, Vec<String>>,
) -> (bool, bool) {
    let policy = Policy::from_json(include_str!("../../config/abac-policy.reference.json"))
        .expect("the reference policy parses");
    let decision =
        policy.evaluate_with_context(&claims_with_attrs(attrs), action, ENTITY, resource, env);
    (
        decision.allowed,
        decision.obligations.iter().any(|o| o == "mask"),
    )
}

/// Subject attributes as the tests write them.
type Attrs<'a> = &'a [(&'a str, &'a [&'a str])];

/// One row of the persona matrix: what the reference policy should
/// decide for a persona.
struct Persona<'a> {
    name: &'a str,
    attrs: Attrs<'a>,
    /// `(allowed, masked)` for a read.
    read: (bool, bool),
    write: bool,
    destructive: bool,
}

impl<'a> Persona<'a> {
    fn new(
        name: &'a str,
        attrs: Attrs<'a>,
        read: (bool, bool),
        write: bool,
        destructive: bool,
    ) -> Self {
        Self {
            name,
            attrs,
            read,
            write,
            destructive,
        }
    }
}

/// The persona matrix over the shipped reference policy (CRM-T32,
/// `auth.md`): who may read unmasked, read masked, write, and take the
/// admin/service tier (destructive actions and privileged reads).
#[test]
fn reference_policy_persona_matrix() {
    let none = BTreeMap::new();
    let owned: BTreeMap<String, Vec<String>> = [(
        "owner".to_string(),
        vec!["11111111-1111-1111-1111-111111111111".to_string()],
    )]
    .into();
    let break_glass: BTreeMap<String, Vec<String>> =
        [("break_glass".to_string(), vec!["true".to_string()])].into();

    let personas = [
        Persona::new("service", &[("svc", &["true"])], (true, false), true, true),
        Persona::new(
            "admin",
            &[("access", &["admin"])],
            (true, false),
            true,
            true,
        ),
        Persona::new(
            "manager",
            &[("manager", &["true"])],
            (true, false),
            true,
            false,
        ),
        Persona::new("rep", &[("rep", &["true"])], (true, true), true, false),
        Persona::new(
            "marketing",
            &[("marketing", &["true"])],
            (true, true),
            true,
            false,
        ),
        Persona::new(
            "support",
            &[("support", &["true"])],
            (true, true),
            true,
            false,
        ),
        Persona::new("plain reader", &[], (true, true), false, false),
    ];
    for persona in personas {
        let Persona {
            name,
            attrs,
            read,
            write,
            destructive,
        } = persona;
        assert_eq!(
            reference_decision(attrs, Action::Read, &none, &none),
            read,
            "{name} read"
        );
        assert_eq!(
            reference_decision(attrs, Action::Write, &none, &none).0,
            write,
            "{name} write"
        );
        assert_eq!(
            reference_decision(attrs, Action::Destructive, &none, &none).0,
            destructive,
            "{name} destructive / privileged read"
        );
    }

    // The record's owner reads unmasked and may write their own record.
    assert_eq!(
        reference_decision(&[], Action::Read, &owned, &none),
        (true, false)
    );
    assert!(reference_decision(&[], Action::Write, &owned, &none).0);
    assert!(!reference_decision(&[], Action::Destructive, &owned, &none).0);

    // Break-glass lifts the mask for reads and nothing else.
    assert_eq!(
        reference_decision(&[], Action::Read, &none, &break_glass),
        (true, false)
    );
    assert!(!reference_decision(&[], Action::Write, &none, &break_glass).0);
    assert!(!reference_decision(&[], Action::Destructive, &none, &break_glass).0);
}
