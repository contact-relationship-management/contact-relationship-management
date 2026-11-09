//! The webhook relay (CRM-R28): turn new outbox rows into deliveries, then send what is due.
//!
//! The rules (signing, patterns, backoff, which addresses are allowed) are pure and live in
//! [`crate::rules::webhooks`]. This module owns the database and, behind [`Sender`], the network.
//!
//! A run is safe to repeat and to run twice at once:
//! - **fan out** marks each outbox row it handled in the same transaction that creates its
//!   deliveries, and `(subscription, event)` is unique;
//! - **send** first *claims* a delivery by pushing its next attempt five minutes ahead, so a
//!   second run skips it, and a crash mid-send only delays the retry.
//!
//! The statements are `sqlx` ones (CRM-D24, CRM-T206).

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::models::audit_logs::Model as Audit;
use crate::rules::webhooks as rules;

/// How many outbox rows one run fans out.
const FAN_OUT_BATCH: i64 = 500;
/// How many deliveries one run sends.
const SEND_BATCH: i64 = 50;
/// How long a claimed delivery is left alone before another run may take it.
const CLAIM_SECONDS: i64 = 300;
/// How long one send may take.
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// What came back from one send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The receiver answered with this status.
    Status(u16),
    /// No answer; the reason, which never contains the address.
    Failed(String),
}

/// The network seam. The real one is [`HttpSender`]; tests record instead.
#[async_trait::async_trait]
pub trait Sender: Send + Sync {
    /// POST `body` to `url` with `headers`.
    async fn post(&self, url: &str, headers: &[(&'static str, String)], body: String) -> Outcome;
}

/// Resolve `host` and keep it only if every answer is a public address.
///
/// # Errors
///
/// The name does not resolve, or any answer is private, loopback, link-local or reserved.
pub async fn public_addresses(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    let found: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| "the name did not resolve".to_string())?
        .collect();
    if found.is_empty() {
        return Err("the name did not resolve".to_string());
    }
    if found.iter().any(|a| !rules::is_public_ip(a.ip())) {
        return Err("the address is not public".to_string());
    }
    Ok(found)
}

/// Sends over HTTPS, to an address that was checked at this send and connected to as
/// checked (so a name that changes its answer between the check and the connection cannot
/// redirect the call), without following redirects.
#[derive(Debug, Default)]
pub struct HttpSender;

#[async_trait::async_trait]
impl Sender for HttpSender {
    async fn post(&self, url: &str, headers: &[(&'static str, String)], body: String) -> Outcome {
        let Ok(parsed) = url::Url::parse(url) else {
            return Outcome::Failed("the address is not valid".into());
        };
        if rules::check_target(url).is_err() {
            return Outcome::Failed("the address is not allowed".into());
        }
        let port = parsed.port_or_known_default().unwrap_or(443);
        let mut builder = reqwest::Client::builder()
            .timeout(SEND_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .https_only(true);
        match parsed.host() {
            Some(url::Host::Domain(name)) => match public_addresses(name, port).await {
                Ok(addresses) => builder = builder.resolve_to_addrs(name, &addresses),
                Err(why) => return Outcome::Failed(why),
            },
            Some(url::Host::Ipv4(ip)) => {
                if !rules::is_public_ip(IpAddr::V4(ip)) {
                    return Outcome::Failed("the address is not public".into());
                }
            }
            Some(url::Host::Ipv6(ip)) => {
                if !rules::is_public_ip(IpAddr::V6(ip)) {
                    return Outcome::Failed("the address is not public".into());
                }
            }
            None => return Outcome::Failed("the address has no host".into()),
        }
        let Ok(client) = builder.build() else {
            return Outcome::Failed("the client could not be built".into());
        };
        let mut request = client.post(url).body(body);
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        match request.send().await {
            Ok(response) => Outcome::Status(response.status().as_u16()),
            Err(error) if error.is_timeout() => Outcome::Failed("timed out".into()),
            Err(error) if error.is_connect() => Outcome::Failed("could not connect".into()),
            Err(_) => Outcome::Failed("the request failed".into()),
        }
    }
}

/// What one relay run did.
#[derive(Debug, Default, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WebhookRelay")]
pub struct Report {
    /// Outbox rows turned into deliveries (or passed over because nobody wanted them).
    pub events_fanned_out: u64,
    /// Deliveries created.
    pub deliveries_created: u64,
    /// Deliveries attempted.
    pub attempted: u64,
    /// Attempts the receiver accepted (`2xx`).
    pub delivered: u64,
    /// Attempts that failed and will be tried again.
    pub retrying: u64,
    /// Deliveries that ran out of attempts.
    pub dead: u64,
    /// Subscriptions disabled by a `410 Gone`.
    pub subscriptions_disabled: u64,
}

/// Create the deliveries for outbox rows not yet handled, oldest first.
async fn fan_out(
    pool: &PgPool,
    now: DateTime<Utc>,
    report: &mut Report,
) -> Result<(), sqlx::Error> {
    let mut txn = pool.begin().await?;
    let rows = sqlx::query!(
        "SELECT id, event_id, entity, kind, entity_pid, occurred_at FROM event_outbox \
         WHERE webhook_fanned_out_at IS NULL ORDER BY id LIMIT $1 \
         FOR UPDATE SKIP LOCKED",
        FAN_OUT_BATCH
    )
    .fetch_all(&mut *txn)
    .await?;
    if rows.is_empty() {
        return txn.commit().await;
    }
    let subscriptions: Vec<(i32, Vec<String>, DateTime<Utc>)> =
        sqlx::query!("SELECT id, events, created_at FROM webhook_subscriptions WHERE enabled")
            .fetch_all(&mut *txn)
            .await?
            .into_iter()
            .filter_map(|s| {
                let patterns: Vec<String> = serde_json::from_value(s.events).ok()?;
                Some((s.id, patterns, s.created_at))
            })
            .collect();
    let mut ids: Vec<i32> = Vec::with_capacity(rows.len());
    for row in &rows {
        for (subscription, patterns, since) in &subscriptions {
            // A subscription hears what happens after it exists, not the backlog.
            if row.occurred_at >= *since && rules::wants(patterns, &row.entity, &row.kind) {
                let made = sqlx::query!(
                    "INSERT INTO webhook_deliveries (subscription_id, event_id, entity_pid, next_attempt_at) \
                     VALUES ($1, $2, $3, $4) ON CONFLICT (subscription_id, event_id) DO NOTHING",
                    subscription,
                    row.event_id,
                    row.entity_pid,
                    now,
                )
                .execute(&mut *txn)
                .await?;
                report.deliveries_created += made.rows_affected();
            }
        }
        ids.push(row.id);
        report.events_fanned_out += 1;
    }
    sqlx::query!(
        "UPDATE event_outbox SET webhook_fanned_out_at = CURRENT_TIMESTAMP WHERE id = ANY($1)",
        &ids
    )
    .execute(&mut *txn)
    .await?;
    txn.commit().await
}

/// Claim up to [`SEND_BATCH`] due deliveries and return their ids.
async fn claim_due(pool: &PgPool, now: DateTime<Utc>) -> Result<Vec<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE webhook_deliveries SET next_attempt_at = $2 + make_interval(secs => $3) \
         WHERE id IN (SELECT d.id FROM webhook_deliveries d \
                      JOIN webhook_subscriptions s ON s.id = d.subscription_id \
                      WHERE d.status = 'pending' AND d.next_attempt_at <= $2 AND s.enabled \
                      ORDER BY d.next_attempt_at, d.id LIMIT $1 \
                      FOR UPDATE OF d SKIP LOCKED) \
         RETURNING id",
        SEND_BATCH,
        now,
        f64::from(i32::try_from(CLAIM_SECONDS).unwrap_or(300)),
    )
    .fetch_all(pool)
    .await
}

/// Run the relay once: fan out, then send what is due.
///
/// # Errors
///
/// A database error. A receiver that fails is not an error: it is counted in the report.
pub async fn run(
    pool: &PgPool,
    sender: &dyn Sender,
    now: DateTime<Utc>,
) -> Result<Report, sqlx::Error> {
    let mut report = Report::default();
    fan_out(pool, now, &mut report).await?;
    for id in claim_due(pool, now).await? {
        send_one(pool, sender, id, now, &mut report).await?;
    }
    Ok(report)
}

/// A claimed delivery, ready to send.
struct Prepared {
    attempts: i32,
    subscription: i32,
    subscription_pid: Uuid,
    url: String,
    headers: Vec<(&'static str, String)>,
    body: String,
}

/// What loading a claimed delivery came to.
enum Loaded {
    /// Nothing to do: it was cancelled or finished since the claim.
    Skip,
    /// Close it without sending, for this reason.
    Close(&'static str, i32),
    /// Send this.
    Send(Prepared),
}

/// Load a claimed delivery with its subscription and event, and sign it.
async fn prepare(pool: &PgPool, id: i32, now: DateTime<Utc>) -> Result<Loaded, sqlx::Error> {
    // The event is a left join: it may have been pruned since the delivery was made.
    let Some(row) = sqlx::query!(
        "SELECT d.attempts, d.event_id, d.subscription_id AS sub, s.pid AS sub_pid, s.url, \
                s.secret, s.previous_secret, s.previous_secret_expires_at, \
                o.entity AS \"entity?\", o.kind AS \"kind?\", o.entity_pid AS \"entity_pid?\", \
                o.occurred_at AS \"occurred_at?\" \
         FROM webhook_deliveries d \
         JOIN webhook_subscriptions s ON s.id = d.subscription_id \
         LEFT JOIN event_outbox o ON o.event_id = d.event_id \
         WHERE d.id = $1 AND d.status = 'pending'",
        id
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(Loaded::Skip);
    };
    let attempts = row.attempts;
    let event_id = row.event_id;
    let (Some(entity), Some(kind), Some(entity_pid), Some(occurred_at)) =
        (row.entity, row.kind, row.entity_pid, row.occurred_at)
    else {
        return Ok(Loaded::Close("the event is gone", attempts));
    };

    let mut secrets: Vec<String> = Vec::new();
    if let Ok(secret) = crate::crypto::open(&row.secret) {
        secrets.push(secret);
    }
    let previous_valid = row
        .previous_secret_expires_at
        .is_some_and(|until| until > now);
    if previous_valid
        && let Some(sealed) = &row.previous_secret
        && let Ok(secret) = crate::crypto::open(sealed)
    {
        secrets.push(secret);
    }
    let body = rules::body(event_id, &entity, &kind, entity_pid, occurred_at);
    let timestamp = now.timestamp();
    let refs: Vec<&str> = secrets.iter().map(String::as_str).collect();
    let Ok(signature) = rules::signature_header(&refs, &event_id.to_string(), timestamp, &body)
    else {
        return Ok(Loaded::Close("no usable signing secret", attempts + 1));
    };
    Ok(Loaded::Send(Prepared {
        attempts,
        subscription: row.sub,
        subscription_pid: row.sub_pid,
        url: row.url,
        headers: vec![
            ("content-type", "application/json".to_string()),
            ("webhook-id", event_id.to_string()),
            ("webhook-timestamp", timestamp.to_string()),
            ("webhook-signature", signature),
        ],
        body,
    }))
}

/// Send one claimed delivery and record what happened.
async fn send_one(
    pool: &PgPool,
    sender: &dyn Sender,
    id: i32,
    now: DateTime<Utc>,
    report: &mut Report,
) -> Result<(), sqlx::Error> {
    let prepared = match prepare(pool, id, now).await? {
        Loaded::Skip => return Ok(()),
        Loaded::Close(why, attempts) => {
            let status = if why == "the event is gone" {
                "cancelled"
            } else {
                "dead"
            };
            if status == "dead" {
                report.dead += 1;
            }
            return finish(pool, id, status, attempts, None, Some(why)).await;
        }
        Loaded::Send(prepared) => prepared,
    };
    report.attempted += 1;
    let attempts = prepared.attempts + 1;
    let outcome = sender
        .post(&prepared.url, &prepared.headers, prepared.body.clone())
        .await;
    match outcome {
        Outcome::Status(code) if (200..300).contains(&code) => {
            report.delivered += 1;
            finish(pool, id, "delivered", attempts, Some(code), None).await
        }
        Outcome::Status(410) => {
            report.subscriptions_disabled += 1;
            disable_gone(pool, id, attempts, &prepared, now).await
        }
        failure => {
            let (status, why) = match failure {
                Outcome::Status(code) => (Some(code), format!("the receiver answered {code}")),
                Outcome::Failed(why) => (None, why),
            };
            if let Some(delay) = rules::retry_delay_seconds(attempts) {
                report.retrying += 1;
                let next = now + chrono::Duration::seconds(delay);
                let mut conn = pool.acquire().await?;
                record_attempt(
                    &mut conn,
                    id,
                    "pending",
                    attempts,
                    status,
                    Some(&why),
                    Some(next),
                )
                .await
            } else {
                report.dead += 1;
                finish(pool, id, "dead", attempts, status, Some(&why)).await
            }
        }
    }
}

/// A `410 Gone`: the receiver does not want these any more. Disable the subscription, cancel what
/// is queued for it, and audit.
async fn disable_gone(
    pool: &PgPool,
    id: i32,
    attempts: i32,
    prepared: &Prepared,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    const WHY: &str = "the receiver answered 410 Gone";
    let mut txn = pool.begin().await?;
    sqlx::query!(
        "UPDATE webhook_subscriptions SET enabled = FALSE, disabled_at = $2, \
         disabled_reason = $3, updated_at = $2 WHERE id = $1 AND enabled",
        prepared.subscription,
        now,
        WHY,
    )
    .execute(&mut *txn)
    .await?;
    cancel_pending(&mut txn, prepared.subscription, WHY).await?;
    record_attempt(
        &mut txn,
        id,
        "cancelled",
        attempts,
        Some(410),
        Some(WHY),
        None,
    )
    .await?;
    Audit::record_on(
        &mut txn,
        "webhook_subscription",
        prepared.subscription_pid,
        "webhook_subscription_disabled",
        None,
        Some(serde_json::json!({ "reason": "410 Gone" })),
    )
    .await?;
    txn.commit().await
}

/// Mark a delivery finished (delivered, dead or cancelled).
async fn finish(
    pool: &PgPool,
    id: i32,
    status: &str,
    attempts: i32,
    code: Option<u16>,
    why: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut conn = pool.acquire().await?;
    record_attempt(&mut conn, id, status, attempts, code, why, None).await
}

async fn record_attempt(
    conn: &mut PgConnection,
    id: i32,
    status: &str,
    attempts: i32,
    code: Option<u16>,
    why: Option<&str>,
    next: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE webhook_deliveries SET status = $2::text, attempts = $3, last_status = $4, \
         last_error = $5, next_attempt_at = COALESCE($6, next_attempt_at), \
         delivered_at = CASE WHEN $2::text = 'delivered' THEN CURRENT_TIMESTAMP ELSE delivered_at END, \
         updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        id,
        status,
        attempts,
        code.map(i32::from),
        why,
        next,
    )
    .execute(conn)
    .await
    .map(|_| ())
}

/// Cancel the queued deliveries of one subscription; returns how many.
///
/// # Errors
///
/// A database error.
pub async fn cancel_pending(
    conn: &mut PgConnection,
    subscription_id: i32,
    why: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE webhook_deliveries SET status = 'cancelled', last_error = $2, \
         updated_at = CURRENT_TIMESTAMP WHERE status = 'pending' AND subscription_id = $1",
        subscription_id,
        why,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}
