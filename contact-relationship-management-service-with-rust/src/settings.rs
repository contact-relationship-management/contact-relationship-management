//! The service's settings (CRM-T78): every `CONTACT_RELATIONSHIP_MANAGEMENT_*` environment variable that
//! was read lazily, in one typed struct.
//!
//! [`Settings::from_lookup`] parses them from any `name -> value` function, so
//! tests build one without touching the process environment (which is `unsafe`
//! to change in edition 2024, and leaks between tests). The process holds one
//! snapshot: [`current`] returns it, built from the environment on first use,
//! and [`install`] or [`update`] replace it (start-up and tests).
//!
//! Not here, on purpose:
//! - the startup-only settings that already take a lookup function: mail
//!   (`mail::from_settings`), unsubscribe links, and the rate limit
//!   (`rate_limit::Guard::from_env`);
//! - `CONTACT_RELATIONSHIP_MANAGEMENT_KEYCLOAK_*`, which the `authentication-verifier` library reads itself;
//! - the per-service base URLs (`CONTACT_RELATIONSHIP_MANAGEMENT_PERSON_SERVICE_URL` and its siblings),
//!   whose variable name depends on the kind of record asked about.

use std::sync::{Arc, PoisonError, RwLock};

use crate::clients::Mode;
use crate::rules::bounds::within;
use crate::streaming::EventTransport;

/// Shortest accepted inbound-webhook secret.
pub const MIN_WEBHOOK_SECRET: usize = 32;

/// Every lazily-read setting.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is its own environment variable"
)]
pub struct Settings {
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH`: blanket `/api/*` enforcement.
    pub require_auth: bool,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PROTECT_OPERATIONAL`: the metrics and `OpenAPI` endpoints need a token.
    pub protect_operational: bool,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_ALLOW_INSECURE`: production may start with authentication off.
    pub allow_insecure: bool,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_FIELD_ENCRYPTION`: refuse to start without field keys.
    pub require_field_encryption: bool,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_UPSTREAM_MODE`: `http` looks names up; anything else is the stub.
    pub upstream_mode: Mode,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS`, raw: how long the event outbox and webhook deliveries are kept
    /// (parsed and floored by the retention rules; CRM-R81).
    pub event_retention_days: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS`, raw: how long email deliveries and events are kept (CRM-R81).
    pub mail_retention_days: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_MIN_CELL_SIZE`: a report leaves out a group with fewer records than this (CRM-R78):
    /// 5 by default, 0 for none, at most 1000.
    pub min_cell_size: i64,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PPM_SERVICE_URL`: the project portfolio service the bridge sends delivery ideas to.
    pub ppm_service_url: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_URL`: the worker service (WPM) the team import reads workers from.
    pub worker_service_url: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PPM_SERVICE_TOKEN`: a bearer token sent to the project portfolio service, when that has
    /// authentication on (CRM-R87). Issued by the deployer's identity provider; never logged.
    pub ppm_service_token: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_TOKEN`: the same for the worker service (CRM-R87).
    pub worker_service_token: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WPM_ORGANIZATION_REF`: the organization the employment check asks the worker service
    /// about (CRM-R94), an `organization:` URN; anything else is taken as unset. It is the
    /// organization whose HR identity holds `CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_TOKEN` (CRM-D16).
    pub wpm_organization_ref: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_TRANSPORT`: `memory` (default) or `outbox`.
    pub event_transport: EventTransport,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS`, raw (parsed and clamped by the retention rules).
    pub retention_days: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_BATCH`: contacts handled per send call, 1 to 5000 (default 200).
    pub mail_batch: i64,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WEBHOOK_SECRET`, only when long enough ([`MIN_WEBHOOK_SECRET`]).
    pub webhook_secret: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WPM_WEBHOOK_SECRET`: the secret the worker service signs its events with (CRM-R92),
    /// only when long enough ([`MIN_WEBHOOK_SECRET`]). Unset, `POST /api/bridges/wpm/events` is `404`.
    pub wpm_webhook_secret: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_SIGNING_KEY`, when not blank.
    pub mailgun_signing_key: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_DEV_DOMAIN`: development catch-all domain for stub-mode addresses.
    pub mail_dev_domain: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS`: the published key set, as JSON.
    pub paseto_keys: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_URL`: where to fetch the key set.
    pub paseto_keys_url: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_REFRESH_SECS`: key re-fetch interval; `0` disables.
    pub paseto_keys_refresh_secs: Option<u64>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_TOKEN_ISSUER`.
    pub token_issuer: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_TOKEN_AUDIENCE`.
    pub token_audience: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY`: the policy, inline as JSON.
    pub abac_policy: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY_FILE`: the policy file (watched for changes).
    pub abac_policy_file: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS`: field-encryption keys.
    pub field_keys: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS_FILE`: a file holding them.
    pub field_keys_file: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_CORS_ORIGIN`: the one browser origin allowed to call the service directly.
    pub cors_origin: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
    pub redis_url: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_MCP_WRITES`: offer the MCP write tools (off: the MCP endpoint is read-only).
    pub mcp_writes: bool,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_MCP_ALLOWED_ORIGINS`: browser origins the MCP endpoint answers (comma-separated;
    /// none by default, so only clients that send no `Origin` header, such as agents, connect).
    pub mcp_allowed_origins: Vec<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORMS`: the web forms (CRM-R31), a JSON list; entries that do not parse or fail
    /// [`WebForm::valid`] are left out.
    pub web_forms: Vec<WebForm>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_CLIENT_LIMIT`: submissions per client per form in 10 minutes (default 5).
    pub web_form_client_limit: u32,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_LIMIT`: submissions per form per hour, all clients (default 300).
    pub web_form_limit: u32,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PUBLIC_KB`: serve the published knowledge-base articles to anyone, without a token,
    /// under `/kb/` (CRM-R57). Off by default: nothing is public until it is turned on.
    pub public_kb: bool,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_PUBLIC_KB_CLIENT_LIMIT`: public knowledge-base requests per client per minute
    /// (default 120; `0` turns the limit off).
    pub public_kb_client_limit: u32,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_ATTACHMENT_DIR`: where attached files are kept. Unset, attachments are off.
    pub attachment_dir: Option<String>,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_ATTACHMENT_MAX_BYTES`: the largest file (default 10 MiB, at most 100 MiB).
    pub attachment_max_bytes: u64,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_ATTACHMENT_SCAN_COMMAND`: a program (and arguments, split on spaces, no shell) run
    /// with the path of each upload as its last argument: exit 0 clean, 1 infected, anything else
    /// is a failure. Unset, no scan is run.
    pub attachment_scan_command: Option<String>,
    /// The numeric settings whose value was not a number or lay outside its range. Each holds its
    /// default here, and start-up refuses while any is named; the value is never kept.
    pub unusable: Vec<&'static str>,
}

/// One web form (CRM-R31).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct WebForm {
    /// The name in the URL: lower-case letters, digits and hyphens.
    pub slug: String,
    /// The origins that may post it, exactly (`https://example.org`, no path).
    pub origins: Vec<String>,
    /// The consent purpose the form asks for.
    #[serde(default = "default_purpose")]
    pub purpose: String,
    /// The nurture sequence to enrol into once the visitor confirms.
    #[serde(default)]
    pub sequence: Option<uuid::Uuid>,
}

fn default_purpose() -> String {
    crate::rules::tokens::DEFAULT_PURPOSE.to_string()
}

impl WebForm {
    /// Whether the entry can be served: a short slug, a known purpose, and at least one origin.
    #[must_use]
    pub fn valid(&self) -> bool {
        !self.slug.is_empty()
            && self.slug.len() <= 40
            && self
                .slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && crate::rules::tokens::PURPOSES.contains(&self.purpose.as_str())
            && !self.origins.is_empty()
    }

    /// Whether `origin` (as a browser sends it) is one of this form's.
    #[must_use]
    pub fn allows(&self, origin: &str) -> bool {
        let origin = origin.trim().trim_end_matches('/');
        self.origins
            .iter()
            .any(|o| o.trim().trim_end_matches('/').eq_ignore_ascii_case(origin))
    }
}

impl Default for Settings {
    /// Everything unset: what a bare environment gives.
    fn default() -> Self {
        Self::from_lookup(&|_| None)
    }
}

/// Lenient boolean: `1`, `true`, `yes` or `on` (any case, trimmed) is true;
/// anything else, including unset, is false.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// A numeric setting: `default` when it is not set at all, the value when it is a number in `range`, and
/// otherwise (a blank value included, RCA-IN-3) `default` with `name` recorded in `unusable` so that start-up refuses. A
/// setting that cannot be read is an error and is never moved to a bound or quietly defaulted.
fn number<T: std::str::FromStr + PartialOrd>(
    get: &dyn Fn(&str) -> Option<String>,
    name: &'static str,
    range: std::ops::RangeInclusive<T>,
    default: T,
    unusable: &mut Vec<&'static str>,
) -> T {
    let Some(raw) = get(name) else {
        return default;
    };
    if let Some(n) = raw.trim().parse::<T>().ok().and_then(|n| within(n, range)) {
        n
    } else {
        unusable.push(name);
        default
    }
}

/// The event transport: `memory` when it is not set at all, the one named when it is `memory` or
/// `outbox`, and otherwise `memory` with the setting recorded in `unusable`, so that start-up refuses
/// (RCA-IN-4).
fn event_transport(
    get: &dyn Fn(&str) -> Option<String>,
    unusable: &mut Vec<&'static str>,
) -> EventTransport {
    const NAME: &str = "CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_TRANSPORT";
    let Some(raw) = get(NAME) else {
        return EventTransport::Memory;
    };
    EventTransport::parse(&raw).unwrap_or_else(|| {
        unusable.push(NAME);
        EventTransport::Memory
    })
}

/// A numeric setting with no default: `None` when it is not set at all, the number when it is one,
/// and otherwise `None` with `name` recorded in `unusable` so that start-up refuses (RCA-IN-4).
fn optional_number<T: std::str::FromStr>(
    get: &dyn Fn(&str) -> Option<String>,
    name: &'static str,
    unusable: &mut Vec<&'static str>,
) -> Option<T> {
    let raw = get(name)?;
    let parsed = raw.trim().parse::<T>().ok();
    if parsed.is_none() {
        unusable.push(name);
    }
    parsed
}

/// A retention setting, kept as written: not set at all is `None`; a value that is not a whole number
/// of days at or above `floor` is recorded in `unusable` (RCA-IN-4) and not raised to the floor.
fn horizon_setting(
    get: &dyn Fn(&str) -> Option<String>,
    name: &'static str,
    floor: i64,
    unusable: &mut Vec<&'static str>,
) -> Option<String> {
    let raw = get(name)?;
    if !crate::rules::privacy::valid_horizon(&raw, floor) {
        unusable.push(name);
    }
    Some(raw)
}

impl Settings {
    /// Parse the settings from `get`, a `name -> value` function. Unset and
    /// blank are the same; a number that does not parse or lies outside its range
    /// is named in [`Settings::unusable`], and start-up refuses.
    #[must_use]
    #[expect(clippy::too_many_lines, reason = "one field for each setting")]
    pub fn from_lookup(get: &dyn Fn(&str) -> Option<String>) -> Self {
        let text = |name: &str| get(name).filter(|v| !v.trim().is_empty());
        let mut unusable: Vec<&'static str> = Vec::new();
        let flag = |name: &str| get(name).is_some_and(|v| parse_bool(&v));
        Self {
            require_auth: flag("CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH"),
            protect_operational: flag("CONTACT_RELATIONSHIP_MANAGEMENT_PROTECT_OPERATIONAL"),
            allow_insecure: flag("CONTACT_RELATIONSHIP_MANAGEMENT_ALLOW_INSECURE"),
            require_field_encryption: flag(
                "CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_FIELD_ENCRYPTION",
            ),
            upstream_mode: if get("CONTACT_RELATIONSHIP_MANAGEMENT_UPSTREAM_MODE")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("http"))
            {
                Mode::Http
            } else {
                Mode::Stub
            },
            event_retention_days: horizon_setting(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS",
                crate::rules::privacy::EVENT_FLOOR_DAYS,
                &mut unusable,
            ),
            mail_retention_days: horizon_setting(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS",
                crate::rules::privacy::MAIL_FLOOR_DAYS,
                &mut unusable,
            ),
            min_cell_size: number(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_MIN_CELL_SIZE",
                0..=1000,
                5,
                &mut unusable,
            ),
            ppm_service_url: text("CONTACT_RELATIONSHIP_MANAGEMENT_PPM_SERVICE_URL"),
            worker_service_url: text("CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_URL"),
            ppm_service_token: text("CONTACT_RELATIONSHIP_MANAGEMENT_PPM_SERVICE_TOKEN"),
            worker_service_token: text("CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_TOKEN"),
            wpm_organization_ref: text("CONTACT_RELATIONSHIP_MANAGEMENT_WPM_ORGANIZATION_REF")
                .map(|v| v.trim().to_string())
                .filter(|v| {
                    <entity_ref::EntityRef as std::str::FromStr>::from_str(v)
                        .is_ok_and(|r| r.entity_type == entity_ref::EntityType::Organization)
                }),
            event_transport: event_transport(get, &mut unusable),
            retention_days: horizon_setting(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS",
                crate::rules::privacy::RETENTION_FLOOR_DAYS,
                &mut unusable,
            ),
            mail_batch: number(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_BATCH",
                1..=5000,
                200,
                &mut unusable,
            ),
            webhook_secret: get("CONTACT_RELATIONSHIP_MANAGEMENT_WEBHOOK_SECRET")
                .filter(|s| s.len() >= MIN_WEBHOOK_SECRET),
            wpm_webhook_secret: get("CONTACT_RELATIONSHIP_MANAGEMENT_WPM_WEBHOOK_SECRET")
                .filter(|s| s.len() >= MIN_WEBHOOK_SECRET),
            mailgun_signing_key: text("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_SIGNING_KEY"),
            mail_dev_domain: text("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_DEV_DOMAIN")
                .map(|v| v.trim().to_string()),
            paseto_keys: text("CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS"),
            paseto_keys_url: text("CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_URL")
                .map(|v| v.trim().to_string()),
            paseto_keys_refresh_secs: optional_number(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_REFRESH_SECS",
                &mut unusable,
            ),
            token_issuer: text("CONTACT_RELATIONSHIP_MANAGEMENT_TOKEN_ISSUER"),
            token_audience: text("CONTACT_RELATIONSHIP_MANAGEMENT_TOKEN_AUDIENCE"),
            abac_policy: text("CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY"),
            abac_policy_file: text("CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY_FILE")
                .map(|v| v.trim().to_string()),
            field_keys: text("CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS"),
            field_keys_file: get("CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS_FILE"),
            cors_origin: text("CONTACT_RELATIONSHIP_MANAGEMENT_CORS_ORIGIN"),
            redis_url: text("CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL"),
            mcp_writes: flag("CONTACT_RELATIONSHIP_MANAGEMENT_MCP_WRITES"),
            mcp_allowed_origins: text("CONTACT_RELATIONSHIP_MANAGEMENT_MCP_ALLOWED_ORIGINS")
                .map_or_default(|v| {
                    v.split(',')
                        .map(|o| o.trim().trim_end_matches('/').to_ascii_lowercase())
                        .filter(|o| !o.is_empty())
                        .collect()
                }),
            web_forms: text("CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORMS")
                .and_then(|v| serde_json::from_str::<Vec<WebForm>>(&v).ok())
                .map_or_default(|forms| forms.into_iter().filter(WebForm::valid).collect()),
            web_form_client_limit: number(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_CLIENT_LIMIT",
                0..=u32::MAX,
                5,
                &mut unusable,
            ),
            web_form_limit: number(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_LIMIT",
                0..=u32::MAX,
                300,
                &mut unusable,
            ),
            public_kb: flag("CONTACT_RELATIONSHIP_MANAGEMENT_PUBLIC_KB"),
            public_kb_client_limit: number(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_PUBLIC_KB_CLIENT_LIMIT",
                0..=u32::MAX,
                120,
                &mut unusable,
            ),
            attachment_dir: text("CONTACT_RELATIONSHIP_MANAGEMENT_ATTACHMENT_DIR")
                .map(|v| v.trim().to_string()),
            attachment_max_bytes: number(
                get,
                "CONTACT_RELATIONSHIP_MANAGEMENT_ATTACHMENT_MAX_BYTES",
                1..=100 * 1024 * 1024,
                10 * 1024 * 1024,
                &mut unusable,
            ),
            attachment_scan_command: text(
                "CONTACT_RELATIONSHIP_MANAGEMENT_ATTACHMENT_SCAN_COMMAND",
            )
            .map(|v| v.trim().to_string()),
            unusable,
        }
    }

    /// `Err` naming each numeric setting that was blank, not a number or out of range, so that start-up
    /// stops (RCA-IN-4). The message names the settings and never their values.
    ///
    /// # Errors
    ///
    /// When [`Settings::unusable`] is not empty.
    pub fn refuse_unusable(&self) -> Result<(), String> {
        if self.unusable.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} must be a number within its range",
                self.unusable.join(", ")
            ))
        }
    }

    /// The settings in the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        // A value that is not valid text is read lossily, so a numeric setting fails to parse and is
        // refused, and is not taken as unset (RCA-IN-2).
        Self::from_lookup(&|name| {
            std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
        })
    }
}

/// A snapshot that can be replaced. The process has one ([`current`]); tests
/// of the holder itself use their own.
struct Holder {
    slot: RwLock<Option<Arc<Settings>>>,
}

impl Holder {
    const fn new() -> Self {
        Self {
            slot: RwLock::new(None),
        }
    }

    /// The snapshot, built from the environment the first time anything asks.
    fn current(&self) -> Arc<Settings> {
        if let Some(settings) = self
            .slot
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            return Arc::clone(settings);
        }
        let mut held = self.slot.write().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(held.get_or_insert_with(|| Arc::new(Settings::from_env())))
    }

    fn install(&self, settings: Settings) {
        *self.slot.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(settings));
    }

    fn update(&self, edit: impl FnOnce(&mut Settings)) {
        let mut next = Settings::clone(&self.current());
        edit(&mut next);
        self.install(next);
    }
}

static PROCESS: Holder = Holder::new();

/// The process's settings: built from the environment the first time anything
/// asks, unless [`install`] got there first.
#[must_use]
pub fn current() -> Arc<Settings> {
    PROCESS.current()
}

/// Replace the process's settings (start-up, and tests).
pub fn install(settings: Settings) {
    PROCESS.install(settings);
}

/// Change the process's settings in place: `edit` gets a copy of the current
/// ones, and the result is installed. Tests use it to set one thing.
pub fn update(edit: impl FnOnce(&mut Settings)) {
    PROCESS.update(edit);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn a_bare_environment_gives_the_safe_defaults() {
        let s = Settings::default();
        assert!(!s.require_auth && !s.protect_operational && !s.allow_insecure);
        assert!(!s.require_field_encryption);
        assert_eq!(s.upstream_mode, Mode::Stub);
        assert_eq!(s.event_transport, EventTransport::Memory);
        assert_eq!(s.mail_batch, 200);
        assert_eq!(s.webhook_secret, None);
        assert_eq!(s.wpm_webhook_secret, None);
        assert_eq!(s.wpm_organization_ref, None);
        assert_eq!(s.paseto_keys, None);
    }

    #[test]
    fn flags_are_lenient_and_everything_else_is_off() {
        for on in ["1", "true", "YES", " on "] {
            assert!(
                Settings::from_lookup(&lookup(&[(
                    "CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH",
                    on
                )]))
                .require_auth,
                "{on:?}"
            );
        }
        for off in ["", "0", "no", "enabled", "2"] {
            assert!(
                !Settings::from_lookup(&lookup(&[(
                    "CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH",
                    off
                )]))
                .require_auth,
                "{off:?}"
            );
        }
    }

    #[test]
    fn web_forms_are_read_from_json_and_bad_entries_are_left_out() {
        let json = r#"[
            {"slug": "news", "origins": ["https://a.example/"], "purpose": "newsletter"},
            {"slug": "plain", "origins": ["https://a.example"]},
            {"slug": "Bad Slug", "origins": ["https://a.example"]},
            {"slug": "no-origin", "origins": []},
            {"slug": "odd-purpose", "origins": ["https://a.example"], "purpose": "spam"}
        ]"#;
        let s = Settings::from_lookup(&lookup(&[(
            "CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORMS",
            json,
        )]));
        let slugs: Vec<&str> = s.web_forms.iter().map(|f| f.slug.as_str()).collect();
        assert_eq!(slugs, ["news", "plain"]);
        assert_eq!(s.web_forms[1].purpose, "marketing");
        assert!(s.web_forms[0].allows("HTTPS://A.example"));
        assert!(!s.web_forms[0].allows("https://a.example.evil.test"));
        assert_eq!((s.web_form_client_limit, s.web_form_limit), (5, 300));
        // The public knowledge base is off until asked for, with a limit of 120 a minute.
        assert!(!s.public_kb);
        assert_eq!(s.public_kb_client_limit, 120);
        let on = Settings::from_lookup(&lookup(&[
            ("CONTACT_RELATIONSHIP_MANAGEMENT_PUBLIC_KB", "yes"),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_PUBLIC_KB_CLIENT_LIMIT",
                "7",
            ),
        ]));
        assert!(on.public_kb);
        assert_eq!(on.public_kb_client_limit, 7);
        // Not JSON: no forms, rather than a failed start.
        let broken = Settings::from_lookup(&lookup(&[(
            "CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORMS",
            "news=https://a",
        )]));
        assert!(broken.web_forms.is_empty());
    }

    #[test]
    fn a_blank_value_is_the_same_as_unset() {
        let s = Settings::from_lookup(&lookup(&[
            ("CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS", "  "),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY", ""),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL", " "),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_SIGNING_KEY", "\t"),
        ]));
        assert_eq!(s.paseto_keys, None);
        assert_eq!(s.abac_policy, None);
        assert_eq!(s.redis_url, None);
        assert_eq!(s.mailgun_signing_key, None);
    }

    /// a number outside its range, or not a number, is named for start-up to refuse; it
    /// is neither moved to a bound nor quietly defaulted.
    #[test]
    fn a_numeric_setting_out_of_range_or_junk_is_named_unusable() {
        let batch = "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_BATCH";
        let cell = "CONTACT_RELATIONSHIP_MANAGEMENT_MIN_CELL_SIZE";
        let bytes = "CONTACT_RELATIONSHIP_MANAGEMENT_ATTACHMENT_MAX_BYTES";
        let get = |key: &str, v: &str| Settings::from_lookup(&lookup(&[(key, v)]));
        // In range, at both ends: used, and nothing is named.
        for (key, v) in [
            (batch, "50"),
            (batch, "1"),
            (batch, "5000"),
            (cell, "0"),
            (cell, "1000"),
            (bytes, "1"),
            (bytes, "104857600"),
        ] {
            assert!(get(key, v).unusable.is_empty(), "{key}={v}");
        }
        assert_eq!(get(batch, "5000").mail_batch, 5000);
        assert_eq!(get(cell, "1000").min_cell_size, 1000);
        assert_eq!(get(bytes, "104857600").attachment_max_bytes, 104_857_600);
        // Not set at all is the default and not an error; a blank value is supplied and invalid.
        assert!(Settings::default().unusable.is_empty());
        assert_eq!(get(batch, "").unusable, vec![batch]);
        for (key, bad) in [
            ("CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_LIMIT", "many"),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_CLIENT_LIMIT",
                "-1",
            ),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_PUBLIC_KB_CLIENT_LIMIT", ""),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_REFRESH_SECS",
                "soon",
            ),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS", "29"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS", "6"),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS",
                "junk",
            ),
        ] {
            assert_eq!(get(key, bad).unusable, vec![key], "{key}={bad}");
        }
        for (key, good) in [
            ("CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_LIMIT", "0"),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_REFRESH_SECS",
                "0",
            ),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS", "30"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS", "7"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS", "400"),
        ] {
            assert!(get(key, good).unusable.is_empty(), "{key}={good}");
        }
        assert_eq!(get(batch, "  ").unusable, vec![batch]);
        // Out of range, or not a number: named.
        for (key, v) in [
            (batch, "0"),
            (batch, "-5"),
            (batch, "5001"),
            (batch, "lots"),
            (cell, "-1"),
            (cell, "1001"),
            (bytes, "0"),
            (bytes, "104857601"),
        ] {
            assert_eq!(get(key, v).unusable, vec![key], "{key}={v}");
        }
    }

    #[test]
    fn the_event_transport_is_memory_unset_and_a_junk_one_is_refused() {
        let name = "CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_TRANSPORT";
        let get = |v: &str| Settings::from_lookup(&lookup(&[(name, v)]));
        assert_eq!(Settings::default().event_transport, EventTransport::Memory);
        assert!(Settings::default().unusable.is_empty());
        assert_eq!(get(" Outbox ").event_transport, EventTransport::Outbox);
        assert!(get("memory").unusable.is_empty());
        for bad in ["", "junk", "kafka"] {
            assert_eq!(get(bad).unusable, vec![name], "{bad:?}");
        }
    }

    #[test]
    fn start_up_names_the_settings_it_refuses_and_never_their_values() {
        assert_eq!(Settings::default().refuse_unusable(), Ok(()));
        let bad = Settings::from_lookup(&lookup(&[
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_BATCH", "99999999"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_WEB_FORM_LIMIT", "many"),
        ]));
        let refused = bad.refuse_unusable().unwrap_err();
        assert!(
            refused.contains("MAIL_BATCH") && refused.contains("WEB_FORM_LIMIT"),
            "{refused}"
        );
        assert!(
            !refused.contains("99999999") && !refused.contains("many"),
            "{refused}"
        );
    }

    #[test]
    fn a_short_webhook_secret_is_ignored() {
        let short = "x".repeat(MIN_WEBHOOK_SECRET - 1);
        let long = "x".repeat(MIN_WEBHOOK_SECRET);
        assert_eq!(
            Settings::from_lookup(&lookup(&[(
                "CONTACT_RELATIONSHIP_MANAGEMENT_WEBHOOK_SECRET",
                &short
            )]))
            .webhook_secret,
            None
        );
        assert_eq!(
            Settings::from_lookup(&lookup(&[(
                "CONTACT_RELATIONSHIP_MANAGEMENT_WEBHOOK_SECRET",
                &long
            )]))
            .webhook_secret,
            Some(long.clone())
        );
        // The worker service's secret is its own setting, with the same floor (CRM-R92).
        let wpm = |v: &str| {
            Settings::from_lookup(&lookup(&[(
                "CONTACT_RELATIONSHIP_MANAGEMENT_WPM_WEBHOOK_SECRET",
                v,
            )]))
            .wpm_webhook_secret
        };
        assert_eq!(wpm(&short), None);
        assert_eq!(wpm(&long), Some(long.clone()));
        assert_eq!(
            Settings::from_lookup(&lookup(&[(
                "CONTACT_RELATIONSHIP_MANAGEMENT_WEBHOOK_SECRET",
                &long
            )]))
            .wpm_webhook_secret,
            None,
            "the provider secret does not open the worker service's route"
        );
    }

    #[test]
    fn modes_transports_and_numbers_parse() {
        let s = Settings::from_lookup(&lookup(&[
            ("CONTACT_RELATIONSHIP_MANAGEMENT_UPSTREAM_MODE", " HTTP "),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_TRANSPORT", "Outbox"),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_REFRESH_SECS",
                " 0 ",
            ),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY_FILE",
                " /etc/crm/policy.json ",
            ),
        ]));
        assert_eq!(s.upstream_mode, Mode::Http);
        assert_eq!(s.event_transport, EventTransport::Outbox);
        assert_eq!(s.paseto_keys_refresh_secs, Some(0));
        assert_eq!(s.abac_policy_file.as_deref(), Some("/etc/crm/policy.json"));
        let junk = Settings::from_lookup(&lookup(&[(
            "CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_REFRESH_SECS",
            "soon",
        )]));
        assert_eq!(junk.paseto_keys_refresh_secs, None);
    }

    #[test]
    fn update_changes_one_setting_and_keeps_the_rest() {
        let holder = Holder::new();
        holder.install(Settings::from_lookup(&lookup(&[(
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_BATCH",
            "7",
        )])));
        holder.update(|s| s.require_auth = true);
        let now = holder.current();
        assert!(now.require_auth);
        assert_eq!(now.mail_batch, 7);
    }

    #[test]
    fn a_snapshot_taken_before_an_update_does_not_change() {
        let holder = Holder::new();
        holder.install(Settings::default());
        let before = holder.current();
        holder.update(|s| s.require_auth = true);
        assert!(!before.require_auth);
        assert!(holder.current().require_auth);
    }

    #[test]
    fn the_employment_checks_organization_must_be_an_organization_reference() {
        let org = |v: &str| {
            Settings::from_lookup(&lookup(&[(
                "CONTACT_RELATIONSHIP_MANAGEMENT_WPM_ORGANIZATION_REF",
                v,
            )]))
            .wpm_organization_ref
        };
        let good = "organization:0c4f1e2a-0000-4000-8000-0000000000aa";
        assert_eq!(org(good), Some(good.to_string()));
        assert_eq!(org(&format!("  {good} ")), Some(good.to_string()));
        assert_eq!(org("person:0c4f1e2a-0000-4000-8000-0000000000aa"), None);
        assert_eq!(org("Acme Ltd"), None);
        assert_eq!(org("  "), None);
    }
}
