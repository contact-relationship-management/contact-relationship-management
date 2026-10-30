//! `GET /api/search?q=` (CRM-R33): names and titles across the main records.

use axum::http::Uri;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::paging::{Page, PageParams};
use super::unprocessable;
use crate::auth::{self, MaybeAuthUser};
use crate::models::audit_logs::Model as Audit;
use crate::rules::search as rules;

/// Most hits in one page.
const PAGE_CAP: u64 = 50;

/// One hit: the record, named, with one plain descriptor.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "SearchHit")]
pub(crate) struct SearchHit {
    /// `contact`, `account`, `lead`, `deal`, `ticket`, `campaign` or `article`.
    kind: String,
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    title: String,
    /// A job title, industry, status or address domain; absent when there is none.
    detail: Option<String>,
}

/// `GET /api/search` query.
#[derive(Debug, Deserialize)]
pub(crate) struct SearchQuery {
    #[serde(default)]
    q: String,
}

/// `GET /api/search?q=` — best match first, without records the caller may not read.
#[debug_handler]
async fn search(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(query): Query<SearchQuery>,
    Query(paging): Query<PageParams>,
    uri: Uri,
) -> Result<Response> {
    let typed = query.q.trim();
    let length = typed.chars().count();
    if !rules::long_enough(typed) || length > rules::MAX_LEN {
        return Err(unprocessable(&format!(
            "q must be {} to {} characters, or one ideograph",
            rules::MIN_LEN,
            rules::MAX_LEN
        )));
    }
    let words = rules::words(typed);
    let Some(tsquery) = rules::tsquery(&words) else {
        return Err(unprocessable("q must contain a letter or a digit"));
    };
    let page = Page::new(&paging, PAGE_CAP)?;
    let (limit, offset) = page.window();
    let txn = ctx.db.begin().await?;
    // Fuzzy matches below the default 0.6 are what a typo needs.
    txn.execute_unprepared("SET LOCAL pg_trgm.word_similarity_threshold = 0.45")
        .await?;
    let rows = txn
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            crate::search::query(),
            [
                Value::from(tsquery),
                Value::from(rules::phrase(&words)),
                Value::from(i64::try_from(limit).unwrap_or(i64::MAX)),
                Value::from(i64::try_from(offset).unwrap_or(i64::MAX)),
            ],
        ))
        .await?;
    txn.commit().await?;
    let mut found = Vec::with_capacity(rows.len());
    for row in &rows {
        found.push(SearchHit {
            kind: row.try_get("", "kind")?,
            pid: row.try_get::<Uuid>("", "pid")?.to_string(),
            title: row.try_get("", "title")?,
            detail: row.try_get("", "detail")?,
        });
    }
    let (found, more) = page.split(found);
    let mut shown = Vec::with_capacity(found.len());
    for hit in found {
        let pid = hit.pid.parse::<Uuid>().unwrap_or_default();
        // Only contacts and deals carry record-level attributes; the rest are decided on the
        // entity, which the blanket guard has already done.
        let allowed = if matches!(hit.kind.as_str(), "contact" | "deal") && auth::require_auth() {
            match crate::notifications::subject_attrs(&ctx.db, &hit.kind, pid).await? {
                Some(attrs) => caller.claims().is_some_and(|c| auth::may_read(c, &attrs)),
                None => false,
            }
        } else {
            true
        };
        if allowed {
            shown.push(hit);
        }
    }
    Audit::record_read(
        &ctx.db,
        "search",
        Uuid::nil(),
        "search",
        caller.actor(),
        Some(serde_json::json!({ "returned": shown.len() })),
    )
    .await?;
    page.respond(shown, more, &uri)
}

/// The search route.
pub fn routes() -> Routes {
    Routes::new().prefix("/api").add("/search", get(search))
}
