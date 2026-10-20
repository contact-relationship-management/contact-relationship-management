//! The process-wide token verifier: where the keys come from, and keeping them fresh.

#[cfg(any(feature = "paseto", feature = "keycloak"))]
use super::OnceLock;
#[cfg(feature = "paseto")]
use super::{ReloadableVerifier, Verifier};

/// Default issuer expected in tokens (`iss`).
#[cfg(feature = "paseto")]
pub(super) const DEFAULT_ISSUER: &str = "authentication-service";

/// Default audience expected in tokens (`aud`).
#[cfg(feature = "paseto")]
pub(super) const DEFAULT_AUDIENCE: &str = "main-x-service";

/// The process-wide **hot-reloadable** token verifier. Lazily built from
/// the environment on first use ([`build_from_env`]); [`init`] swaps in
/// the boot-time fetched key set, and [`spawn_key_refresh`] swaps in a
/// re-fetched key set periodically (**key rotation without a restart**).
#[cfg(feature = "paseto")]
pub(super) static VERIFIER: OnceLock<ReloadableVerifier> = OnceLock::new();

/// The process-wide reloadable token verifier. Read the active snapshot
/// with `verifier().current()` per request; it is swapped by [`init`]
/// (boot fetch) and [`spawn_key_refresh`] (periodic re-fetch).
#[cfg(feature = "paseto")]
#[must_use]
pub fn verifier() -> &'static ReloadableVerifier {
    VERIFIER.get_or_init(|| ReloadableVerifier::new(build_from_env()))
}

/// Seed the process-wide [`verifier`] before the app serves traffic
/// (called from `App::after_routes`). When `CRM_PASETO_KEYS_URL` is set
/// (non-blank) the published key set is fetched over HTTP **once at boot**
/// and, on success, swapped in over the env-built one; on fetch failure,
/// or with the URL unset/blank, the env-built verifier stands, so the
/// service always boots. Idempotent enough to call once at boot.
///
/// With the `keycloak` feature it also builds the Keycloak verifier from
/// `CRM_KEYCLOAK_*` (see [`keycloak_verifier`]).
// `async` in every feature shape so `app.rs` has one call site; only the
// `paseto` half awaits (the key fetch).
#[allow(clippy::unused_async)]
pub async fn init() {
    #[cfg(feature = "paseto")]
    init_paseto().await;
    #[cfg(feature = "keycloak")]
    init_keycloak();
}

/// Seed the PASETO [`verifier`] (the `paseto` half of [`init`]).
#[cfg(feature = "paseto")]
pub(super) async fn init_paseto() {
    if let Some(url) = std::env::var("CRM_PASETO_KEYS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        let issuer = env_or("CRM_TOKEN_ISSUER", DEFAULT_ISSUER);
        let audience = env_or("CRM_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
        // `fetch_or` keeps the env-built verifier as the fallback on a
        // failed fetch, so the service always has a usable verifier.
        let fetched = fetch_or(url.trim(), &issuer, &audience, build_from_env()).await;
        verifier().store(fetched);
    }
}

/// The process-wide Keycloak verifier, present only when
/// `CRM_KEYCLOAK_URL` is set and valid.
#[cfg(feature = "keycloak")]
pub(super) static KEYCLOAK: OnceLock<authentication_verifier::keycloak::KeycloakVerifier> =
    OnceLock::new();

/// The Keycloak verifier, or `None` when Keycloak is not configured (or was
/// configured badly — see [`init`]).
#[cfg(feature = "keycloak")]
#[must_use]
pub fn keycloak_verifier() -> Option<&'static authentication_verifier::keycloak::KeycloakVerifier> {
    KEYCLOAK.get()
}

/// Build the Keycloak verifier from `CRM_KEYCLOAK_*`. Fail-closed and
/// non-fatal, like the PASETO key loaders: a missing URL leaves Keycloak
/// off; a malformed configuration logs an error and leaves it off
/// (Keycloak tokens are then rejected), and the service still boots.
/// Must run inside the Tokio runtime (discovery starts in the background).
#[cfg(feature = "keycloak")]
pub(super) fn init_keycloak() {
    use authentication_verifier::keycloak::{KeycloakSettings, KeycloakVerifier};
    if KEYCLOAK.get().is_some() {
        return;
    }
    match KeycloakSettings::from_env("CRM") {
        Ok(None) => {}
        Ok(Some(settings)) => {
            let issuer = settings.expected_issuer();
            match KeycloakVerifier::new(settings) {
                Ok(verifier) => {
                    let _ = KEYCLOAK.set(verifier);
                    tracing::info!(%issuer, "auth: Keycloak access tokens accepted");
                }
                Err(error) => {
                    tracing::error!(%error, "auth: Keycloak configuration rejected; Keycloak tokens will be refused");
                }
            }
        }
        Err(error) => {
            tracing::error!(%error, "auth: CRM_KEYCLOAK_* is invalid; Keycloak tokens will be refused");
        }
    }
}

/// Default key-set refresh interval (seconds) when
/// `CRM_PASETO_KEYS_REFRESH_SECS` is unset. One hour — key rotation is
/// infrequent, so a slow poll suffices.
#[cfg(feature = "paseto")]
pub(super) const KEY_REFRESH_DEFAULT_SECS: u64 = 3600;

/// Spawn a background task that periodically re-fetches the published
/// key set from `CRM_PASETO_KEYS_URL` and swaps it into the live
/// [`verifier`], so a **key rotation** at the auth-service is picked up
/// **without restarting** this service. On a failed fetch it keeps the
/// current key set (a transient auth-service outage never locks callers
/// out). Interval from `CRM_PASETO_KEYS_REFRESH_SECS` (default
/// [`KEY_REFRESH_DEFAULT_SECS`]); **`0` disables** the loop.
///
/// A no-op when `CRM_PASETO_KEYS_URL` is unset (env-supplied keys have
/// nothing to re-fetch). Call once at boot (`app.rs::after_routes`).
pub fn spawn_key_refresh() {
    #[cfg(feature = "paseto")]
    spawn_paseto_key_refresh();
}

/// The `paseto` half of [`spawn_key_refresh`].
#[cfg(feature = "paseto")]
pub(super) fn spawn_paseto_key_refresh() {
    let Some(url) = std::env::var("CRM_PASETO_KEYS_URL")
        .ok()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        return;
    };
    let secs = std::env::var("CRM_PASETO_KEYS_REFRESH_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(KEY_REFRESH_DEFAULT_SECS);
    if secs == 0 {
        return; // explicitly disabled
    }
    let issuer = env_or("CRM_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("CRM_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(secs));
        ticker.tick().await; // the first tick is immediate — skip it
        loop {
            ticker.tick().await;
            match Verifier::from_paseto_keys_url(&url, &issuer, &audience).await {
                Ok(fetched) => {
                    tracing::info!(keys = fetched.key_count(), "refreshed PASETO key set");
                    verifier().store(fetched);
                }
                Err(error) => {
                    tracing::warn!(%error, "PASETO key-set refresh failed; keeping current keys");
                }
            }
        }
    });
    tracing::info!(secs, "polling CRM_PASETO_KEYS_URL for key rotation");
}

/// Build a verifier by fetching the published key set from `url`
/// ([`Verifier::from_paseto_keys_url`]); on success the fetched key set
/// wins (`tracing::info!`), on any fetch/parse failure the given
/// `fallback` verifier is returned after a `tracing::warn!` — never a
/// panic, so the caller always boots. Pure dependency injection (URL,
/// issuer, audience and fallback are all passed in), so it is testable
/// against a local HTTP listener without touching the process global.
#[cfg(feature = "paseto")]
pub async fn fetch_or(url: &str, issuer: &str, audience: &str, fallback: Verifier) -> Verifier {
    match Verifier::from_paseto_keys_url(url, issuer, audience).await {
        Ok(fetched) => {
            tracing::info!(
                url,
                keys = fetched.key_count(),
                "PASETO key set fetched over HTTP; fetched key set wins over the env key set"
            );
            fetched
        }
        Err(error) => {
            tracing::warn!(
                url,
                %error,
                "PASETO key set fetch failed; falling back to the env-configured key set"
            );
            fallback
        }
    }
}

/// Read env var `name`, treating unset/blank as absent and falling back
/// to `default`. Used for the issuer/audience so a blank value doesn't
/// override the sensible default.
#[cfg(feature = "paseto")]
pub(super) fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Build the process-wide [`Verifier`] from the environment: issuer,
/// audience, and the published key set. A missing/blank/unparseable key
/// set yields an empty key set (every token rejected) so the service still
/// boots without credentials configured.
#[cfg(feature = "paseto")]
pub(super) fn build_from_env() -> Verifier {
    let issuer = env_or("CRM_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("CRM_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    let keys = std::env::var("CRM_PASETO_KEYS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_paseto_keys_value(&keys, &issuer, &audience)
        .unwrap_or_else(|_| empty_verifier(&issuer, &audience))
}

/// A verifier with no keys: rejects every token until a real key set is
/// configured. Infallible — an empty `keys` array always parses.
#[cfg(feature = "paseto")]
pub(super) fn empty_verifier(issuer: &str, audience: &str) -> Verifier {
    let empty = serde_json::json!({ "keys": [] });
    Verifier::from_paseto_keys_value(&empty, issuer, audience).expect("empty key set always builds")
}
