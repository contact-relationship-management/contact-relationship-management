//! Break-glass endpoints (CRM-T44): take a reasoned, time-limited read
//! elevation; list grants for review; record the after-the-fact review.
//! See [`crate::break_glass`] for the rules and defaults.
//!
//! Runs on `sqlx` (CRM-D24, CRM-T206).

use crate::controllers::PidPath;
use crate::json::Json;
use std::collections::HashMap;

use axa::prelude::*;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_field, record_rejection};
use crate::auth::{AuthUser, MaybeAuthUser};
use crate::break_glass as rules;
use crate::db;
use crate::models::audit_logs::Model as Audit;

/// Body of `POST /api/break-glass`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GrantPayload {
    reason: String,
    #[serde(default)]
    minutes: Option<i64>,
}

/// Body of `POST /api/break-glass/{pid}/review`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewPayload {
    note: String,
}

/// `POST /api/break-glass`: the grant that was taken.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct Grant {
    pid: Uuid,
    expires_at: DateTime<Utc>,
    minutes: i64,
}

/// One grant in the review list.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "GrantRecord")]
pub(crate) struct GrantRecord {
    pid: Uuid,
    subject: String,
    reason: String,
    granted_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    reviewed_at: Option<DateTime<Utc>>,
    reviewed_by: Option<String>,
    review_note: Option<String>,
}

/// `GET /api/break-glass`: the latest grants and how many are still unreviewed.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct Grants {
    unreviewed: u64,
    grants: Vec<GrantRecord>,
}

/// `POST /api/break-glass/{pid}/review`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct Reviewed {
    pid: Uuid,
    reviewed_by: String,
}

/// `POST /api/break-glass` — take a read elevation. Any authenticated caller
/// may ask (that is the point of break-glass), so this route derives the
/// `read` action; what the elevation unlocks is decided by the mounted
/// policy's `env.break_glass` rules. Audited with the reason.
#[debug_handler]
async fn grant(
    State(ctx): State<AppContext>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<GrantPayload>,
) -> Result<Response> {
    let (reason, minutes) = rules::validate_request(&payload.reason, payload.minutes)
        .map_err(|e| invalid_field(e.field(), &e.to_string()))?;
    let now = crate::clock::now();
    let expires_at = rules::expiry(now, minutes);
    let pid = Uuid::new_v4();
    let mut txn = db::pool(&ctx).begin().await?;
    sqlx::query!(
        "INSERT INTO break_glass_grants (pid, subject, reason, granted_at, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
        pid,
        claims.sub,
        reason,
        now,
        expires_at,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "break_glass",
        pid,
        "break_glass_granted",
        Some(&claims.sub),
        Some(serde_json::json!({ "reason": reason, "minutes": minutes, "expires_at": expires_at })),
    )
    .await?;
    txn.commit().await?;
    rules::registry().grant(&claims.sub, expires_at);
    if let Some(shared) = crate::redis_store::global()
        && let Err(error) = shared.put_grant(&claims.sub, expires_at).await
    {
        // The database copy still reaches other replicas within one refresh.
        tracing::warn!(%error, "break-glass grant not published to Redis");
    }
    format::json(Grant {
        pid,
        expires_at,
        minutes,
    })
}

/// `GET /api/break-glass` — the grants, newest first, with the unreviewed
/// count a reviewer works from (admin/service tier).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let grants = sqlx::query_as!(
        GrantRecord,
        "SELECT pid, subject, reason, granted_at, expires_at, reviewed_at, reviewed_by, \
         review_note FROM break_glass_grants ORDER BY id DESC LIMIT 200"
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    let unreviewed = grants.iter().filter(|g| g.reviewed_at.is_none()).count() as u64;
    format::json(Grants { unreviewed, grants })
}

/// `POST /api/break-glass/{pid}/review` — record the after-the-fact review
/// (admin/service tier). Refused when the reviewer is the grantee, or the
/// grant is already reviewed.
#[debug_handler]
async fn review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ReviewPayload>,
) -> Result<Response> {
    let row = sqlx::query!(
        "SELECT subject, reviewed_at FROM break_glass_grants WHERE pid = $1",
        pid
    )
    .fetch_optional(db::pool(&ctx))
    .await?
    .ok_or(Error::NotFound)?;
    let subject = row.subject;
    if row.reviewed_at.is_some() {
        return Err(super::unprocessable("this grant has already been reviewed"));
    }
    let reviewer = caller.actor().unwrap_or("unauthenticated").to_string();
    let note = rules::validate_review(&subject, &reviewer, &payload.note).map_err(|e| match e {
        rules::ReviewError::SelfReview => record_rejection((StatusCode::FORBIDDEN, e.to_string())),
        rules::ReviewError::NoteTooShort => invalid_field("note", &e.to_string()),
    })?;
    let mut txn = db::pool(&ctx).begin().await?;
    sqlx::query!(
        "UPDATE break_glass_grants SET reviewed_at = now(), reviewed_by = $2, review_note = $3 \
         WHERE pid = $1",
        pid,
        reviewer,
        note,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "break_glass",
        pid,
        "break_glass_reviewed",
        Some(&reviewer),
        Some(serde_json::json!({ "grantee": subject, "note": note })),
    )
    .await?;
    txn.commit().await?;
    format::json(Reviewed {
        pid,
        reviewed_by: reviewer,
    })
}

/// Reload the live grants from the database into the process registry.
///
/// # Errors
///
/// When the query fails.
pub async fn refresh(
    pool: &PgPool,
) -> std::result::Result<HashMap<String, DateTime<Utc>>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT subject, max(expires_at) AS \"expires_at!\" FROM break_glass_grants \
         WHERE expires_at > now() GROUP BY subject"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.subject, row.expires_at))
        .collect())
}

/// The break-glass routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/break-glass", post(grant))
        .add("/break-glass", get(list))
        .add("/break-glass/{pid}/review", post(review))
}
