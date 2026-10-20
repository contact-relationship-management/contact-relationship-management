//! Consent: the send-path law (CRM-R6). Recording it, its history, and the shared checks.

use super::{
    ActiveModelTrait, ActiveValue, AppContext, Audit, ColumnTrait, Deserialize, EntityTrait, Json,
    MaybeAuthUser, Path, Problems, QueryFilter, QueryOrder, Response, Result, State,
    TransactionTrait, Uuid, accounts, consent_events, contacts, debug_handler, ensure_valid,
    format, nurture_enrollments, records, segment_rules, streaming, tokens,
};

/// `POST /api/contacts/{pid}/consent` body.
#[derive(Debug, Deserialize)]
pub(super) struct ConsentPayload {
    action: String,
    source: String,
}

/// The contact's segment facts (consent + status + channel + tier).
pub(super) async fn contact_facts<C: sea_orm::ConnectionTrait>(
    db: &C,
    contact: &contacts::Model,
) -> Result<segment_rules::ContactFacts> {
    let tier = if let Some(account_pid) = contact.account_pid {
        accounts::Entity::find()
            .filter(accounts::Column::Pid.eq(account_pid))
            .filter(accounts::Column::DeletedAt.is_null())
            .one(db)
            .await?
            .map(|a| a.tier)
    } else {
        None
    };
    Ok(segment_rules::ContactFacts {
        consent: contact.marketing_consent.clone(),
        status: contact.status.clone(),
        channel: contact.preferred_channel.as_str().to_string(),
        account_tier: tier,
    })
}

/// `POST /api/contacts/{pid}/consent` — record a consent change:
/// appends the `ConsentEvent`, flips the contact, and a withdrawal
/// exits every active nurture enrolment (CRM-R6, CRM-D6). Audited
/// (compliance evidence).
#[debug_handler]
pub(super) async fn record_consent(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ConsentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("action", tokens::CONSENT_ACTIONS, &payload.action);
    problems.require_text("source", &payload.source);
    ensure_valid(&problems.into_vec())?;
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    apply_consent(
        &ctx.db,
        contact,
        &payload.action,
        &payload.source,
        caller.actor(),
    )
    .await?;
    format::empty_json()
}

/// Record one consent change in a single transaction: append the
/// `ConsentEvent`, flip the contact, exit every active enrolment on a
/// withdrawal, audit, and emit. Shared by the authenticated endpoint and the
/// public unsubscribe link, so both leave the same evidence.
pub(super) async fn apply_consent(
    db: &sea_orm::DatabaseConnection,
    contact: contacts::Model,
    action: &str,
    source: &str,
    actor: Option<&str>,
) -> Result<()> {
    let txn = db.begin().await?;
    apply_consent_in(&txn, contact, action, source, actor).await?;
    txn.commit().await?;
    Ok(())
}

/// The body of [`apply_consent`], inside the caller's transaction.
pub(super) async fn apply_consent_in(
    txn: &sea_orm::DatabaseTransaction,
    contact: contacts::Model,
    action: &str,
    source: &str,
    actor: Option<&str>,
) -> Result<()> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    consent_events::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        contact_pid: ActiveValue::set(contact.pid),
        action: ActiveValue::set(action.to_string()),
        source: ActiveValue::set(source.to_string()),
        occurred_at: ActiveValue::set(now),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    let contact_pid = contact.pid;
    let mut active: contacts::ActiveModel = contact.into();
    active.marketing_consent = ActiveValue::set(action.to_string());
    active.consent_changed_at = ActiveValue::set(Some(now));
    active.update(txn).await?;
    if action == "withdrawn" {
        // Exit every active enrolment immediately.
        let enrollments = nurture_enrollments::Entity::find()
            .filter(nurture_enrollments::Column::ContactPid.eq(contact_pid))
            .filter(nurture_enrollments::Column::Status.eq("active"))
            .filter(nurture_enrollments::Column::DeletedAt.is_null())
            .all(txn)
            .await?;
        for enrollment in enrollments {
            let mut active: nurture_enrollments::ActiveModel = enrollment.into();
            active.status = ActiveValue::set("exited".to_string());
            active.next_due_at = ActiveValue::set(None);
            active.update(txn).await?;
        }
    }
    let kind = format!("consent_{action}");
    Audit::record(
        txn,
        "contact",
        contact_pid,
        &kind,
        actor,
        Some(serde_json::json!({ "source": source })),
    )
    .await?;
    streaming::emit_on(
        txn,
        "contact",
        &kind,
        &contact_pid.to_string(),
        "",
        actor,
        None,
    )
    .await?;
    Ok(())
}

/// `GET /api/contacts/{pid}/consent` — the append-only history
/// (audited read: compliance evidence, CRM-D7).
#[debug_handler]
pub(super) async fn consent_history(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = consent_events::Entity::find()
        .filter(consent_events::Column::ContactPid.eq(contact.pid))
        .order_by_asc(consent_events::Column::Id)
        .all(&ctx.db)
        .await?;
    Audit::record(
        &ctx.db,
        "contact",
        contact.pid,
        "consent_history_read",
        caller.actor(),
        None,
    )
    .await?;
    format::json(rows)
}
