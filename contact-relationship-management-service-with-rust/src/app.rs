//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, the blanket auth guard, the API-version
//! middleware, and the truncate lifecycle for
//! `contact-relationship-management-service`.

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

use crate::{auth, controllers, models::_entities::prelude::*, rate_limit, redis_store, tasks};

/// Blanket auth-enforcement middleware: reads `CRM_REQUIRE_AUTH` per
/// request and delegates to [`auth::enforce_request`]. Off by
/// default — see `auth.rs` and `agents/share/security.md` §4.
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
        let trust_forwarded =
            auth::parse_bool(&std::env::var("CRM_TRUST_FORWARDED_FOR").unwrap_or_default());
        rate_limit::client_key(
            header("authorization"),
            header("x-forwarded-for"),
            trust_forwarded,
        )
    };
    let now = std::time::Instant::now();
    let refused = |retry_after_secs: u64| {
        (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            [(
                axum::http::header::RETRY_AFTER,
                retry_after_secs.to_string(),
            )],
            "too many requests",
        )
            .into_response()
    };
    if let Some(shared) = &shared {
        // Shared counters (CRM_REDIS_URL): one count across all replicas.
        // A Redis error falls through to the in-process counters below.
        match shared_verdict(&guard, shared, &key).await {
            Ok(Some(retry_after_secs)) => return refused(retry_after_secs),
            Ok(None) => {
                let response = next.run(req).await;
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
    let response = next.run(req).await;
    if response.status() == axum::http::StatusCode::UNAUTHORIZED {
        let _ = guard.failures.hit(&key, now);
    }
    response
}

/// The shared-counter decision: `Some(retry_after)` to refuse.
async fn shared_verdict(
    guard: &rate_limit::Guard,
    shared: &redis_store::Shared,
    key: &str,
) -> Result<Option<u64>, redis::RedisError> {
    if !guard.failures.disabled() {
        let (failed, ttl) = shared.peek(&format!("crm:fail:{key}")).await?;
        if failed >= u64::from(guard.failures.limit()) {
            return Ok(Some(ttl));
        }
    }
    if !guard.requests.disabled() {
        let (count, ttl) = shared.hit(&format!("crm:req:{key}"), 60).await?;
        if count > u64::from(guard.requests.limit()) {
            return Ok(Some(ttl));
        }
    }
    Ok(None)
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
    use sha2::{Digest, Sha256};
    const GEN_KEY: &str = "crm:cache:gen";
    const TTL: std::time::Duration = std::time::Duration::from_secs(30);
    const MAX_BODY: usize = 1024 * 1024;
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    if method != axum::http::Method::GET && path.starts_with("/api/") {
        let response = next.run(req).await;
        if response.status().is_success() {
            let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
            if let Err(error) = cache.insert_with_expiry(GEN_KEY, &stamp, TTL * 4).await {
                tracing::warn!(%error, "cache generation bump failed");
            }
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
    let generation: i64 = cache.get(GEN_KEY).await.ok().flatten().unwrap_or_default();
    let digest = Sha256::digest(
        format!(
            "{token}|{glass}|{generation}|{path}?{}",
            req.uri().query().unwrap_or("")
        )
        .as_bytes(),
    );
    let hex = digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    let key = format!("crm:insight:{hex}");
    if let Ok(Some(hit)) = cache.get::<CachedResponse>(&key).await {
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
        if let Err(error) = cache.insert_with_expiry(&key, &entry, TTL).await {
            tracing::warn!(%error, "insight cache write failed");
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
/// `contact-relationship-management-service`.
pub struct App;
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
            .add_route(controllers::marketing::routes())
            .add_route(controllers::support::routes())
            .add_route(controllers::dashboards::routes())
            .add_route(controllers::insights::routes())
            .add_route(controllers::engagement::routes())
            .add_route(controllers::privacy::routes())
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
        crate::hardening::check_field_encryption().map_err(loco_rs::Error::string)?;
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
        let guard = std::sync::Arc::new(rate_limit::Guard::from_env(matches!(
            ctx.environment,
            Environment::Production
        )));
        Ok(router
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
            )))
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
        tasks.register(tasks::rotate_field_keys::RotateFieldKeys);
        // tasks-inject (do not remove)
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
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
