//! The public knowledge base and its deflection report (CRM-R57).
//!
//! `GET /kb/articles`, `GET /kb/articles/{pid}` and `POST /kb/articles/{pid}/feedback` need no
//! token and show **published** articles only. They answer `404` unless `CRM_PUBLIC_KB` is on, so
//! nothing is public until it is asked for, and they are rate limited per client. What a visit
//! leaves behind is a count per day (views, "this helped" votes, searches, searches that found
//! nothing); no address, no visitor and no text a visitor typed is kept.
//! `GET /api/insights/kb-deflection` reads the counts beside the tickets raised.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use chrono::{DateTime, NaiveDate, Utc};
use loco_rs::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, QueryFilter, QueryOrder, QuerySelect, Statement,
    Value,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::PidPath;
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::_entities::articles;
use crate::models::audit_logs::Model as Audit;
use crate::models::live::FindLive;
use crate::rules::analytics::{self, Ratio};
use crate::rules::forms::Counter;
use crate::settings;
use crate::validation::Problems;

/// The window the per-client limit counts over.
const CLIENT_SPAN: Duration = Duration::from_mins(1);
/// Most articles one search returns.
const MAX_LISTED: u64 = 50;
/// How much of an article a search result shows.
const SNIPPET_CHARS: usize = 200;
/// Longest period the deflection report covers, in days.
const MAX_DAYS: i64 = 365;
/// The id the portal's own counts (searches) are kept under.
const PORTAL: Uuid = Uuid::nil();

fn counter() -> &'static Mutex<Counter> {
    static COUNTER: OnceLock<Mutex<Counter>> = OnceLock::new();
    COUNTER.get_or_init(|| Mutex::new(Counter::default()))
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// `None` when the request may go on; otherwise the response that refuses it: `404` while the
/// public knowledge base is off, `429` when the client is over its limit.
fn admit(headers: &HeaderMap) -> Option<Response> {
    let current = settings::current();
    if !current.public_kb {
        return Some(StatusCode::NOT_FOUND.into_response());
    }
    let client = crate::rate_limit::client_key(
        None,
        headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()),
        current.trust_forwarded_for,
    );
    let hit = counter()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .hit(
            &format!("kb|{client}"),
            current.public_kb_client_limit,
            CLIENT_SPAN,
            Instant::now(),
        );
    hit.err().map(|wait| {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, wait.to_string())],
            "too many requests",
        )
            .into_response()
    })
}

/// Which count a visit adds to.
#[derive(Clone, Copy)]
enum Count {
    Views,
    HelpfulYes,
    HelpfulNo,
    Searches,
    SearchesEmpty,
}

impl Count {
    fn column(self) -> &'static str {
        match self {
            Self::Views => "views",
            Self::HelpfulYes => "helpful_yes",
            Self::HelpfulNo => "helpful_no",
            Self::Searches => "searches",
            Self::SearchesEmpty => "searches_empty",
        }
    }
}

/// Add one to today's count for an article (or the portal); "today" is the organization's day
/// (CRM-T179), so the deflection window's days are the days its reports use. A failure is
/// logged, never shown: a reader is not refused an article because a counter could not be
/// written.
async fn bump(ctx: &AppContext, article: Uuid, count: Count, also: Option<Count>) {
    let day = match super::organization::organization_today(&ctx.db).await {
        Ok(day) => day,
        Err(error) => {
            tracing::warn!(%error, "could not count a knowledge-base visit");
            return;
        }
    };
    let column = count.column();
    let extra = also.map_or(String::new(), |c| {
        format!(", {0} = kb_daily.{0} + 1", c.column())
    });
    let also_column = also.map_or(String::new(), |c| format!(", {}", c.column()));
    let also_value = if also.is_some() { ", 1" } else { "" };
    let sql = format!(
        "INSERT INTO kb_daily (day, article_pid, {column}{also_column}) \
         VALUES ($2, $1, 1{also_value}) \
         ON CONFLICT (day, article_pid) DO UPDATE SET {column} = kb_daily.{column} + 1{extra}"
    );
    if let Err(error) = ctx
        .db
        .execute_raw(stmt(&sql, [article.into(), day.into()]))
        .await
    {
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
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<SearchQuery>,
) -> Result<Response> {
    if let Some(refused) = admit(&headers) {
        return Ok(refused);
    }
    let mut found = articles::Entity::find_live().filter(articles::Column::Status.eq("published"));
    let asked = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    if let Some(q) = asked {
        let escaped = q
            .chars()
            .take(100)
            .collect::<String>()
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        found = found.filter(sea_orm::sea_query::Expr::cust_with_values(
            "(lower(title) LIKE $1 OR lower(coalesce(keywords, '')) LIKE $1 OR lower(body) LIKE $1)",
            [format!("%{escaped}%")],
        ));
    }
    let rows = found
        .order_by_asc(articles::Column::Title)
        .limit(MAX_LISTED)
        .all(&ctx.db)
        .await?;
    if asked.is_some() {
        let empty = rows.is_empty().then_some(Count::SearchesEmpty);
        bump(&ctx, PORTAL, Count::Searches, empty).await;
    }
    format::json(
        rows.iter()
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
    Ok(articles::Entity::find_live()
        .filter(articles::Column::Pid.eq(pid))
        .filter(articles::Column::Status.eq("published"))
        .one(&ctx.db)
        .await?)
}

/// `GET /kb/articles/{pid}` — one published article; counted as a view.
#[debug_handler]
async fn read(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    PidPath(pid): PidPath,
) -> Result<Response> {
    if let Some(refused) = admit(&headers) {
        return Ok(refused);
    }
    let Some(article) = published(&ctx, pid).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    bump(&ctx, article.pid, Count::Views, None).await;
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
    headers: HeaderMap,
    PidPath(pid): PidPath,
    Json(payload): Json<FeedbackPayload>,
) -> Result<Response> {
    if let Some(refused) = admit(&headers) {
        return Ok(refused);
    }
    let Some(article) = published(&ctx, pid).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let count = if payload.helpful {
        Count::HelpfulYes
    } else {
        Count::HelpfulNo
    };
    bump(&ctx, article.pid, count, None).await;
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
    axum::extract::Query(query): axum::extract::Query<DeflectionQuery>,
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
    let to = crate::rules::periods::today_in(Utc::now(), calendar.zone);
    let from = to - chrono::Duration::days(days - 1);
    let opened_on = format!(
        "{}::date",
        crate::rules::periods::local("opened_at", &calendar)
    );
    let totals = ctx
        .db
        .query_one_raw(stmt(
            "SELECT COALESCE(SUM(views), 0)::bigint AS views, \
                    COALESCE(SUM(helpful_yes), 0)::bigint AS yes, \
                    COALESCE(SUM(helpful_no), 0)::bigint AS no, \
                    COALESCE(SUM(searches), 0)::bigint AS searches, \
                    COALESCE(SUM(searches_empty), 0)::bigint AS empty \
             FROM kb_daily WHERE day BETWEEN $1 AND $2",
            [from.into(), to.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let tickets = ctx
        .db
        .query_one_raw(stmt(
            &format!(
                "SELECT count(*)::bigint AS n FROM tickets \
                 WHERE deleted_at IS NULL AND {opened_on} BETWEEN $1 AND $2"
            ),
            [from.into(), to.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let top = ctx
        .db
        .query_all_raw(stmt(
            "SELECT k.article_pid AS pid, COALESCE(a.title, '(deleted article)') AS title, \
                    SUM(k.views)::bigint AS views, SUM(k.helpful_yes)::bigint AS yes, \
                    SUM(k.helpful_no)::bigint AS no \
             FROM kb_daily k LEFT JOIN articles a ON a.pid = k.article_pid AND a.deleted_at IS NULL \
             WHERE k.day BETWEEN $1 AND $2 AND k.article_pid <> $3 \
             GROUP BY k.article_pid, a.title HAVING SUM(k.views) > 0 \
             ORDER BY views DESC, title LIMIT 10",
            [from.into(), to.into(), PORTAL.into()],
        ))
        .await?;
    let yes: i64 = totals.try_get("", "yes")?;
    let tickets_opened: i64 = tickets.try_get("", "n")?;
    let mut top_articles = Vec::with_capacity(top.len());
    for row in &top {
        top_articles.push(ArticleStats {
            pid: row.try_get::<Uuid>("", "pid")?.to_string(),
            title: row.try_get("", "title")?,
            views: row.try_get("", "views")?,
            helpful_yes: row.try_get("", "yes")?,
            helpful_no: row.try_get("", "no")?,
        });
    }
    Audit::record_read(
        &ctx.db,
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
        views: totals.try_get("", "views")?,
        searches: totals.try_get("", "searches")?,
        searches_without_result: totals.try_get("", "empty")?,
        helpful_yes: yes,
        helpful_no: totals.try_get("", "no")?,
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
