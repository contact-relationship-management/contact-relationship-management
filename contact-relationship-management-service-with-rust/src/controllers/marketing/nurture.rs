//! Nurture sequences: create, enrol, and the idempotent advance sweep (CRM-R9, CRM-D8).

use super::{
    AppContext, Audit, Deserialize, Error, MaybeAuthUser, Metrics, PidRef, Problems, Response,
    Result, State, Uuid, clients, contacts, debug_handler, deliveries, format, mail,
    nurture_enrollments, nurture_sequences, nurture_steps, streaming, tokens, unprocessable,
    with_unsubscribe,
};
use sqlx::PgConnection;

use crate::controllers::PidPath;
use crate::controllers::templates;
use crate::db;
use crate::json::Json;
use crate::models::DateTimeWithTimeZone;
use crate::models::encrypted::EncryptedText;
use crate::models::rows;

/// `POST /api/nurture-sequences` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SequencePayload {
    name: String,
    steps: Vec<StepPayload>,
    /// The consent purpose it is sent under (`marketing` when absent).
    #[serde(default)]
    purpose: Option<String>,
}

/// One nurture step.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StepPayload {
    delay_hours: i32,
    /// A label for the step; the email template's id when `template_pid` is given and this is absent.
    #[serde(default)]
    template_ref: String,
    subject: Option<String>,
    body: Option<String>,
    /// An email template (CRM-R36) to make the message from instead of `subject` and `body`.
    #[serde(default)]
    template_pid: Option<Uuid>,
}

/// `POST /api/nurture-sequences/{pid}/enrollments` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EnrollPayload {
    contact_pid: Uuid,
}

/// `POST /api/nurture-sequences` — create with ordered steps.
#[debug_handler]
pub(super) async fn create_sequence(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SequencePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.token_opt("purpose", tokens::PURPOSES, payload.purpose.as_deref());
    if payload.steps.is_empty() || payload.steps.len() > 32 {
        problems.push("a sequence needs 1-32 steps".to_string());
    }
    for step in &payload.steps {
        if step.template_pid.is_none() {
            problems.require_text("steps[].template_ref", &step.template_ref);
        } else if step.subject.is_some() || step.body.is_some() {
            problems
                .push("steps[].template_pid cannot be given with a subject or body".to_string());
        }
        if step.delay_hours < 0 {
            problems.push("delay_hours must be non-negative".to_string());
        }
        if let Some(subject) = &step.subject
            && (subject.contains(['\r', '\n']) || subject.len() > 998)
        {
            problems.push("steps[].subject must be a single line".to_string());
        }
        if step.body.as_ref().is_some_and(|b| b.len() > 20_000) {
            problems.push("steps[].body is too long".to_string());
        }
    }
    if mail::current().needs_address()
        && payload.steps.iter().any(|s| {
            s.template_pid.is_none() && s.body.as_deref().is_none_or(|b| b.trim().is_empty())
        })
    {
        problems.push("every step needs a body when real mail is on".to_string());
    }
    problems.finish()?;
    for step in &payload.steps {
        if let Some(template) = step.template_pid
            && !templates::exists(db::pool(&ctx), template).await?
        {
            return Err(unprocessable(
                "steps[].template_pid is not an email template",
            ));
        }
    }
    let sequence = Uuid::new_v4();
    let mut txn = db::pool(&ctx).begin().await?;
    sqlx::query!(
        "INSERT INTO nurture_sequences (pid, name, purpose) VALUES ($1, $2, $3)",
        sequence,
        payload.name,
        payload
            .purpose
            .as_deref()
            .unwrap_or(tokens::DEFAULT_PURPOSE),
    )
    .execute(&mut *txn)
    .await?;
    for (position, step) in payload.steps.iter().enumerate() {
        let template_ref = if step.template_ref.trim().is_empty() {
            step.template_pid.map(|p| p.to_string()).unwrap_or_default()
        } else {
            step.template_ref.clone()
        };
        sqlx::query!(
            "INSERT INTO nurture_steps (pid, sequence_pid, position, delay_hours, template_ref, \
             subject, body, template_pid) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            Uuid::new_v4(),
            sequence,
            i32::try_from(position).unwrap_or(i32::MAX),
            step.delay_hours,
            template_ref,
            step.subject,
            step.body,
            step.template_pid,
        )
        .execute(&mut *txn)
        .await?;
    }
    Audit::record_on(
        &mut txn,
        "nurture_sequence",
        sequence,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: sequence.to_string(),
    })
}

/// `POST /api/nurture-sequences/{pid}/enrollments` — enrol a
/// consented contact; the first step is due immediately + its delay.
#[debug_handler]
pub(super) async fn enroll(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<EnrollPayload>,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let sequence = rows::sequence(pool, pid).await?.ok_or(Error::NotFound)?;
    let contact = rows::contact(pool, payload.contact_pid)
        .await?
        .ok_or(Error::NotFound)?;
    let mut txn = pool.begin().await?;
    if crate::consents::state_of(&mut *txn, &contact, &sequence.purpose).await? != "granted" {
        return Err(unprocessable(&format!(
            "contact has not granted {} consent",
            sequence.purpose
        )));
    }
    let pid = enrol_in(&mut txn, &sequence, contact.pid, caller.actor())
        .await?
        .ok_or_else(|| unprocessable("sequence has no steps"))?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// Start `contact` on `sequence`, due after its first step's delay, inside the caller's
/// transaction. The caller has checked consent. `None` when the sequence has no steps.
pub(crate) async fn enrol_in(
    txn: &mut PgConnection,
    sequence: &nurture_sequences::Model,
    contact: Uuid,
    actor: Option<&str>,
) -> Result<Option<Uuid>> {
    let Some(first) = rows::steps_of(&mut *txn, sequence.pid)
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let due = crate::clock::now() + chrono::Duration::hours(i64::from(first.delay_hours));
    let pid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO nurture_enrollments (pid, sequence_pid, contact_pid, current_step, \
         next_due_at, status) VALUES ($1, $2, $3, 0, $4, 'active')",
        pid,
        sequence.pid,
        contact,
        due,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(txn, "nurture_enrollment", pid, "created", actor, None).await?;
    Ok(Some(pid))
}

/// What happened to one real nurture-step send.
pub(super) enum StepSend {
    /// Sent (now or earlier); carries the `Message-ID`.
    Sent(String),
    /// Could not be sent this time; the enrolment stays due.
    Retry(&'static str),
    /// Never going to be sent; the enrolment is exited.
    Exit,
}

/// Send one nurture step through the real sender. The delivery row is keyed by
/// `(step, contact)`, so a step already sent (a crash after the send, before
/// the enrolment advanced) is not mailed twice. It uses the sweep's
/// transaction rather than a second connection: the pool can be a single
/// connection, and holding one while waiting for another would stall.
pub(super) async fn send_step(
    txn: &mut PgConnection,
    sender: &mail::Sender,
    contact: &contacts::Model,
    step: &nurture_steps::Model,
) -> Result<StepSend> {
    let (row_id, state, message_id) = deliveries::row_for(txn, step.pid, contact.pid).await?;
    if let (true, Some(id)) = (state == "sent", message_id) {
        return Ok(StepSend::Sent(id));
    }
    if deliveries::is_suppressed(&mut *txn, contact.pid).await? {
        deliveries::settle(
            &mut *txn,
            row_id,
            "suppressed",
            None,
            Some("mailbox suppressed"),
        )
        .await?;
        return Ok(StepSend::Exit);
    }
    let Some(address) = clients::email_address(&contact.person_ref).await else {
        deliveries::settle(
            &mut *txn,
            row_id,
            "no_address",
            None,
            Some("no address on file"),
        )
        .await?;
        return Ok(StepSend::Retry("no address"));
    };
    // The unsubscribe token names the step where a campaign would be named.
    let unsubscribe_url = crate::unsubscribe::current().map(|u| u.url(contact.pid, step.pid));
    let content = if let Some(template) = step.template_pid {
        let made =
            templates::content_for(txn, template, contact, unsubscribe_url.as_deref()).await?;
        let Some(content) = made else {
            let why = "permanent: the email template is gone";
            deliveries::settle(&mut *txn, row_id, "failed", None, Some(why)).await?;
            return Ok(StepSend::Exit);
        };
        content
    } else if let Some(body) = step.body.as_deref().filter(|b| !b.trim().is_empty()) {
        (
            step.subject
                .clone()
                .unwrap_or_else(|| step.template_ref.clone()),
            with_unsubscribe(body.to_string(), unsubscribe_url.as_deref()),
        )
    } else {
        deliveries::settle(
            &mut *txn,
            row_id,
            "failed",
            None,
            Some("permanent: step has no body"),
        )
        .await?;
        return Ok(StepSend::Retry("step has no body"));
    };
    let message = mail::Message {
        to: address,
        to_name: contact.display_name.clone(),
        subject: content.0,
        text: content.1,
        unsubscribe_url,
        message_id: sender.new_message_id(),
    };
    match sender.send(&message).await {
        Ok(provider_id) => {
            deliveries::settle(&mut *txn, row_id, "sent", Some(&message.message_id), None).await?;
            if let Some(provider_id) = provider_id {
                deliveries::note_provider_id(&mut *txn, row_id, &provider_id).await?;
            }
            Ok(StepSend::Sent(message.message_id))
        }
        Err(error) => {
            deliveries::settle(&mut *txn, row_id, "failed", None, Some(&error.describe())).await?;
            if matches!(error, mail::SendError::Permanent(_)) {
                Ok(StepSend::Exit)
            } else {
                Ok(StepSend::Retry("send failed"))
            }
        }
    }
}

/// The touch activity for a nurture step that went out. `summary` is sealed: binding
/// `EncryptedText` seals it (CRM-T41).
pub(super) async fn record_step_touch(
    txn: &mut PgConnection,
    contact_pid: Uuid,
    step: &nurture_steps::Model,
    now: DateTimeWithTimeZone,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO activities (pid, subject_kind, subject_pid, kind, occurred_at, summary, \
         done) VALUES ($1, 'contact', $2, 'email', $3, $4, false)",
        Uuid::new_v4(),
        contact_pid,
        now,
        EncryptedText(format!(
            "Nurture step {} ({})",
            step.position, step.template_ref
        )) as _,
    )
    .execute(txn)
    .await?;
    Ok(())
}

/// End an enrolment early (consent withdrawn, mailbox suppressed, send refused).
pub(super) async fn exit_enrollment(
    row: &nurture_enrollments::Model,
    txn: &mut PgConnection,
) -> Result<()> {
    sqlx::query!(
        "UPDATE nurture_enrollments SET status = 'exited', next_due_at = NULL \
         WHERE id = $1 AND deleted_at IS NULL",
        row.id,
    )
    .execute(txn)
    .await?;
    Ok(())
}

/// Where an enrolment goes past the step just sent: the next step (due after its delay) or
/// `completed`, as `(current_step, next_due_at, status)`; and whether it completed.
pub(super) fn advance_enrollment(
    row: &nurture_enrollments::Model,
    steps: &[nurture_steps::Model],
    now: DateTimeWithTimeZone,
) -> ((i32, Option<DateTimeWithTimeZone>, &'static str), bool) {
    let next_index = row.current_step + 1;
    match steps.get(usize::try_from(next_index).unwrap_or(usize::MAX)) {
        Some(next_step) => {
            let next_due = now + chrono::Duration::hours(i64::from(next_step.delay_hours));
            ((next_index, Some(next_due), "active"), false)
        }
        None => ((next_index, None, "completed"), true),
    }
}

/// `POST /api/nurture/advance`: what one sweep did.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct NurtureSweep {
    sent: u64,
    completed: u64,
    /// Steps the mail sender could not send this time; they are tried again on a later sweep.
    held: u64,
}

/// What the send-time checks say of a due step.
enum Gate {
    /// No consent for the sequence's purpose: the enrolment ends (CRM-D6).
    Exit,
    /// An employee, by the worker service: the step waits, and goes on when the employment ends
    /// (CRM-R92), under the consent they still have.
    Hold,
    /// Send it.
    Go,
}

/// The send-time checks for a step of `sequence_pid` to `contact`: consent for the sequence's
/// purpose, then employment.
async fn send_gate(
    txn: &mut PgConnection,
    contact: &contacts::Model,
    sequence_pid: Uuid,
) -> Result<Gate> {
    // A deleted sequence keeps its purpose for an enrolment still under way, as before.
    let purpose = sqlx::query!(
        "SELECT purpose, deleted_at FROM nurture_sequences WHERE pid = $1",
        sequence_pid,
    )
    .fetch_optional(&mut *txn)
    .await?
    .map_or_else(|| tokens::DEFAULT_PURPOSE.to_string(), |q| q.purpose);
    if crate::consents::state_of(&mut *txn, contact, &purpose).await? != "granted" {
        return Ok(Gate::Exit);
    }
    if crate::models::employments::is_employee(&mut *txn, contact.pid).await? {
        return Ok(Gate::Hold);
    }
    Ok(Gate::Go)
}

/// `POST /api/nurture/advance` — the **idempotent advance sweep**
/// (CRM-R9, CRM-D8): for each active enrolment past its `next_due_at`,
/// simulate the step send (touch activity + `nurture_step_sent`),
/// advance to the next step or complete. Consent is re-checked at
/// send time; each (enrolment, step) sends at most once because the
/// row's `current_step` advances in the same transaction.
#[debug_handler]
pub(super) async fn advance_nurture(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let now: DateTimeWithTimeZone = crate::clock::now().into();
    let pool = db::pool(&ctx);
    let due = sqlx::query_scalar!(
        "SELECT pid FROM nurture_enrollments WHERE status = 'active' AND deleted_at IS NULL \
         AND next_due_at <= $1 ORDER BY id LIMIT 500",
        now,
    )
    .fetch_all(pool)
    .await?;
    let sender = mail::current();
    let mut sent = 0_u64;
    let mut completed = 0_u64;
    let mut held = 0_u64;
    for enrollment in due {
        match advance_one(pool, &sender, &caller, enrollment, now).await? {
            Step::Sent { finished } => {
                sent += 1;
                completed += u64::from(finished);
            }
            Step::Completed => completed += 1,
            Step::Held => held += 1,
            Step::Skipped => {}
        }
    }
    format::json(NurtureSweep {
        sent,
        completed,
        held,
    })
}

/// What the sweep did with one due enrolment.
enum Step {
    /// The step went out; `finished` when it was the last.
    Sent {
        /// Whether the enrolment completed with it.
        finished: bool,
    },
    /// No step was left, so the enrolment completed.
    Completed,
    /// The step waits: an employee, or a send to try again.
    Held,
    /// Nothing to count: another sweep had it, or the enrolment ended.
    Skipped,
}

/// Advance one due enrolment in its own transaction, as [`advance_nurture`] describes.
async fn advance_one(
    pool: &sqlx::PgPool,
    sender: &mail::Sender,
    caller: &MaybeAuthUser,
    enrollment: Uuid,
    now: DateTimeWithTimeZone,
) -> Result<Step> {
    let mut txn = pool.begin().await?;
    // Re-load + lock: the sweep is safe to run concurrently.
    let Some(row) = sqlx::query_as!(
        nurture_enrollments::Model,
        "SELECT created_at, updated_at, id, pid, sequence_pid, contact_pid, current_step, next_due_at \
        AS \"next_due_at: DateTimeWithTimeZone\", status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM nurture_enrollments WHERE pid = $1 AND status = 'active' AND \
        deleted_at IS NULL FOR UPDATE",
        enrollment,
    )
    .fetch_optional(&mut *txn)
    .await?
    else {
        txn.commit().await?;
        return Ok(Step::Skipped);
    };
    if row.next_due_at.is_none_or(|d| d > now) {
        txn.commit().await?;
        return Ok(Step::Skipped); // another sweep advanced it already
    }
    // Send-time consent re-check (CRM-D6).
    let contact = rows::contact(&mut *txn, row.contact_pid)
        .await?
        .ok_or(Error::NotFound)?;
    match send_gate(&mut txn, &contact, row.sequence_pid).await? {
        Gate::Go => {}
        Gate::Hold => {
            txn.commit().await?;
            return Ok(Step::Held);
        }
        Gate::Exit => {
            exit_enrollment(&row, &mut txn).await?;
            txn.commit().await?;
            return Ok(Step::Skipped);
        }
    }
    let steps = rows::steps_of(&mut *txn, row.sequence_pid).await?;
    let Some(step) = steps.get(usize::try_from(row.current_step).unwrap_or(usize::MAX)) else {
        sqlx::query!(
            "UPDATE nurture_enrollments SET status = 'completed', next_due_at = NULL \
             WHERE id = $1 AND deleted_at IS NULL",
            row.id,
        )
        .execute(&mut *txn)
        .await?;
        txn.commit().await?;
        return Ok(Step::Completed);
    };
    // The real sender mails the step first; the simulated one does not.
    let message_id = if sender.needs_address() {
        match send_step(&mut txn, sender, &contact, step).await? {
            StepSend::Sent(id) => Some(id),
            StepSend::Retry(why) => {
                txn.commit().await?;
                tracing::warn!(enrolment = %row.pid, why, "nurture step not sent; retry next sweep");
                return Ok(Step::Held);
            }
            StepSend::Exit => {
                exit_enrollment(&row, &mut txn).await?;
                txn.commit().await?;
                return Ok(Step::Skipped);
            }
        }
    } else {
        None
    };
    // The touch activity + event.
    record_step_touch(&mut txn, contact.pid, step, now).await?;
    streaming::emit_in(
        &mut txn,
        "nurture_enrollment",
        "nurture_step_sent",
        &row.pid.to_string(),
        &step.template_ref,
        caller.actor(),
        Some(serde_json::json!({ "step": step.position, "message_id": message_id })),
    )
    .await?;
    let ((current_step, next_due_at, status), finished) = advance_enrollment(&row, &steps, now);
    sqlx::query!(
        "UPDATE nurture_enrollments SET current_step = $2, next_due_at = $3, status = $4 \
         WHERE id = $1 AND deleted_at IS NULL",
        row.id,
        current_step,
        next_due_at,
        status,
    )
    .execute(&mut *txn)
    .await?;
    txn.commit().await?;
    Metrics::global().nurture_step_sent_total.inc();
    Ok(Step::Sent { finished })
}

/// A nurture sequence with its steps and how many enrolments are active.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct SequenceView {
    sequence: nurture_sequences::Model,
    steps: Vec<nurture_steps::Model>,
    active_enrollments: u64,
}

/// `GET /api/nurture-sequences` — sequences with steps + live
/// enrolment counts.
#[debug_handler]
pub(super) async fn list_sequences(State(ctx): State<AppContext>) -> Result<Response> {
    let pool = db::pool(&ctx);
    let rows = rows::sequences(pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for sequence in rows {
        let steps = rows::steps_of(pool, sequence.pid).await?;
        let active = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"n!\" FROM nurture_enrollments WHERE sequence_pid = $1 \
             AND status = 'active' AND deleted_at IS NULL",
            sequence.pid,
        )
        .fetch_one(pool)
        .await?;
        out.push(SequenceView {
            sequence,
            steps,
            active_enrollments: u64::try_from(active).unwrap_or(0),
        });
    }
    format::json(out)
}
