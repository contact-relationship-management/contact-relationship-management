//! The notifications inbox (CRM-R32): list, mark read, and the sweep that makes them.
//!
//! A notification carries no record text. Each one is checked against the access policy again
//! when it is read, with the caller's own token, and left out if the caller may not read the
//! record it is about.

use axum::http::{StatusCode, Uri};
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::paging::{Page, PageParams};
use super::{PidPath, unprocessable};
use crate::auth::{self, MaybeAuthUser};
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
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    /// `task_reminder`, `sla_breach` or `workflow`.
    kind: String,
    /// What the notification is about: `contact`, `account`, `lead`, `deal` or `ticket`.
    subject_kind: String,
    #[schemars(with = "uuid::Uuid")]
    subject_pid: String,
    created_at: chrono::DateTime<chrono::Utc>,
    read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// The query for the inbox routes.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct InboxQuery {
    /// Only the unread.
    #[serde(default)]
    unread: bool,
    /// Whose inbox, as a `worker:` URN. Used only when sign-in is off; with it on the inbox is
    /// the caller's own.
    #[serde(default)]
    recipient: Option<String>,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// Whose inbox this is: the caller's (`worker:<sub>`), or, with sign-in off, the one named.
fn recipient_of(caller: &MaybeAuthUser, query: &InboxQuery) -> Result<String> {
    if auth::require_auth() {
        return caller
            .actor()
            .map(|sub| format!("worker:{sub}"))
            .ok_or_else(|| {
                Error::CustomError(
                    StatusCode::UNAUTHORIZED,
                    loco_rs::controller::ErrorDetail::new("unauthorized", "missing bearer token"),
                )
            });
    }
    let named = query
        .recipient
        .as_deref()
        .ok_or_else(|| unprocessable("recipient is required when sign-in is off"))?;
    let mut problems = crate::validation::Problems::new();
    problems.require_ref("recipient", entity_ref::EntityType::Worker, named);
    problems.finish()?;
    Ok(named.to_string())
}

/// Whether the caller may read the record this notification is about.
async fn visible(
    db: &sea_orm::DatabaseConnection,
    caller: &MaybeAuthUser,
    n: &Notification,
) -> Result<bool> {
    let Ok(pid) = n.subject_pid.parse::<Uuid>() else {
        return Ok(false);
    };
    let Some(attrs) = inbox::subject_attrs(db, &n.subject_kind, pid).await? else {
        return Ok(false);
    };
    if !auth::require_auth() {
        return Ok(true);
    }
    Ok(caller.claims().is_some_and(|c| auth::may_read(c, &attrs)))
}

fn to_notification(row: &sea_orm::QueryResult) -> Result<Notification> {
    Ok(Notification {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        kind: row.try_get("", "kind")?,
        subject_kind: row.try_get("", "subject_kind")?,
        subject_pid: row.try_get::<Uuid>("", "subject_pid")?.to_string(),
        created_at: row.try_get("", "created_at")?,
        read_at: row.try_get("", "read_at")?,
    })
}

const COLUMNS: &str = "pid, kind, subject_kind, subject_pid, created_at, read_at";

/// `GET /api/notifications` — the caller's inbox, newest first, without what they may not read.
#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(query): Query<InboxQuery>,
    Query(paging): Query<PageParams>,
    uri: Uri,
) -> Result<Response> {
    let recipient = recipient_of(&caller, &query)?;
    let page = Page::new(&paging, PAGE_CAP)?;
    let (limit, offset) = page.window();
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM notifications \
                 WHERE recipient_ref = $1 AND suppressed = FALSE \
                   AND ($2 = FALSE OR read_at IS NULL) \
                 ORDER BY id DESC LIMIT $3 OFFSET $4"
            ),
            [
                recipient.into(),
                query.unread.into(),
                i64::try_from(limit).unwrap_or(i64::MAX).into(),
                i64::try_from(offset).unwrap_or(i64::MAX).into(),
            ],
        ))
        .await?;
    let all = rows
        .iter()
        .map(to_notification)
        .collect::<Result<Vec<_>>>()?;
    let (all, more) = page.split(all);
    let mut shown = Vec::with_capacity(all.len());
    for n in all {
        if visible(&ctx.db, &caller, &n).await? {
            shown.push(n);
        }
    }
    page.respond(shown, more, &uri)
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
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM notifications \
                 WHERE recipient_ref = $1 AND suppressed = FALSE AND read_at IS NULL \
                 ORDER BY id DESC LIMIT $2"
            ),
            [recipient.into(), (COUNT_CAP + 1).into()],
        ))
        .await?;
    let capped = i64::try_from(rows.len()).unwrap_or(i64::MAX) > COUNT_CAP;
    let mut unread = 0_u64;
    for row in rows.iter().take(usize::try_from(COUNT_CAP).unwrap_or(0)) {
        if visible(&ctx.db, &caller, &to_notification(row)?).await? {
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
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!(
                "UPDATE notifications SET read_at = COALESCE(read_at, now()) \
                 WHERE pid = $1 AND recipient_ref = $2 AND suppressed = FALSE \
                 RETURNING {COLUMNS}"
            ),
            [pid.into(), recipient.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    Audit::record(&txn, "notification", pid, "read", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(to_notification(&row)?)
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
    let txn = ctx.db.begin().await?;
    let marked = txn
        .execute_raw(stmt(
            "UPDATE notifications SET read_at = now() \
             WHERE recipient_ref = $1 AND suppressed = FALSE AND read_at IS NULL",
            [recipient.into()],
        ))
        .await?
        .rows_affected();
    Audit::record(
        &txn,
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
pub async fn run_sweep(
    db: &sea_orm::DatabaseConnection,
    actor: Option<&str>,
) -> Result<inbox::Outcome> {
    let outcome = inbox::sweep(db).await?;
    if outcome.created + outcome.suppressed > 0 {
        Audit::record(
            db,
            "notification",
            Uuid::nil(),
            "notifications_swept",
            actor,
            Some(serde_json::json!({
                "created": outcome.created, "suppressed": outcome.suppressed,
            })),
        )
        .await?;
    }
    Ok(outcome)
}

/// `POST /api/notifications/sweep` — make the notifications that are due now.
#[debug_handler]
async fn sweep(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    format::json(run_sweep(&ctx.db, caller.actor()).await?)
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
        let outcome = run_sweep(&self.ctx.db, None).await?;
        if outcome.created + outcome.suppressed > 0 {
            tracing::info!(
                created = outcome.created,
                suppressed = outcome.suppressed,
                "notification sweep"
            );
        }
        // The same minute tick runs the scheduled workflows (CRM-R40).
        crate::workflows::run_due(&self.ctx.db).await?;
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
