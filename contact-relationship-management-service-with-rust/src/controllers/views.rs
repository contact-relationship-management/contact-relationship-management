//! Saved views of the lists (CRM-R35): `/api/views`.
//!
//! A view is a filter, a sort and columns over one list, owned by one worker and optionally
//! shared. It holds no record data, and applying it is the list read of the caller who applies
//! it, so sharing a view shares no access.
//!
//! The routes run on `sqlx` (CRM-D24, CRM-T205).

use std::collections::BTreeMap;

use axa::error::ErrorDetail;
use axa::extract::Query;
use axa::prelude::*;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{PidPath, PidRef, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::db;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::views as rules;
use crate::validation::Problems;

/// The owner used when sign-in is off and nobody is identified.
const ANONYMOUS: &str = "person:00000000-0000-0000-0000-000000000000";

/// How a view sorts.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ViewSort")]
#[serde(deny_unknown_fields)]
pub(crate) struct SortSpec {
    field: String,
    /// `asc` or `desc`.
    direction: String,
}

/// `POST /api/views` and `PUT /api/views/{pid}` body (a `PUT` replaces the whole view).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ViewPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewPayload {
    /// `contact`, `account`, `deal`, `lead` or `ticket`.
    kind: String,
    name: String,
    /// Field to the values to keep: alternatives within a field, AND across fields.
    #[serde(default)]
    filter: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    sort: Option<SortSpec>,
    /// The columns to show, in order; all when absent.
    #[serde(default)]
    columns: Option<Vec<String>>,
    /// Let every signed-in user list and use it.
    #[serde(default)]
    shared: bool,
}

/// One view.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "View")]
pub(crate) struct View {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    owner_ref: String,
    /// Whether the caller owns it (and so may change it).
    mine: bool,
    kind: String,
    name: String,
    filter: BTreeMap<String, Vec<String>>,
    sort: Option<SortSpec>,
    columns: Option<Vec<String>>,
    shared: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/views` query.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ViewQuery {
    #[serde(default)]
    kind: Option<String>,
}

/// The caller as a CRM user reference, `person:<sign-in subject>` (CRM-D18): what owns a view or a
/// report and what `set_by` records. With sign-in off and no token, the all-zero person.
pub(super) fn owner_of(caller: &MaybeAuthUser) -> Result<String> {
    match caller.actor() {
        Some(sub) => Ok(crate::rules::user_ref::of_subject(sub)),
        None if auth::require_auth() => Err(Error::CustomError(
            StatusCode::UNAUTHORIZED,
            ErrorDetail::new("unauthorized", "missing bearer token"),
        )),
        None => Ok(ANONYMOUS.to_string()),
    }
}

fn forbidden() -> Error {
    Error::CustomError(
        StatusCode::FORBIDDEN,
        ErrorDetail::new("forbidden", "only the owner can change a view"),
    )
}

/// A stored view, as `saved_views` holds it.
struct Row {
    pid: Uuid,
    owner_ref: String,
    kind: String,
    name: String,
    filter: serde_json::Value,
    sort_field: Option<String>,
    sort_direction: Option<String>,
    columns: Option<serde_json::Value>,
    shared: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
}

fn view(row: Row, me: &str) -> View {
    View {
        pid: row.pid.to_string(),
        mine: row.owner_ref == me,
        owner_ref: row.owner_ref,
        kind: row.kind,
        name: row.name,
        filter: serde_json::from_value(row.filter).unwrap_or_default(),
        sort: row
            .sort_field
            .zip(row.sort_direction)
            .map(|(field, direction)| SortSpec { field, direction }),
        columns: row.columns.and_then(|c| serde_json::from_value(c).ok()),
        shared: row.shared,
        updated_at: row.updated_at,
    }
}

fn validate(payload: &ViewPayload) -> Result<()> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if payload.name.trim().chars().count() > rules::MAX_NAME {
        problems.push_field(
            "name",
            format!("is longer than {} characters", rules::MAX_NAME),
        );
    }
    for (field, message) in rules::check(
        &payload.kind,
        &payload.filter,
        payload
            .sort
            .as_ref()
            .map(|s| (s.field.as_str(), s.direction.as_str())),
        payload.columns.as_deref(),
    ) {
        problems.push_field(field, message);
    }
    problems.finish()
}

/// Whether another view of the owner's already has this name on this list.
fn taken() -> Error {
    unprocessable("you already have a view with that name for this list")
}

/// `GET /api/views` — the caller's own views and everyone's shared ones.
#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(query): Query<ViewQuery>,
) -> Result<Response> {
    let me = owner_of(&caller)?;
    if let Some(kind) = &query.kind
        && !rules::KINDS.contains(&kind.as_str())
    {
        return Err(unprocessable("kind is not a list that has views"));
    }
    let rows = sqlx::query_as!(
        Row,
        "SELECT pid, owner_ref, kind, name, filter, sort_field, sort_direction, columns, shared, \
         updated_at FROM saved_views \
         WHERE (owner_ref = $1 OR shared) AND ($2::text IS NULL OR kind = $2) \
         ORDER BY (owner_ref = $1) DESC, lower(name), id",
        me,
        query.kind,
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    format::json(rows.into_iter().map(|r| view(r, &me)).collect::<Vec<_>>())
}

fn sort_of(payload: &ViewPayload) -> (Option<&str>, Option<&str>) {
    payload.sort.as_ref().map_or((None, None), |s| {
        (Some(s.field.as_str()), Some(s.direction.as_str()))
    })
}

fn columns_of(payload: &ViewPayload) -> Option<serde_json::Value> {
    payload.columns.as_ref().map(|c| serde_json::json!(c))
}

/// `POST /api/views`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ViewPayload>,
) -> Result<Response> {
    validate(&payload)?;
    let me = owner_of(&caller)?;
    let pid = Uuid::new_v4();
    let mut txn = db::pool(&ctx).begin().await?;
    let owned = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM saved_views WHERE owner_ref = $1"#,
        me,
    )
    .fetch_one(&mut *txn)
    .await?;
    if owned >= rules::MAX_PER_OWNER {
        return Err(unprocessable(&format!(
            "you can keep at most {} views",
            rules::MAX_PER_OWNER
        )));
    }
    let (sort_field, sort_direction) = sort_of(&payload);
    let inserted = sqlx::query!(
        "INSERT INTO saved_views (pid, owner_ref, kind, name, filter, sort_field, \
         sort_direction, columns, shared) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT DO NOTHING",
        pid,
        me,
        payload.kind,
        payload.name.trim(),
        serde_json::json!(payload.filter),
        sort_field,
        sort_direction,
        columns_of(&payload),
        payload.shared,
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record_on(
        &mut txn,
        "view",
        pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "kind": payload.kind, "shared": payload.shared })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// The owner of the view, or why the caller cannot change it.
async fn owned(conn: &mut PgConnection, pid: Uuid, me: &str) -> Result<(String, bool)> {
    let row = sqlx::query!(
        "SELECT owner_ref, shared, kind FROM saved_views WHERE pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(conn)
    .await?
    .ok_or(Error::NotFound)?;
    if row.owner_ref == me {
        Ok((row.kind, row.shared))
    } else if row.shared {
        Err(forbidden())
    } else {
        Err(Error::NotFound)
    }
}

/// `PUT /api/views/{pid}` — replace a view you own.
#[debug_handler]
async fn replace(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ViewPayload>,
) -> Result<Response> {
    validate(&payload)?;
    let me = owner_of(&caller)?;
    let mut txn = db::pool(&ctx).begin().await?;
    let (kind, _) = owned(&mut txn, pid, &me).await?;
    if kind != payload.kind {
        return Err(unprocessable("a view cannot move to another list"));
    }
    let (sort_field, sort_direction) = sort_of(&payload);
    let changed = sqlx::query!(
        "UPDATE saved_views SET kind = $2, name = $3, filter = $4, sort_field = $5, \
         sort_direction = $6, columns = $7, shared = $8, updated_at = now() \
         WHERE pid = $1",
        pid,
        payload.kind,
        payload.name.trim(),
        serde_json::json!(payload.filter),
        sort_field,
        sort_direction,
        columns_of(&payload),
        payload.shared,
    )
    .execute(&mut *txn)
    .await;
    if changed.is_err() {
        // The only constraint a replace can break is the owner's unique name.
        return Err(taken());
    }
    Audit::record_on(
        &mut txn,
        "view",
        pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "kind": payload.kind, "shared": payload.shared })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `DELETE /api/views/{pid}` — delete a view you own.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let me = owner_of(&caller)?;
    let mut txn = db::pool(&ctx).begin().await?;
    let (kind, shared) = owned(&mut txn, pid, &me).await?;
    sqlx::query!("DELETE FROM saved_views WHERE pid = $1", pid)
        .execute(&mut *txn)
        .await?;
    Audit::record_on(
        &mut txn,
        "view",
        pid,
        "deleted",
        caller.actor(),
        Some(serde_json::json!({ "kind": kind, "shared": shared })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// The view routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/views", get(list))
        .add("/views", post(create))
        .add("/views/{pid}", put(replace))
        .add("/views/{pid}", delete(remove))
}
