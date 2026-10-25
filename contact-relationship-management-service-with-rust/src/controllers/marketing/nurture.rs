//! Nurture sequences: create, enrol, and the idempotent advance sweep (CRM-R9, CRM-D8).

use super::{
    ActiveModelTrait, ActiveValue, AppContext, Audit, ColumnTrait, Deserialize, EntityTrait,
    MaybeAuthUser, Metrics, PidRef, Problems, QueryFilter, QueryOrder, QuerySelect, Response,
    Result, State, TransactionTrait, Uuid, activities, clients, contacts, debug_handler,
    deliveries, format, mail, nurture_enrollments, nurture_sequences, nurture_steps, records,
    streaming, tokens, unprocessable, with_unsubscribe,
};
use crate::controllers::PidPath;
use crate::controllers::templates;
use crate::json::Json;
use crate::models::live::FindLive;

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
            && !templates::exists(&ctx.db, template).await?
        {
            return Err(unprocessable(
                "steps[].template_pid is not an email template",
            ));
        }
    }
    let txn = ctx.db.begin().await?;
    let sequence = nurture_sequences::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        purpose: ActiveValue::set(
            payload
                .purpose
                .clone()
                .unwrap_or_else(|| tokens::DEFAULT_PURPOSE.to_string()),
        ),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for (position, step) in payload.steps.iter().enumerate() {
        nurture_steps::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            sequence_pid: ActiveValue::set(sequence.pid),
            position: ActiveValue::set(i32::try_from(position).unwrap_or(i32::MAX)),
            delay_hours: ActiveValue::set(step.delay_hours),
            template_ref: ActiveValue::set(if step.template_ref.trim().is_empty() {
                step.template_pid.map(|p| p.to_string()).unwrap_or_default()
            } else {
                step.template_ref.clone()
            }),
            subject: ActiveValue::set(step.subject.clone()),
            body: ActiveValue::set(step.body.clone()),
            template_pid: ActiveValue::set(step.template_pid),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    Audit::record(
        &txn,
        "nurture_sequence",
        sequence.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: sequence.pid.to_string(),
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
    let sequence = records::find_sequence(&ctx.db, pid).await?;
    let contact = records::find_contact(&ctx.db, payload.contact_pid).await?;
    if crate::consents::state_of(&ctx.db, &contact, &sequence.purpose).await? != "granted" {
        return Err(unprocessable(&format!(
            "contact has not granted {} consent",
            sequence.purpose
        )));
    }
    let txn = ctx.db.begin().await?;
    let pid = enrol_in(&txn, &sequence, contact.pid, caller.actor())
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
    txn: &sea_orm::DatabaseTransaction,
    sequence: &nurture_sequences::Model,
    contact: Uuid,
    actor: Option<&str>,
) -> Result<Option<Uuid>> {
    let Some(first_step) = nurture_steps::Entity::find()
        .filter(nurture_steps::Column::SequencePid.eq(sequence.pid))
        .filter(nurture_steps::Column::DeletedAt.is_null())
        .order_by_asc(nurture_steps::Column::Position)
        .one(txn)
        .await?
    else {
        return Ok(None);
    };
    let due = chrono::Utc::now() + chrono::Duration::hours(i64::from(first_step.delay_hours));
    let row = nurture_enrollments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        sequence_pid: ActiveValue::set(sequence.pid),
        contact_pid: ActiveValue::set(contact),
        current_step: ActiveValue::set(0),
        next_due_at: ActiveValue::set(Some(due.into())),
        status: ActiveValue::set("active".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Audit::record(txn, "nurture_enrollment", row.pid, "created", actor, None).await?;
    Ok(Some(row.pid))
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
    txn: &sea_orm::DatabaseTransaction,
    sender: &mail::Sender,
    contact: &contacts::Model,
    step: &nurture_steps::Model,
) -> Result<StepSend> {
    let (row_id, state, message_id) = deliveries::row_for(txn, step.pid, contact.pid).await?;
    if let (true, Some(id)) = (state == "sent", message_id) {
        return Ok(StepSend::Sent(id));
    }
    if deliveries::is_suppressed(txn, contact.pid).await? {
        deliveries::settle(txn, row_id, "suppressed", None, Some("mailbox suppressed")).await?;
        return Ok(StepSend::Exit);
    }
    let Some(address) = clients::email_address(&contact.person_ref).await else {
        deliveries::settle(txn, row_id, "no_address", None, Some("no address on file")).await?;
        return Ok(StepSend::Retry("no address"));
    };
    // The unsubscribe token names the step where a campaign would be named.
    let unsubscribe_url = crate::unsubscribe::current().map(|u| u.url(contact.pid, step.pid));
    let content = if let Some(template) = step.template_pid {
        let made = templates::content_for(txn, template, contact, unsubscribe_url.as_deref());
        let Some(content) = made.await? else {
            let why = "permanent: the email template is gone";
            deliveries::settle(txn, row_id, "failed", None, Some(why)).await?;
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
            txn,
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
            deliveries::settle(txn, row_id, "sent", Some(&message.message_id), None).await?;
            if let Some(provider_id) = provider_id {
                deliveries::note_provider_id(txn, row_id, &provider_id).await?;
            }
            Ok(StepSend::Sent(message.message_id))
        }
        Err(error) => {
            deliveries::settle(txn, row_id, "failed", None, Some(&error.describe())).await?;
            if matches!(error, mail::SendError::Permanent(_)) {
                Ok(StepSend::Exit)
            } else {
                Ok(StepSend::Retry("send failed"))
            }
        }
    }
}

/// The touch activity for a nurture step that went out.
pub(super) async fn record_step_touch(
    txn: &sea_orm::DatabaseTransaction,
    contact_pid: Uuid,
    step: &nurture_steps::Model,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<()> {
    activities::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set("contact".to_string()),
        subject_pid: ActiveValue::set(contact_pid),
        kind: ActiveValue::set("email".to_string()),
        occurred_at: ActiveValue::set(now),
        actor_ref: ActiveValue::set(None),
        summary: ActiveValue::set(
            format!("Nurture step {} ({})", step.position, step.template_ref).into(),
        ),
        due_on: ActiveValue::set(None),
        done: ActiveValue::set(false),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(())
}

/// End an enrolment early (consent withdrawn, mailbox suppressed, send refused).
pub(super) async fn exit_enrollment(
    row: nurture_enrollments::Model,
    txn: &sea_orm::DatabaseTransaction,
) -> Result<()> {
    let mut active: nurture_enrollments::ActiveModel = row.into();
    active.status = ActiveValue::set("exited".to_string());
    active.next_due_at = ActiveValue::set(None);
    active.update(txn).await?;
    Ok(())
}

/// Move an enrolment past the step just sent: to the next step (due after its
/// delay) or to `completed`. Returns the update and whether it completed.
pub(super) fn advance_enrollment(
    row: nurture_enrollments::Model,
    steps: &[nurture_steps::Model],
    now: chrono::DateTime<chrono::FixedOffset>,
) -> (nurture_enrollments::ActiveModel, bool) {
    let next_index = row.current_step + 1;
    let mut active: nurture_enrollments::ActiveModel = row.into();
    active.current_step = ActiveValue::set(next_index);
    if let Some(next_step) = steps.get(usize::try_from(next_index).unwrap_or(usize::MAX)) {
        let next_due = now + chrono::Duration::hours(i64::from(next_step.delay_hours));
        active.next_due_at = ActiveValue::set(Some(next_due));
        (active, false)
    } else {
        active.status = ActiveValue::set("completed".to_string());
        active.next_due_at = ActiveValue::set(None);
        (active, true)
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
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let due = nurture_enrollments::Entity::find()
        .filter(nurture_enrollments::Column::Status.eq("active"))
        .filter(nurture_enrollments::Column::DeletedAt.is_null())
        .filter(nurture_enrollments::Column::NextDueAt.lte(now))
        .limit(500)
        .all(&ctx.db)
        .await?;
    let sender = mail::current();
    let mut sent = 0_u64;
    let mut completed = 0_u64;
    let mut held = 0_u64;
    for enrollment in due {
        let txn = ctx.db.begin().await?;
        // Re-load + lock: the sweep is safe to run concurrently.
        let Some(row) = nurture_enrollments::Entity::find()
            .filter(nurture_enrollments::Column::Pid.eq(enrollment.pid))
            .filter(nurture_enrollments::Column::Status.eq("active"))
            .lock_exclusive()
            .one(&txn)
            .await?
        else {
            txn.commit().await?;
            continue;
        };
        if row.next_due_at.is_none_or(|d| d > now) {
            txn.commit().await?;
            continue; // another sweep advanced it already
        }
        // Send-time consent re-check (CRM-D6).
        let contact = records::find_contact(&txn, row.contact_pid).await?;
        let purpose = nurture_sequences::Entity::find()
            .filter(nurture_sequences::Column::Pid.eq(row.sequence_pid))
            .one(&txn)
            .await?
            .map_or_else(|| tokens::DEFAULT_PURPOSE.to_string(), |q| q.purpose);
        if crate::consents::state_of(&txn, &contact, &purpose).await? != "granted" {
            exit_enrollment(row, &txn).await?;
            txn.commit().await?;
            continue;
        }
        let steps = nurture_steps::Entity::find()
            .filter(nurture_steps::Column::SequencePid.eq(row.sequence_pid))
            .filter(nurture_steps::Column::DeletedAt.is_null())
            .order_by_asc(nurture_steps::Column::Position)
            .all(&txn)
            .await?;
        let Some(step) = steps.get(usize::try_from(row.current_step).unwrap_or(usize::MAX)) else {
            let mut active: nurture_enrollments::ActiveModel = row.into();
            active.status = ActiveValue::set("completed".to_string());
            active.next_due_at = ActiveValue::set(None);
            active.update(&txn).await?;
            txn.commit().await?;
            completed += 1;
            continue;
        };
        // The real sender mails the step first; the simulated one does not.
        let message_id = if sender.needs_address() {
            match send_step(&txn, &sender, &contact, step).await? {
                StepSend::Sent(id) => Some(id),
                StepSend::Retry(why) => {
                    txn.commit().await?;
                    tracing::warn!(enrolment = %row.pid, why, "nurture step not sent; retry next sweep");
                    held += 1;
                    continue;
                }
                StepSend::Exit => {
                    exit_enrollment(row, &txn).await?;
                    txn.commit().await?;
                    continue;
                }
            }
        } else {
            None
        };
        // The touch activity + event.
        record_step_touch(&txn, contact.pid, step, now).await?;
        streaming::emit_on(
            &txn,
            "nurture_enrollment",
            "nurture_step_sent",
            &row.pid.to_string(),
            &step.template_ref,
            caller.actor(),
            Some(serde_json::json!({ "step": step.position, "message_id": message_id })),
        )
        .await?;
        let (active, finished) = advance_enrollment(row, &steps, now);
        completed += u64::from(finished);
        active.update(&txn).await?;
        txn.commit().await?;
        sent += 1;
        Metrics::global().nurture_step_sent_total.inc();
    }
    format::json(NurtureSweep {
        sent,
        completed,
        held,
    })
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
    use sea_orm::PaginatorTrait;
    let rows = nurture_sequences::Entity::find_live()
        .order_by_asc(nurture_sequences::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for sequence in rows {
        let steps = nurture_steps::Entity::find()
            .filter(nurture_steps::Column::SequencePid.eq(sequence.pid))
            .filter(nurture_steps::Column::DeletedAt.is_null())
            .order_by_asc(nurture_steps::Column::Position)
            .all(&ctx.db)
            .await?;
        let active = nurture_enrollments::Entity::find()
            .filter(nurture_enrollments::Column::SequencePid.eq(sequence.pid))
            .filter(nurture_enrollments::Column::Status.eq("active"))
            .filter(nurture_enrollments::Column::DeletedAt.is_null())
            .count(&ctx.db)
            .await?;
        out.push(SequenceView {
            sequence,
            steps,
            active_enrollments: active,
        });
    }
    format::json(out)
}
