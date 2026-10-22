//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, the blanket auth guard, the API-version
//! middleware, and the truncate lifecycle for
//! `contact-relationship-management`.

use async_trait::async_trait;
use axum::{
    Router as AxumRouter,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use loco_rs::{
    Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use std::path::Path;

use crate::{
    auth, controllers, idempotency_store, idempotency_store::Stored, models::_entities::prelude::*,
    rate_limit, redis_store, tasks,
};

/// Blanket auth-enforcement middleware: reads `CRM_REQUIRE_AUTH` per
/// request and delegates to [`auth::enforce_request`]. Off by
/// default — see `auth/` and `agents/share/security.md` §4.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let policy = auth::policy().current();
    match auth::enforce_request(auth::require_auth(), &method, &path, req.headers(), &policy).await
    {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// Insight read audit (CRM-T47 remainder): every successful `GET
/// /api/insights/*` writes a `insight_read` entry naming the view and the
/// caller. Fails closed — an unaudited disclosure is worse than an error.
async fn insight_audit_mw(
    axum::extract::State(db): axum::extract::State<sea_orm::DatabaseConnection>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let is_read = req.method() == axum::http::Method::GET && path.starts_with("/api/insights/");
    let actor = if is_read {
        auth::request_claims(req.headers())
            .await
            .ok()
            .map(|c| c.sub)
    } else {
        None
    };
    let response = next.run(req).await;
    if !is_read || !response.status().is_success() {
        return response;
    }
    let view = path.trim_start_matches("/api/insights/").to_string();
    match crate::models::audit_logs::Model::record_read(
        &db,
        "insight",
        uuid::Uuid::nil(),
        "insight_read",
        actor.as_deref(),
        Some(serde_json::json!({ "view": view })),
    )
    .await
    {
        Ok(_) => response,
        Err(error) => {
            tracing::error!(%error, "insight read audit failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// A JSON error from the idempotency layer.
fn idempotency_problem(status: axum::http::StatusCode, message: &str) -> Response {
    (
        status,
        axum::Json(serde_json::json!({ "error": "idempotency", "description": message })),
    )
        .into_response()
}

/// The store failed: refuse rather than run the request unprotected.
fn idempotency_unavailable(error: &sea_orm::DbErr) -> Response {
    tracing::error!(%error, "idempotency store failed");
    idempotency_problem(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "idempotency store unavailable",
    )
}

/// The answer to a request whose key an earlier request already holds: the
/// stored response, or the reason there is none yet.
fn answer_for_existing(existing: Option<&Stored>, fingerprint: &str) -> Response {
    use axum::http::{HeaderValue, StatusCode, header};
    let Some(stored) = existing else {
        // The first attempt failed and cleaned up between our two statements;
        // the client can retry at once.
        return idempotency_problem(
            StatusCode::CONFLICT,
            "the original request just finished without a stored result; retry",
        );
    };
    if stored.fingerprint != fingerprint {
        return idempotency_problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Idempotency-Key was already used with a different request",
        );
    }
    if stored.state != "done" {
        let mut response = idempotency_problem(
            StatusCode::CONFLICT,
            "the original request is still being processed; retry shortly",
        );
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
        return response;
    }
    let mut response = Response::new(axum::body::Body::from(stored.body.clone()));
    *response.status_mut() = u16::try_from(stored.status)
        .ok()
        .and_then(|s| StatusCode::from_u16(s).ok())
        .unwrap_or(StatusCode::OK);
    if let Some(value) = stored
        .content_type
        .as_deref()
        .and_then(|v| HeaderValue::from_str(v).ok())
    {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    response
        .headers_mut()
        .insert("idempotent-replayed", HeaderValue::from_static("true"));
    response
}

/// Idempotency keys (CRM-T52); see [`crate::idempotency`] for the contract.
/// Runs inside auth, so only authenticated callers reach it and the key is
/// scoped to the caller. Fails closed: a caller who asked for idempotency
/// never gets an unprotected execution because the store was unavailable.
async fn idempotency_mw(
    axum::extract::State(db): axum::extract::State<sea_orm::DatabaseConnection>,
    req: Request,
    next: Next,
) -> Response {
    use axum::http::{StatusCode, header};
    let wants = req.method() == axum::http::Method::POST
        && req.uri().path().starts_with("/api/")
        && req.headers().contains_key(crate::idempotency::HEADER);
    if !wants {
        return next.run(req).await;
    }
    let key = req
        .headers()
        .get(crate::idempotency::HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if let Err(message) = crate::idempotency::validate_key(&key) {
        return idempotency_problem(StatusCode::UNPROCESSABLE_ENTITY, &message);
    }
    let scope = auth::request_claims(req.headers())
        .await
        .map_or_else(|_| "anon".to_string(), |c| c.sub);
    let (parts, body) = req.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, crate::idempotency::MAX_BODY).await else {
        return idempotency_problem(StatusCode::PAYLOAD_TOO_LARGE, "request body too large");
    };
    let fingerprint = crate::idempotency::fingerprint(
        parts.method.as_str(),
        parts.uri.path(),
        parts.uri.query().unwrap_or(""),
        &bytes,
    );

    match idempotency_store::claim(&db, &scope, &key, &fingerprint).await {
        Ok(true) => {}
        Ok(false) => {
            return match idempotency_store::lookup(&db, &scope, &key).await {
                Ok(existing) => answer_for_existing(existing.as_ref(), &fingerprint),
                Err(error) => idempotency_unavailable(&error),
            };
        }
        Err(error) => return idempotency_unavailable(&error),
    }

    // We own the key: run the request, then record or forget the outcome.
    let response = next
        .run(Request::from_parts(parts, axum::body::Body::from(bytes)))
        .await;
    let (head, body) = response.into_parts();
    let body = axum::body::to_bytes(body, crate::idempotency::MAX_BODY)
        .await
        .ok();
    let text = body
        .as_ref()
        .and_then(|b| String::from_utf8(b.to_vec()).ok())
        .filter(|_| crate::idempotency::remember(head.status.as_u16()));
    let result = if let Some(text) = text {
        let content_type = head
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        idempotency_store::record(&db, &scope, &key, head.status.as_u16(), content_type, &text)
            .await
    } else {
        // Failed, oversized or not text: forget the key so a retry runs again.
        idempotency_store::forget(&db, &scope, &key).await
    };
    if let Err(error) = result {
        // The work is done but could not be recorded: a retry would repeat it.
        tracing::error!(%error, "idempotency result not recorded");
    }
    match body {
        Some(bytes) => Response::from_parts(head, axum::body::Body::from(bytes)),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Abuse guard (CRM-T43): per-client request limit and a failed-auth
/// lockout, answering `429` with `Retry-After`. Runs outside the auth
/// middleware so it sees the `401`s the auth middleware produces.
async fn abuse_guard_mw(
    axum::extract::State((guard, shared)): axum::extract::State<(
        std::sync::Arc<rate_limit::Guard>,
        Option<redis_store::Shared>,
    )>,
    req: Request,
    next: Next,
) -> Response {
    // Orchestrator probes must never be throttled or locked out: a refused
    // probe would restart a healthy pod.
    if matches!(req.uri().path(), "/_health" | "/_ping" | "/_readiness") {
        return next.run(req).await;
    }
    let key = {
        let header = |name: &str| req.headers().get(name).and_then(|v| v.to_str().ok());
        let trust_forwarded = crate::settings::current().trust_forwarded_for;
        rate_limit::client_key(
            header("authorization"),
            header("x-forwarded-for"),
            trust_forwarded,
        )
    };
    let now = std::time::Instant::now();
    let limit = u64::from(guard.requests.limit());
    let refused = |retry_after_secs: u64| {
        with_budget(
            (
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                [(
                    axum::http::header::RETRY_AFTER,
                    retry_after_secs.to_string(),
                )],
                "too many requests",
            )
                .into_response(),
            (limit > 0).then_some((limit, 0, retry_after_secs)),
        )
    };
    if let Some(shared) = &shared {
        // Shared counters (CRM_REDIS_URL): one count across all replicas.
        // A Redis error falls through to the in-process counters below.
        match shared_verdict(&guard, shared, &key).await {
            Ok(Err(retry_after_secs)) => return refused(retry_after_secs),
            Ok(Ok(budget)) => {
                let response = with_budget(next.run(req).await, budget);
                if response.status() == axum::http::StatusCode::UNAUTHORIZED
                    && !guard.failures.disabled()
                    && let Err(error) = shared
                        .hit(&format!("crm:fail:{key}"), guard.failure_window_secs())
                        .await
                {
                    tracing::warn!(%error, "shared lockout counter failed");
                }
                return response;
            }
            Err(error) => tracing::warn!(%error, "shared rate limit failed; using local counters"),
        }
    }
    if let rate_limit::Decision::Limited { retry_after_secs } = guard.failures.blocked(&key, now) {
        return refused(retry_after_secs);
    }
    if let rate_limit::Decision::Limited { retry_after_secs } = guard.requests.hit(&key, now) {
        return refused(retry_after_secs);
    }
    let budget = guard
        .requests
        .budget(&key, now)
        .map(|(limit, remaining, reset)| (u64::from(limit), u64::from(remaining), reset));
    let response = with_budget(next.run(req).await, budget);
    if response.status() == axum::http::StatusCode::UNAUTHORIZED {
        let _ = guard.failures.hit(&key, now);
    }
    response
}

/// Tell the client its budget (`RateLimit-Limit`, `-Remaining`, `-Reset`; the
/// IETF draft names). The GDS standard asks services to warn clients before
/// they hit the limit.
fn with_budget(mut response: Response, budget: Option<(u64, u64, u64)>) -> Response {
    if let Some((limit, remaining, reset)) = budget {
        let headers = response.headers_mut();
        for (name, value) in [
            ("ratelimit-limit", limit),
            ("ratelimit-remaining", remaining),
            ("ratelimit-reset", reset),
        ] {
            headers.insert(name, axum::http::HeaderValue::from(value));
        }
    }
    response
}

/// Every `/api/*` response says it must not be stored by a cache: the bodies
/// hold personal data (GDS: never cache sensitive or user-specific data). A
/// handler that set its own policy keeps it.
async fn no_store_mw(req: Request, next: Next) -> Response {
    let api = req.uri().path().starts_with("/api/");
    let mut response = next.run(req).await;
    if api {
        response
            .headers_mut()
            .entry(axum::http::header::CACHE_CONTROL)
            .or_insert(axum::http::HeaderValue::from_static("no-store"));
    }
    response
}

/// A client's budget: `(limit, remaining, seconds until reset)`.
type Budget = Option<(u64, u64, u64)>;

/// The shared-counter decision: `Err(retry_after)` to refuse, otherwise the
/// client's remaining budget.
async fn shared_verdict(
    guard: &rate_limit::Guard,
    shared: &redis_store::Shared,
    key: &str,
) -> Result<std::result::Result<Budget, u64>, redis::RedisError> {
    if !guard.failures.disabled() {
        let (failed, ttl) = shared.peek(&format!("crm:fail:{key}")).await?;
        if failed >= u64::from(guard.failures.limit()) {
            return Ok(Err(ttl));
        }
    }
    if !guard.requests.disabled() {
        let limit = u64::from(guard.requests.limit());
        let (count, ttl) = shared.hit(&format!("crm:req:{key}"), 60).await?;
        if count > limit {
            return Ok(Err(ttl));
        }
        return Ok(Ok(Some((limit, limit - count, ttl))));
    }
    Ok(Ok(None))
}

/// The generation counter every successful write bumps.
const CACHE_GEN_KEY: &str = "crm:cache:gen";
/// How long an insight response is cached.
const INSIGHT_TTL: std::time::Duration = std::time::Duration::from_secs(30);
/// The cache is an optimisation: a Redis that is down or frozen must cost a
/// request this long at most, never the pool's own timeout (CRM-R25).
const CACHE_WAIT: std::time::Duration = std::time::Duration::from_millis(750);

/// The cache key for an insight read: a hash of the caller's token (so masking
/// never crosses callers), their break-glass state, the current generation, and
/// the path and query.
fn insight_key(token: &str, glass: bool, generation: i64, path: &str, query: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("{token}|{glass}|{generation}|{path}?{query}").as_bytes());
    let hex = digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    format!("crm:insight:{hex}")
}

/// Make every cached insight stale: a write has happened.
async fn bump_cache_generation(cache: &loco_rs::cache::Cache) {
    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    match tokio::time::timeout(
        CACHE_WAIT,
        cache.insert_with_expiry(CACHE_GEN_KEY, &stamp, INSIGHT_TTL * 4),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::warn!(%error, "cache generation bump failed"),
        Err(_) => tracing::warn!("cache generation bump timed out"),
    }
}

/// Insight response cache (Loco's cache, Redis-backed). Sits inside auth and
/// the read audit, so every request is still authenticated and audited; only
/// the computation is skipped. The key carries the caller's token hash (so
/// masking never crosses callers), their break-glass state, the path and
/// query, and a generation bumped by every successful write, so a write is
/// visible at once. A policy reload is visible after at most the TTL.
async fn insight_cache_mw(
    axum::extract::State(cache): axum::extract::State<std::sync::Arc<loco_rs::cache::Cache>>,
    req: Request,
    next: Next,
) -> Response {
    const MAX_BODY: usize = 1024 * 1024;
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    if method != axum::http::Method::GET && path.starts_with("/api/") {
        let response = next.run(req).await;
        if response.status().is_success() {
            bump_cache_generation(&cache).await;
        }
        return response;
    }
    if method != axum::http::Method::GET
        || !path.starts_with("/api/insights/")
        || req
            .headers()
            .contains_key(axum::http::header::IF_NONE_MATCH)
    {
        return next.run(req).await;
    }
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let claims = auth::request_claims(req.headers()).await.ok();
    let glass = claims
        .as_ref()
        .is_some_and(|c| crate::break_glass::registry().active(&c.sub, chrono::Utc::now()));
    let generation: i64 = tokio::time::timeout(CACHE_WAIT, cache.get(CACHE_GEN_KEY))
        .await
        .ok()
        .and_then(Result::ok)
        .flatten()
        .unwrap_or_default();
    let key = insight_key(
        token,
        glass,
        generation,
        &path,
        req.uri().query().unwrap_or(""),
    );
    if let Ok(Ok(Some(hit))) =
        tokio::time::timeout(CACHE_WAIT, cache.get::<CachedResponse>(&key)).await
    {
        return hit.into_response();
    }
    let response = next.run(req).await;
    if response.status() != axum::http::StatusCode::OK {
        return response;
    }
    let (parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, MAX_BODY).await else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        // Keep the response headers (ETag, cache control, content type) so a
        // cached read is indistinguishable from a computed one.
        let headers = parts
            .headers
            .iter()
            .filter(|(name, _)| *name != axum::http::header::CONTENT_LENGTH)
            .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
            .collect();
        let entry = CachedResponse { headers, text };
        match tokio::time::timeout(
            CACHE_WAIT,
            cache.insert_with_expiry(&key, &entry, INSIGHT_TTL),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "insight cache write failed"),
            Err(_) => tracing::warn!("insight cache write timed out"),
        }
    }
    Response::from_parts(parts, axum::body::Body::from(bytes))
}

/// A cached 200 response: its headers and body.
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedResponse {
    headers: Vec<(String, String)>,
    text: String,
}

impl CachedResponse {
    fn into_response(self) -> Response {
        let mut response = Response::new(axum::body::Body::from(self.text));
        for (name, value) in self.headers {
            if let (Ok(name), Ok(value)) = (
                axum::http::HeaderName::from_bytes(name.as_bytes()),
                axum::http::HeaderValue::from_str(&value),
            ) {
                response.headers_mut().append(name, value);
            }
        }
        response
    }
}

/// The loco.rs application hooks for
/// `contact-relationship-management`.
pub struct App;

/// Everything a process needs before it can send campaign mail: the field-encryption check,
/// the mail sender chosen by `CRM_MAIL_MODE` and the signed unsubscribe links. Called by the
/// server (`after_routes`) and by the worker (`connect_workers`); calling it twice in one
/// process (`--server-and-worker`) installs the same settings again.
fn init_sending(ctx: &AppContext) -> Result<()> {
    let production = matches!(ctx.environment, Environment::Production);
    crate::hardening::check_field_encryption().map_err(loco_rs::Error::string)?;
    // CRM-T39: choose the mail sender (simulated unless CRM_MAIL_MODE=smtp);
    // a bad or unsafe setting stops the start rather than failing at send time.
    crate::mail::init(production).map_err(|e| loco_rs::Error::string(&e))?;
    // Real mail must carry signed unsubscribe links.
    crate::unsubscribe::init(!crate::mail::current().is_simulated(), production)
        .map_err(|e| loco_rs::Error::string(&e))?;
    Ok(())
}

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::relationships::routes())
            .add_route(controllers::sales::routes())
            .add_route(controllers::board::routes())
            .add_route(controllers::marketing::routes())
            .add_route(controllers::marketing::public_routes())
            .add_route(controllers::support::routes())
            .add_route(controllers::dashboards::routes())
            .add_route(controllers::insights::routes())
            .add_route(controllers::engagement::routes())
            .add_route(controllers::privacy::routes())
            .add_route(controllers::external_links::routes())
            .add_route(controllers::merge::routes())
            .add_route(controllers::unmerge::routes())
            .add_route(controllers::audits::routes())
            .add_route(controllers::break_glass::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // CRM-T30: fail closed. (`Hooks::boot` is bypassed by `loco start`,
        // so the check lives here, which every start path reaches.)
        crate::hardening::check_process(matches!(ctx.environment, Environment::Production))
            .map_err(loco_rs::Error::string)?;
        init_sending(ctx)?;
        auth::init().await;
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        let shared = redis_store::Shared::from_env()
            .await
            .map_err(|e| loco_rs::Error::string(&format!("CRM_REDIS_URL: {e}")))?;
        if let Some(shared) = &shared {
            redis_store::set_global(shared.clone());
        }
        // CRM-T44: keep the live break-glass set fresh across replicas. The
        // database is the record (re-read every 10 s); with Redis, new grants
        // also arrive within a second, and the live set is the union of both.
        let db = ctx.db.clone();
        let poll = shared.clone();
        tokio::spawn(async move {
            let (tick, db_every) = if poll.is_some() { (1, 10) } else { (10, 1) };
            let mut from_db = std::collections::HashMap::new();
            let mut count = 0_u64;
            loop {
                if count.is_multiple_of(db_every) {
                    match controllers::break_glass::refresh(&db).await {
                        Ok(live) => from_db = live,
                        Err(error) => tracing::warn!(%error, "break-glass refresh failed"),
                    }
                }
                let mut live = from_db.clone();
                if let Some(shared) = &poll {
                    match shared.grants().await {
                        Ok(grants) => {
                            for (subject, expires_at) in grants {
                                let entry = live.entry(subject).or_insert(expires_at);
                                if expires_at > *entry {
                                    *entry = expires_at;
                                }
                            }
                        }
                        Err(error) => tracing::warn!(%error, "break-glass Redis poll failed"),
                    }
                }
                crate::break_glass::registry().replace(live);
                count += 1;
                tokio::time::sleep(std::time::Duration::from_secs(tick)).await;
            }
        });
        // CRM-T52: drop idempotency keys past their 24 hours.
        let idem_db = ctx.db.clone();
        tokio::spawn(async move {
            use sea_orm::ConnectionTrait;
            loop {
                if let Err(error) = idem_db
                    .execute_unprepared(
                        "DELETE FROM idempotency_keys WHERE created_at < now() - interval '24 hours'",
                    )
                    .await
                {
                    tracing::warn!(%error, "idempotency cleanup failed");
                }
                tokio::time::sleep(std::time::Duration::from_hours(1)).await;
            }
        });
        let guard = std::sync::Arc::new(rate_limit::Guard::from_env(matches!(
            ctx.environment,
            Environment::Production
        )));
        Ok(router
            .layer(axum::middleware::from_fn_with_state(
                ctx.db.clone(),
                idempotency_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                ctx.cache.clone(),
                insight_cache_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                ctx.db.clone(),
                insight_audit_mw,
            ))
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn_with_state(
                (guard, shared),
                abuse_guard_mw,
            ))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            ))
            // Outermost: every error above, whoever produced it, leaves as
            // `application/problem+json` (CRM-T54).
            .layer(axum::middleware::from_fn(crate::problem::problem_mw))
            // Outside the problem layer so errors carry the policy too.
            .layer(axum::middleware::from_fn(no_store_mw)))
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        // A `--worker` process never builds routes, so `after_routes` does not run there:
        // without this it would send campaigns with the simulated sender and mark them
        // delivered (found by `scripts/image-smoke.sh`, CRM-T72).
        init_sending(ctx)?;
        queue
            .register(
                <controllers::marketing::CampaignSendWorker as loco_rs::prelude::BackgroundWorker<_>>::build(ctx),
            )
            .await?;
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
        tasks.register(tasks::rotate_field_keys::RotateFieldKeys);
        // tasks-inject (do not remove)
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        {
            use sea_orm::ConnectionTrait;
            ctx.db
                .execute_unprepared(
                    "TRUNCATE idempotency_keys, email_deliveries, email_events, email_suppressions",
                )
                .await?;
        }
        truncate_table(&ctx.db, EventOutbox).await?;
        truncate_table(&ctx.db, AuditLogs).await?;
        truncate_table(&ctx.db, Articles).await?;
        truncate_table(&ctx.db, Tickets).await?;
        truncate_table(&ctx.db, SlaPolicies).await?;
        truncate_table(&ctx.db, NurtureEnrollments).await?;
        truncate_table(&ctx.db, NurtureSteps).await?;
        truncate_table(&ctx.db, NurtureSequences).await?;
        truncate_table(&ctx.db, Campaigns).await?;
        truncate_table(&ctx.db, Segments).await?;
        truncate_table(&ctx.db, ForecastSnapshots).await?;
        truncate_table(&ctx.db, Deals).await?;
        truncate_table(&ctx.db, PipelineStages).await?;
        truncate_table(&ctx.db, Pipelines).await?;
        truncate_table(&ctx.db, Leads).await?;
        truncate_table(&ctx.db, ConsentEvents).await?;
        truncate_table(&ctx.db, Activities).await?;
        truncate_table(&ctx.db, Contacts).await?;
        truncate_table(&ctx.db, Accounts).await?;
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::insight_key;

    #[test]
    fn an_insight_key_depends_on_everything_that_changes_the_answer() {
        let base = insight_key("Bearer a", false, 1, "/api/insights/x", "days=7");
        assert_eq!(
            base,
            insight_key("Bearer a", false, 1, "/api/insights/x", "days=7"),
            "the same request gets the same key"
        );
        for other in [
            insight_key("Bearer b", false, 1, "/api/insights/x", "days=7"),
            insight_key("Bearer a", true, 1, "/api/insights/x", "days=7"),
            insight_key("Bearer a", false, 2, "/api/insights/x", "days=7"),
            insight_key("Bearer a", false, 1, "/api/insights/y", "days=7"),
            insight_key("Bearer a", false, 1, "/api/insights/x", "days=30"),
        ] {
            assert_ne!(base, other);
        }
        assert!(base.starts_with("crm:insight:"));
        assert!(
            !base.contains("Bearer"),
            "the token is hashed, never stored"
        );
    }
}
