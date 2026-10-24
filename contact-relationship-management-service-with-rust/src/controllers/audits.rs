//! Audit + event read surface: the recent trail, the per-record
//! slice, the **owner-scoped query** (spec `audit.md`), and the
//! recent event stream.

use crate::controllers::PidPath;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::audit_logs::Model as Audit;
use crate::models::event_outbox::Model as Outbox;
use crate::streaming;

/// `GET /api/audits/recent` — the most recent entries (cap 100).
#[debug_handler]
async fn recent(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(Audit::recent(&ctx.db, 100).await?)
}

/// The first row where the hash chain breaks.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ChainBreak {
    id: i64,
    reason: &'static str,
}

/// `GET /api/audits/verify`: the result of recomputing the chain.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ChainVerdict {
    ok: bool,
    rows: usize,
    legacy_rows: usize,
    head: Option<String>,
    first_break: Option<ChainBreak>,
}

/// `GET /api/audits/verify` — recompute the tamper-evident hash chain
/// (CRM-T31). Returns the row count, legacy (pre-chain) rows, the head hash
/// to anchor outside the database, and the first break if any.
#[debug_handler]
async fn verify(State(ctx): State<AppContext>) -> Result<Response> {
    let verdict = crate::models::audit_logs::verify_chain(&ctx.db).await?;
    format::json(ChainVerdict {
        ok: verdict.ok(),
        rows: verdict.rows,
        legacy_rows: verdict.legacy_rows,
        head: verdict.head,
        first_break: verdict
            .first_break
            .map(|(id, reason)| ChainBreak { id, reason }),
    })
}

/// `GET /api/audits/{entity_pid}` — one record's trail.
#[debug_handler]
async fn for_entity(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    format::json(Audit::for_entity(&ctx.db, pid).await?)
}

/// `GET /api/audits?owner=<worker-urn>&since=<rfc3339>` — everything
/// that changed for one owner since a timestamp (cap 200).
#[derive(Debug, Deserialize)]
struct OwnerParams {
    owner: String,
    since: chrono::DateTime<chrono::FixedOffset>,
}

#[debug_handler]
async fn owner(
    State(ctx): State<AppContext>,
    Query(params): Query<OwnerParams>,
) -> Result<Response> {
    format::json(Audit::for_owner_since(&ctx.db, &params.owner, params.since, 200).await?)
}

/// `GET /api/events/recent` — the flat event view (memory ring under
/// the default transport; the outbox table under `outbox`).
#[debug_handler]
async fn events_recent(State(ctx): State<AppContext>) -> Result<Response> {
    if streaming::transport().is_outbox() {
        format::json(Outbox::recent(&ctx.db, 100).await?)
    } else {
        format::json(streaming::recent(100))
    }
}

/// The audit/event routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/audits/recent", get(recent))
        .add("/audits", get(owner))
        .add("/audits/verify", get(verify))
        .add("/audits/{entity_pid}", get(for_entity))
        .add("/events/recent", get(events_recent))
}
