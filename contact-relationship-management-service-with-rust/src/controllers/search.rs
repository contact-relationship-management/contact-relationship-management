//! `GET /api/search?q=` (CRM-R33): names and titles across the main records.
//!
//! The search runs on `sqlx` (CRM-D24, CRM-T205). Its statement is built from
//! `crate::search_docs::SOURCES`, so the macros cannot check it: every branch names
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
    let pool = db::pool(&ctx);
    let (tsquery, phrase, caller_ref) = (&tsquery, &rules::phrase(&words), &caller);
    // Filled from hits the caller may read (CRM-T326).
    let filled = page
        .fill(|limit, offset| async move {
            let mut txn = pool.begin().await?;
            // Fuzzy matches below the default 0.6 are what a typo needs.
            sqlx::query!("SET LOCAL pg_trgm.word_similarity_threshold = 0.45")
                .execute(&mut *txn)
                .await?;
            // Built from a fixed table in the code, never from what the caller sent, which is bound.
            let rows: Vec<(String, Uuid, String, Option<String>)> =
                sqlx::query_as(AssertSqlSafe(crate::search::query()))
                    .bind(tsquery)
                    .bind(phrase)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&mut *txn)
                    .await?;
            txn.commit().await?;
            let mut shown = Vec::with_capacity(rows.len());
            for (kind, pid, title, detail) in rows {
                // The record's `Uuid` travels with its hit, so it is never parsed back out of the text.
                shown.push(shown_hit(pool, caller_ref, kind, pid, title, detail).await?);
            }
            Ok(shown)
        })
        .await?;
    Audit::record_read_on(
        pool,
        "search",
        Uuid::nil(),
        "search",
        caller.actor(),
        Some(serde_json::json!({ "returned": filled.rows.len() })),
    )
    .await?;
    page.respond_filled(filled)
}

/// One hit as `caller` may see it, or `None` when it may not read the record. Only contacts,
/// deals and tickets carry record-level attributes; the rest are decided on the entity, which the
/// blanket guard has already done. A ticket read masked shows no title, as its own read would not
/// (CRM-T321); a contact's or deal's mask does not cover its name.
async fn shown_hit(
    pool: &sqlx::PgPool,
    caller: &MaybeAuthUser,
    kind: String,
    pid: Uuid,
    title: String,
    detail: Option<String>,
) -> Result<Option<SearchHit>> {
    let mut title = title;
    if matches!(kind.as_str(), "contact" | "deal" | "ticket") && auth::require_auth() {
        let Some(attrs) = crate::notifications::subject_attrs(pool, &kind, pid).await? else {
            return Ok(None);
        };
        let Ok(obligations) =
            auth::authorize_record(caller, authentication_verifier::Action::Read, &attrs)
        else {
            return Ok(None);
        };
        if kind == "ticket" && obligations.iter().any(|o| o == "mask") {
            title = auth::MASKED.to_string();
        }
    }
    Ok(Some(SearchHit {
        kind,
        pid: pid.to_string(),
        title,
        detail,
    }))
}

/// The search route.
pub fn routes() -> Routes {
    Routes::new().prefix("/api").add("/search", get(search))
}
