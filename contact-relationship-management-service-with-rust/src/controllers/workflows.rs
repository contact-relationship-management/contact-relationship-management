//! Explainable workflow rules (CRM-R40): `/api/workflows`.
//!
//! A workflow is stored here and run by [`crate::workflows`]. Every part of it is closed and
//! readable: the trigger, the conditions (the segment filter language) and the actions. A dry run
//! shows what it would do to the current data, and every real run is in the run log.

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{PidPath, PidRef, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
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

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

const COLUMNS: &str = "pid, name, enabled, trigger, every_minutes, conditions, actions, updated_at";

fn view(row: &sea_orm::QueryResult) -> Result<WorkflowView> {
    let conditions: serde_json::Value = row.try_get("", "conditions")?;
    let actions: serde_json::Value = row.try_get("", "actions")?;
    Ok(WorkflowView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        name: row.try_get("", "name")?,
        enabled: row.try_get("", "enabled")?,
        trigger: row.try_get("", "trigger")?,
        every_minutes: row.try_get("", "every_minutes")?,
        conditions: serde_json::from_value(conditions).unwrap_or_default(),
        actions: serde_json::from_value(actions).unwrap_or_default(),
        updated_at: row.try_get("", "updated_at")?,
    })
}

async fn validate<C: ConnectionTrait>(db: &C, payload: &WorkflowPayload) -> Result<()> {
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
        let worker = |p: &mut Problems, value: Option<&str>| {
            p.ref_opt(&at, entity_ref::EntityType::Worker, value);
        };
        match action {
            Action::CreateTask { assignee_ref, .. } => {
                worker(&mut problems, assignee_ref.as_deref());
            }
            Action::Notify { recipient_ref } => worker(&mut problems, Some(recipient_ref)),
            Action::SetField { field, value } if field == "owner_ref" => {
                worker(&mut problems, value.as_deref());
            }
            Action::EnrolNurture { sequence_pid }
                if records::find_sequence(db, *sequence_pid).await.is_err() =>
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

fn binds(payload: &WorkflowPayload) -> Vec<Value> {
    vec![
        payload.name.trim().to_string().into(),
        payload.enabled.into(),
        payload.trigger.clone().into(),
        payload.every_minutes.into(),
        serde_json::json!(payload.conditions).into(),
        serde_json::json!(payload.actions).into(),
    ]
}

/// `GET /api/workflows`.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!("SELECT {COLUMNS} FROM workflows ORDER BY lower(name), id"),
            [],
        ))
        .await?;
    format::json(rows.iter().map(view).collect::<Result<Vec<_>>>()?)
}

async fn fetch<C: ConnectionTrait>(db: &C, pid: Uuid) -> Result<sea_orm::QueryResult> {
    db.query_one_raw(stmt(
        &format!("SELECT {COLUMNS} FROM workflows WHERE pid = $1"),
        [pid.into()],
    ))
    .await?
    .ok_or(Error::NotFound)
}

/// `GET /api/workflows/{pid}`.
#[debug_handler]
async fn show(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    format::json(view(&fetch(&ctx.db, pid).await?)?)
}

/// `POST /api/workflows`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<WorkflowPayload>,
) -> Result<Response> {
    validate(&ctx.db, &payload).await?;
    let pid = Uuid::new_v4();
    let mut values = vec![pid.into()];
    values.extend(binds(&payload));
    let txn = ctx.db.begin().await?;
    let inserted = txn
        .execute_raw(stmt(
            "INSERT INTO workflows (pid, name, enabled, trigger, every_minutes, conditions, actions) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
            values,
        ))
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record(
        &txn,
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
    Json(payload): Json<WorkflowPayload>,
) -> Result<Response> {
    validate(&ctx.db, &payload).await?;
    let mut values = vec![pid.into()];
    values.extend(binds(&payload));
    let txn = ctx.db.begin().await?;
    // The scan position starts again when the workflow changes.
    let changed = txn
        .execute_raw(stmt(
            "UPDATE workflows SET name = $2, enabled = $3, trigger = $4, every_minutes = $5, \
             conditions = $6, actions = $7, scan_after_id = 0, updated_at = now() WHERE pid = $1",
            values,
        ))
        .await;
    // The only constraint a replace can break is the unique name.
    if changed.is_err() {
        return Err(taken());
    }
    if changed.is_ok_and(|r| r.rows_affected() == 0) {
        return Err(Error::NotFound);
    }
    Audit::record(
        &txn,
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
    let txn = ctx.db.begin().await?;
    let gone = txn
        .execute_raw(stmt("DELETE FROM workflows WHERE pid = $1", [pid.into()]))
        .await?
        .rows_affected();
    if gone == 0 {
        return Err(Error::NotFound);
    }
    Audit::record(&txn, "workflow", pid, "deleted", caller.actor(), None).await?;
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
    let row = fetch(&ctx.db, pid).await?;
    let stored = view(&row)?;
    let workflow = Workflow {
        pid,
        name: stored.name,
        trigger: stored.trigger,
        conditions: stored.conditions,
        actions: stored.actions,
    };
    let result: DryRun = engine::dry_run(&ctx.db, &workflow).await?;
    Audit::record_read(
        &ctx.db,
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
    let rows = ctx
        .db
        .query_all_raw(stmt(
            "SELECT pid, workflow_pid, workflow_name, trigger, subject_kind, subject_pid, depth, \
             outcome, reason, steps, created_at FROM workflow_runs WHERE workflow_pid = $1 \
             ORDER BY id DESC LIMIT $2",
            [pid.into(), RUNS_PAGE.into()],
        ))
        .await?;
    let out = rows
        .iter()
        .map(|row| {
            let steps: serde_json::Value = row.try_get("", "steps")?;
            Ok(RunView {
                pid: row.try_get::<Uuid>("", "pid")?.to_string(),
                workflow_pid: row.try_get::<Uuid>("", "workflow_pid")?.to_string(),
                workflow_name: row.try_get("", "workflow_name")?,
                trigger: row.try_get("", "trigger")?,
                subject_kind: row.try_get("", "subject_kind")?,
                subject_pid: row.try_get::<Uuid>("", "subject_pid")?.to_string(),
                depth: row.try_get("", "depth")?,
                outcome: row.try_get("", "outcome")?,
                reason: row.try_get("", "reason")?,
                steps: serde_json::from_value(steps).unwrap_or_default(),
                created_at: row.try_get("", "created_at")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    format::json(out)
}

/// `POST /api/workflows/run-due` — run the scheduled workflows now. The server also does this
/// every minute.
#[debug_handler]
async fn run_due(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(Sweep {
        runs: engine::run_due(&ctx.db).await?,
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
