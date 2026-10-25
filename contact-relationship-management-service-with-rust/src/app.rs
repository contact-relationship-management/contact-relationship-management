//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, the blanket auth guard, the API-version
//! middleware, and the truncate lifecycle for
//! `contact-relationship-management`.

use async_trait::async_trait;
use axum::Router as AxumRouter;
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
    auth, controllers, middleware, models::_entities::prelude::*, rate_limit, redis_store, tasks,
};

/// Every route the service mounts. Free of the application context so the `OpenAPI` document
/// can be built from the same list the server serves.
#[must_use]
pub fn app_routes() -> AppRoutes {
    AppRoutes::with_default_routes()
        .add_route(controllers::relationships::routes())
        .add_route(controllers::sales::routes())
        .add_route(controllers::board::routes())
        .add_route(controllers::marketing::routes())
        .add_route(controllers::marketing::public_routes())
        .add_route(controllers::forms::routes())
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
        .add_route(controllers::webhooks::routes())
        .add_route(controllers::notifications::routes())
        .add_route(controllers::search::routes())
        .add_route(controllers::lead_imports::routes())
        .add_route(controllers::views::routes())
        .add_route(controllers::templates::routes())
        .add_route(controllers::workflows::routes())
        .add_route(controllers::docs::routes())
        .add_route(controllers::metrics::routes())
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

/// CRM-R32: every minute, queue the job that makes due task reminders and SLA-breach
/// notifications. It is idempotent (a uniqueness rule on the rows), so replicas may all do it.
fn spawn_notification_sweep(ctx: &AppContext) {
    let sweep_ctx = ctx.clone();
    tokio::spawn(async move {
        use loco_rs::prelude::BackgroundWorker;
        loop {
            tokio::time::sleep(std::time::Duration::from_mins(1)).await;
            if let Err(error) =
                controllers::notifications::NotificationSweepWorker::perform_later(&sweep_ctx, ())
                    .await
            {
                tracing::warn!(%error, "notification sweep not queued");
            }
        }
    });
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
        app_routes()
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
        spawn_notification_sweep(ctx);
        let guard = std::sync::Arc::new(rate_limit::Guard::from_env(matches!(
            ctx.environment,
            Environment::Production
        )));
        let api = router
            .layer(axum::middleware::from_fn_with_state(
                ctx.db.clone(),
                middleware::idempotency::idempotency_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                ctx.cache.clone(),
                middleware::insight_cache::insight_cache_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                ctx.db.clone(),
                middleware::insight_audit::insight_audit_mw,
            ))
            .layer(axum::middleware::from_fn(
                middleware::require_auth::require_auth_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                (guard, shared),
                middleware::abuse_guard::abuse_guard_mw,
            ))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            ))
            // Outermost: every error above, whoever produced it, leaves as
            // `application/problem+json` (CRM-T54).
            .layer(axum::middleware::from_fn(crate::problem::problem_mw))
            // Outside the problem layer so errors carry the policy too.
            .layer(axum::middleware::from_fn(middleware::no_store::no_store_mw));
        // The MCP endpoint (CRM-R30) calls the service's own router, as it stands above, with the
        // caller's credentials; it is added last so it is not inside those layers itself.
        Ok(api.clone().merge(crate::mcp::router(api, ctx.db.clone())))
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
        queue
            .register(
                <controllers::notifications::NotificationSweepWorker as loco_rs::prelude::BackgroundWorker<_>>::build(ctx),
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
                    "TRUNCATE workflow_runs, workflows, email_templates, saved_views, lead_imports, notifications, form_submissions, webhook_deliveries, webhook_subscriptions, idempotency_keys, \
                     email_deliveries, email_events, email_suppressions",
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
