//! Explainable workflow rules (CRM-R40): `/api/workflows`.
//!
//! A workflow is stored here and run by [`crate::workflows`]. Every part of it is closed and
//! readable: the trigger, the conditions (the segment filter language) and the actions. A dry run
//! shows what it would do to the current data, and every real run is in the run log.

use axa::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{PidPath, PidRef, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::db;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::models::rows;
use crate::rules::workflows::{self as rules, Action, Conditions};
use crate::validation::Problems;
use crate::workflows::{self as engine, DryRun, Step, Workflow};

/// `POST /api/workflows` and `PUT /api/workflows/{pid}` body (a `PUT` replaces the workflow).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowPayload {
    /// Unique, ignoring case.
    name: String,
    /// A workflow that is off never runs.
    #[serde(default = "on")]
    enabled: bool,
    /// `contact_created`, `contact_updated`, `deal_stage_changed` or `schedule`.
    trigger: String,
    /// For a schedule only: the least time between runs on the same contact, in minutes.
    #[serde(default)]
    every_minutes: Option<i32>,
    /// What the contact must look like (the segment filter language, without consent).
    #[serde(default)]
    conditions: Conditions,
    /// One to five actions, done in order.
    actions: Vec<Action>,
}

fn on() -> bool {
    true
}

/// One workflow.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Workflow")]
pub(crate) struct WorkflowView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    enabled: bool,
    trigger: String,
    every_minutes: Option<i32>,
    conditions: Conditions,
    actions: Vec<Action>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// One run: why it fired and what it did.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowRun")]
pub(crate) struct RunView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    #[schemars(with = "uuid::Uuid")]
    workflow_pid: String,
    /// The workflow's name when it ran.
    workflow_name: String,
    trigger: String,
    /// The kind of record that changed.
    subject_kind: String,
    #[schemars(with = "uuid::Uuid")]
    subject_pid: String,
    /// How many workflows fired this one (0 for a person's write or the clock).
    depth: i32,
    /// `done`, `failed` (its actions were undone) or `blocked` (held back by the loop guard).
    outcome: String,
    /// Why it fired, or why it was held back.
    reason: String,
    steps: Vec<Step>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// What `POST /api/workflows/run-due` did.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowSweep")]
pub(crate) struct Sweep {
    /// How many scheduled runs were made.
    runs: u64,
}

/// The most runs one page of the log returns.
const RUNS_PAGE: i64 = 100;

/// One `workflows` row as a view reads it.
struct Stored {
    pid: Uuid,
    name: String,
    enabled: bool,
    trigger: String,
    every_minutes: Option<i32>,
    conditions: serde_json::Value,
    actions: serde_json::Value,
    updated_at: chrono::DateTime<chrono::Utc>,
}

fn view(row: Stored) -> WorkflowView {
    WorkflowView {
        pid: row.pid.to_string(),
        name: row.name,
        enabled: row.enabled,
        trigger: row.trigger,
        every_minutes: row.every_minutes,
        conditions: serde_json::from_value(row.conditions).unwrap_or_default(),
        actions: serde_json::from_value(row.actions).unwrap_or_default(),
        updated_at: row.updated_at,
    }
}

/// The user references in the actions as the CRM stores them (CRM-D18).
fn normalise(payload: &mut WorkflowPayload) {
    for action in &mut payload.actions {
        action.normalise_user_refs();
    }
}

async fn validate(pool: &PgPool, payload: &WorkflowPayload) -> Result<()> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    for (field, message) in rules::check(
        &payload.trigger,
        payload.every_minutes,
        &payload.conditions,
        &payload.actions,
    ) {
        problems.push_field(&field, message);
    }
    for (i, action) in payload.actions.iter().enumerate() {
        let at = format!("actions[{i}]");
        let worker = |p: &mut Problems, value: Option<&str>| p.user_opt(&at, value);
        match action {
            Action::CreateTask { assignee_ref, .. } => {
                worker(&mut problems, assignee_ref.as_deref());
            }
            Action::Notify { recipient_ref } => worker(&mut problems, Some(recipient_ref)),
            Action::AssignOwner { pool, .. } => {
                for member in pool {
                    worker(&mut problems, Some(member));
                }
            }
            Action::SetField { field, value } if field == "owner_ref" => {
                worker(&mut problems, value.as_deref());
            }
            Action::EnrolNurture { sequence_pid }
                if rows::sequence(pool, *sequence_pid).await?.is_none() =>
            {
                problems.push_field(&at, "that nurture sequence does not exist".to_string());
            }
            _ => {}
        }
    }
    problems.finish()
}

fn taken() -> Error {
    unprocessable("a workflow with that name already exists")
}

/// `GET /api/workflows`.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = sqlx::query_as!(
        Stored,
        "SELECT pid, name, enabled, trigger, every_minutes, conditions, actions, updated_at \
         FROM workflows ORDER BY lower(name), id"
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    format::json(rows.into_iter().map(view).collect::<Vec<_>>())
}

async fn fetch(pool: &PgPool, pid: Uuid) -> Result<Stored> {
    sqlx::query_as!(
        Stored,
        "SELECT pid, name, enabled, trigger, every_minutes, conditions, actions, updated_at \
         FROM workflows WHERE pid = $1",
        pid,
    )
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound)
}

/// `GET /api/workflows/{pid}`.
#[debug_handler]
async fn show(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    format::json(view(fetch(db::pool(&ctx), pid).await?))
}

/// `POST /api/workflows`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<WorkflowPayload>,
) -> Result<Response> {
    normalise(&mut payload);
    let pool = db::pool(&ctx);
    validate(pool, &payload).await?;
    let pid = Uuid::new_v4();
    let mut txn = pool.begin().await?;
    let inserted = sqlx::query!(
        "INSERT INTO workflows (pid, name, enabled, trigger, every_minutes, conditions, actions) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
        pid,
        payload.name.trim(),
        payload.enabled,
        payload.trigger,
        payload.every_minutes,
        serde_json::json!(payload.conditions),
        serde_json::json!(payload.actions),
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record_on(
        &mut txn,
        "workflow",
        pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "trigger": payload.trigger, "enabled": payload.enabled })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `PUT /api/workflows/{pid}` — replace a workflow.
#[debug_handler]
async fn replace(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(mut payload): Json<WorkflowPayload>,
) -> Result<Response> {
    normalise(&mut payload);
    let pool = db::pool(&ctx);
    validate(pool, &payload).await?;
    let mut txn = pool.begin().await?;
    // The scan position starts again when the workflow changes.
    let changed = sqlx::query!(
        "UPDATE workflows SET name = $2, enabled = $3, trigger = $4, every_minutes = $5, \
         conditions = $6, actions = $7, scan_after_id = 0, updated_at = now() WHERE pid = $1",
        pid,
        payload.name.trim(),
        payload.enabled,
        payload.trigger,
        payload.every_minutes,
        serde_json::json!(payload.conditions),
        serde_json::json!(payload.actions),
    )
    .execute(&mut *txn)
    .await;
    // The only constraint a replace can break is the unique name.
    let changed = match changed {
        Ok(done) => done.rows_affected(),
        Err(_) => return Err(taken()),
    };
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Audit::record_on(
        &mut txn,
        "workflow",
        pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "trigger": payload.trigger, "enabled": payload.enabled })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `DELETE /api/workflows/{pid}` — the run log stays.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let gone = sqlx::query!("DELETE FROM workflows WHERE pid = $1", pid)
        .execute(&mut *txn)
        .await?
        .rows_affected();
    if gone == 0 {
        return Err(Error::NotFound);
    }
    Audit::record_on(&mut txn, "workflow", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `POST /api/workflows/{pid}/dry-run` — what the workflow would do to the current data.
/// Nothing is changed, sent or logged as a run; the dry run itself is audited.
#[debug_handler]
async fn dry_run(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let stored = view(fetch(pool, pid).await?);
    let workflow = Workflow {
        pid,
        name: stored.name,
        trigger: stored.trigger,
        conditions: stored.conditions,
        actions: stored.actions,
    };
    let result: DryRun = engine::dry_run(&mut *pool.acquire().await?, &workflow).await?;
    Audit::record_read_on(
        pool,
        "workflow",
        pid,
        "dry_run",
        caller.actor(),
        Some(serde_json::json!({
            "examined": result.examined, "would_fire": result.would_fire,
        })),
    )
    .await?;
    format::json(result)
}

/// `GET /api/workflows/{pid}/runs` — the newest runs of a workflow, even one since deleted.
#[debug_handler]
async fn runs(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    let rows = sqlx::query!(
        "SELECT pid, workflow_pid, workflow_name, trigger, subject_kind, subject_pid, depth, \
         outcome, reason, steps, created_at FROM workflow_runs WHERE workflow_pid = $1 \
         ORDER BY id DESC LIMIT $2",
        pid,
        RUNS_PAGE,
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    let out: Vec<RunView> = rows
        .into_iter()
        .map(|row| RunView {
            pid: row.pid.to_string(),
            workflow_pid: row.workflow_pid.to_string(),
            workflow_name: row.workflow_name,
            trigger: row.trigger,
            subject_kind: row.subject_kind,
            subject_pid: row.subject_pid.to_string(),
            depth: row.depth,
            outcome: row.outcome,
            reason: row.reason,
            steps: serde_json::from_value(row.steps).unwrap_or_default(),
            created_at: row.created_at,
        })
        .collect();
    format::json(out)
}

/// `POST /api/workflows/run-due` — run the scheduled workflows now. The server also does this
/// every minute.
#[debug_handler]
async fn run_due(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(Sweep {
        runs: engine::run_due(db::pool(&ctx)).await?,
    })
}

/// The workflow routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/workflows", get(list))
        .add("/workflows", post(create))
        .add("/workflows/run-due", post(run_due))
        .add("/workflows/{pid}", get(show))
        .add("/workflows/{pid}", put(replace))
        .add("/workflows/{pid}", delete(remove))
        .add("/workflows/{pid}/dry-run", post(dry_run))
        .add("/workflows/{pid}/runs", get(runs))
}
