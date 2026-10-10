//! Break-glass endpoints (CRM-T44): take a reasoned, time-limited read
//! elevation; list grants for review; record the after-the-fact review.
//! See [`crate::break_glass`] for the rules and defaults.

use std::collections::HashMap;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use super::{ensure_valid, record_rejection};
use crate::auth::{AuthUser, MaybeAuthUser};
use crate::break_glass as rules;
use crate::models::audit_logs::Model as Audit;
use crate::models::records;

/// Body of `POST /api/break-glass`.
#[derive(Debug, Deserialize)]
struct GrantPayload {
    reason: String,
    #[serde(default)]
    minutes: Option<i64>,
}

/// Body of `POST /api/break-glass/{pid}/review`.
#[derive(Debug, Deserialize)]
struct ReviewPayload {
    note: String,
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
        .map_err(|message| ensure_valid(&[message]).unwrap_err())?;
    let now = Utc::now();
    let expires_at = rules::expiry(now, minutes);
    let pid = Uuid::new_v4();
    let txn = ctx.db.begin().await?;
    txn.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO break_glass_grants (pid, subject, reason, granted_at, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
        [
            pid.into(),
            claims.sub.clone().into(),
            reason.clone().into(),
            now.into(),
            expires_at.into(),
        ],
    ))
    .await?;
    Audit::record(
        &txn,
        "break_glass",
        pid,
        "break_glass_granted",
        Some(&claims.sub),
        Some(serde_json::json!({ "reason": reason, "minutes": minutes, "expires_at": expires_at })),
    )
    .await?;
    txn.commit().await?;
    rules::registry().grant(&claims.sub, expires_at);
    format::json(serde_json::json!({ "pid": pid, "expires_at": expires_at, "minutes": minutes }))
}

/// `GET /api/break-glass` — the grants, newest first, with the unreviewed
/// count a reviewer works from (admin/service tier).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pid, subject, reason, granted_at, expires_at, reviewed_at, reviewed_by, \
             review_note FROM break_glass_grants ORDER BY id DESC LIMIT 200"
                .to_string(),
        ))
        .await?;
    let mut grants = Vec::with_capacity(rows.len());
    let mut unreviewed = 0_u64;
    for row in &rows {
        let reviewed_at: Option<DateTime<Utc>> = row.try_get("", "reviewed_at").ok();
        if reviewed_at.is_none() {
            unreviewed += 1;
        }
        grants.push(serde_json::json!({
            "pid": row.try_get::<Uuid>("", "pid").ok(),
            "subject": row.try_get::<String>("", "subject").ok(),
            "reason": row.try_get::<String>("", "reason").ok(),
            "granted_at": row.try_get::<DateTime<Utc>>("", "granted_at").ok(),
            "expires_at": row.try_get::<DateTime<Utc>>("", "expires_at").ok(),
            "reviewed_at": reviewed_at,
            "reviewed_by": row.try_get::<String>("", "reviewed_by").ok(),
            "review_note": row.try_get::<String>("", "review_note").ok(),
        }));
    }
    format::json(serde_json::json!({ "unreviewed": unreviewed, "grants": grants }))
}

/// `POST /api/break-glass/{pid}/review` — record the after-the-fact review
/// (admin/service tier). Refused when the reviewer is the grantee, or the
/// grant is already reviewed.
#[debug_handler]
async fn review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ReviewPayload>,
) -> Result<Response> {
    let pid = records::parse_pid(&pid)?;
    let row = ctx
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT subject, reviewed_at FROM break_glass_grants WHERE pid = $1",
            [pid.into()],
        ))
        .await?
        .ok_or_else(|| Error::NotFound)?;
    let subject: String = row.try_get("", "subject")?;
    let already: Option<DateTime<Utc>> = row.try_get("", "reviewed_at").ok();
    if already.is_some() {
        return Err(super::unprocessable("this grant has already been reviewed"));
    }
    let reviewer = caller.actor().unwrap_or("unauthenticated").to_string();
    let note = rules::validate_review(&subject, &reviewer, &payload.note).map_err(|message| {
        if subject == reviewer {
            record_rejection((StatusCode::FORBIDDEN, message))
        } else {
            ensure_valid(&[message]).unwrap_err()
        }
    })?;
    let txn = ctx.db.begin().await?;
    txn.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE break_glass_grants SET reviewed_at = now(), reviewed_by = $2, review_note = $3 \
         WHERE pid = $1",
        [pid.into(), reviewer.clone().into(), note.clone().into()],
    ))
    .await?;
    Audit::record(
        &txn,
        "break_glass",
        pid,
        "break_glass_reviewed",
        Some(&reviewer),
        Some(serde_json::json!({ "grantee": subject, "note": note })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pid": pid, "reviewed_by": reviewer }))
}

/// Reload the live grants from the database into the process registry.
///
/// # Errors
///
/// When the query fails.
pub async fn refresh(db: &DatabaseConnection) -> std::result::Result<(), sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT subject, max(expires_at) AS expires_at FROM break_glass_grants \
             WHERE expires_at > now() GROUP BY subject"
                .to_string(),
        ))
        .await?;
    let mut live = HashMap::new();
    for row in &rows {
        if let (Ok(subject), Ok(expires_at)) = (
            row.try_get::<String>("", "subject"),
            row.try_get::<DateTime<Utc>>("", "expires_at"),
        ) {
            live.insert(subject, expires_at);
        }
    }
    rules::registry().replace(live);
    Ok(())
}

/// The break-glass routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/break-glass", post(grant))
        .add("/break-glass", get(list))
        .add("/break-glass/{pid}/review", post(review))
}
