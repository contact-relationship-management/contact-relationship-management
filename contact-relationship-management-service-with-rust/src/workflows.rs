//! The workflow engine (CRM-R40): runs the rules of [`crate::rules::workflows`] against the
//! database.
//!
//! A run happens inside the transaction of the write that triggered it, so the write, its actions
//! and the run log commit together or not at all. The actions of one run sit in a savepoint: if one
//! fails, none of them stays, the run is logged as failed, and the triggering write is unharmed.
//! A failed *log* write does fail the triggering write, like any audit row (CRM-R19).

use std::future::Future;
use std::pin::Pin;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, Statement, TransactionTrait,
    Value,
};
use serde::Serialize;
use uuid::Uuid;

use crate::models::_entities::{activities, contacts, nurture_enrollments};
use crate::models::audit_logs::Model as Audit;
use crate::models::live::FindLive;
use crate::rules::segment::{ContactFacts, clauses_match};
use crate::rules::workflows::{Action, Conditions, SCHEDULE_BATCH, SCHEDULE_SCAN, guard};
use crate::streaming;

type Result<T> = loco_rs::Result<T>;

/// What happened that may start a workflow.
#[derive(Debug, Clone)]
pub struct Event {
    /// One of [`crate::rules::workflows::TRIGGERS`].
    pub trigger: &'static str,
    /// The kind of record that changed.
    pub subject_kind: &'static str,
    /// The record that changed.
    pub subject_pid: Uuid,
    /// The contact the actions are about: the contact itself, or a deal's primary contact.
    pub contact_pid: Option<Uuid>,
}

impl Event {
    /// A contact was created or updated, or a scheduled workflow reached it.
    #[must_use]
    pub fn contact(trigger: &'static str, pid: Uuid) -> Self {
        Self {
            trigger,
            subject_kind: "contact",
            subject_pid: pid,
            contact_pid: Some(pid),
        }
    }

    /// A deal moved to another (non-closing) stage.
    #[must_use]
    pub fn deal_stage(deal: Uuid, primary_contact: Option<Uuid>) -> Self {
        Self {
            trigger: "deal_stage_changed",
            subject_kind: "deal",
            subject_pid: deal,
            contact_pid: primary_contact,
        }
    }
}

/// One action of a run, or of a dry run.
#[derive(Debug, Clone, Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowStep")]
pub struct Step {
    /// What the action does, in words.
    pub action: String,
    /// `done`, `skipped`, `failed`, `rolled_back` (a run that failed undid it), or, in a dry run,
    /// `would_do` or `would_skip`.
    pub result: String,
    /// Why, in words; never contact data.
    pub detail: String,
}

fn step(action: &Action, result: &str, detail: impl Into<String>) -> Step {
    Step {
        action: action.describe(),
        result: result.to_string(),
        detail: detail.into(),
    }
}

/// A stored workflow.
#[derive(Debug, Clone)]
pub struct Workflow {
    /// Its id.
    pub pid: Uuid,
    /// Its name.
    pub name: String,
    /// The trigger.
    pub trigger: String,
    /// What it looks at.
    pub conditions: Conditions,
    /// What it does.
    pub actions: Vec<Action>,
}

/// A row limit as the database takes it.
fn limit(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

fn read(row: &sea_orm::QueryResult) -> Result<Workflow> {
    let conditions: serde_json::Value = row.try_get("", "conditions")?;
    let actions: serde_json::Value = row.try_get("", "actions")?;
    Ok(Workflow {
        pid: row.try_get("", "pid")?,
        name: row.try_get("", "name")?,
        trigger: row.try_get("", "trigger")?,
        conditions: serde_json::from_value(conditions).unwrap_or_default(),
        actions: serde_json::from_value(actions).unwrap_or_default(),
    })
}

/// The enabled workflows with this trigger.
async fn enabled<C: ConnectionTrait>(db: &C, trigger: &str) -> Result<Vec<Workflow>> {
    let rows = db
        .query_all_raw(stmt(
            "SELECT pid, name, trigger, conditions, actions FROM workflows \
             WHERE enabled AND trigger = $1 ORDER BY id",
            [trigger.to_string().into()],
        ))
        .await?;
    rows.iter().map(read).collect()
}

/// A contact and the tier of its account, if it is live.
async fn load<C: ConnectionTrait>(
    db: &C,
    pid: Uuid,
) -> Result<Option<(contacts::Model, Option<String>)>> {
    let Some(contact) = contacts::Entity::find_live()
        .filter(contacts::Column::Pid.eq(pid))
        .filter(contacts::Column::MergedIntoPid.is_null())
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let tier = match contact.account_pid {
        Some(account) => db
            .query_one_raw(stmt(
                "SELECT tier FROM accounts WHERE pid = $1 AND deleted_at IS NULL",
                [account.into()],
            ))
            .await?
            .map(|r| r.try_get::<String>("", "tier"))
            .transpose()?,
        None => None,
    };
    Ok(Some((contact, tier)))
}

/// Whether the contact meets the conditions. Consent is not a condition (see
/// [`clauses_match`]); the actions that lead to a send check it.
fn meets(conditions: &Conditions, contact: &contacts::Model, tier: Option<String>) -> bool {
    clauses_match(
        &conditions.filter(),
        &ContactFacts {
            consent: String::new(),
            status: contact.status.clone(),
            channel: contact.preferred_channel.as_str().to_string(),
            account_tier: tier,
        },
    )
}

/// Run every enabled workflow whose trigger is `event.trigger`. `depth` counts the workflows that
/// fired this one (0 for a write by a person or the clock).
///
/// # Errors
///
/// A database error, including a failed write of the run log.
pub fn fire<'a>(
    txn: &'a DatabaseTransaction,
    event: &'a Event,
    depth: i32,
    actor: Option<&'a str>,
) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        for workflow in enabled(txn, event.trigger).await? {
            run_one(txn, &workflow, event, depth, actor).await?;
        }
        Ok(())
    })
}

/// How many runs of the workflow completed in the last minute.
async fn recent_runs<C: ConnectionTrait>(db: &C, workflow: Uuid, outcomes: &str) -> Result<i64> {
    let row = db
        .query_one_raw(stmt(
            &format!(
                "SELECT count(*) AS n FROM workflow_runs WHERE workflow_pid = $1 \
                 AND outcome IN ({outcomes}) AND created_at > now() - interval '1 minute'"
            ),
            [workflow.into()],
        ))
        .await?
        .ok_or(loco_rs::Error::NotFound)?;
    Ok(row.try_get("", "n")?)
}

struct Logged<'a> {
    run: Uuid,
    workflow: &'a Workflow,
    event: &'a Event,
    depth: i32,
    outcome: &'a str,
    reason: &'a str,
    steps: &'a [Step],
}

/// Write the run log row and its audit row.
async fn log_run(txn: &DatabaseTransaction, log: &Logged<'_>, actor: Option<&str>) -> Result<()> {
    txn.execute_raw(stmt(
        "INSERT INTO workflow_runs (pid, workflow_pid, workflow_name, trigger, subject_kind, \
         subject_pid, depth, outcome, reason, steps) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        [
            log.run.into(),
            log.workflow.pid.into(),
            log.workflow.name.clone().into(),
            log.event.trigger.to_string().into(),
            log.event.subject_kind.to_string().into(),
            log.event.subject_pid.into(),
            log.depth.into(),
            log.outcome.to_string().into(),
            log.reason.to_string().into(),
            serde_json::json!(log.steps).into(),
        ],
    ))
    .await?;
    let action = match log.outcome {
        "done" => "fired",
        other => other,
    };
    Audit::record(
        txn,
        "workflow_run",
        log.run,
        action,
        actor,
        Some(serde_json::json!({
            "workflow_pid": log.workflow.pid, "trigger": log.event.trigger,
            "subject_kind": log.event.subject_kind, "subject_pid": log.event.subject_pid,
            "depth": log.depth, "steps": log.steps.len(),
        })),
    )
    .await?;
    Ok(())
}

/// Run one workflow for one event. Returns whether it ran (met its conditions and was not held
/// back by the loop guard).
///
/// # Errors
///
/// A database error outside the actions, or a failed write of the run log.
pub async fn run_one(
    txn: &DatabaseTransaction,
    workflow: &Workflow,
    event: &Event,
    depth: i32,
    actor: Option<&str>,
) -> Result<bool> {
    let Some(contact_pid) = event.contact_pid else {
        return Ok(false);
    };
    let Some((contact, tier)) = load(txn, contact_pid).await? else {
        return Ok(false);
    };
    if !meets(&workflow.conditions, &contact, tier) {
        return Ok(false);
    }
    let run = Uuid::new_v4();
    let held = guard(
        depth,
        recent_runs(txn, workflow.pid, "'done', 'failed'").await?,
    );
    if let Some(why) = held {
        // One note a minute is enough; a storm must not fill the log.
        if recent_runs(txn, workflow.pid, "'blocked'").await? == 0 {
            let log = Logged {
                run,
                workflow,
                event,
                depth,
                outcome: "blocked",
                reason: why,
                steps: &[],
            };
            log_run(txn, &log, actor).await?;
        }
        return Ok(false);
    }
    let reason = format!(
        "{} on a {}; conditions met ({})",
        event.trigger.replace('_', " "),
        event.subject_kind,
        workflow.conditions.describe()
    );
    let by = format!("workflow:{}", workflow.pid);
    let savepoint = txn.begin().await?;
    let mut steps = Vec::new();
    let mut failed = false;
    for (index, action) in workflow.actions.iter().enumerate() {
        let ctx = ActionContext {
            workflow: workflow.pid,
            run,
            index,
            depth,
            by: &by,
        };
        match act(&savepoint, &ctx, &contact, action).await {
            Ok((result, detail)) => steps.push(step(action, result, detail)),
            Err(error) => {
                tracing::warn!(%error, workflow = %workflow.pid, "workflow action failed");
                steps.push(step(action, "failed", "the action could not be completed"));
                failed = true;
                break;
            }
        }
    }
    if failed {
        savepoint.rollback().await?;
        for s in &mut steps {
            if s.result == "done" {
                s.result = "rolled_back".to_string();
            }
        }
    } else {
        savepoint.commit().await?;
    }
    let outcome = if failed { "failed" } else { "done" };
    let log = Logged {
        run,
        workflow,
        event,
        depth,
        outcome,
        reason: &reason,
        steps: &steps,
    };
    log_run(txn, &log, actor).await?;
    Ok(true)
}

struct ActionContext<'a> {
    workflow: Uuid,
    run: Uuid,
    index: usize,
    depth: i32,
    by: &'a str,
}

/// Do one action; returns its result word and a reason.
async fn act(
    txn: &DatabaseTransaction,
    ctx: &ActionContext<'_>,
    contact: &contacts::Model,
    action: &Action,
) -> Result<(&'static str, String)> {
    match action {
        Action::CreateTask {
            summary,
            assignee_ref,
            due_in_days,
        } => {
            let due = due_in_days
                .map(|d| chrono::Utc::now().date_naive() + chrono::Duration::days(i64::from(d)));
            let row = activities::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                subject_kind: ActiveValue::set("contact".to_string()),
                subject_pid: ActiveValue::set(contact.pid),
                kind: ActiveValue::set("task".to_string()),
                occurred_at: ActiveValue::set(chrono::Utc::now().into()),
                actor_ref: ActiveValue::set(None),
                summary: ActiveValue::set(summary.clone().into()),
                due_on: ActiveValue::set(due),
                assignee_ref: ActiveValue::set(assignee_ref.clone()),
                remind_at: ActiveValue::set(None),
                done: ActiveValue::set(false),
                sentiment: ActiveValue::set(None),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(txn)
            .await?;
            Audit::record(txn, "activity", row.pid, "created", Some(ctx.by), None).await?;
            Ok(("done", "task created".to_string()))
        }
        Action::Notify { recipient_ref } => {
            let made = txn
                .execute_raw(stmt(
                    "INSERT INTO notifications (pid, recipient_ref, kind, subject_kind, \
                     subject_pid, source_pid, source_key, suppressed) \
                     VALUES ($1, $2, 'workflow', 'contact', $3, $4, $5, FALSE) \
                     ON CONFLICT DO NOTHING",
                    [
                        Uuid::new_v4().into(),
                        recipient_ref.clone().into(),
                        contact.pid.into(),
                        ctx.run.into(),
                        ctx.index.to_string().into(),
                    ],
                ))
                .await?
                .rows_affected();
            Ok(if made == 0 {
                ("skipped", "already notified".to_string())
            } else {
                ("done", "notification added".to_string())
            })
        }
        Action::SetField { field, value } => {
            set_field(txn, ctx, contact, field, value.as_ref()).await
        }
        Action::EnrolNurture { sequence_pid } => enrol(txn, ctx, contact, *sequence_pid).await,
        Action::SendWebhook { event } => {
            streaming::emit_on(
                txn,
                "workflow",
                event,
                &ctx.run.to_string(),
                "",
                Some(ctx.by),
                Some(serde_json::json!({
                    "workflow_pid": ctx.workflow, "subject_kind": "contact",
                    "subject_pid": contact.pid,
                })),
            )
            .await?;
            Ok(("done", "event emitted".to_string()))
        }
    }
}

/// The value a contact field has now, as text.
fn current(contact: &contacts::Model, field: &str) -> Option<String> {
    match field {
        "job_title" => contact.job_title.clone(),
        "owner_ref" => contact.owner_ref.clone(),
        "preferred_channel" => Some(contact.preferred_channel.as_str().to_string()),
        _ => Some(contact.status.clone()),
    }
}

async fn set_field(
    txn: &DatabaseTransaction,
    ctx: &ActionContext<'_>,
    contact: &contacts::Model,
    field: &str,
    value: Option<&String>,
) -> Result<(&'static str, String)> {
    if current(contact, field).as_ref() == value {
        return Ok(("skipped", format!("{field} already has that value")));
    }
    let mut active: contacts::ActiveModel = contact.clone().into();
    match field {
        "job_title" => active.job_title = ActiveValue::set(value.cloned()),
        "owner_ref" => active.owner_ref = ActiveValue::set(value.cloned()),
        "preferred_channel" => {
            active.preferred_channel = ActiveValue::set(value.cloned().unwrap_or_default().into());
        }
        _ => active.status = ActiveValue::set(value.cloned().unwrap_or_default()),
    }
    let row = active.update(txn).await?;
    Audit::record(
        txn,
        "contact",
        row.pid,
        "updated",
        Some(ctx.by),
        Some(serde_json::json!({ "fields": [field] })),
    )
    .await?;
    streaming::emit_on(
        txn,
        "contact",
        "updated",
        &row.pid.to_string(),
        &row.display_name,
        Some(ctx.by),
        None,
    )
    .await?;
    // The change is itself an event, one level deeper.
    fire(
        txn,
        &Event::contact("contact_updated", row.pid),
        ctx.depth + 1,
        Some(ctx.by),
    )
    .await?;
    Ok(("done", format!("{field} changed")))
}

/// Why a contact cannot be enrolled in the sequence now, or the sequence if they can.
async fn enrolment_blocker<C: ConnectionTrait>(
    db: &C,
    contact: &contacts::Model,
    sequence_pid: Uuid,
) -> Result<std::result::Result<crate::models::_entities::nurture_sequences::Model, String>> {
    let Ok(sequence) = crate::models::records::find_sequence(db, sequence_pid).await else {
        return Ok(Err("the sequence no longer exists".to_string()));
    };
    // Consent is read now, inside the action, not taken from the trigger (CRM-R40).
    if crate::consents::state_of(db, contact, &sequence.purpose).await? != "granted" {
        return Ok(Err(format!(
            "no {} consent at this moment",
            sequence.purpose
        )));
    }
    let active = nurture_enrollments::Entity::find()
        .filter(nurture_enrollments::Column::SequencePid.eq(sequence_pid))
        .filter(nurture_enrollments::Column::ContactPid.eq(contact.pid))
        .filter(nurture_enrollments::Column::Status.eq("active"))
        .filter(nurture_enrollments::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    if active.is_some() {
        return Ok(Err("already enrolled in that sequence".to_string()));
    }
    Ok(Ok(sequence))
}

async fn enrol(
    txn: &DatabaseTransaction,
    ctx: &ActionContext<'_>,
    contact: &contacts::Model,
    sequence_pid: Uuid,
) -> Result<(&'static str, String)> {
    let sequence = match enrolment_blocker(txn, contact, sequence_pid).await? {
        Ok(sequence) => sequence,
        Err(why) => return Ok(("skipped", why)),
    };
    let made =
        crate::controllers::marketing::nurture::enrol_in(txn, &sequence, contact.pid, Some(ctx.by))
            .await?;
    Ok(match made {
        Some(_) => ("done", "enrolled".to_string()),
        None => ("skipped", "the sequence has no steps".to_string()),
    })
}

/// What a workflow would do for one contact, changing nothing.
async fn preview<C: ConnectionTrait>(
    db: &C,
    contact: &contacts::Model,
    action: &Action,
) -> Result<Step> {
    Ok(match action {
        Action::SetField { field, value } if current(contact, field) == *value => step(
            action,
            "would_skip",
            format!("{field} already has that value"),
        ),
        Action::EnrolNurture { sequence_pid } => {
            match enrolment_blocker(db, contact, *sequence_pid).await? {
                Ok(_) => step(action, "would_do", "the contact has consented"),
                Err(why) => step(action, "would_skip", why),
            }
        }
        _ => step(action, "would_do", ""),
    })
}

/// A contact the dry run examined that met the conditions.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowDryRunSample")]
pub struct Sample {
    /// The contact.
    pub contact_pid: Uuid,
    /// What each action would do for them.
    pub steps: Vec<Step>,
}

/// The result of a dry run.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WorkflowDryRun")]
pub struct DryRun {
    /// How many contacts were looked at (at most 500).
    pub examined: i64,
    /// How many of them meet the conditions.
    pub would_fire: i64,
    /// Whether there were more contacts than were looked at.
    pub truncated: bool,
    /// The first few that meet them, with what would happen.
    pub samples: Vec<Sample>,
}

/// How many matching contacts a dry run describes in full.
const SAMPLES: usize = 5;

/// Try a workflow against the current data. Nothing is changed, sent or logged.
///
/// # Errors
///
/// A database error.
pub async fn dry_run<C: ConnectionTrait>(db: &C, workflow: &Workflow) -> Result<DryRun> {
    let only_deal_contacts = if workflow.trigger == "deal_stage_changed" {
        "AND EXISTS (SELECT 1 FROM deals d WHERE d.primary_contact_pid = c.pid \
         AND d.deleted_at IS NULL)"
    } else {
        ""
    };
    let rows = db
        .query_all_raw(stmt(
            &format!(
                "SELECT c.pid FROM contacts c WHERE c.deleted_at IS NULL \
                 AND c.merged_into_pid IS NULL {only_deal_contacts} ORDER BY c.id LIMIT $1"
            ),
            [limit(SCHEDULE_SCAN + 1).into()],
        ))
        .await?;
    let truncated = rows.len() > SCHEDULE_SCAN;
    let mut out = DryRun {
        examined: 0,
        would_fire: 0,
        truncated,
        samples: Vec::new(),
    };
    for row in rows.iter().take(SCHEDULE_SCAN) {
        out.examined += 1;
        let Some((contact, tier)) = load(db, row.try_get("", "pid")?).await? else {
            continue;
        };
        if !meets(&workflow.conditions, &contact, tier) {
            continue;
        }
        out.would_fire += 1;
        if out.samples.len() < SAMPLES {
            let mut steps = Vec::new();
            for action in &workflow.actions {
                steps.push(preview(db, &contact, action).await?);
            }
            out.samples.push(Sample {
                contact_pid: contact.pid,
                steps,
            });
        }
    }
    Ok(out)
}

/// Run the scheduled workflows that are due. Each looks at the next slice of contacts after where
/// it stopped, acts on at most [`SCHEDULE_BATCH`] of those it has not run for within its interval,
/// and remembers where it stopped, so a large book is worked through over several ticks. Returns
/// how many runs were made.
///
/// # Errors
///
/// A database error reading the workflows; a failure inside one workflow's slice is logged and
/// leaves that workflow for the next tick.
pub async fn run_due(db: &DatabaseConnection) -> Result<u64> {
    let mut made = 0;
    for workflow in enabled(db, "schedule").await? {
        match tick(db, &workflow).await {
            Ok(n) => made += n,
            Err(error) => {
                tracing::warn!(%error, workflow = %workflow.pid, "scheduled workflow failed");
            }
        }
    }
    Ok(made)
}

async fn tick(db: &DatabaseConnection, workflow: &Workflow) -> Result<u64> {
    let txn = db.begin().await?;
    // One replica at a time; the others skip this workflow until the next tick.
    let locked = txn
        .query_one_raw(stmt(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0)) AS ok",
            [format!("workflow:{}", workflow.pid).into()],
        ))
        .await?
        .map(|r| r.try_get::<bool>("", "ok"))
        .transpose()?
        .unwrap_or(false);
    if !locked {
        return Ok(0);
    }
    let state = txn
        .query_one_raw(stmt(
            "SELECT scan_after_id, every_minutes FROM workflows WHERE pid = $1 AND enabled FOR UPDATE",
            [workflow.pid.into()],
        ))
        .await?;
    let Some(state) = state else { return Ok(0) };
    let after: i32 = state.try_get("", "scan_after_id")?;
    let every: i32 = state
        .try_get::<Option<i32>>("", "every_minutes")?
        .unwrap_or(60);
    let rows = txn
        .query_all_raw(stmt(
            "SELECT c.id, c.pid FROM contacts c WHERE c.deleted_at IS NULL \
             AND c.merged_into_pid IS NULL AND c.id > $2 \
             AND NOT EXISTS (SELECT 1 FROM workflow_runs r WHERE r.workflow_pid = $1 \
                 AND r.subject_pid = c.pid AND r.outcome IN ('done', 'failed') \
                 AND r.created_at > now() - make_interval(mins => $3)) \
             ORDER BY c.id LIMIT $4",
            [
                workflow.pid.into(),
                after.into(),
                every.into(),
                limit(SCHEDULE_SCAN).into(),
            ],
        ))
        .await?;
    let exhausted = rows.len() < SCHEDULE_SCAN;
    let mut cursor = after;
    let mut ran = 0_usize;
    let mut stopped_early = false;
    for row in &rows {
        let pid: Uuid = row.try_get("", "pid")?;
        cursor = row.try_get("", "id")?;
        if run_one(&txn, workflow, &Event::contact("schedule", pid), 0, None).await? {
            ran += 1;
            if ran >= SCHEDULE_BATCH {
                stopped_early = true;
                break;
            }
        }
    }
    if exhausted && !stopped_early {
        cursor = 0;
    }
    txn.execute_raw(stmt(
        "UPDATE workflows SET scan_after_id = $2 WHERE pid = $1",
        [workflow.pid.into(), cursor.into()],
    ))
    .await?;
    txn.commit().await?;
    Ok(u64::try_from(ran).unwrap_or(u64::MAX))
}
