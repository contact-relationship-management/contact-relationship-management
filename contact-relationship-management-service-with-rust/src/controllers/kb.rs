//! The public knowledge base and its deflection report (CRM-R57).
//!
//! `GET /kb/articles`, `GET /kb/articles/{pid}` and `POST /kb/articles/{pid}/feedback` need no
//! token and show **published** articles only. They answer `404` unless `CRM_PUBLIC_KB` is on, so
//! nothing is public until it is asked for, and they are rate limited per client. What a visit
//! leaves behind is a count per day (views, "this helped" votes, searches, searches that found
//! nothing); no address, no visitor and no text a visitor typed is kept.
//! `GET /api/insights/kb-deflection` reads the counts beside the tickets raised.

use std::net::IpAddr;
use std::num::NonZeroU32;

use axa::client::ClientAddr;
use axa::prelude::*;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::PidPath;
use crate::auth::MaybeAuthUser;
use crate::db;
use crate::json::Json;
use crate::models::_entities::articles;
use crate::models::audit_logs::Model as Audit;
use crate::models::rows;
use crate::rate_limit::Settable;
use crate::rules::analytics::{self, Ratio};
use crate::settings;
use crate::validation::Problems;

/// Requests per client, over a minute.
static PER_CLIENT: Settable = Settable::new(NonZeroU32::new(60).unwrap());
/// Most articles one search returns.
const MAX_LISTED: i64 = 50;
/// How much of an article a search result shows.
const SNIPPET_CHARS: usize = 200;
/// Longest period the deflection report covers, in days.
const MAX_DAYS: i64 = 365;
/// The id the portal's own counts (searches) are kept under.
const PORTAL: Uuid = Uuid::nil();

/// `None` when the request may go on; otherwise the response that refuses it: `404` while the
/// public knowledge base is off, `429` when the client is over its limit.
fn admit(address: Option<IpAddr>) -> Option<Response> {
    let current = settings::current();
    if !current.public_kb {
        return Some(StatusCode::NOT_FOUND.into_response());
    }
    let client = crate::rate_limit::client_key(None, address);
    let hit = PER_CLIENT.check(current.public_kb_client_limit, client);
    hit.err().map(|wait| {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, wait.to_string())],
            "too many requests",
        )
            .into_response()
    })
}

/// What a visit adds one to.
#[derive(Clone, Copy)]
enum Visit {
    View,
    Helpful,
    Unhelpful,
    /// A search with words in it; `empty` when it found no article.
    Search {
        empty: bool,
    },
}

/// Add one to today's count for an article (or the portal); "today" is the organization's day
/// (CRM-T179), so the deflection window's days are the days its reports use. A failure is
/// logged, never shown: a reader is not refused an article because a counter could not be
/// written.
async fn bump(ctx: &AppContext, article: Uuid, visit: Visit) {
    let day = match super::organization::organization_today(&ctx.db).await {
        Ok(day) => day,
        Err(error) => {
            tracing::warn!(%error, "could not count a knowledge-base visit");
            return;
        }
    };
    let pool = db::pool(ctx);
    // One statement per count, so each is checked against the schema.
    let counted = match visit {
        Visit::View => {
            sqlx::query!(
                "INSERT INTO kb_daily (day, article_pid, views) VALUES ($2, $1, 1) \
                 ON CONFLICT (day, article_pid) DO UPDATE SET views = kb_daily.views + 1",
                article,
                day,
            )
            .execute(pool)
            .await
        }
        Visit::Helpful => {
            sqlx::query!(
                "INSERT INTO kb_daily (day, article_pid, helpful_yes) VALUES ($2, $1, 1) \
                 ON CONFLICT (day, article_pid) \
                 DO UPDATE SET helpful_yes = kb_daily.helpful_yes + 1",
                article,
                day,
            )
            .execute(pool)
            .await
        }
        Visit::Unhelpful => {
            sqlx::query!(
                "INSERT INTO kb_daily (day, article_pid, helpful_no) VALUES ($2, $1, 1) \
                 ON CONFLICT (day, article_pid) \
                 DO UPDATE SET helpful_no = kb_daily.helpful_no + 1",
                article,
                day,
            )
            .execute(pool)
            .await
        }
        Visit::Search { empty: false } => {
            sqlx::query!(
                "INSERT INTO kb_daily (day, article_pid, searches) VALUES ($2, $1, 1) \
                 ON CONFLICT (day, article_pid) DO UPDATE SET searches = kb_daily.searches + 1",
                article,
                day,
            )
            .execute(pool)
            .await
        }
        Visit::Search { empty: true } => {
            sqlx::query!(
                "INSERT INTO kb_daily (day, article_pid, searches, searches_empty) \
                 VALUES ($2, $1, 1, 1) \
                 ON CONFLICT (day, article_pid) DO UPDATE SET searches = kb_daily.searches + 1, \
                 searches_empty = kb_daily.searches_empty + 1",
                article,
                day,
            )
            .execute(pool)
            .await
        }
    };
    if let Err(error) = counted {
        tracing::warn!(%error, "could not count a knowledge-base visit");
    }
}

/// An article in a search: enough to choose one.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "KbListing")]
pub(crate) struct Listing {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    title: String,
    /// The start of the article.
    snippet: String,
    keywords: Option<String>,
    version: i32,
    updated_at: DateTime<Utc>,
}

/// A published article, as the public reads it.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "KbArticle")]
pub(crate) struct PublicArticle {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    title: String,
    /// Plain text: show it as text, never as markup.
    body: String,
    keywords: Option<String>,
    version: i32,
    updated_at: DateTime<Utc>,
}

/// What `GET /kb/articles` may be asked.
#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

/// The first `SNIPPET_CHARS` characters, cut at a character and marked when cut.
fn snippet(body: &str) -> String {
    let mut chars = body.chars();
    let head: String = chars.by_ref().take(SNIPPET_CHARS).collect();
    if chars.next().is_some() {
        format!("{}…", head.trim_end())
    } else {
        head
    }
}

/// `GET /kb/articles?q=` — the published articles, or those whose title, keywords or text contain
/// `q` (at most 50, by title). A search with words is counted; one that finds nothing is counted
/// as such.
#[debug_handler]
async fn search(
    State(ctx): State<AppContext>,
    ClientAddr(address): ClientAddr,
    axa::extract::Query(query): axa::extract::Query<SearchQuery>,
) -> Result<Response> {
    if let Some(refused) = admit(address) {
        return Ok(refused);
    }
    let asked = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    let pattern = asked.map(|q| {
        let escaped = axa::cursor_pagination::escape_like(
            &q.chars().take(100).collect::<String>().to_lowercase(),
        );
        format!("%{escaped}%")
    });
    let found = rows::published_articles(db::pool(&ctx), pattern.as_deref(), MAX_LISTED).await?;
    if asked.is_some() {
        bump(
            &ctx,
            PORTAL,
            Visit::Search {
                empty: found.is_empty(),
            },
        )
        .await;
    }
    format::json(
        found
            .iter()
            .map(|a| Listing {
                pid: a.pid.to_string(),
                title: a.title.clone(),
                snippet: snippet(&a.body),
                keywords: a.keywords.clone(),
                version: a.version,
                updated_at: a.updated_at.to_utc(),
            })
            .collect::<Vec<_>>(),
    )
}

/// A published, live article, or `None` (a draft or archived article is not found, so its
/// existence is not given away).
async fn published(ctx: &AppContext, pid: Uuid) -> Result<Option<articles::Model>> {
    Ok(rows::published_article(db::pool(ctx), pid).await?)
}

/// `GET /kb/articles/{pid}` — one published article; counted as a view.
#[debug_handler]
async fn read(
    State(ctx): State<AppContext>,
    ClientAddr(address): ClientAddr,
    PidPath(pid): PidPath,
) -> Result<Response> {
    if let Some(refused) = admit(address) {
        return Ok(refused);
    }
    let Some(article) = published(&ctx, pid).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    bump(&ctx, article.pid, Visit::View).await;
    format::json(PublicArticle {
        pid: article.pid.to_string(),
        title: article.title,
        body: article.body,
        keywords: article.keywords,
        version: article.version,
        updated_at: article.updated_at.to_utc(),
    })
}

/// `POST /kb/articles/{pid}/feedback` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FeedbackPayload {
    /// Whether the article solved the visitor's problem.
    helpful: bool,
}

/// `POST /kb/articles/{pid}/feedback` — "this helped" or "this did not". Anonymous and counted
/// only; answers `202`.
#[debug_handler]
async fn feedback(
    State(ctx): State<AppContext>,
    ClientAddr(address): ClientAddr,
    PidPath(pid): PidPath,
    Json(payload): Json<FeedbackPayload>,
) -> Result<Response> {
    if let Some(refused) = admit(address) {
        return Ok(refused);
    }
    let Some(article) = published(&ctx, pid).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let visit = if payload.helpful {
        Visit::Helpful
    } else {
        Visit::Unhelpful
    };
    bump(&ctx, article.pid, visit).await;
    Ok((
        StatusCode::ACCEPTED,
        axum::Json(serde_json::json!({ "status": "received" })),
    )
        .into_response())
}

/// One article's counts over the period.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "KbArticleStats")]
pub(crate) struct ArticleStats {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    /// The article's title; `(deleted article)` when it is gone.
    title: String,
    views: i64,
    helpful_yes: i64,
    helpful_no: i64,
}

/// What the public knowledge base did over a period, beside the tickets raised.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "KbDeflection")]
#[allow(clippy::struct_field_names)] // `deflection` is the response member clients read
pub(crate) struct Deflection {
    /// First day counted.
    from: NaiveDate,
    /// Last day counted (today).
    to: NaiveDate,
    days: i64,
    views: i64,
    /// Searches with words in them.
    searches: i64,
    /// Of those, searches that found no article.
    searches_without_result: i64,
    helpful_yes: i64,
    helpful_no: i64,
    /// Tickets opened in the period, by anyone.
    tickets_opened: i64,
    /// `helpful_yes / (helpful_yes + tickets_opened)`: of the people who either said an article
    /// solved their problem or went on to raise a ticket, the share who did not need one. An
    /// estimate; see `note`. `value` is `null` when both are zero.
    deflection: Ratio,
    /// The articles viewed most, at most ten.
    top_articles: Vec<ArticleStats>,
    note: &'static str,
}

/// What `GET /api/insights/kb-deflection` may be asked.
#[derive(Debug, Deserialize)]
struct DeflectionQuery {
    days: Option<i64>,
}

/// `GET /api/insights/kb-deflection?days=` — the public knowledge base's counts beside the tickets
/// raised over the last `days` days (1 to 365, default 30). An audited read.
#[debug_handler]
async fn deflection(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    axa::extract::Query(query): axa::extract::Query<DeflectionQuery>,
) -> Result<Response> {
    let days = query.days.unwrap_or(30);
    if !(1..=MAX_DAYS).contains(&days) {
        let mut problems = Problems::new();
        problems.push_field("days", format!("days is 1 to {MAX_DAYS}"));
        problems.finish()?;
    }
    // The organization's days (CRM-T179): the counts are kept by them, and a ticket counts on
    // the day it was opened there.
    let calendar = super::organization::periods(&ctx.db).await?;
    let to = crate::rules::periods::today_in(crate::clock::now(), calendar.zone);
    let from = to - chrono::Duration::days(days - 1);
    let pool = db::pool(&ctx);
    let totals = sqlx::query!(
        "SELECT COALESCE(SUM(views), 0)::bigint AS \"views!\", \
                COALESCE(SUM(helpful_yes), 0)::bigint AS \"yes!\", \
                COALESCE(SUM(helpful_no), 0)::bigint AS \"no!\", \
                COALESCE(SUM(searches), 0)::bigint AS \"searches!\", \
                COALESCE(SUM(searches_empty), 0)::bigint AS \"empty!\" \
         FROM kb_daily WHERE day BETWEEN $1 AND $2",
        from,
        to,
    )
    .fetch_one(pool)
    .await?;
    // A ticket counts on the day it was opened in the organization's zone, which is bound by name
    // as `rules::periods::local` writes it.
    let tickets_opened = sqlx::query_scalar!(
        "SELECT count(*)::bigint AS \"n!\" FROM tickets \
         WHERE deleted_at IS NULL AND (opened_at AT TIME ZONE $3)::date BETWEEN $1 AND $2",
        from,
        to,
        calendar.zone.name(),
    )
    .fetch_one(pool)
    .await?;
    let top = sqlx::query!(
        "SELECT k.article_pid AS pid, COALESCE(a.title, '(deleted article)') AS \"title!\", \
                SUM(k.views)::bigint AS \"views!\", SUM(k.helpful_yes)::bigint AS \"yes!\", \
                SUM(k.helpful_no)::bigint AS \"no!\" \
         FROM kb_daily k LEFT JOIN articles a ON a.pid = k.article_pid AND a.deleted_at IS NULL \
         WHERE k.day BETWEEN $1 AND $2 AND k.article_pid <> $3 \
         GROUP BY k.article_pid, a.title HAVING SUM(k.views) > 0 \
         ORDER BY 3 DESC, 2 LIMIT 10",
        from,
        to,
        PORTAL,
    )
    .fetch_all(pool)
    .await?;
    let yes = totals.yes;
    let top_articles: Vec<ArticleStats> = top
        .into_iter()
        .map(|row| ArticleStats {
            pid: row.pid.to_string(),
            title: row.title,
            views: row.views,
            helpful_yes: row.yes,
            helpful_no: row.no,
        })
        .collect();
    Audit::record_read_on(
        pool,
        "kb",
        Uuid::nil(),
        "kb_deflection_read",
        caller.actor(),
        Some(serde_json::json!({ "days": days })),
    )
    .await?;
    format::json(Deflection {
        from,
        to,
        days,
        views: totals.views,
        searches: totals.searches,
        searches_without_result: totals.empty,
        helpful_yes: yes,
        helpful_no: totals.no,
        tickets_opened,
        deflection: analytics::ratio(yes, yes.saturating_add(tickets_opened)),
        top_articles,
        note: "counts only, kept per day; a visitor is not identified, so a vote is a visitor's own \
               word and a ticket is not tied to a visit; deflection is an estimate, not a measure",
    })
}

/// The public routes, outside `/api`.
pub fn public_routes() -> Routes {
    Routes::new()
        .add("/kb/articles", get(search))
        .add("/kb/articles/{pid}", get(read))
        .add("/kb/articles/{pid}/feedback", post(feedback))
}

/// The report route.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/insights/kb-deflection", get(deflection))
}
