//! Application wiring: the routes, the blanket auth guard, the API-version middleware, the
//! command line, and the truncate lifecycle for `contact-relationship-management`.

use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{self, Cli, Command, DbCommand, parse_vars},
    config::{Config as ServiceConfig, Defaults, MB},
    db::truncate_table,
    queue::{self, Queue, QueueOptions},
    redis::{Cache, CacheConfig},
    routes::AppRoutes,
    server,
    task::Tasks,
    worker::{BackgroundWorker as _, WorkerMode},
};
use axum::Router as AxumRouter;
use migration::{Migrator, MigratorTrait as _};
use std::{sync::Arc, time::Duration};

use crate::{
    auth, controllers, middleware, models::_entities::prelude::*, rate_limit, redis_store, tasks,
};

/// What this service does when the environment says nothing. The pool of one is for development
/// and tests: production's size comes from `axa`, or `DB_MAX_CONNECTIONS`.
fn defaults() -> Defaults {
    Defaults {
        database: "contact_relationship_management",
        port: 5150,
        production_port: None,
        body_limit: 2 * MB,
        max_connections: 1,
        ..Defaults::default()
    }
}

/// How long an entry lives unless a call says otherwise. The insight cache says.
const CACHE_TTL: Duration = Duration::from_secs(30);

/// The response cache. `url` is `CRM_REDIS_URL`. Replicas must share what they cache, so
/// production refuses to start without it, and elsewhere the cache stays in this process.
async fn connect_cache(url: Option<&str>, production: bool) -> Result<Cache> {
    let cache = match url {
        Some(url) => Cache::connect(&CacheConfig::new(url, "", CACHE_TTL)).await,
        None if production => return Err(Error::string("CRM_REDIS_URL is required in production")),
        None => Cache::memory("", CACHE_TTL),
    };
    cache.map_err(|e| Error::string(&format!("CRM_REDIS_URL: {e}")))
}

/// The browser origins allowed to call the service directly. Browsers reach it through the
/// same-origin BFF (CRM-T33), so with neither setting CORS stays closed in every environment.
fn cors_origins(
    listed: Option<&str>,
    origin: Option<String>,
    from_axa: Vec<String>,
) -> Vec<String> {
    if listed.is_some() {
        from_axa
    } else {
        origin.into_iter().collect()
    }
}

/// The service's settings for `environment`.
fn service_config(environment: &Environment) -> Result<ServiceConfig> {
    let mut config = ServiceConfig::from_env(environment, &defaults())?;
    let listed = std::env::var("CORS_ALLOWED_ORIGINS").ok();
    let origin = crate::settings::current().cors_origin.clone();
    config.http.cors_origins = cors_origins(listed.as_deref(), origin, config.http.cors_origins);
    Ok(config)
}

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
        .add_route(controllers::kb::public_routes())
        .add_route(controllers::kb::routes())
        .add_route(controllers::support::routes())
        .add_route(controllers::support::public_routes())
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
        .add_route(controllers::custom_fields::routes())
        .add_route(controllers::attachments::routes())
        .add_route(controllers::calendars::routes())
        .add_route(controllers::tags::routes())
        .add_route(controllers::reports::routes())
        .add_route(controllers::attribution::routes())
        .add_route(controllers::fx_rates::routes())
        .add_route(controllers::quotas::routes())
        .add_route(controllers::teams::routes())
        .add_route(controllers::bridges::routes())
        .add_route(controllers::territories::routes())
        .add_route(controllers::record_imports::routes())
        .add_route(controllers::docs::routes())
        .add_route(controllers::metrics::routes())
        .add_route(controllers::organization::routes())
}

/// The `contact-relationship-management` application.
pub struct App;

/// Everything a process needs before it can send campaign mail: the field-encryption check,
/// the mail sender chosen by `CRM_MAIL_MODE` and the signed unsubscribe links. Called by the
/// server (`after_routes`) and by the worker (`connect_workers`); calling it twice in one
/// process (`--server-and-worker`) installs the same settings again.
fn init_sending(ctx: &AppContext) -> Result<()> {
    let production = ctx.environment.is_production_like();
    crate::hardening::check_field_encryption().map_err(Error::string)?;
    // CRM-T39: choose the mail sender (simulated unless CRM_MAIL_MODE=smtp);
    // a bad or unsafe setting stops the start rather than failing at send time.
    crate::mail::init(production).map_err(|e| Error::string(&e))?;
    // Real mail must carry signed unsubscribe links.
    crate::unsubscribe::init(!crate::mail::current().is_simulated(), production)
        .map_err(|e| Error::string(&e))?;
    Ok(())
}

/// Keep the live break-glass set fresh across replicas, polling `poll` when there is one.
fn spawn_break_glass_refresh(ctx: &AppContext, poll: Option<redis_store::Shared>) {
    // CRM-T44: keep the live break-glass set fresh across replicas. The
    // database is the record (re-read every 10 s); with Redis, new grants
    // also arrive within a second, and the live set is the union of both.
    let db = crate::db::pool(ctx).clone();
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
}

/// Drop expired idempotency keys and spent link tokens, every hour.
fn spawn_token_sweep(ctx: &AppContext) {
    // CRM-T52: drop idempotency keys past their 24 hours.
    let pool = crate::db::pool(ctx).clone();
    tokio::spawn(async move {
        loop {
            if let Err(error) = sqlx::query!(
                "DELETE FROM idempotency_keys WHERE created_at < now() - interval '24 hours'"
            )
            .execute(&pool)
            .await
            {
                tracing::warn!(%error, "idempotency cleanup failed");
            }
            // CRM-R79: link tokens that expired or were spent more than a week ago.
            if let Err(error) = crate::link_tokens::sweep(&pool).await {
                tracing::warn!(%error, "link token cleanup failed");
            }
            tokio::time::sleep(std::time::Duration::from_hours(1)).await;
        }
    });
}

/// CRM-R32: every minute, queue the job that makes due task reminders and SLA-breach
/// notifications. It is idempotent (a uniqueness rule on the rows), so replicas may all do it.
fn spawn_notification_sweep(ctx: &AppContext) {
    let sweep_ctx = ctx.clone();
    tokio::spawn(async move {
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

impl App {
    /// Connect the database, the cache and, when jobs go through the queue, the queue. A process
    /// that runs jobs (`run_jobs`) also registers the workers.
    ///
    /// # Errors
    ///
    /// When a connection fails, or production has no Redis.
    pub async fn context(
        environment: Environment,
        config: ServiceConfig,
        run_jobs: bool,
    ) -> Result<AppContext> {
        let db = axa::db::connect(&config.database).await?;
        let mut ctx = AppContext::new(environment, db, config);
        // Tests never share a Redis, whatever the environment says.
        let settings = crate::settings::current();
        let url = settings
            .redis_url
            .as_deref()
            .filter(|_| ctx.environment != Environment::Test);
        let cache = connect_cache(url, ctx.environment.is_production_like()).await?;
        ctx.shared_store.insert(cache);
        if ctx.config.workers == WorkerMode::BackgroundQueue {
            let mut queue =
                Queue::connect(&ctx.config.database.uri, QueueOptions::default()).await?;
            if run_jobs {
                Self::connect_workers(&ctx, &mut queue)?;
            }
            ctx.queue_provider = Some(Arc::new(queue));
        }
        Ok(ctx)
    }

    /// The whole router: the routes, the service's own layers, then `axa`'s standard middleware
    /// around them all. In that order a refusal from one of those layers carries the request id,
    /// the security headers and CORS, a panic in one is caught, they read plain bodies rather than
    /// compressed ones, and the abuse guard sees the address `axa` resolved (CRM-T217).
    ///
    /// # Errors
    ///
    /// When a start-up check refuses, or a setting is invalid.
    pub async fn router(ctx: &AppContext) -> Result<AxumRouter> {
        let router = Self::routes(ctx).into_router(ctx.clone());
        let router = Self::after_routes(router, ctx).await?;
        axa::middleware::apply(router, &ctx.config.http)
    }

    async fn db(ctx: &AppContext, command: DbCommand) -> Result<()> {
        if command.destroys_data() && ctx.environment.is_production_like() {
            return Err(Error::string(
                "db down, db reset and db truncate refuse to run in production",
            ));
        }
        match command {
            DbCommand::Migrate => Migrator::up(&ctx.db, None).await?,
            DbCommand::Down { steps } => Migrator::down(&ctx.db, Some(steps)).await?,
            DbCommand::Reset => Migrator::fresh(&ctx.db).await?,
            DbCommand::Status => Migrator::status(&ctx.db).await?,
            DbCommand::Truncate => Self::truncate(ctx).await?,
        }
        Ok(())
    }

    /// Run one command line.
    ///
    /// # Errors
    ///
    /// When the settings are refused, a connection fails, or the command fails.
    pub async fn run(cli: Cli) -> Result<()> {
        let environment = cli
            .environment
            .as_deref()
            .map_or_else(Environment::from_env, Environment::parse);
        if matches!(cli.command, Command::Check) {
            return cli::check(&environment, &defaults());
        }
        let mut config = service_config(&environment)?;
        config.apply_cli(&cli);
        let (serve, jobs) = match &cli.command {
            Command::Start { worker, all, .. } => (!*worker, *worker || *all),
            _ => (false, false),
        };
        axa::logger::init(env!("CARGO_CRATE_NAME"), &config.logger);
        let ctx = Self::context(environment, config, jobs).await?;
        if ctx.config.database.auto_migrate && !matches!(cli.command, Command::Db { .. }) {
            Migrator::up(&ctx.db, None).await?;
        }
        match cli.command {
            Command::Start { .. } => {
                queue::warn_if_unserved(&ctx, jobs);
                let router = if serve {
                    Some(Self::router(&ctx).await?)
                } else {
                    None
                };
                let queue = if jobs {
                    ctx.queue_provider.clone()
                } else {
                    None
                };
                if jobs && queue.is_none() {
                    return Err(Error::string(
                        "running jobs needs WORKER_MODE=BackgroundQueue",
                    ));
                }
                server::run(router, queue, &ctx.config.server).await
            }
            Command::Db { command } => Self::db(&ctx, command).await,
            Command::Task { name, params } => {
                let mut tasks = Tasks::default();
                Self::register_tasks(&mut tasks);
                let Some(name) = name else {
                    for task in tasks.list() {
                        println!("{}\t{}", task.name, task.detail);
                    }
                    return Ok(());
                };
                tasks.run(&name, &ctx, &parse_vars(&params)?).await
            }
            Command::Check => Ok(()),
        }
    }

    /// Every route, for this context.
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        app_routes()
    }

    /// The checks every server start makes, the background loops, and the service's own layers.
    ///
    /// # Errors
    ///
    /// When a start-up check refuses, or a setting is invalid.
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // CRM-T30: fail closed. (Here because every server start reaches it.)
        crate::hardening::check_process(ctx.environment.is_production_like())
            .map_err(Error::string)?;
        // CRM-T221: record times follow the database's clock; measure it before serving.
        crate::clock::start(crate::db::pool(ctx).clone()).await;
        init_sending(ctx)?;
        // CRM-T223: list cursors are signed, and replicas must share the key.
        crate::controllers::paging::init(ctx.environment.is_production_like())
            .map_err(|e| Error::string(&e))?;
        auth::init().await;
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        let shared = redis_store::Shared::from_env()
            .await
            .map_err(|e| Error::string(&format!("CRM_REDIS_URL: {e}")))?;
        if let Some(shared) = &shared {
            redis_store::set_global(shared.clone());
        }
        spawn_break_glass_refresh(ctx, shared.clone());
        spawn_token_sweep(ctx);
        spawn_notification_sweep(ctx);
        // Unread since CRM-T217, so a deployment that relied on it now counts its proxy as one
        // client. A log line rather than a refusal: nothing it did was safe to keep.
        if std::env::var_os("CRM_TRUST_FORWARDED_FOR").is_some() {
            tracing::warn!(
                "CRM_TRUST_FORWARDED_FOR is ignored: list your proxies in TRUSTED_PROXIES instead"
            );
        }
        let guard = std::sync::Arc::new(rate_limit::Guard::from_env(
            ctx.environment.is_production_like(),
        ));
        let address_lockout = rate_limit::address_lockout(
            std::env::var(rate_limit::ADDRESS_LOCKOUT_VAR)
                .ok()
                .as_deref(),
            guard.failure_window_secs(),
        )
        .map_err(|why| Error::string(&why))?;
        let cache = ctx
            .shared_store
            .get_ref::<Cache>()
            .ok_or_else(|| Error::string("the cache was not connected"))?;
        let api = router
            .layer(axum::middleware::from_fn_with_state(
                crate::db::pool(ctx).clone(),
                middleware::idempotency::idempotency_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                cache,
                middleware::insight_cache::insight_cache_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                crate::db::pool(ctx).clone(),
                middleware::insight_audit::insight_audit_mw,
            ))
            .layer(axum::middleware::from_fn(
                middleware::require_auth::require_auth_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                (guard, shared),
                middleware::abuse_guard::abuse_guard_mw,
            ));
        // CRM-T224: axa's lockout by client address, outside the abuse guard and sign-in so it
        // sees their `401`s, and inside `axa`'s stack so it reads the address that stack resolved.
        let api = match address_lockout {
            Some(limiter) => api.layer(axum::middleware::from_fn_with_state(
                limiter,
                axa::ratelimit::lock_out_failures,
            )),
            None => api,
        };
        let api = api
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            ))
            // Every error above, whoever produced it, leaves as `application/problem+json`
            // (CRM-T54). What `axa` answers itself, outside this, is axa's own problem: a timeout's
            // `408` and the `500` for a panic carry `code` but not `error`, `description`,
            // `instance` or `no-store` (CRM-T217, CRM-T218).
            .layer(axum::middleware::from_fn(crate::problem::problem_mw))
            // Outside the problem layer so errors carry the policy too.
            .layer(axum::middleware::from_fn(middleware::no_store::no_store_mw));
        // The MCP endpoint (CRM-R30) calls the service's own router, as it stands above, with the
        // caller's credentials; it is added last so it is not inside those layers itself.
        // `merge` keeps the fallback of the router it is given, so this order matters:
        // the other way round, a request for an unknown path skips every layer above.
        // That inner router is called directly, so it is given the limit `axa` sets from outside.
        // axa 0.11's `body_limit` raises one route's limit; this is the extractors' limit it set
        // in 0.10, on the whole inner router.
        let inner = api.clone().layer(axum::extract::DefaultBodyLimit::max(
            ctx.config.http.body_limit,
        ));
        let router = crate::mcp::router(inner, crate::db::pool(ctx).clone()).merge(api);
        Ok(if ctx.environment.is_production_like() {
            middleware::secure_headers::install(router)
        } else {
            router
        })
    }

    /// Register the job workers.
    ///
    /// # Errors
    ///
    /// When the mail settings are refused.
    pub fn connect_workers(ctx: &AppContext, queue: &mut Queue) -> Result<()> {
        // A `--worker` process never builds routes, so `after_routes` does not run there:
        // without this it would send campaigns with the simulated sender and mark them
        // delivered (found by `scripts/image-smoke.sh`, CRM-T72).
        init_sending(ctx)?;
        // CRM-T221: the worker's record times follow the database's clock too. Its first
        // measurement lands in the background, a moment after the workers start.
        tokio::spawn(crate::clock::start(crate::db::pool(ctx).clone()));
        queue.register(controllers::marketing::CampaignSendWorker::build(ctx));
        queue.register(controllers::notifications::NotificationSweepWorker::build(
            ctx,
        ));
        Ok(())
    }

    /// Register the command-line tasks.
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
        tasks.register(tasks::rotate_field_keys::RotateFieldKeys);
        // tasks-inject (do not remove)
    }

    /// Empty the tables.
    ///
    /// # Errors
    ///
    /// A database error.
    pub async fn truncate(ctx: &AppContext) -> Result<()> {
        {
            use sea_orm::ConnectionTrait;
            ctx.db
                .execute_unprepared(
                    "TRUNCATE wpm_events, contact_employments, fx_rates, reports, taggings, tags, record_imports, attachments, custom_field_values, custom_field_defs, workflow_runs, workflows, email_templates, saved_views, lead_imports, notifications, form_submissions, webhook_deliveries, webhook_subscriptions, idempotency_keys, \
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
}

impl TestApp for App {
    async fn boot_test() -> Result<BootResult> {
        let environment = Environment::Test;
        let config = service_config(&environment)?;
        let ctx = Self::context(environment, config, false).await?;
        Migrator::fresh(&ctx.db).await.map_err(Error::from)?;
        let router = Self::router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn production_refuses_to_start_with_no_redis() {
        let refused = connect_cache(None, true).await.unwrap_err();
        assert!(refused.to_string().contains("CRM_REDIS_URL is required"));
    }

    #[tokio::test]
    async fn a_redis_url_that_does_not_parse_stops_the_start() {
        assert!(connect_cache(Some("not a url"), false).await.is_err());
    }

    #[tokio::test]
    async fn elsewhere_no_redis_means_a_cache_in_this_process() {
        let cache = connect_cache(None, false).await.unwrap();
        cache.set("key", &1_u32).await.unwrap();
        assert_eq!(cache.get::<u32>("key").await.unwrap(), Some(1));
    }

    #[test]
    fn with_neither_setting_cors_stays_closed() {
        assert!(cors_origins(None, None, vec!["*".to_string()]).is_empty());
    }

    #[test]
    fn the_older_single_origin_setting_still_works() {
        let origin = Some("https://crm.example.org".to_string());
        assert_eq!(
            cors_origins(None, origin, Vec::new()),
            ["https://crm.example.org"]
        );
    }

    #[test]
    fn the_list_wins_over_the_single_origin() {
        let listed = vec!["https://a.example".to_string()];
        let origin = Some("https://b.example".to_string());
        assert_eq!(
            cors_origins(Some("https://a.example"), origin, listed.clone()),
            listed
        );
    }

    #[test]
    fn the_defaults_keep_the_service_port_and_body_limit() {
        let defaults = defaults();
        assert_eq!((defaults.port, defaults.body_limit), (5150, 2_000_000));
    }

    #[test]
    fn production_does_not_migrate_by_itself() {
        let lookup = |name: &str| {
            (name == "DB_URL").then(|| "postgres://db/crm?sslmode=require".to_string())
        };
        let config =
            ServiceConfig::from_lookup(&Environment::Production, &super::defaults(), lookup)
                .unwrap();
        assert!(!config.database.auto_migrate);
    }

    #[test]
    fn an_unknown_environment_name_counts_as_production() {
        assert!(Environment::parse("prod-eu").is_production_like());
    }
}
