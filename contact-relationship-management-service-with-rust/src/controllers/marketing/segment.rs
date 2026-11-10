//! Segments and their preview (CRM-R7).

use super::{
    AppContext, Audit, Deserialize, Error, MaybeAuthUser, PidRef, Problems, Response, Result,
    State, Uuid, contact_facts, contacts, debug_handler, format, segment_rules, segments, tokens,
};
use sqlx::PgPool;

use crate::controllers::PidPath;
use crate::db;
use crate::json::Json;
use crate::models::rows;

/// `POST /api/segments` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SegmentPayload {
    name: String,
    /// The filter language of `rules::segment` (see the segment spec).
    #[serde(default)]
    #[schemars(with = "serde_json::Map<String, serde_json::Value>")]
    filter: segment_rules::Filter,
}

/// `POST /api/segments`.
#[debug_handler]
pub(super) async fn create_segment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SegmentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    for status in &payload.filter.statuses {
        problems.require_token("filter.statuses", tokens::CONTACT_STATUSES, status);
    }
    for channel in &payload.filter.channels {
        problems.require_token("filter.channels", tokens::CHANNELS, channel);
    }
    for tier in &payload.filter.account_tiers {
        problems.require_token("filter.account_tiers", tokens::ACCOUNT_TIERS, tier);
    }
    // Tags are tidied the way a record's are, so a segment and a record agree on a name.
    let mut filter = payload.filter.clone();
    match crate::rules::tags::set_of(&payload.filter.tags) {
        Ok(tags) => filter.tags = tags,
        Err(why) => problems.push_field("filter.tags", why),
    }
    problems.finish()?;
    let pid = Uuid::new_v4();
    let mut txn = db::pool(&ctx).begin().await?;
    sqlx::query!(
        "INSERT INTO segments (pid, name, filter) VALUES ($1, $2, $3)",
        pid,
        payload.name,
        serde_json::to_value(&filter).unwrap_or_default(),
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(&mut txn, "segment", pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// The contacts a segment matches right now, gated on consent for `purpose`.
pub(super) async fn evaluate_segment(
    pool: &PgPool,
    segment: &segments::Model,
    purpose: &str,
) -> Result<Vec<contacts::Model>> {
    let filter: segment_rules::Filter =
        serde_json::from_value(segment.filter.clone()).unwrap_or_default();
    let all = rows::contacts(pool).await?;
    let mut conn = pool.acquire().await?;
    let mut matched = Vec::new();
    for contact in all {
        let facts = contact_facts(&mut conn, &contact, purpose).await?;
        if segment_rules::matches(&filter, &facts) {
            matched.push(contact);
        }
    }
    Ok(matched)
}

/// `GET /api/segments/{pid}/preview` query.
#[derive(Debug, Deserialize)]
pub(super) struct PreviewQuery {
    /// The consent purpose to gate on (`marketing` when absent).
    purpose: Option<String>,
}

/// `GET /api/segments/{pid}/preview`: how many contacts match, and a few names.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct Preview {
    count: usize,
    sample: Vec<String>,
}

/// `GET /api/segments/{pid}/preview` — count + sample before
/// scheduling (CRM-R7).
#[debug_handler]
pub(super) async fn preview_segment(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
    axa::extract::Query(query): axa::extract::Query<PreviewQuery>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt("purpose", tokens::PURPOSES, query.purpose.as_deref());
    problems.finish()?;
    let pool = db::pool(&ctx);
    let segment = rows::segment(pool, pid).await?.ok_or(Error::NotFound)?;
    let purpose = query.purpose.as_deref().unwrap_or(tokens::DEFAULT_PURPOSE);
    let matched = evaluate_segment(pool, &segment, purpose).await?;
    let sample: Vec<_> = matched
        .iter()
        .take(5)
        .map(|c| c.display_name.clone())
        .collect();
    format::json(Preview {
        count: matched.len(),
        sample,
    })
}

/// `GET /api/segments`.
#[debug_handler]
pub(super) async fn list_segments(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(rows::segments(db::pool(&ctx)).await?)
}
