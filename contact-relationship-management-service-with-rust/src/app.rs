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

use crate::{auth, controllers, models::_entities::prelude::*, rate_limit, tasks};

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
    axum::extract::State(guard): axum::extract::State<std::sync::Arc<rate_limit::Guard>>,
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
        // CRM-T44: keep the live break-glass set fresh across replicas.
        let db = ctx.db.clone();
        tokio::spawn(async move {
            loop {
                if let Err(error) = controllers::break_glass::refresh(&db).await {
                    tracing::warn!(%error, "break-glass refresh failed");
                }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        });
        let guard = std::sync::Arc::new(rate_limit::Guard::from_env(matches!(
            ctx.environment,
            Environment::Production
        )));
        Ok(router
            .layer(axum::middleware::from_fn_with_state(
                ctx.db.clone(),
                insight_audit_mw,
            ))
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn_with_state(guard, abuse_guard_mw))
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
