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

/// Find or create the organization called `name` and return the `organization:` reference
/// (CRM-R43), like [`resolve_person`]: in `http` mode `POST {CRM_ORGANIZATION_SERVICE_URL}/api/organizations`
/// with `{"name"}`, expected to find or make one and answer `{"pid"}` (or `{"id"}`); in `stub`
/// mode a process-local organization is made, one for the same name (any case) being reused.
/// The organization service's contract is not confirmed (`spec/integrations.md`); this is the
/// same assumption made for persons.
///
/// # Errors
///
/// A short reason when the service is unconfigured, unreachable or answers something that is not
/// an organization reference.
pub async fn resolve_organization(name: &str) -> Result<String, String> {
    if mode() != Mode::Http {
        static KNOWN: std::sync::OnceLock<Mutex<HashMap<String, String>>> =
            std::sync::OnceLock::new();
        let mut known = KNOWN
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let urn = known
            .entry(name.trim().to_lowercase())
            .or_insert_with(|| format!("organization:{}", uuid::Uuid::new_v4()));
        return Ok(urn.clone());
    }
    let base = std::env::var("CRM_ORGANIZATION_SERVICE_URL")
        .map_err(|_| "CRM_ORGANIZATION_SERVICE_URL is not set".to_string())?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "no http client".to_string())?;
    let response = client
        .post(format!("{}/api/organizations", base.trim_end_matches('/')))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await
        .map_err(|_| "the organization service could not be reached".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "the organization service answered {}",
            response.status().as_u16()
        ));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "the organization service answer was not JSON".to_string())?;
    let id = ["pid", "id"]
        .iter()
        .find_map(|k| body.get(k).and_then(serde_json::Value::as_str))
        .ok_or_else(|| "the organization service gave no id".to_string())?;
    let urn = format!("organization:{id}");
    <EntityRef as std::str::FromStr>::from_str(&urn)
        .map_err(|_| "the organization service gave an id that is not a reference".to_string())?;
    Ok(urn)
}

/// A worker as the worker service lists it: the only two things a team is made from. Everything
/// else the service sends (pay, employment) is never read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamWorker {
    /// The worker's id there, which is the `worker:<id>` reference here.
    pub pid: String,
    /// The department the worker is in; empty when none.
    pub department: String,
}

/// Workers asked for per page of the worker service's list.
const WORKER_PAGE: usize = 200;
/// Most pages read in one import, so a runaway answer cannot loop forever. The worker service
/// refuses an offset above 10,000, which the 51st page of 200 reaches, so a longer list stops
/// here with a reason of ours instead of a bare `400` from there.
const MAX_WORKER_PAGES: usize = 51;

fn bridge_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "no http client".to_string())
}

/// Attach `token`, when there is one, as a bearer credential (CRM-R87) — but only to an address
/// that keeps it private: `https`, or `http` to this machine. A token is never sent in the clear
/// across a network, and the client follows no redirect, so it goes to the address configured and
/// nowhere else.
///
/// # Errors
///
/// A short reason, holding neither the token nor the address, when the address is not one the
/// token may be sent to.
fn with_credential(
    request: reqwest::RequestBuilder,
    base: &str,
    token: Option<&str>,
) -> Result<reqwest::RequestBuilder, String> {
    let Some(token) = token else {
        return Ok(request);
    };
    let url = reqwest::Url::parse(base)
        .map_err(|_| "the service address is not a URL, so no credential is sent".to_string())?;
    let local = match url.host() {
        Some(url::Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    if url.scheme() == "https" || (url.scheme() == "http" && local) {
        Ok(request.bearer_auth(token))
    } else {
        Err("a credential is only sent over https, or to this machine".to_string())
    }
}

/// Open a delivery idea in the project portfolio service (CRM-R72): `POST {CRM_PPM_SERVICE_URL}/api/ideas`
/// with `{"title", "pitch", "tags"}`, answered `{"pid"}`. The contract was checked against that
/// service's source (`strategy.rs`, `create_idea`) on 2026-10-28 and is not run against a live one.
/// With `CRM_PPM_SERVICE_TOKEN` set it sends that as a bearer credential (CRM-R87); without, it
/// reaches a service with authentication off.
///
/// # Errors
///
/// A short reason when the service is unconfigured, unreachable or answers something that is not
/// an id. The reason holds no deal data.
pub async fn open_delivery_idea(title: &str, pitch: &str, tags: &[&str]) -> Result<String, String> {
    let settings = crate::settings::current();
    let base = settings
        .ppm_service_url
        .clone()
        .ok_or_else(|| "CRM_PPM_SERVICE_URL is not set".to_string())?;
    let response = with_credential(
        bridge_client()?
            .post(format!("{}/api/ideas", base.trim_end_matches('/')))
            .json(&serde_json::json!({ "title": title, "pitch": pitch, "tags": tags })),
        &base,
        settings.ppm_service_token.as_deref(),
    )?
    .send()
    .await
    .map_err(|_| "the project portfolio service could not be reached".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "the project portfolio service answered {}",
            response.status().as_u16()
        ));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "the project portfolio service answer was not JSON".to_string())?;
    let id = ["pid", "id"]
        .iter()
        .find_map(|k| body.get(k).and_then(serde_json::Value::as_str))
        .ok_or_else(|| "the project portfolio service gave no id".to_string())?;
    uuid::Uuid::parse_str(id)
        .map(|u| u.to_string())
        .map_err(|_| "the project portfolio service gave an id that is not a UUID".to_string())
}

/// The active workers of the worker service, with their department (CRM-R73): `GET
/// {CRM_WORKER_SERVICE_URL}/api/workers?status=active&limit=&offset=`, read page by page. Checked
/// against that service's source (`hr_core.rs`, `list_workers`: a bare JSON array, `pid` and
/// `department` members, a page of at most 500, an offset of at most 10,000, `active` a real status)
/// on 2026-10-28; not run against a live one. With `CRM_WORKER_SERVICE_TOKEN` set it sends that as a
/// bearer credential (CRM-R87), and the list is then the one that token may see.
///
/// # Errors
///
/// A short reason when the service is unconfigured, unreachable, answers something that is not a
/// list, or has more than [`MAX_WORKER_PAGES`] pages.
pub async fn active_workers() -> Result<Vec<UpstreamWorker>, String> {
    let settings = crate::settings::current();
    let base = settings
        .worker_service_url
        .clone()
        .ok_or_else(|| "CRM_WORKER_SERVICE_URL is not set".to_string())?;
    let client = bridge_client()?;
    let mut workers = Vec::new();
    for page in 0..MAX_WORKER_PAGES {
        let response = with_credential(
            client.get(format!(
                "{}/api/workers?status=active&limit={WORKER_PAGE}&offset={}",
                base.trim_end_matches('/'),
                page * WORKER_PAGE
            )),
            &base,
            settings.worker_service_token.as_deref(),
        )?
        .send()
        .await
        .map_err(|_| "the worker service could not be reached".to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "the worker service answered {}",
                response.status().as_u16()
            ));
        }
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "the worker service answer was not JSON".to_string())?;
        let rows = body
            .as_array()
            .ok_or_else(|| "the worker service answer was not a list".to_string())?;
        for row in rows {
            let text = |key: &str| row.get(key).and_then(serde_json::Value::as_str);
            if let Some(pid) = text("pid") {
                workers.push(UpstreamWorker {
                    pid: pid.to_string(),
                    department: text("department").unwrap_or_default().trim().to_string(),
                });
            }
        }
        if rows.len() < WORKER_PAGE {
            return Ok(workers);
        }
    }
    Err("the worker service has more workers than one import reads".to_string())
}

/// Ask the worker service which of `person_refs` (at most 500) are workers of `organization_ref`
/// today (CRM-R94, WPM-R208): `POST {CRM_WORKER_SERVICE_URL}/api/bridges/overlap` with
/// `{"organization_ref", "person_refs"}`, answered `{"organization_ref", "results": [{"person_ref",
/// "worker"}]}`. Checked against that service's source (`controllers/bridges.rs`, `rules/bridge.rs`)
/// on 2026-10-30; not run against a live one. WPM requires an HR caller of the organization, so
/// `CRM_WORKER_SERVICE_TOKEN` is sent as the bearer credential (CRM-R87, CRM-D16). Only references
/// are sent, never a name.
///
/// # Errors
///
/// A short reason, naming no reference, when the service is unconfigured, unreachable, refuses
/// the call or answers something that is not JSON.
pub async fn overlap_check(
    organization_ref: &str,
    person_refs: &[String],
) -> Result<serde_json::Value, String> {
    let settings = crate::settings::current();
    let base = settings
        .worker_service_url
        .clone()
        .ok_or_else(|| "CRM_WORKER_SERVICE_URL is not set".to_string())?;
    let response = with_credential(
        bridge_client()?
            .post(format!(
                "{}/api/bridges/overlap",
                base.trim_end_matches('/')
            ))
            .json(&serde_json::json!({
                "organization_ref": organization_ref,
                "person_refs": person_refs,
            })),
        &base,
        settings.worker_service_token.as_deref(),
    )?
    .send()
    .await
    .map_err(|_| "the worker service could not be reached".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "the worker service answered {}",
            response.status().as_u16()
        ));
    }
    response
        .json()
        .await
        .map_err(|_| "the worker service answer was not JSON".to_string())
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
    #[test]
    fn a_credential_goes_only_over_https_or_to_this_machine() {
        let client = reqwest::Client::new();
        let sent = |base: &str, token: Option<&str>| {
            super::with_credential(client.get("http://example.invalid/"), base, token).map(|r| {
                r.build()
                    .unwrap()
                    .headers()
                    .get("authorization")
                    .map(|v| v.to_str().unwrap().to_string())
            })
        };
        for base in [
            "https://ppm.example.com",
            "http://localhost:5150",
            "http://127.0.0.1:1",
            "http://[::1]:5150",
        ] {
            assert_eq!(
                sent(base, Some("t0ken")),
                Ok(Some("Bearer t0ken".into())),
                "{base}"
            );
        }
        for base in [
            "http://ppm.example.com",
            "http://10.0.0.5:5150",
            "ftp://x",
            "not a url",
        ] {
            let why = sent(base, Some("t0ken")).unwrap_err();
            assert!(!why.contains("t0ken") && !why.contains(base), "{why}");
        }
        // Without a token nothing is attached, whatever the address.
        assert_eq!(sent("http://ppm.example.com", None), Ok(None));
    }

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
