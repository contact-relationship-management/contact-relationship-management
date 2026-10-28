//! The service's settings (CRM-T78): every `CRM_*` environment variable that
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
//! - `CRM_KEYCLOAK_*`, which the `authentication-verifier` library reads itself;
//! - the per-service base URLs (`CRM_PERSON_SERVICE_URL` and its siblings),
//!   whose variable name depends on the kind of record asked about.

use std::sync::{Arc, PoisonError, RwLock};

use crate::clients::Mode;
use crate::streaming::EventTransport;

/// Shortest accepted inbound-webhook secret.
pub const MIN_WEBHOOK_SECRET: usize = 32;

/// Every lazily-read setting.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is its own environment variable"
)]
pub struct Settings {
    /// `CRM_REQUIRE_AUTH`: blanket `/api/*` enforcement.
    pub require_auth: bool,
    /// `CRM_PROTECT_OPERATIONAL`: the metrics and `OpenAPI` endpoints need a token.
    pub protect_operational: bool,
    /// `CRM_ALLOW_INSECURE`: production may start with authentication off.
    pub allow_insecure: bool,
    /// `CRM_REQUIRE_FIELD_ENCRYPTION`: refuse to start without field keys.
    pub require_field_encryption: bool,
    /// `CRM_TRUST_FORWARDED_FOR`: count callers by `X-Forwarded-For`.
    pub trust_forwarded_for: bool,
    /// `CRM_UPSTREAM_MODE`: `http` looks names up; anything else is the stub.
    pub upstream_mode: Mode,
    /// `CRM_EVENT_RETENTION_DAYS`, raw: how long the event outbox and webhook deliveries are kept
    /// (parsed and floored by the retention rules; CRM-R81).
    pub event_retention_days: Option<String>,
    /// `CRM_MAIL_RETENTION_DAYS`, raw: how long email deliveries and events are kept (CRM-R81).
    pub mail_retention_days: Option<String>,
    /// `CRM_MIN_CELL_SIZE`: a report leaves out a group with fewer records than this (CRM-R78):
    /// 5 by default, 0 for none, at most 1000.
    pub min_cell_size: i64,
    /// `CRM_PPM_SERVICE_URL`: the project portfolio service the bridge sends delivery ideas to.
    pub ppm_service_url: Option<String>,
    /// `CRM_WORKER_SERVICE_URL`: the worker service (WPM) the team import reads workers from.
    pub worker_service_url: Option<String>,
    /// `CRM_PPM_SERVICE_TOKEN`: a bearer token sent to the project portfolio service, when that has
    /// authentication on (CRM-R87). Issued by the deployer's identity provider; never logged.
    pub ppm_service_token: Option<String>,
    /// `CRM_WORKER_SERVICE_TOKEN`: the same for the worker service (CRM-R87).
    pub worker_service_token: Option<String>,
    /// `CRM_EVENT_TRANSPORT`: `memory` (default) or `outbox`.
    pub event_transport: EventTransport,
    /// `CRM_RETENTION_DAYS`, raw (parsed and clamped by the retention rules).
    pub retention_days: Option<String>,
    /// `CRM_MAIL_BATCH`: contacts handled per send call, 1 to 5000 (default 200).
    pub mail_batch: i64,
    /// `CRM_WEBHOOK_SECRET`, only when long enough ([`MIN_WEBHOOK_SECRET`]).
    pub webhook_secret: Option<String>,
    /// `CRM_MAILGUN_SIGNING_KEY`, when not blank.
    pub mailgun_signing_key: Option<String>,
    /// `CRM_MAIL_DEV_DOMAIN`: development catch-all domain for stub-mode addresses.
    pub mail_dev_domain: Option<String>,
    /// `CRM_PASETO_KEYS`: the published key set, as JSON.
    pub paseto_keys: Option<String>,
    /// `CRM_PASETO_KEYS_URL`: where to fetch the key set.
    pub paseto_keys_url: Option<String>,
    /// `CRM_PASETO_KEYS_REFRESH_SECS`: key re-fetch interval; `0` disables.
    pub paseto_keys_refresh_secs: Option<u64>,
    /// `CRM_TOKEN_ISSUER`.
    pub token_issuer: Option<String>,
    /// `CRM_TOKEN_AUDIENCE`.
    pub token_audience: Option<String>,
    /// `CRM_ABAC_POLICY`: the policy, inline as JSON.
    pub abac_policy: Option<String>,
    /// `CRM_ABAC_POLICY_FILE`: the policy file (watched for changes).
    pub abac_policy_file: Option<String>,
    /// `CRM_FIELD_KEYS`: field-encryption keys.
    pub field_keys: Option<String>,
    /// `CRM_FIELD_KEYS_FILE`: a file holding them.
    pub field_keys_file: Option<String>,
    /// `CRM_REDIS_URL`.
    pub redis_url: Option<String>,
    /// `CRM_MCP_WRITES`: offer the MCP write tools (off: the MCP endpoint is read-only).
    pub mcp_writes: bool,
    /// `CRM_MCP_ALLOWED_ORIGINS`: browser origins the MCP endpoint answers (comma-separated;
    /// none by default, so only clients that send no `Origin` header, such as agents, connect).
    pub mcp_allowed_origins: Vec<String>,
    /// `CRM_WEB_FORMS`: the web forms (CRM-R31), a JSON list; entries that do not parse or fail
    /// [`WebForm::valid`] are left out.
    pub web_forms: Vec<WebForm>,
    /// `CRM_WEB_FORM_CLIENT_LIMIT`: submissions per client per form in 10 minutes (default 5).
    pub web_form_client_limit: u32,
    /// `CRM_WEB_FORM_LIMIT`: submissions per form per hour, all clients (default 300).
    pub web_form_limit: u32,
    /// `CRM_PUBLIC_KB`: serve the published knowledge-base articles to anyone, without a token,
    /// under `/kb/` (CRM-R57). Off by default: nothing is public until it is turned on.
    pub public_kb: bool,
    /// `CRM_PUBLIC_KB_CLIENT_LIMIT`: public knowledge-base requests per client per minute
    /// (default 120; `0` turns the limit off).
    pub public_kb_client_limit: u32,
    /// `CRM_ATTACHMENT_DIR`: where attached files are kept. Unset, attachments are off.
    pub attachment_dir: Option<String>,
    /// `CRM_ATTACHMENT_MAX_BYTES`: the largest file (default 10 MiB, at most 100 MiB).
    pub attachment_max_bytes: u64,
    /// `CRM_ATTACHMENT_SCAN_COMMAND`: a program (and arguments, split on spaces, no shell) run
    /// with the path of each upload as its last argument: exit 0 clean, 1 infected, anything else
    /// is a failure. Unset, no scan is run.
    pub attachment_scan_command: Option<String>,
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

impl Settings {
    /// Parse the settings from `get`, a `name -> value` function. Unset and
    /// blank are the same; an unparsable number falls back to its default,
    /// as it always did.
    #[must_use]
    pub fn from_lookup(get: &dyn Fn(&str) -> Option<String>) -> Self {
        let text = |name: &str| get(name).filter(|v| !v.trim().is_empty());
        let flag = |name: &str| get(name).is_some_and(|v| parse_bool(&v));
        Self {
            require_auth: flag("CRM_REQUIRE_AUTH"),
            protect_operational: flag("CRM_PROTECT_OPERATIONAL"),
            allow_insecure: flag("CRM_ALLOW_INSECURE"),
            require_field_encryption: flag("CRM_REQUIRE_FIELD_ENCRYPTION"),
            trust_forwarded_for: flag("CRM_TRUST_FORWARDED_FOR"),
            upstream_mode: if get("CRM_UPSTREAM_MODE")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("http"))
            {
                Mode::Http
            } else {
                Mode::Stub
            },
            event_retention_days: get("CRM_EVENT_RETENTION_DAYS"),
            mail_retention_days: get("CRM_MAIL_RETENTION_DAYS"),
            min_cell_size: get("CRM_MIN_CELL_SIZE")
                .and_then(|v| v.trim().parse::<i64>().ok())
                .map_or(5, |n| n.clamp(0, 1000)),
            ppm_service_url: text("CRM_PPM_SERVICE_URL"),
            worker_service_url: text("CRM_WORKER_SERVICE_URL"),
            ppm_service_token: text("CRM_PPM_SERVICE_TOKEN"),
            worker_service_token: text("CRM_WORKER_SERVICE_TOKEN"),
            event_transport: EventTransport::parse(&get("CRM_EVENT_TRANSPORT").unwrap_or_default()),
            retention_days: get("CRM_RETENTION_DAYS"),
            mail_batch: get("CRM_MAIL_BATCH")
                .and_then(|v| v.trim().parse::<i64>().ok())
                .map_or(200, |n| n.clamp(1, 5000)),
            webhook_secret: get("CRM_WEBHOOK_SECRET").filter(|s| s.len() >= MIN_WEBHOOK_SECRET),
            mailgun_signing_key: text("CRM_MAILGUN_SIGNING_KEY"),
            mail_dev_domain: text("CRM_MAIL_DEV_DOMAIN").map(|v| v.trim().to_string()),
            paseto_keys: text("CRM_PASETO_KEYS"),
            paseto_keys_url: text("CRM_PASETO_KEYS_URL").map(|v| v.trim().to_string()),
            paseto_keys_refresh_secs: get("CRM_PASETO_KEYS_REFRESH_SECS")
                .and_then(|v| v.trim().parse::<u64>().ok()),
            token_issuer: text("CRM_TOKEN_ISSUER"),
            token_audience: text("CRM_TOKEN_AUDIENCE"),
            abac_policy: text("CRM_ABAC_POLICY"),
            abac_policy_file: text("CRM_ABAC_POLICY_FILE").map(|v| v.trim().to_string()),
            field_keys: text("CRM_FIELD_KEYS"),
            field_keys_file: get("CRM_FIELD_KEYS_FILE"),
            redis_url: text("CRM_REDIS_URL"),
            mcp_writes: flag("CRM_MCP_WRITES"),
            mcp_allowed_origins: text("CRM_MCP_ALLOWED_ORIGINS")
                .map(|v| {
                    v.split(',')
                        .map(|o| o.trim().trim_end_matches('/').to_ascii_lowercase())
                        .filter(|o| !o.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            web_forms: text("CRM_WEB_FORMS")
                .and_then(|v| serde_json::from_str::<Vec<WebForm>>(&v).ok())
                .map(|forms| forms.into_iter().filter(WebForm::valid).collect())
                .unwrap_or_default(),
            web_form_client_limit: get("CRM_WEB_FORM_CLIENT_LIMIT")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(5),
            web_form_limit: get("CRM_WEB_FORM_LIMIT")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(300),
            public_kb: flag("CRM_PUBLIC_KB"),
            public_kb_client_limit: get("CRM_PUBLIC_KB_CLIENT_LIMIT")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(120),
            attachment_dir: text("CRM_ATTACHMENT_DIR").map(|v| v.trim().to_string()),
            attachment_max_bytes: get("CRM_ATTACHMENT_MAX_BYTES")
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map_or(10 * 1024 * 1024, |n| n.clamp(1, 100 * 1024 * 1024)),
            attachment_scan_command: text("CRM_ATTACHMENT_SCAN_COMMAND")
                .map(|v| v.trim().to_string()),
        }
    }

    /// The settings in the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_lookup(&|name| std::env::var(name).ok())
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
        assert!(!s.require_field_encryption && !s.trust_forwarded_for);
        assert_eq!(s.upstream_mode, Mode::Stub);
        assert_eq!(s.event_transport, EventTransport::Memory);
        assert_eq!(s.mail_batch, 200);
        assert_eq!(s.webhook_secret, None);
        assert_eq!(s.paseto_keys, None);
    }

    #[test]
    fn flags_are_lenient_and_everything_else_is_off() {
        for on in ["1", "true", "YES", " on "] {
            assert!(
                Settings::from_lookup(&lookup(&[("CRM_REQUIRE_AUTH", on)])).require_auth,
                "{on:?}"
            );
        }
        for off in ["", "0", "no", "enabled", "2"] {
            assert!(
                !Settings::from_lookup(&lookup(&[("CRM_REQUIRE_AUTH", off)])).require_auth,
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
        let s = Settings::from_lookup(&lookup(&[("CRM_WEB_FORMS", json)]));
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
            ("CRM_PUBLIC_KB", "yes"),
            ("CRM_PUBLIC_KB_CLIENT_LIMIT", "7"),
        ]));
        assert!(on.public_kb);
        assert_eq!(on.public_kb_client_limit, 7);
        // Not JSON: no forms, rather than a failed start.
        let broken = Settings::from_lookup(&lookup(&[("CRM_WEB_FORMS", "news=https://a")]));
        assert!(broken.web_forms.is_empty());
    }

    #[test]
    fn a_blank_value_is_the_same_as_unset() {
        let s = Settings::from_lookup(&lookup(&[
            ("CRM_PASETO_KEYS", "  "),
            ("CRM_ABAC_POLICY", ""),
            ("CRM_REDIS_URL", " "),
            ("CRM_MAILGUN_SIGNING_KEY", "\t"),
        ]));
        assert_eq!(s.paseto_keys, None);
        assert_eq!(s.abac_policy, None);
        assert_eq!(s.redis_url, None);
        assert_eq!(s.mailgun_signing_key, None);
    }

    #[test]
    fn the_mail_batch_is_clamped_and_junk_is_the_default() {
        let batch = |v: &str| Settings::from_lookup(&lookup(&[("CRM_MAIL_BATCH", v)])).mail_batch;
        assert_eq!(batch("50"), 50);
        assert_eq!(batch("0"), 1);
        assert_eq!(batch("-5"), 1);
        assert_eq!(batch("999999"), 5000);
        assert_eq!(batch("lots"), 200);
    }

    #[test]
    fn a_short_webhook_secret_is_ignored() {
        let short = "x".repeat(MIN_WEBHOOK_SECRET - 1);
        let long = "x".repeat(MIN_WEBHOOK_SECRET);
        assert_eq!(
            Settings::from_lookup(&lookup(&[("CRM_WEBHOOK_SECRET", &short)])).webhook_secret,
            None
        );
        assert_eq!(
            Settings::from_lookup(&lookup(&[("CRM_WEBHOOK_SECRET", &long)])).webhook_secret,
            Some(long.clone())
        );
    }

    #[test]
    fn modes_transports_and_numbers_parse() {
        let s = Settings::from_lookup(&lookup(&[
            ("CRM_UPSTREAM_MODE", " HTTP "),
            ("CRM_EVENT_TRANSPORT", "Outbox"),
            ("CRM_PASETO_KEYS_REFRESH_SECS", " 0 "),
            ("CRM_ABAC_POLICY_FILE", " /etc/crm/policy.json "),
        ]));
        assert_eq!(s.upstream_mode, Mode::Http);
        assert_eq!(s.event_transport, EventTransport::Outbox);
        assert_eq!(s.paseto_keys_refresh_secs, Some(0));
        assert_eq!(s.abac_policy_file.as_deref(), Some("/etc/crm/policy.json"));
        let junk = Settings::from_lookup(&lookup(&[("CRM_PASETO_KEYS_REFRESH_SECS", "soon")]));
        assert_eq!(junk.paseto_keys_refresh_secs, None);
    }

    #[test]
    fn update_changes_one_setting_and_keeps_the_rest() {
        let holder = Holder::new();
        holder.install(Settings::from_lookup(&lookup(&[("CRM_MAIL_BATCH", "7")])));
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
}
