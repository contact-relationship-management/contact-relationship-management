//! The notifications inbox (CRM-R32): list, mark read, and the sweep that makes them.
//!
//! A notification carries no record text. Each one is checked against the access policy again
//! when it is read, with the caller's own token, and left out if the caller may not read the
//! record it is about.
//!
//! Runs on `sqlx` (CRM-D24, CRM-T206), but for the policy check, which reads the record through
//! [`inbox::subject_attrs`] on `SeaORM`'s side of the shared pool.

use axa::extract::Query;
use axa::prelude::*;
use axum::http::{StatusCode, Uri};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::paging::Page;
use super::{PidPath, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::db;
use crate::models::audit_logs::Model as Audit;
use crate::notifications as inbox;

/// Most notifications in one page.
const PAGE_CAP: u64 = 100;
/// How many unread notifications the count looks at.
const COUNT_CAP: i64 = 200;

/// One inbox entry.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Notification")]
pub(crate) struct Notification {
    pid: Uuid,
    /// `task_reminder`, `sla_breach` or `workflow`.
    kind: String,
    /// What the notification is about: `contact`, `account`, `lead`, `deal` or `ticket`.
    subject_kind: String,
    subject_pid: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// The query for the inbox routes.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct InboxQuery {
    /// Only the unread.
    #[serde(default)]
    unread: bool,
    /// Whose inbox, as a `person:` URN (`worker:` is accepted and read as `person:`, CRM-D18). Used
    /// only when sign-in is off; with it on the inbox is the caller's own.
    #[serde(default)]
    recipient: Option<String>,
}

/// Whose inbox this is: the caller's (`person:<sub>`), or, with sign-in off, the one named.
fn recipient_of(caller: &MaybeAuthUser, query: &InboxQuery) -> Result<String> {
    if auth::require_auth() {
        return caller
            .actor()
            .map(crate::rules::user_ref::of_subject)
            .ok_or_else(|| {
                Error::CustomError(
                    StatusCode::UNAUTHORIZED,
                    axa::error::ErrorDetail::new("unauthorized", "missing bearer token"),
                )
            });
    }
    let named = query
        .recipient
        .as_deref()
        .ok_or_else(|| unprocessable("recipient is required when sign-in is off"))?;
    let named = crate::rules::user_ref::normalise(named);
    let mut problems = crate::validation::Problems::new();
    problems.require_user("recipient", &named);
    problems.finish()?;
    Ok(named)
}

/// Whether the caller may read the record this notification is about.
async fn visible(db: &sqlx::PgPool, caller: &MaybeAuthUser, n: &Notification) -> Result<bool> {
    let Some(attrs) = inbox::subject_attrs(db, &n.subject_kind, n.subject_pid).await? else {
        return Ok(false);
    };
    if !auth::require_auth() {
        return Ok(true);
    }
    Ok(caller.claims().is_some_and(|c| auth::may_read(c, &attrs)))
}

/// `GET /api/notifications` — the caller's inbox, newest first, without what they may not read.
#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(query): Query<InboxQuery>,
    uri: Uri,
) -> Result<Response> {
    let recipient = recipient_of(&caller, &query)?;
    let page = Page::new(&uri, PAGE_CAP)?;
    let (limit, offset) = page.bounds();
    let all = sqlx::query_as!(
        Notification,
        "SELECT pid, kind, subject_kind, subject_pid, created_at, read_at FROM notifications \
         WHERE recipient_ref = $1 AND suppressed = FALSE \
           AND ($2 = FALSE OR read_at IS NULL) \
         ORDER BY id DESC LIMIT $3 OFFSET $4",
        recipient,
        query.unread,
        limit,
        offset,
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    let (all, more) = page.split(all);
    let mut shown = Vec::with_capacity(all.len());
    for n in all {
        if visible(db::pool(&ctx), &caller, &n).await? {
            shown.push(n);
        }
    }
    page.respond(shown, more)
}

/// `GET /api/notifications/unread-count` response.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "UnreadCount")]
pub(crate) struct UnreadCount {
    /// Unread notifications about records the caller may read.
    unread: u64,
    /// True when more than 200 were unread and only the newest 200 were counted.
    capped: bool,
}

/// `GET /api/notifications/unread-count` — for the badge in the navigation.
#[debug_handler]
async fn unread_count(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(query): Query<InboxQuery>,
) -> Result<Response> {
    let recipient = recipient_of(&caller, &query)?;
    let rows = sqlx::query_as!(
        Notification,
        "SELECT pid, kind, subject_kind, subject_pid, created_at, read_at FROM notifications \
         WHERE recipient_ref = $1 AND suppressed = FALSE AND read_at IS NULL \
         ORDER BY id DESC LIMIT $2",
        recipient,
        COUNT_CAP + 1,
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    let capped = i64::try_from(rows.len()).unwrap_or(i64::MAX) > COUNT_CAP;
    let mut unread = 0_u64;
    for n in rows.iter().take(usize::try_from(COUNT_CAP).unwrap_or(0)) {
        if visible(db::pool(&ctx), &caller, n).await? {
            unread += 1;
        }
    }
    format::json(UnreadCount { unread, capped })
}

/// `PUT /api/notifications/{pid}/read` — mark one read. Someone else's is a `404`.
#[debug_handler]
async fn mark_read(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(query): Query<InboxQuery>,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let recipient = recipient_of(&caller, &query)?;
    let mut txn = db::pool(&ctx).begin().await?;
    let read = sqlx::query_as!(
        Notification,
        "UPDATE notifications SET read_at = COALESCE(read_at, now()) \
         WHERE pid = $1 AND recipient_ref = $2 AND suppressed = FALSE \
         RETURNING pid, kind, subject_kind, subject_pid, created_at, read_at",
        pid,
        recipient,
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    Audit::record_on(&mut txn, "notification", pid, "read", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(read)
}

/// `POST /api/notifications/read-all` response.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "MarkedRead")]
pub(crate) struct MarkedRead {
    marked: u64,
}

/// `POST /api/notifications/read-all` — mark the whole inbox read.
#[debug_handler]
async fn read_all(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(query): Query<InboxQuery>,
) -> Result<Response> {
    let recipient = recipient_of(&caller, &query)?;
    let mut txn = db::pool(&ctx).begin().await?;
    let marked = sqlx::query!(
        "UPDATE notifications SET read_at = now() \
         WHERE recipient_ref = $1 AND suppressed = FALSE AND read_at IS NULL",
        recipient,
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    Audit::record_on(
        &mut txn,
        "notification",
        Uuid::nil(),
        "read_all",
        caller.actor(),
        Some(serde_json::json!({ "marked": marked })),
    )
    .await?;
    txn.commit().await?;
    format::json(MarkedRead { marked })
}

/// Run the sweep and audit it when it did anything. Used by the endpoint and the job.
///
/// # Errors
///
/// A database error.
pub async fn run_sweep(pool: &PgPool, actor: Option<&str>) -> Result<inbox::Outcome> {
    let outcome = inbox::sweep(pool).await?;
    if outcome.created + outcome.suppressed > 0 {
        let mut txn = pool.begin().await?;
        Audit::record_on(
            &mut txn,
            "notification",
            Uuid::nil(),
            "notifications_swept",
            actor,
            Some(serde_json::json!({
                "created": outcome.created, "suppressed": outcome.suppressed,
            })),
        )
        .await?;
        txn.commit().await?;
    }
    Ok(outcome)
}

/// `POST /api/notifications/sweep` — make the notifications that are due now.
#[debug_handler]
async fn sweep(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    format::json(run_sweep(db::pool(&ctx), caller.actor()).await?)
}

/// The job that makes due notifications, queued every minute by the server.
pub struct NotificationSweepWorker {
    ctx: AppContext,
}

#[async_trait]
impl BackgroundWorker<()> for NotificationSweepWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, (): ()) -> Result<()> {
        let outcome = run_sweep(db::pool(&self.ctx), None).await?;
        if outcome.created + outcome.suppressed > 0 {
            tracing::info!(
                created = outcome.created,
                suppressed = outcome.suppressed,
                "notification sweep"
            );
        }
        // The same minute tick runs the scheduled workflows (CRM-R40).
        crate::workflows::run_due(db::pool(&self.ctx)).await?;
        Ok(())
    }
}

/// The notification routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/notifications", get(list))
        .add("/notifications/unread-count", get(unread_count))
        .add("/notifications/read-all", post(read_all))
        .add("/notifications/sweep", post(sweep))
        .add("/notifications/{pid}/read", put(mark_read))
}
