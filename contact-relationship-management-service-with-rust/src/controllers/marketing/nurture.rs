//! Nurture sequences: create, enrol, and the idempotent advance sweep (CRM-R9, CRM-D8).

use super::{
    ActiveModelTrait, ActiveValue, AppContext, Audit, ColumnTrait, Deserialize, EntityTrait,
    MaybeAuthUser, Metrics, Path, PidRef, Problems, QueryFilter, QueryOrder, QuerySelect, Response,
    Result, State, TransactionTrait, Uuid, activities, clients, contacts, debug_handler,
    deliveries, ensure_valid, format, mail, nurture_enrollments, nurture_sequences, nurture_steps,
    records, streaming, unprocessable, with_unsubscribe,
};
use crate::json::Json;

/// `POST /api/nurture-sequences` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SequencePayload {
    name: String,
    steps: Vec<StepPayload>,
}

/// One nurture step.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StepPayload {
    delay_hours: i32,
    template_ref: String,
    subject: Option<String>,
    body: Option<String>,
}

/// `POST /api/nurture-sequences/{pid}/enrollments` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EnrollPayload {
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
    if payload.steps.is_empty() || payload.steps.len() > 32 {
        problems.push("a sequence needs 1-32 steps".to_string());
    }
    for step in &payload.steps {
        problems.require_text("steps[].template_ref", &step.template_ref);
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
        && payload
            .steps
            .iter()
            .any(|s| s.body.as_deref().is_none_or(|b| b.trim().is_empty()))
    {
        problems.push("every step needs a body when real mail is on".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let sequence = nurture_sequences::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
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
            template_ref: ActiveValue::set(step.template_ref.clone()),
            subject: ActiveValue::set(step.subject.clone()),
            body: ActiveValue::set(step.body.clone()),
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
    Path(pid): Path<String>,
    Json(payload): Json<EnrollPayload>,
) -> Result<Response> {
    let sequence = records::find_sequence(&ctx.db, records::parse_pid(&pid)?).await?;
    let contact = records::find_contact(&ctx.db, payload.contact_pid).await?;
    if contact.marketing_consent != "granted" {
        return Err(unprocessable("contact has not granted marketing consent"));
    }
    let first_step = nurture_steps::Entity::find()
        .filter(nurture_steps::Column::SequencePid.eq(sequence.pid))
        .filter(nurture_steps::Column::DeletedAt.is_null())
        .order_by_asc(nurture_steps::Column::Position)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| unprocessable("sequence has no steps"))?;
    let due = chrono::Utc::now() + chrono::Duration::hours(i64::from(first_step.delay_hours));
    let txn = ctx.db.begin().await?;
    let row = nurture_enrollments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        sequence_pid: ActiveValue::set(sequence.pid),
        contact_pid: ActiveValue::set(contact.pid),
        current_step: ActiveValue::set(0),
        next_due_at: ActiveValue::set(Some(due.into())),
        status: ActiveValue::set("active".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "nurture_enrollment",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
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
    let Some(body) = step.body.as_deref().filter(|b| !b.trim().is_empty()) else {
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
    let Some(address) = clients::email_address(&contact.person_ref).await else {
        deliveries::settle(txn, row_id, "no_address", None, Some("no address on file")).await?;
        return Ok(StepSend::Retry("no address"));
    };
    // The unsubscribe token names the step where a campaign would be named.
    let unsubscribe_url = crate::unsubscribe::current().map(|u| u.url(contact.pid, step.pid));
    let message = mail::Message {
        to: address,
        to_name: contact.display_name.clone(),
        subject: step
            .subject
            .clone()
            .unwrap_or_else(|| step.template_ref.clone()),
        text: with_unsubscribe(body.to_string(), unsubscribe_url.as_deref()),
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
        if contact.marketing_consent != "granted" {
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
    format::json(serde_json::json!({ "sent": sent, "completed": completed, "held": held }))
}

/// `GET /api/nurture-sequences` — sequences with steps + live
/// enrolment counts.
#[debug_handler]
pub(super) async fn list_sequences(State(ctx): State<AppContext>) -> Result<Response> {
    use sea_orm::PaginatorTrait;
    let rows = nurture_sequences::Entity::find()
        .filter(nurture_sequences::Column::DeletedAt.is_null())
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
        out.push(serde_json::json!({
            "sequence": sequence, "steps": steps, "active_enrollments": active,
        }));
    }
    format::json(out)
}
