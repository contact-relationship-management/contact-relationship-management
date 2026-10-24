//! Outbound webhooks (CRM-R28): subscriptions that are told when records change.
//!
//! Admin or service callers only (the route prefix derives the destructive action, see
//! [`crate::auth::policy`]). The relay itself is in [`crate::webhooks`].

use crate::controllers::PidPath;
use crate::json::Json;
use chrono::{DateTime, Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::unprocessable;
use crate::auth::MaybeAuthUser;
use crate::models::audit_logs::Model as Audit;
use crate::rules::webhooks as rules;
use crate::validation::Problems;

/// The most subscriptions that can be live at once.
const MAX_SUBSCRIPTIONS: i64 = 20;
/// How many deliveries the log shows.
const LOG_LIMIT: i64 = 100;

/// `POST /api/webhook-subscriptions` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "WebhookSubscriptionPayload")]
pub(crate) struct SubscriptionPayload {
    /// Where to send events. `https` only, to a public address.
    url: String,
    /// Which events: `*`, `entity.*`, `*.kind` or `entity.kind`.
    events: Vec<String>,
}

/// A subscription as listed. The secret is never shown again.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WebhookSubscription")]
pub(crate) struct SubscriptionView {
    pid: Uuid,
    url: String,
    events: Vec<String>,
    enabled: bool,
    disabled_reason: Option<String>,
    /// Whether the previous secret still signs, after a rotation.
    rotating: bool,
    created_at: DateTime<Utc>,
}

/// `POST /api/webhook-subscriptions`: the subscription and its secret, shown once.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WebhookSubscriptionCreated")]
pub(crate) struct SubscriptionCreated {
    pid: Uuid,
    url: String,
    events: Vec<String>,
    /// The signing secret (`whsec_…`). Keep it: it is not shown again.
    secret: String,
}

/// `POST /api/webhook-subscriptions/{pid}/rotate-secret`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WebhookSecretRotated")]
pub(crate) struct SecretRotated {
    pid: Uuid,
    /// The new signing secret, shown once.
    secret: String,
    /// Until when the old secret also signs, so receivers can switch.
    previous_secret_valid_until: DateTime<Utc>,
}

/// One delivery in the log.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WebhookDelivery")]
pub(crate) struct DeliveryView {
    /// The event id: the `webhook-id` the receiver saw.
    event_id: Uuid,
    entity_pid: Uuid,
    /// `pending`, `delivered`, `dead` or `cancelled`.
    status: String,
    attempts: i32,
    next_attempt_at: DateTime<Utc>,
    last_status: Option<i32>,
    last_error: Option<String>,
    delivered_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

fn sql(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

fn patterns_of(value: serde_json::Value) -> Vec<String> {
    serde_json::from_value(value).unwrap_or_default()
}

/// `POST /api/webhook-subscriptions`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SubscriptionPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("url", &payload.url);
    let address = rules::check_target(&payload.url);
    if let Err(why) = &address
        && !payload.url.trim().is_empty()
    {
        problems.push_field("url", why.clone());
    }
    let events = rules::check_patterns(&payload.events);
    if let Err(why) = &events {
        problems.push_field("events", why.clone());
    }
    problems.finish()?;
    let (Ok(address), Ok(events)) = (address, events) else {
        return Err(Error::BadRequest("invalid".into()));
    };
    let live = ctx
        .db
        .query_one_raw(sql(
            "SELECT COUNT(*) AS n FROM webhook_subscriptions WHERE enabled",
            [],
        ))
        .await?
        .and_then(|r| r.try_get::<i64>("", "n").ok())
        .unwrap_or(0);
    if live >= MAX_SUBSCRIPTIONS {
        return Err(unprocessable(
            "at most 20 webhook subscriptions can be live",
        ));
    }
    let pid = Uuid::new_v4();
    let secret = rules::new_secret();
    let txn = ctx.db.begin().await?;
    txn.execute_raw(sql(
        "INSERT INTO webhook_subscriptions (pid, url, events, secret, created_by) \
         VALUES ($1, $2, $3, $4, $5)",
        [
            pid.into(),
            address.clone().into(),
            serde_json::json!(events).into(),
            crate::crypto::seal(&secret).into(),
            caller.actor().map(str::to_string).into(),
        ],
    ))
    .await?;
    // The address is not written to the audit trail: it can carry a token in its path.
    Audit::record(
        &txn,
        "webhook_subscription",
        pid,
        "webhook_subscription_created",
        caller.actor(),
        Some(serde_json::json!({ "events": events })),
    )
    .await?;
    txn.commit().await?;
    format::json(SubscriptionCreated {
        pid,
        url: address,
        events,
        secret,
    })
}

/// `GET /api/webhook-subscriptions`.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(sql(
            "SELECT pid, url, events, enabled, disabled_reason, created_at, \
                    (previous_secret IS NOT NULL AND previous_secret_expires_at > now()) AS rotating \
             FROM webhook_subscriptions ORDER BY id",
            [],
        ))
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(SubscriptionView {
            pid: row.try_get("", "pid")?,
            url: row.try_get("", "url")?,
            events: patterns_of(row.try_get("", "events")?),
            enabled: row.try_get("", "enabled")?,
            disabled_reason: row.try_get("", "disabled_reason")?,
            rotating: row.try_get("", "rotating")?,
            created_at: row.try_get("", "created_at")?,
        });
    }
    format::json(out)
}

/// `DELETE /api/webhook-subscriptions/{pid}`: disable it and cancel what is queued. The log stays.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(sql(
            "SELECT id FROM webhook_subscriptions WHERE pid = $1 FOR UPDATE",
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let id: i32 = row.try_get("", "id")?;
    txn.execute_raw(sql(
        "UPDATE webhook_subscriptions SET enabled = FALSE, disabled_at = now(), \
         disabled_reason = COALESCE(disabled_reason, 'removed'), updated_at = now() WHERE id = $1",
        [id.into()],
    ))
    .await?;
    let cancelled =
        crate::webhooks::cancel_pending(&txn, "subscription_id = $1", id.into(), "removed").await?;
    Audit::record(
        &txn,
        "webhook_subscription",
        pid,
        "webhook_subscription_removed",
        caller.actor(),
        Some(serde_json::json!({ "deliveries_cancelled": cancelled })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "deleted": pid, "deliveries_cancelled": cancelled }))
}

/// `POST /api/webhook-subscriptions/{pid}/rotate-secret`: a new secret now; the old one keeps
/// signing for 24 hours so the receiver can switch without dropping events.
#[debug_handler]
async fn rotate_secret(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let secret = rules::new_secret();
    let until = Utc::now() + Duration::hours(rules::ROTATION_GRACE_HOURS);
    let txn = ctx.db.begin().await?;
    let changed = txn
        .execute_raw(sql(
            "UPDATE webhook_subscriptions SET previous_secret = secret, \
             previous_secret_expires_at = $3, secret = $2, updated_at = now() WHERE pid = $1",
            [
                pid.into(),
                crate::crypto::seal(&secret).into(),
                until.into(),
            ],
        ))
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Audit::record(
        &txn,
        "webhook_subscription",
        pid,
        "webhook_secret_rotated",
        caller.actor(),
        Some(serde_json::json!({ "previous_valid_until": until })),
    )
    .await?;
    txn.commit().await?;
    format::json(SecretRotated {
        pid,
        secret,
        previous_secret_valid_until: until,
    })
}

/// `GET /api/webhook-subscriptions/{pid}/deliveries`: the latest deliveries, newest first.
#[debug_handler]
async fn deliveries(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    if ctx
        .db
        .query_one_raw(sql(
            "SELECT 1 FROM webhook_subscriptions WHERE pid = $1",
            [pid.into()],
        ))
        .await?
        .is_none()
    {
        return Err(Error::NotFound);
    }
    let rows = ctx
        .db
        .query_all_raw(sql(
            "SELECT d.event_id, d.entity_pid, d.status, d.attempts, d.next_attempt_at, \
                    d.last_status, d.last_error, d.delivered_at, d.created_at \
             FROM webhook_deliveries d JOIN webhook_subscriptions s ON s.id = d.subscription_id \
             WHERE s.pid = $1 ORDER BY d.id DESC LIMIT $2",
            [pid.into(), LOG_LIMIT.into()],
        ))
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(DeliveryView {
            event_id: row.try_get("", "event_id")?,
            entity_pid: row.try_get("", "entity_pid")?,
            status: row.try_get("", "status")?,
            attempts: row.try_get("", "attempts")?,
            next_attempt_at: row.try_get("", "next_attempt_at")?,
            last_status: row.try_get("", "last_status")?,
            last_error: row.try_get("", "last_error")?,
            delivered_at: row.try_get("", "delivered_at")?,
            created_at: row.try_get("", "created_at")?,
        });
    }
    format::json(out)
}

/// `POST /api/webhook-subscriptions/relay`: fan out new events and send what is due. Call it
/// on a schedule (every minute is typical).
#[debug_handler]
async fn relay(State(ctx): State<AppContext>) -> Result<Response> {
    let report = crate::webhooks::run(&ctx.db, &crate::webhooks::HttpSender, Utc::now()).await?;
    format::json(report)
}

/// The webhook routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/webhook-subscriptions", post(create))
        .add("/webhook-subscriptions", get(list))
        .add("/webhook-subscriptions/relay", post(relay))
        .add("/webhook-subscriptions/{pid}", delete(remove))
        .add(
            "/webhook-subscriptions/{pid}/rotate-secret",
            post(rotate_secret),
        )
        .add("/webhook-subscriptions/{pid}/deliveries", get(deliveries))
}
