//! Blanket enforcement and the access policy: which routes need a token, what action a
//! request is, and who the policy lets through.

use super::{
    Action, Claims, ENTITY, HeaderMap, Method, OnceLock, Policy, ReloadablePolicy, StatusCode,
    request_claims,
};
#[cfg(feature = "paseto")]
use super::{Verifier, bearer_claims};

/// Path suffixes of this crate's **destructive named POSTs** (per
/// `authorization-attributes.md` §2, fixed family-wide): record merge
/// (`POST /api/wards/merge`, live today), batch deduplicate, and bulk
/// import — the latter two listed ahead of the corresponding features
/// (dedup scan and bulk import are §13 work), so the guard is already
/// correct when they land — plus contact erasure (`/erase`) and the
/// retention sweep (`/sweep`, CRM-R20), and undoing a contact merge
/// (`/unmerge`, CRM-R26). A POST whose path ends with
/// one of these derives [`Action::Destructive`] instead of
/// [`Action::Write`].
pub const DESTRUCTIVE_POST_SUFFIXES: [&str; 7] = [
    "/merge",
    "/unmerge",
    "/deduplicate",
    "/import",
    "/erase",
    "/sweep",
    "/review",
];

/// Path prefixes whose **reads** are operational or compliance material
/// (the audit trail and its verification, the event stream, the retention
/// report; CRM-T38). A `GET` under one of these derives
/// [`Action::Destructive`], so only `svc=true` or `access=admin` reach it
/// under the shipped policies; an ordinary reader, manager, marketing or
/// support caller does not. (The shared engine has no "privileged read"
/// action, and `Destructive` is already the admin/service tier.)
pub const PRIVILEGED_READ_PREFIXES: [&str; 4] = [
    "/api/audits",
    "/api/events",
    "/api/retention",
    "/api/break-glass",
];

/// Path prefixes that only an admin or service caller may use with *any* method
/// (outbound webhook subscriptions, CRM-R28: they decide where change notifications go; quotas and
/// commission plans, CRM-R59: what a person is paid is neither theirs to set nor everyone's to read;
/// the bridges to other services, CRM-R72: they send data out and read people in). Every
/// request under one derives [`Action::Destructive`].
pub const ADMIN_ONLY_PREFIXES: [&str; 4] = [
    "/api/webhook-subscriptions",
    "/api/quotas",
    "/api/commission-plans",
    "/api/bridges",
];

/// Whether `path` is under an [`ADMIN_ONLY_PREFIXES`] entry, on whole path segments.
pub(super) fn is_admin_only(path: &str) -> bool {
    ADMIN_ONLY_PREFIXES.iter().any(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Whether `path` is under a [`PRIVILEGED_READ_PREFIXES`] entry, matched on
/// whole path segments so `/api/auditsfoo` does not qualify.
pub(super) fn is_privileged_read(path: &str) -> bool {
    PRIVILEGED_READ_PREFIXES.iter().any(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Whether blanket `/api/*` enforcement is on (`CRM_REQUIRE_AUTH`, from
/// [`crate::settings`]). Off by default — see the module docs and
/// `agents/share/jwt-enforcement.md`.
#[must_use]
pub fn require_auth() -> bool {
    crate::settings::current().require_auth
}

/// Lenient boolean parse: `1`/`true`/`yes`/`on` (case-insensitive,
/// surrounding whitespace ignored) ⇒ `true`; everything else
/// (incl. empty) ⇒ `false`.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    crate::settings::parse_bool(value)
}

/// Paths that stay public even when enforcement is on: health/ping, the
/// `OpenAPI` doc + Swagger UI, and the Prometheus metrics endpoint (so a
/// scraper needs no bearer token). Everything else requires a valid bearer
/// token.
pub(super) fn is_public_path(path: &str) -> bool {
    path == "/_health"
        || path == "/_ping"
        || path.starts_with("/unsubscribe/")
        || path.starts_with("/confirm/")
        || path.starts_with("/preferences/")
        || path.starts_with("/forms/")
        || path.starts_with("/kb/")
        || path == "/webhooks/email"
        || path.starts_with("/webhooks/email/")
        || path == "/webhooks/inbound-email"
        // Signed by the worker service, not sent with a session (CRM-R92).
        || path == "/api/bridges/wpm/events"
        || (!protect_operational() && is_operational_path(path))
}

/// The operational endpoints that are public by default: the `OpenAPI` doc,
/// Swagger UI, and Prometheus metrics.
pub(super) fn is_operational_path(path: &str) -> bool {
    path == "/api-docs/openapi.json" || path.starts_with("/swagger-ui") || path == "/metrics.prom"
}

/// Whether `CRM_PROTECT_OPERATIONAL` is on (CRM-T38): the operational
/// endpoints then require a bearer token like everything else, and a metrics
/// scraper must present one. Off by default so a scraper inside a private
/// network keeps working.
pub(super) fn protect_operational() -> bool {
    crate::settings::current().protect_operational
}

/// Derive the request's ABAC action from its HTTP method and path (per
/// `authorization-attributes.md` §2): `GET`/`HEAD`/`OPTIONS` ⇒ `Read`;
/// `DELETE` ⇒ `Delete`; a `POST` whose path ends with a
/// [`DESTRUCTIVE_POST_SUFFIXES`] entry ⇒ `Destructive`; every other
/// `POST`/`PUT`/`PATCH` (and any unrecognised method) ⇒ `Write`.
#[must_use]
pub fn derive_action(method: &Method, path: &str) -> Action {
    // SEC-G6: normalise a trailing slash before the destructive-suffix
    // check, so `POST /api/wards/merge/` (and `//`) is still classified as
    // `Destructive` rather than silently downgraded to `Write` — which would
    // let an `access=write` (non-admin) caller reach a destructive op.
    let path = path.trim_end_matches('/');
    if is_admin_only(path) {
        return Action::Destructive;
    }
    match *method {
        // Taking a break-glass grant is open to any authenticated caller
        // (CRM-T44); the grant, not the route, carries the privilege.
        Method::POST if path == "/api/break-glass" => Action::Read,
        // Marking your own notifications read changes only your inbox (CRM-R32), so a reader may.
        Method::PUT if path.starts_with("/api/notifications/") && path.ends_with("/read") => {
            Action::Read
        }
        Method::POST if path == "/api/notifications/read-all" => Action::Read,
        // Your own saved views are not CRM data (CRM-R35): a reader may make, change and delete them.
        Method::POST | Method::PUT | Method::DELETE
            if path == "/api/views" || path.starts_with("/api/views/") =>
        {
            Action::Read
        }
        // A report is a read; your own saved reports are not CRM data (CRM-R48).
        Method::POST if path == "/api/reports/run" => Action::Read,
        Method::POST | Method::PUT | Method::DELETE
            if path == "/api/reports" || path.starts_with("/api/reports/") =>
        {
            Action::Read
        }
        // A preview renders a draft for a made-up contact and stores nothing (CRM-R36).
        Method::POST if path == "/api/email-templates/preview" => Action::Read,
        Method::GET | Method::HEAD | Method::OPTIONS if is_privileged_read(path) => {
            Action::Destructive
        }
        Method::GET | Method::HEAD | Method::OPTIONS => Action::Read,
        Method::DELETE => Action::Delete,
        Method::POST
            if DESTRUCTIVE_POST_SUFFIXES
                .iter()
                .any(|suffix| path.ends_with(suffix)) =>
        {
            Action::Destructive
        }
        _ => Action::Write,
    }
}

/// Load the ABAC policy: `CRM_ABAC_POLICY` (inline JSON) wins, then
/// `CRM_ABAC_POLICY_FILE` (path to a JSON file), else the built-in
/// default policy. A present-but-unparsable policy (bad JSON, unknown
/// effect/action names, unreadable file) `tracing::warn!`s and falls
/// back to the default — the service always boots, matching the
/// key-fetch posture. Read once per process via [`policy`]; restart to
/// change.
#[must_use]
pub fn policy_from_env() -> Policy {
    let settings = crate::settings::current();
    let source = settings
        .abac_policy
        .clone()
        .or_else(|| {
            let path = settings.abac_policy_file.as_deref()?;
            match std::fs::read_to_string(path) {
                Ok(contents) => Some(contents),
                Err(error) => {
                    tracing::warn!(%error, %path, "ABAC policy file unreadable; using the built-in default policy");
                    None
                }
            }
        });
    match source {
        Some(json) => Policy::from_json(&json).unwrap_or_else(|error| {
            tracing::warn!(%error, "ABAC policy JSON invalid; using the built-in default policy");
            Policy::default_policy()
        }),
        None => Policy::default_policy(),
    }
}

/// The process-wide **hot-reloadable** ABAC policy, initialised from
/// `CRM_ABAC_POLICY` / `CRM_ABAC_POLICY_FILE` (else the built-in
/// default). Read the active snapshot with `policy().current()` per
/// request; swap it at runtime with [`reload_policy`] (e.g. from the
/// policy-file watcher in `app.rs`) — **no restart needed**.
#[must_use]
pub fn policy() -> &'static ReloadablePolicy {
    static POLICY: OnceLock<ReloadablePolicy> = OnceLock::new();
    POLICY.get_or_init(|| ReloadablePolicy::new(policy_from_env()))
}

/// Re-read the ABAC policy from the environment (`CRM_ABAC_POLICY` /
/// `CRM_ABAC_POLICY_FILE`, same rules and default-fallback as
/// [`policy_from_env`]) and swap it into the live [`policy`] holder.
/// Called by the policy-file watcher (`app.rs`) when the file changes;
/// a malformed policy falls back to the built-in default (never leaves
/// the service unprotected) and warn-logs. New requests see the new
/// policy immediately; in-flight ones finish against their snapshot.
pub fn reload_policy() {
    policy().store(policy_from_env());
    tracing::info!("ABAC policy reloaded");
}

/// How often the policy-file watcher polls for a change (seconds).
pub(super) const POLICY_WATCH_SECS: u64 = 15;

/// Spawn a background task that hot-reloads the ABAC policy when the
/// `CRM_ABAC_POLICY_FILE` changes on disk — so operators can edit the
/// policy file and have it take effect without a restart. It polls the
/// file's mtime every [`POLICY_WATCH_SECS`] and calls [`reload_policy`]
/// on a change (mtime-poll, not an OS notify, to stay dependency-light).
///
/// A no-op when `CRM_ABAC_POLICY_FILE` is unset — an inline
/// `CRM_ABAC_POLICY` (or the built-in default) has nothing to watch,
/// and env vars do not change at runtime. Call once at boot
/// (`app.rs::after_routes`).
pub fn spawn_policy_watcher() {
    let Some(path) = crate::settings::current().abac_policy_file.clone() else {
        return;
    };
    tokio::spawn(async move {
        let mut last = file_mtime(&path);
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(POLICY_WATCH_SECS));
        // The first tick fires immediately; skip it so we react only to
        // subsequent changes, not the initial state we already loaded.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = file_mtime(&path);
            if now != last {
                last = now;
                reload_policy();
            }
        }
    });
    tracing::info!(
        secs = POLICY_WATCH_SECS,
        "watching CRM_ABAC_POLICY_FILE for changes"
    );
}

/// The last-modified time of `path`, or `None` if it cannot be read
/// (missing file, permission error, or a platform without mtime).
pub(super) fn file_mtime(path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// The blanket-enforcement decision: authentication, then ABAC
/// authorization. `Ok(())` ⇒ let the request through; `Err((401|403,
/// msg))` ⇒ reject. Pure: the caller passes the flag, method, path,
/// headers, verifier and policy, so it is fully unit-testable without
/// booting the app or a database.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the request
/// carries no valid bearer token (missing/malformed/expired/tampered).
/// `403` when the token is valid but the ABAC policy denies the derived
/// action (the message names the deciding rule, per
/// `authorization-attributes.md` §5).
#[cfg(feature = "paseto")]
pub fn enforce(
    require_auth: bool,
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
    policy: &Policy,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(());
    }
    let claims = bearer_claims(headers, verifier)?;
    decide(&claims, method, path, policy)
}

/// The ABAC half of [`enforce`]: `Ok` when the policy allows the request's
/// derived action for `claims`, else `403` carrying the deciding rule.
pub(super) fn decide(
    claims: &Claims,
    method: &Method,
    path: &str,
    policy: &Policy,
) -> Result<(), (StatusCode, String)> {
    let decision = policy.evaluate(claims, derive_action(method, path), ENTITY);
    if decision.allowed {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

/// The blanket guard the middleware runs: like [`enforce`], but the caller
/// is authenticated through every credential type this build accepts
/// (PASETO with `paseto`, a Keycloak access token with `keycloak`) via
/// [`request_claims`], against the process-wide verifiers.
///
/// # Errors
///
/// `401` when enforcement is on and no accepted credential is presented;
/// `403` when the ABAC policy denies the derived action.
pub async fn enforce_request(
    require_auth: bool,
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    policy: &Policy,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(());
    }
    let claims = request_claims(headers).await?;
    decide(&claims, method, path, policy)
}
