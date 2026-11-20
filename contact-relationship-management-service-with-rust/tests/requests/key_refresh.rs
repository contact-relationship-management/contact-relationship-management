//! The PASETO key set from its URL (CRM-T292): fetched at start, kept from the settings when the
//! fetch fails, empty when neither gives a usable key; and the refresh loop that picks up a key
//! rotation without a restart, keeping the current keys through an outage. Serial, as the
//! settings and the live verifier are process-wide; no database.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use contact_relationship_management::auth::{self, spawn_key_refresh, verifier};
use contact_relationship_management::settings;
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

use crate::scoped_auth::scoped;

/// The published key set for a throwaway key made from `seed`.
fn keys(seed: u8) -> Value {
    let public = SigningKey::from_bytes(&[seed; 32])
        .verifying_key()
        .to_bytes();
    json!({ "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                       "kid": URL_SAFE_NO_PAD.encode(Sha256::digest(public)),
                       "x": URL_SAFE_NO_PAD.encode(public) }] })
}

/// A stand-in key-set server answering `body`, or `500` while `down` is set.
async fn key_server(body: Value, down: Arc<AtomicBool>) -> String {
    let app = Router::new().fallback(move || {
        let (body, down) = (body.clone(), down.clone());
        async move {
            if down.load(Ordering::SeqCst) {
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            } else {
                Ok(axum::Json(body))
            }
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/.well-known/paseto-keys",
        listener.local_addr().unwrap()
    );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    url
}

fn count() -> usize {
    verifier().current().key_count()
}

#[tokio::test]
#[serial]
async fn the_key_set_is_fetched_at_start_and_falls_back_to_the_settings_then_to_none() {
    contact_relationship_management::app::install_tls_provider();
    let url = key_server(keys(7), Arc::default()).await;
    {
        let _scope = scoped(|s| s.paseto_keys_url = Some(url.clone()));
        auth::init().await;
        assert_eq!(count(), 1, "fetched");
    }
    {
        // Unreachable: the key set in the settings stands in.
        let _scope = scoped(|s| {
            s.paseto_keys_url = Some("http://127.0.0.1:1/keys".into());
            s.paseto_keys =
                Some(json!({ "keys": [keys(8)["keys"][0], keys(9)["keys"][0]] }).to_string());
        });
        auth::init().await;
        assert_eq!(count(), 2, "from the settings");
    }
    {
        // Unreachable, and the settings' key set does not hold a usable key: no keys at all.
        let _scope = scoped(|s| {
            s.paseto_keys_url = Some("http://127.0.0.1:1/keys".into());
            s.paseto_keys = Some(r#"{ "keys": [{ "kty": "nonsense" }] }"#.into());
        });
        auth::init().await;
        assert_eq!(count(), 0, "every token is refused");
    }
    settings::install(settings::Settings::default());
    auth::init().await;
}

#[tokio::test]
#[serial]
async fn the_refresh_loop_picks_up_a_rotation_and_keeps_the_keys_through_an_outage() {
    contact_relationship_management::app::install_tls_provider();
    let down = Arc::new(AtomicBool::new(false));
    let url = key_server(
        json!({ "keys": [keys(3)["keys"][0], keys(4)["keys"][0]] }),
        down.clone(),
    )
    .await;
    // Disabled with 0, and nothing to refresh without a URL: neither starts a loop.
    {
        let _scope = scoped(|s| s.paseto_keys_url = None);
        spawn_key_refresh();
    }
    let _scope = scoped(|s| {
        s.paseto_keys_url = Some(url.clone());
        s.paseto_keys_refresh_secs = Some(0);
    });
    spawn_key_refresh();
    drop(_scope);

    let _scope = scoped(|s| {
        s.paseto_keys_url = Some(url.clone());
        s.paseto_keys = Some(keys(1).to_string());
        s.paseto_keys_refresh_secs = Some(1);
    });
    // Start from the settings' one key, as if the auth service was down at start.
    down.store(true, Ordering::SeqCst);
    auth::init().await;
    assert_eq!(count(), 1);
    spawn_key_refresh();

    // The service comes back with a rotated set of two: picked up within a tick or two.
    down.store(false, Ordering::SeqCst);
    let mut waited = 0;
    while count() != 2 && waited < 50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        waited += 1;
    }
    assert_eq!(count(), 2, "the rotation was picked up");

    // An outage keeps the current keys.
    down.store(true, Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(2_500)).await;
    assert_eq!(count(), 2, "kept through the outage");
    drop(_scope);
    settings::install(settings::Settings::default());
    auth::init().await;
}
