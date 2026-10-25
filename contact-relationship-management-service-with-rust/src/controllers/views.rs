//! Saved views of the lists (CRM-R35): `/api/views`.
//!
//! A view is a filter, a sort and columns over one list, owned by one worker and optionally
//! shared. It holds no record data, and applying it is the list read of the caller who applies
//! it, so sharing a view shares no access.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{PidPath, PidRef, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::views as rules;
use crate::validation::Problems;

/// The owner used when sign-in is off and nobody is identified.
const ANONYMOUS: &str = "worker:00000000-0000-0000-0000-000000000000";

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

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

pub(super) fn owner_of(caller: &MaybeAuthUser) -> Result<String> {
    match caller.actor() {
        Some(sub) => Ok(format!("worker:{sub}")),
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

const COLUMNS: &str = "pid, owner_ref, kind, name, filter, sort_field, sort_direction, columns, \
                       shared, updated_at";

fn view(row: &sea_orm::QueryResult, me: &str) -> Result<View> {
    let owner: String = row.try_get("", "owner_ref")?;
    let filter: serde_json::Value = row.try_get("", "filter")?;
    let columns: Option<serde_json::Value> = row.try_get("", "columns")?;
    let field: Option<String> = row.try_get("", "sort_field")?;
    let direction: Option<String> = row.try_get("", "sort_direction")?;
    Ok(View {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        mine: owner == me,
        owner_ref: owner,
        kind: row.try_get("", "kind")?,
        name: row.try_get("", "name")?,
        filter: serde_json::from_value(filter).unwrap_or_default(),
        sort: field
            .zip(direction)
            .map(|(field, direction)| SortSpec { field, direction }),
        columns: columns.and_then(|c| serde_json::from_value(c).ok()),
        shared: row.try_get("", "shared")?,
        updated_at: row.try_get("", "updated_at")?,
    })
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
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM saved_views \
                 WHERE (owner_ref = $1 OR shared) AND ($2::text IS NULL OR kind = $2) \
                 ORDER BY (owner_ref = $1) DESC, lower(name), id"
            ),
            [me.clone().into(), query.kind.clone().into()],
        ))
        .await?;
    format::json(
        rows.iter()
            .map(|r| view(r, &me))
            .collect::<Result<Vec<_>>>()?,
    )
}

fn bind(payload: &ViewPayload) -> Vec<Value> {
    vec![
        payload.kind.clone().into(),
        payload.name.trim().to_string().into(),
        serde_json::json!(payload.filter).into(),
        payload.sort.as_ref().map(|s| s.field.clone()).into(),
        payload.sort.as_ref().map(|s| s.direction.clone()).into(),
        payload
            .columns
            .as_ref()
            .map(|c| serde_json::json!(c))
            .into(),
        payload.shared.into(),
    ]
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
    let txn = ctx.db.begin().await?;
    let owned = txn
        .query_one_raw(stmt(
            "SELECT count(*) AS n FROM saved_views WHERE owner_ref = $1",
            [me.clone().into()],
        ))
        .await?
        .map_or(0, |r| r.try_get::<i64>("", "n").unwrap_or(0));
    if owned >= rules::MAX_PER_OWNER {
        return Err(unprocessable(&format!(
            "you can keep at most {} views",
            rules::MAX_PER_OWNER
        )));
    }
    let mut values = vec![pid.into(), me.into()];
    values.extend(bind(&payload));
    let inserted = txn
        .execute_raw(stmt(
            "INSERT INTO saved_views (pid, owner_ref, kind, name, filter, sort_field, \
             sort_direction, columns, shared) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT DO NOTHING",
            values,
        ))
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record(
        &txn,
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
async fn owned(txn: &sea_orm::DatabaseTransaction, pid: Uuid, me: &str) -> Result<(String, bool)> {
    let row = txn
        .query_one_raw(stmt(
            "SELECT owner_ref, shared, kind FROM saved_views WHERE pid = $1 FOR UPDATE",
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let owner: String = row.try_get("", "owner_ref")?;
    let shared: bool = row.try_get("", "shared")?;
    if owner == me {
        Ok((row.try_get("", "kind")?, shared))
    } else if shared {
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
    let txn = ctx.db.begin().await?;
    let (kind, _) = owned(&txn, pid, &me).await?;
    if kind != payload.kind {
        return Err(unprocessable("a view cannot move to another list"));
    }
    let mut values = vec![pid.into()];
    values.extend(bind(&payload));
    let changed = txn
        .execute_raw(stmt(
            "UPDATE saved_views SET kind = $2, name = $3, filter = $4, sort_field = $5, \
             sort_direction = $6, columns = $7, shared = $8, updated_at = now() \
             WHERE pid = $1",
            values,
        ))
        .await;
    if changed.is_err() {
        // The only constraint a replace can break is the owner's unique name.
        return Err(taken());
    }
    Audit::record(
        &txn,
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
    let txn = ctx.db.begin().await?;
    let (kind, shared) = owned(&txn, pid, &me).await?;
    txn.execute_raw(stmt("DELETE FROM saved_views WHERE pid = $1", [pid.into()]))
        .await?;
    Audit::record(
        &txn,
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
