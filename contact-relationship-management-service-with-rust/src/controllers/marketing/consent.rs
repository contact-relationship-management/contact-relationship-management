//! Consent: the send-path law (CRM-R6). Recording it, its history, and the shared checks.

use super::{
    AppContext, Audit, Deserialize, Error, MaybeAuthUser, Problems, Response, Result, State, Uuid,
    consent_events, contacts, debug_handler, format, segment_rules, streaming, tokens,
    unprocessable,
};
use sqlx::{PgConnection, PgPool};

use crate::controllers::PidPath;
use crate::db::{self, Bridge};
use crate::json::Json;
use crate::models::rows;

/// The live contact `pid`, or `404`.
async fn find_contact(pool: &PgPool, pid: Uuid) -> Result<contacts::Model> {
    rows::contact(pool, pid).await?.ok_or(Error::NotFound)
}

/// `POST /api/contacts/{pid}/consent` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsentPayload {
    action: String,
    source: String,
    /// `marketing` (the default), `newsletter`, `events` or `product_updates`.
    purpose: Option<String>,
}

/// `POST /api/contacts/{pid}/consent/request` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsentRequestPayload {
    /// What is being asked for.
    purpose: String,
    /// Where the request came from, kept with the event.
    source: String,
}

/// The contact's segment facts (consent for `purpose` + status + channel + tier).
pub(super) async fn contact_facts(
    conn: &mut PgConnection,
    contact: &contacts::Model,
    purpose: &str,
) -> Result<segment_rules::ContactFacts> {
    let tier = match contact.account_pid {
        Some(account_pid) => {
            sqlx::query_scalar!(
                "SELECT tier FROM accounts WHERE pid = $1 AND deleted_at IS NULL",
                account_pid,
            )
            .fetch_optional(&mut *conn)
            .await?
        }
        None => None,
    };
    // The consent, tag and employment helpers are still on `SeaORM`; `Bridge` runs them on `conn`.
    Ok(segment_rules::ContactFacts {
        consent: crate::consents::state_of(&Bridge::new(conn), contact, purpose).await?,
        status: contact.status.clone(),
        channel: contact.preferred_channel.as_str().to_string(),
        account_tier: tier,
        tags: crate::controllers::tags::names_of(&Bridge::new(conn), contact.pid).await?,
        employee: crate::models::employments::is_employee(&Bridge::new(conn), contact.pid).await?,
    })
}

/// `POST /api/contacts/{pid}/consent` — record a consent change for a purpose
/// (default `marketing`): appends the `ConsentEvent`, sets the state, and a
/// withdrawal exits every active enrolment in a sequence of that purpose (CRM-R6,
/// CRM-R29, CRM-D6). Audited (compliance evidence).
#[debug_handler]
pub(super) async fn record_consent(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ConsentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("action", tokens::CONSENT_ACTIONS, &payload.action);
    problems.require_text("source", &payload.source);
    problems.token_opt("purpose", tokens::PURPOSES, payload.purpose.as_deref());
    problems.finish()?;
    let pool = db::pool(&ctx);
    let contact = find_contact(pool, pid).await?;
    apply_consent(
        pool,
        contact,
        payload
            .purpose
            .as_deref()
            .unwrap_or(tokens::DEFAULT_PURPOSE),
        &payload.action,
        &payload.source,
        caller.actor(),
    )
    .await?;
    format::empty_json()
}

/// Record one consent change in a single transaction: append the
/// `ConsentEvent`, set the purpose's state, exit the enrolments of that purpose
/// on a withdrawal, audit, and emit. Shared by the authenticated endpoint, the
/// public unsubscribe link, the confirmation link and the preference page, so
/// all leave the same evidence.
pub(super) async fn apply_consent(
    pool: &PgPool,
    contact: contacts::Model,
    purpose: &str,
    action: &str,
    source: &str,
    actor: Option<&str>,
) -> Result<()> {
    let mut txn = pool.begin().await?;
    apply_consent_in(&mut txn, contact, purpose, action, source, actor).await?;
    txn.commit().await?;
    Ok(())
}

/// The body of [`apply_consent`], inside the caller's transaction. `action` is the
/// new state: `granted`, `withdrawn` or `pending`.
pub(crate) async fn apply_consent_in(
    txn: &mut PgConnection,
    contact: contacts::Model,
    purpose: &str,
    action: &str,
    source: &str,
    actor: Option<&str>,
) -> Result<()> {
    let now: crate::models::DateTimeWithTimeZone = crate::clock::now().into();
    sqlx::query!(
        "INSERT INTO consent_events (pid, contact_pid, action, source, purpose, occurred_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        Uuid::new_v4(),
        contact.pid,
        action,
        source,
        purpose,
        now,
    )
    .execute(&mut *txn)
    .await?;
    let contact_pid = contact.pid;
    if purpose == tokens::DEFAULT_PURPOSE {
        sqlx::query!(
            "UPDATE contacts SET marketing_consent = $2, consent_changed_at = $3 \
             WHERE id = $1 AND deleted_at IS NULL",
            contact.id,
            action,
            now,
        )
        .execute(&mut *txn)
        .await?;
    } else {
        // The consent helpers are still on `SeaORM`; `Bridge` runs them in this transaction.
        crate::consents::set_other(
            &Bridge::new(txn),
            contact_pid,
            purpose,
            action,
            now.to_utc(),
        )
        .await?;
    }
    if action == "withdrawn" {
        // Exit every active enrolment in a sequence sent under this purpose.
        crate::consents::exit_enrolments(&Bridge::new(txn), contact_pid, purpose).await?;
    }
    let kind = format!("consent_{action}");
    Audit::record_on(
        txn,
        "contact",
        contact_pid,
        &kind,
        actor,
        Some(serde_json::json!({ "source": source, "purpose": purpose })),
    )
    .await?;
    streaming::emit_in(
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

/// `GET /api/contacts/{pid}/consents`: where the contact stands on each purpose.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "ConsentPurposes")]
pub(crate) struct ConsentPurposes {
    purposes: Vec<crate::consents::PurposeState>,
}

/// `GET /api/contacts/{pid}/consents`.
#[debug_handler]
pub(super) async fn consent_purposes(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let contact = find_contact(pool, pid).await?;
    let mut conn = pool.acquire().await?;
    format::json(ConsentPurposes {
        purposes: crate::consents::all(&Bridge::new(&mut conn), &contact).await?,
    })
}

/// `POST /api/contacts/{pid}/consent/request` response.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "ConsentRequested")]
pub(crate) struct ConsentRequested {
    purpose: String,
    /// Always `pending`: nothing is sent until the person confirms.
    state: &'static str,
    /// The page to put in the confirmation message. It changes nothing until its button
    /// is pressed.
    confirm_url: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// How long a confirmation link works.
const CONFIRM_DAYS: i64 = 7;
/// How long a preference link works.
const PREFERENCES_DAYS: i64 = 90;

fn unsubscriber() -> Result<std::sync::Arc<crate::unsubscribe::Unsubscriber>> {
    crate::unsubscribe::current().ok_or_else(|| {
        unprocessable("CRM_UNSUBSCRIBE_SECRET and CRM_PUBLIC_URL are needed to make signed links")
    })
}

/// `POST /api/contacts/{pid}/consent/request` — start a double opt-in: the purpose
/// becomes `pending` (which never allows a send) and a confirmation link is made.
/// Sending the message that carries it is the caller's job (CRM-R29).
#[debug_handler]
pub(super) async fn request_consent(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<ConsentRequestPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("purpose", tokens::PURPOSES, &payload.purpose);
    problems.require_text("source", &payload.source);
    problems.finish()?;
    let signer = unsubscriber()?;
    let pool = db::pool(&ctx);
    let contact = find_contact(pool, pid).await?;
    let state = {
        let mut conn = pool.acquire().await?;
        crate::consents::state_of(&Bridge::new(&mut conn), &contact, &payload.purpose).await?
    };
    if state == "granted" {
        return Err(unprocessable("that purpose is already granted"));
    }
    let index = tokens::PURPOSES
        .iter()
        .position(|p| *p == payload.purpose)
        .and_then(|i| u8::try_from(i).ok())
        .ok_or_else(|| unprocessable("unknown purpose"))?;
    let expires_at = crate::clock::now() + chrono::Duration::days(CONFIRM_DAYS);
    apply_consent(
        pool,
        contact,
        &payload.purpose,
        "pending",
        &payload.source,
        caller.actor(),
    )
    .await?;
    let token =
        crate::link_tokens::issue(pool, crate::link_tokens::CONFIRM, pid, index, expires_at)
            .await?;
    format::json(ConsentRequested {
        purpose: payload.purpose,
        state: "pending",
        confirm_url: signer.link("confirm", &token),
        expires_at,
    })
}

/// `POST /api/contacts/{pid}/preferences-link` response.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "PreferencesLink")]
pub(crate) struct PreferencesLink {
    url: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// `POST /api/contacts/{pid}/preferences-link` — a signed link to the page where
/// the person grants or withdraws each purpose.
#[debug_handler]
pub(super) async fn preferences_link(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let signer = unsubscriber()?;
    let pool = db::pool(&ctx);
    let contact = find_contact(pool, pid).await?;
    let expires_at = crate::clock::now() + chrono::Duration::days(PREFERENCES_DAYS);
    let token = crate::link_tokens::issue(
        pool,
        crate::link_tokens::PREFERENCES,
        contact.pid,
        0,
        expires_at,
    )
    .await?;
    format::json(PreferencesLink {
        url: signer.link("preferences", &token),
        expires_at,
    })
}

/// `GET /api/contacts/{pid}/consent` — the append-only history
/// (audited read: compliance evidence, CRM-D7).
#[debug_handler]
pub(super) async fn consent_history(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let contact = find_contact(pool, pid).await?;
    let rows = sqlx::query_as!(
        consent_events::Model,
        "SELECT created_at, updated_at, id, pid, contact_pid, action, source, occurred_at, \
         purpose, channel FROM consent_events WHERE contact_pid = $1 ORDER BY id",
        contact.pid,
    )
    .fetch_all(pool)
    .await?;
    Audit::record_read_on(
        pool,
        "contact",
        contact.pid,
        "consent_history_read",
        caller.actor(),
        None,
    )
    .await?;
    format::json(rows)
}
