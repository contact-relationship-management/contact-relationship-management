//! `GET /api/search?q=` (CRM-R33): names and titles across the main records.
//!
//! The search runs on `sqlx` (CRM-D24, CRM-T205). Its statement is built from
//! `migration::search_docs::SOURCES`, so the macros cannot check it: every branch names
//! `deleted_at`, which the soft-delete census in `db.rs` cannot see here.

use axa::extract::Query;
use axa::prelude::*;
use axum::http::Uri;
use serde::{Deserialize, Serialize};
use sqlx::AssertSqlSafe;
use uuid::Uuid;

use super::paging::Page;
use super::unprocessable;
use crate::auth::{self, MaybeAuthUser};
use crate::db;
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
    let page = Page::new(&uri, PAGE_CAP)?;
    let (limit, offset) = page.bounds();
    let mut txn = db::pool(&ctx).begin().await?;
    // Fuzzy matches below the default 0.6 are what a typo needs.
    sqlx::query!("SET LOCAL pg_trgm.word_similarity_threshold = 0.45")
        .execute(&mut *txn)
        .await?;
    // Built from a fixed table in the code, never from what the caller sent, which is bound.
    let rows: Vec<(String, Uuid, String, Option<String>)> =
        sqlx::query_as(AssertSqlSafe(crate::search::query()))
            .bind(tsquery)
            .bind(rules::phrase(&words))
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *txn)
            .await?;
    txn.commit().await?;
    let found: Vec<SearchHit> = rows
        .into_iter()
        .map(|(kind, pid, title, detail)| SearchHit {
            kind,
            pid: pid.to_string(),
            title,
            detail,
        })
        .collect();
    let (found, more) = page.split(found);
    let mut shown = Vec::with_capacity(found.len());
    for hit in found {
        let pid = hit.pid.parse::<Uuid>().unwrap_or_default();
        // Only contacts and deals carry record-level attributes; the rest are decided on the
        // entity, which the blanket guard has already done.
        let allowed = if matches!(hit.kind.as_str(), "contact" | "deal") && auth::require_auth() {
            match crate::notifications::subject_attrs(crate::db::pool(&ctx), &hit.kind, pid).await?
            {
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
    Audit::record_read_on(
        crate::db::pool(&ctx),
        "search",
        Uuid::nil(),
        "search",
        caller.actor(),
        Some(serde_json::json!({ "returned": shown.len() })),
    )
    .await?;
    page.respond(shown, more)
}

/// The search route.
pub fn routes() -> Routes {
    Routes::new().prefix("/api").add("/search", get(search))
}
