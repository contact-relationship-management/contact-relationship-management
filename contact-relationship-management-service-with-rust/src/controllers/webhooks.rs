//! Outbound webhooks (CRM-R28): subscriptions that are told when records change.
//!
//! Admin or service callers only (the route prefix derives the destructive action, see
//! [`crate::auth::policy`]). The relay itself is in [`crate::webhooks`]. Runs on `sqlx` (CRM-D24,
//! CRM-T206).

use crate::controllers::PidPath;
use crate::json::Json;
use axa::prelude::*;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::unprocessable;
use crate::auth::MaybeAuthUser;
use crate::db;
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
    let live =
        sqlx::query_scalar!("SELECT COUNT(*) AS \"n!\" FROM webhook_subscriptions WHERE enabled")
            .fetch_one(db::pool(&ctx))
            .await?;
    if live >= MAX_SUBSCRIPTIONS {
        return Err(unprocessable(
            "at most 20 webhook subscriptions can be live",
        ));
    }
    let pid = Uuid::new_v4();
    let secret = rules::new_secret();
    let mut txn = db::pool(&ctx).begin().await?;
    // Stamped here, not by the database's default: the relay compares this with an event's
    // `occurred_at`, which `streaming` stamps from this clock, and the two clocks can differ.
    sqlx::query!(
        "INSERT INTO webhook_subscriptions (pid, url, events, secret, created_by, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        pid,
        address,
        serde_json::json!(events),
        crate::crypto::seal(&secret),
        caller.actor(),
        crate::clock::now(),
    )
    .execute(&mut *txn)
    .await?;
    // The address is not written to the audit trail: it can carry a token in its path.
    Audit::record_on(
        &mut txn,
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
    let rows = sqlx::query!(
        "SELECT pid, url, events, enabled, disabled_reason, created_at, \
                COALESCE(previous_secret IS NOT NULL AND previous_secret_expires_at > now(), FALSE) \
                  AS \"rotating!\" \
         FROM webhook_subscriptions ORDER BY id"
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    let out: Vec<SubscriptionView> = rows
        .into_iter()
        .map(|row| SubscriptionView {
            pid: row.pid,
            url: row.url,
            events: patterns_of(row.events),
            enabled: row.enabled,
            disabled_reason: row.disabled_reason,
            rotating: row.rotating,
            created_at: row.created_at,
        })
        .collect();
    format::json(out)
}

/// `DELETE /api/webhook-subscriptions/{pid}`: disable it and cancel what is queued. The log stays.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let id = sqlx::query_scalar!(
        "SELECT id FROM webhook_subscriptions WHERE pid = $1 FOR UPDATE",
        pid
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    sqlx::query!(
        "UPDATE webhook_subscriptions SET enabled = FALSE, disabled_at = now(), \
         disabled_reason = COALESCE(disabled_reason, 'removed'), updated_at = now() WHERE id = $1",
        id
    )
    .execute(&mut *txn)
    .await?;
    let cancelled = crate::webhooks::cancel_pending(&mut txn, id, "removed").await?;
    Audit::record_on(
        &mut txn,
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
    let until = crate::clock::now() + Duration::hours(rules::ROTATION_GRACE_HOURS);
    let mut txn = db::pool(&ctx).begin().await?;
    let changed = sqlx::query!(
        "UPDATE webhook_subscriptions SET previous_secret = secret, \
         previous_secret_expires_at = $3, secret = $2, updated_at = now() WHERE pid = $1",
        pid,
        crate::crypto::seal(&secret),
        until,
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Audit::record_on(
        &mut txn,
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
    let pool = db::pool(&ctx);
    if sqlx::query_scalar!(
        "SELECT 1 AS \"one!\" FROM webhook_subscriptions WHERE pid = $1",
        pid
    )
    .fetch_optional(pool)
    .await?
    .is_none()
    {
        return Err(Error::NotFound);
    }
    let out = sqlx::query_as!(
        DeliveryView,
        "SELECT d.event_id, d.entity_pid, d.status, d.attempts, d.next_attempt_at, \
                d.last_status, d.last_error, d.delivered_at, d.created_at \
         FROM webhook_deliveries d JOIN webhook_subscriptions s ON s.id = d.subscription_id \
         WHERE s.pid = $1 ORDER BY d.id DESC LIMIT $2",
        pid,
        LOG_LIMIT,
    )
    .fetch_all(pool)
    .await?;
    format::json(out)
}

/// `POST /api/webhook-subscriptions/relay`: fan out new events and send what is due. Call it
/// on a schedule (every minute is typical).
#[debug_handler]
async fn relay(State(ctx): State<AppContext>) -> Result<Response> {
    let report = crate::webhooks::run(
        db::pool(&ctx),
        &crate::webhooks::HttpSender,
        crate::clock::now(),
    )
    .await?;
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
