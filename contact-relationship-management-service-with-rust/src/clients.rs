//! Upstream service clients (CRM-D11): **stub-first** display-name
//! resolution for `person:` / `worker:` `EntityRefs`.
//!
//! The CRM references identities by URN and needs only display labels.
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
use std::time::{Duration, Instant};

use entity_ref::EntityRef;

/// Client mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// No network; every lookup misses (default).
    Stub,
    /// HTTP lookups against the sibling services.
    Http,
}

/// The process-wide mode, from `CRM_UPSTREAM_MODE` (see [`crate::settings`]).
#[must_use]
pub fn mode() -> Mode {
    crate::settings::current().upstream_mode
}

/// How long a looked-up display name is kept. A name is personal data and can
/// change upstream, so it is not kept for the life of the process.
pub const NAME_TTL: Duration = Duration::from_secs(600);

/// The most display names kept at once.
pub const NAME_CAPACITY: usize = 10_000;

/// A bounded display-name cache: each entry expires after `ttl`, and when it is
/// full the oldest entry makes room. The clock is passed in, so tests need not
/// sleep.
struct NameCache {
    ttl: Duration,
    capacity: usize,
    entries: HashMap<String, (String, Instant)>,
}

impl NameCache {
    fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            ttl,
            capacity,
            entries: HashMap::new(),
        }
    }

    /// The name for `urn`, unless it has expired (an expired entry is dropped).
    fn get(&mut self, urn: &str, now: Instant) -> Option<String> {
        let (name, stored) = self.entries.get(urn)?;
        if now.saturating_duration_since(*stored) < self.ttl {
            return Some(name.clone());
        }
        self.entries.remove(urn);
        None
    }

    fn insert(&mut self, urn: String, name: String, now: Instant) {
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&urn) {
            let ttl = self.ttl;
            self.entries
                .retain(|_, (_, stored)| now.saturating_duration_since(*stored) < ttl);
            if self.entries.len() >= self.capacity {
                let oldest = self
                    .entries
                    .iter()
                    .min_by_key(|(_, (_, stored))| *stored)
                    .map(|(key, _)| key.clone());
                if let Some(key) = oldest {
                    self.entries.remove(&key);
                }
            }
        }
        self.entries.insert(urn, (name, now));
    }
}

/// Process-wide display-name cache (URN → name).
fn cache() -> &'static Mutex<NameCache> {
    static CACHE: OnceLock<Mutex<NameCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(NameCache::new(NAME_TTL, NAME_CAPACITY)))
}

/// Resolve a display name for `entity_ref`, best-effort: the cache,
/// then (in `http` mode) the owning service, else `None`. Failures
/// are logged at debug and swallowed — upstream reads never fail a
/// CRM request.
pub async fn display_name(entity_ref: &EntityRef) -> Option<String> {
    let urn = entity_ref.to_string();
    if let Some(hit) = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&urn, Instant::now())
    {
        return Some(hit);
    }
    if mode() != Mode::Http {
        return None;
    }
    let fetched = fetch_display_name(entity_ref).await;
    if let Some(name) = &fetched {
        cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(urn, name.clone(), Instant::now());
    }
    fetched
}

/// Seed the cache (used by tests and the seed task).
pub fn prime(urn: &str, name: &str) {
    cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(urn.to_string(), name.to_string(), Instant::now());
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
    let settings = crate::settings::current();
    let domain = settings.mail_dev_domain.as_deref()?;
    Some(format!("{}@{domain}", entity_ref.id))
}

/// Find or create the person for `email` and return the `person:` reference (CRM-R31).
///
/// The CRM stores no address, so a web form's address goes to the person service: in `http`
/// mode `POST {CRM_PERSON_SERVICE_URL}/api/persons` with `{"name", "email"}`, which is expected
/// to find the person by address or make one and answer `{"pid"}` (or `{"id"}`). In `stub` mode
/// a process-local person is made and its address primed, an existing one for the same address
/// being reused; that is lost on restart and is for development.
///
/// # Errors
///
/// A short reason when the person service is unconfigured, unreachable or answers something
/// that is not a person reference. The reason holds no address.
pub async fn resolve_person(name: &str, email: &str) -> Result<String, String> {
    if mode() != Mode::Http {
        let mut known = addresses()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((urn, _)) = known.iter().find(|(_, a)| a.eq_ignore_ascii_case(email)) {
            return Ok(urn.clone());
        }
        let urn = format!("person:{}", uuid::Uuid::new_v4());
        known.insert(urn.clone(), email.to_string());
        return Ok(urn);
    }
    let base = std::env::var("CRM_PERSON_SERVICE_URL")
        .map_err(|_| "CRM_PERSON_SERVICE_URL is not set".to_string())?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "no http client".to_string())?;
    let response = client
        .post(format!("{}/api/persons", base.trim_end_matches('/')))
        .json(&serde_json::json!({ "name": name, "email": email }))
        .send()
        .await
        .map_err(|_| "the person service could not be reached".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "the person service answered {}",
            response.status().as_u16()
        ));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "the person service answer was not JSON".to_string())?;
    let id = ["pid", "id"]
        .iter()
        .find_map(|k| body.get(k).and_then(serde_json::Value::as_str))
        .ok_or_else(|| "the person service gave no id".to_string())?;
    let urn = format!("person:{id}");
    <EntityRef as std::str::FromStr>::from_str(&urn)
        .map_err(|_| "the person service gave an id that is not a reference".to_string())?;
    Ok(urn)
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

    #[test]
    fn a_name_expires_after_the_ttl() {
        let start = Instant::now();
        let mut cache = NameCache::new(Duration::from_secs(60), 10);
        cache.insert("person:a".into(), "Ada".into(), start);
        assert_eq!(
            cache
                .get("person:a", start + Duration::from_secs(59))
                .as_deref(),
            Some("Ada")
        );
        assert_eq!(cache.get("person:a", start + Duration::from_secs(60)), None);
        assert!(cache.entries.is_empty(), "an expired entry is dropped");
    }

    #[test]
    fn a_full_cache_drops_expired_names_first_then_the_oldest() {
        let start = Instant::now();
        let at = |s: u64| start + Duration::from_secs(s);
        let mut cache = NameCache::new(Duration::from_secs(100), 3);
        cache.insert("person:1".into(), "One".into(), at(0));
        cache.insert("person:2".into(), "Two".into(), at(10));
        cache.insert("person:3".into(), "Three".into(), at(20));
        // Full, and nothing has expired: the oldest (1) makes room.
        cache.insert("person:4".into(), "Four".into(), at(30));
        assert_eq!(cache.entries.len(), 3);
        assert_eq!(cache.get("person:1", at(30)), None);
        assert!(cache.get("person:4", at(30)).is_some());
        // Full again, but 2 and 3 have expired by now: both go, not just one.
        cache.insert("person:5".into(), "Five".into(), at(125));
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.get("person:4", at(125)).is_some());
        assert!(cache.get("person:5", at(125)).is_some());
    }

    #[test]
    fn replacing_a_name_in_a_full_cache_evicts_nothing() {
        let start = Instant::now();
        let mut cache = NameCache::new(Duration::from_secs(100), 2);
        cache.insert("person:1".into(), "One".into(), start);
        cache.insert("person:2".into(), "Two".into(), start);
        cache.insert("person:1".into(), "Uno".into(), start);
        assert_eq!(cache.get("person:1", start).as_deref(), Some("Uno"));
        assert_eq!(cache.get("person:2", start).as_deref(), Some("Two"));
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
