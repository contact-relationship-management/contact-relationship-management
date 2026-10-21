//! Segments and their preview (CRM-R7).

use super::{
    ActiveModelTrait, ActiveValue, AppContext, Audit, ColumnTrait, DatabaseConnection, Deserialize,
    EntityTrait, MaybeAuthUser, Path, PidRef, Problems, QueryFilter, QueryOrder, Response, Result,
    State, TransactionTrait, Uuid, contact_facts, contacts, debug_handler, ensure_valid, format,
    records, segment_rules, segments, tokens,
};
use crate::json::Json;

/// `POST /api/segments` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SegmentPayload {
    name: String,
    #[serde(default)]
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
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = segments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        filter: ActiveValue::set(serde_json::to_value(&payload.filter).unwrap_or_default()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "segment", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// The contacts a segment matches right now (consent-gated).
pub(super) async fn evaluate_segment(
    db: &DatabaseConnection,
    segment: &segments::Model,
) -> Result<Vec<contacts::Model>> {
    let filter: segment_rules::Filter =
        serde_json::from_value(segment.filter.clone()).unwrap_or_default();
    let all = contacts::Entity::find()
        .filter(contacts::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let mut matched = Vec::new();
    for contact in all {
        let facts = contact_facts(db, &contact).await?;
        if segment_rules::matches(&filter, &facts) {
            matched.push(contact);
        }
    }
    Ok(matched)
}

/// `GET /api/segments/{pid}/preview` — count + sample before
/// scheduling (CRM-R7).
#[debug_handler]
pub(super) async fn preview_segment(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let segment = records::find_segment(&ctx.db, records::parse_pid(&pid)?).await?;
    let matched = evaluate_segment(&ctx.db, &segment).await?;
    let sample: Vec<_> = matched
        .iter()
        .take(5)
        .map(|c| c.display_name.clone())
        .collect();
    format::json(serde_json::json!({ "count": matched.len(), "sample": sample }))
}

/// `GET /api/segments`.
#[debug_handler]
pub(super) async fn list_segments(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = segments::Entity::find()
        .filter(segments::Column::DeletedAt.is_null())
        .order_by_asc(segments::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}
