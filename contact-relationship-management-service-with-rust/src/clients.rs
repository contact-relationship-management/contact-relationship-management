//! Upstream service clients (WPM-D11): **stub-first** display-name
//! resolution for `person:` / `worker:` `EntityRefs`.
//!
//! WPM references identities by URN and needs only display labels.
//! Lookups are read-only, cached, best-effort, and **never block
//! writes**: an unreachable upstream degrades a view to showing the
//! URN.
//!
//! Selection is per-process via `CRM_UPSTREAM_MODE`:
//! - `stub` (default) — no network; returns `None` so callers fall
//!   back to the caller-supplied or URN-derived label.
//! - `http` — `GET {base}/api/<plural>/{pid}` against
//!   `CRM_PERSON_SERVICE_URL` / `CRM_WORKER_SERVICE_URL` /
//!   `CRM_ORGANIZATION_SERVICE_URL` / `CRM_COURSE_SERVICE_URL`,
//!   reading a best-effort `name`-ish field. (Thin by design; the
//!   family front-ends do the rich rendering.)

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use entity_ref::EntityRef;

/// Client mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// No network; every lookup misses (default).
    Stub,
    /// HTTP lookups against the sibling services.
    Http,
}

/// The process-wide mode, from `CRM_UPSTREAM_MODE`.
#[must_use]
pub fn mode() -> Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    *MODE.get_or_init(|| {
        match std::env::var("CRM_UPSTREAM_MODE")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "http" => Mode::Http,
            _ => Mode::Stub,
        }
    })
}

/// Process-wide display-name cache (URN → name).
fn cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve a display name for `entity_ref`, best-effort: the cache,
/// then (in `http` mode) the owning service, else `None`. Failures
/// are logged at debug and swallowed — upstream reads never fail an
/// WPM request.
pub async fn display_name(entity_ref: &EntityRef) -> Option<String> {
    let urn = entity_ref.to_string();
    if let Some(hit) = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&urn)
    {
        return Some(hit.clone());
    }
    if mode() != Mode::Http {
        return None;
    }
    let fetched = fetch_display_name(entity_ref).await;
    if let Some(name) = &fetched {
        cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(urn, name.clone());
    }
    fetched
}

/// Seed the cache (used by tests and the seed task).
pub fn prime(urn: &str, name: &str) {
    cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(urn.to_string(), name.to_string());
}

/// Process-wide primed addresses (URN → mailbox), for tests and local
/// development. Never filled from a lookup: an address is personal data and is
/// asked for at send time, not kept.
fn addresses() -> &'static Mutex<HashMap<String, String>> {
    static ADDRESSES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    ADDRESSES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Seed an address for tests and local development.
pub fn prime_email(urn: &str, address: &str) {
    addresses()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(urn.to_string(), address.to_string());
}

/// The mailbox to send to for `person_ref` (CRM-T39), or `None`.
///
/// The CRM holds no addresses (identity data stays in the person service), so
/// this asks for one each time: a primed address (tests, development), then in
/// `http` mode the person service's `email` field, then, in `stub` mode only,
/// `<id>@$CRM_MAIL_DEV_DOMAIN` when that is set (for a local mail server with a
/// catch-all). Failures are swallowed like the display-name lookups, and the
/// caller records the contact as having no address.
pub async fn email_address(person_ref: &str) -> Option<String> {
    if let Some(hit) = addresses()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(person_ref)
    {
        return Some(hit.clone());
    }
    let entity_ref = <EntityRef as std::str::FromStr>::from_str(person_ref).ok()?;
    if mode() == Mode::Http {
        let base = std::env::var(base_url_var(&entity_ref)).ok()?;
        let url = format!(
            "{}/api/{}s/{}",
            base.trim_end_matches('/'),
            entity_ref.entity_type.as_str(),
            entity_ref.id
        );
        let body = reqwest_get_json(&url).await?;
        return extract_email(&body);
    }
    let domain = std::env::var("CRM_MAIL_DEV_DOMAIN").ok()?;
    let domain = domain.trim();
    (!domain.is_empty()).then(|| format!("{}@{domain}", entity_ref.id))
}

/// The mailbox in a person record. The accepted shapes are `{"email": "a@b"}`
/// and `{"emails": [...]}` where each entry is a string or an object with an
/// `address`, `email` or `value` member; an entry marked `primary` (or
/// `is_primary`) wins, otherwise the first usable one. The person service's
/// real contract is not confirmed (see `spec/integrations.md`), so this is
/// deliberately a little tolerant and nothing else is guessed.
#[must_use]
pub fn extract_email(record: &serde_json::Value) -> Option<String> {
    use serde_json::Value;
    let usable = |text: &str| {
        let text = text.trim();
        (text.contains('@') && !text.contains(char::is_whitespace)).then(|| text.to_string())
    };
    if let Some(found) = record.get("email").and_then(Value::as_str).and_then(usable) {
        return Some(found);
    }
    let entries = record.get("emails")?.as_array()?;
    let address_of = |entry: &Value| match entry {
        Value::String(text) => usable(text),
        Value::Object(map) => ["address", "email", "value"]
            .iter()
            .find_map(|key| map.get(*key).and_then(Value::as_str).and_then(usable)),
        _ => None,
    };
    let primary = |entry: &Value| {
        ["primary", "is_primary"]
            .iter()
            .any(|key| entry.get(*key).and_then(Value::as_bool) == Some(true))
    };
    entries
        .iter()
        .filter(|entry| primary(entry))
        .find_map(address_of)
        .or_else(|| entries.iter().find_map(address_of))
}

/// The env var naming the base URL for an entity type's service.
fn base_url_var(entity_ref: &EntityRef) -> String {
    format!(
        "CRM_{}_SERVICE_URL",
        entity_ref.entity_type.as_str().to_ascii_uppercase()
    )
}

/// One HTTP lookup against the owning service (`http` mode only).
async fn fetch_display_name(entity_ref: &EntityRef) -> Option<String> {
    let base = std::env::var(base_url_var(entity_ref)).ok()?;
    let url = format!(
        "{}/api/{}s/{}",
        base.trim_end_matches('/'),
        entity_ref.entity_type.as_str(),
        entity_ref.id
    );
    let body: serde_json::Value = reqwest_get_json(&url).await?;
    // Best-effort name extraction across the family's DTO shapes.
    ["display_name", "name", "title"]
        .iter()
        .find_map(|k| body.get(k).and_then(serde_json::Value::as_str))
        .map(ToString::to_string)
}

/// Minimal GET-JSON helper: short timeout, no redirects (SSRF posture,
/// security invariant 7). Errors ⇒ `None` (best-effort).
async fn reqwest_get_json(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;
    match client.get(url).send().await {
        Ok(response) => response.json().await.ok(),
        Err(error) => {
            tracing::debug!(%url, %error, "upstream lookup failed; degrading to URN");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// Stub mode misses; a primed cache hits regardless of mode.
    #[tokio::test]
    async fn a_primed_address_is_found_and_an_unknown_one_is_not() {
        let urn = format!("person:{}", uuid::Uuid::new_v4());
        assert_eq!(email_address(&urn).await, None);
        prime_email(&urn, "ada@example.test");
        assert_eq!(
            email_address(&urn).await.as_deref(),
            Some("ada@example.test")
        );
        assert_eq!(email_address("not a urn").await, None);
    }

    #[tokio::test]
    async fn stub_mode_misses_and_cache_hits() {
        let urn = format!("person:{}", uuid::Uuid::new_v4());
        let entity_ref = EntityRef::from_str(&urn).unwrap();
        assert_eq!(display_name(&entity_ref).await, None);
        prime(&urn, "Ada Lovelace");
        assert_eq!(
            display_name(&entity_ref).await.as_deref(),
            Some("Ada Lovelace")
        );
    }
}
