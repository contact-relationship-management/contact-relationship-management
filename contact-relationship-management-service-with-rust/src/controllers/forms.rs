//! Web form lead capture (CRM-R31): `POST /forms/{slug}`.
//!
//! The form is configured (`CRM_WEB_FORMS`), the caller's `Origin` must be on its list, and
//! each form is rate limited per client and in all. The address goes to the person service
//! (the CRM stores none); the CRM keeps a contact for the returned person, a `source=web`
//! lead, and the consent purpose as `pending` until the visitor confirms by the link in the
//! one mail sent. A nurture sequence the form names is started on that confirmation, not
//! before. Every accepted or ignored submission gets the same answer.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use loco_rs::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction, QueryFilter, Statement,
    TransactionTrait, Value,
};
use serde::Deserialize;
use uuid::Uuid;

use super::marketing::consent::apply_consent_in;
use super::marketing::purpose_label;
use super::sales::{NewLead, capture_lead};
use crate::mail;
use crate::models::_entities::{contacts, leads};
use crate::models::audit_logs::Model as Audit;
use crate::models::live::FindLive;
use crate::models::records;
use crate::rules::forms::{self as rules, Counter};
use crate::rules::tokens;
use crate::settings::{self, WebForm};
use crate::validation::Problems;

/// How long a visitor has to confirm.
const CONFIRM_DAYS: i64 = 7;
/// The window the per-client limit counts over.
const CLIENT_SPAN: Duration = Duration::from_mins(10);
/// The window the per-form limit counts over.
const FORM_SPAN: Duration = Duration::from_hours(1);

/// The routes, outside `/api`.
pub fn routes() -> Routes {
    Routes::new()
        .add("/forms/{slug}", post(submit))
        .add("/forms/{slug}", axum::routing::options(preflight))
}

/// `POST /forms/{slug}` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FormPayload {
    name: String,
    email: String,
    /// A field real visitors never see; a bot fills it.
    #[serde(default)]
    website: Option<String>,
}

fn counter() -> &'static Mutex<Counter> {
    static COUNTER: OnceLock<Mutex<Counter>> = OnceLock::new();
    COUNTER.get_or_init(|| Mutex::new(Counter::default()))
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

/// The form and the origin it was posted from, or the response that refuses the request.
fn admit<'a>(
    forms: &'a [WebForm],
    slug: &str,
    headers: &HeaderMap,
) -> Result<(&'a WebForm, String), Box<Response>> {
    let Some(form) = forms.iter().find(|f| f.slug == slug) else {
        return Err(Box::new(StatusCode::NOT_FOUND.into_response()));
    };
    match header_text(headers, "origin") {
        Some(origin) if form.allows(origin) => Ok((form, origin.trim().to_string())),
        _ => Err(Box::new(
            (StatusCode::FORBIDDEN, "origin not allowed").into_response(),
        )),
    }
}

/// Let the browser read the answer for the one origin that posted.
fn with_cors(mut response: Response, origin: &str) -> Response {
    if let Ok(value) = HeaderValue::from_str(origin) {
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
        headers.append(header::VARY, HeaderValue::from_static("Origin"));
    }
    response
}

/// `OPTIONS /forms/{slug}` — the browser's preflight, answered for allowed origins only.
#[debug_handler]
async fn preflight(Path(slug): Path<String>, headers: HeaderMap) -> Response {
    let forms = settings::current().web_forms.clone();
    match admit(&forms, &slug, &headers) {
        Ok((_, origin)) => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            let out = response.headers_mut();
            out.insert(
                header::ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("POST"),
            );
            out.insert(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type"),
            );
            out.insert(
                header::ACCESS_CONTROL_MAX_AGE,
                HeaderValue::from_static("600"),
            );
            with_cors(response, &origin)
        }
        Err(refused) => *refused,
    }
}

/// The one answer to every submission that was not an error.
fn received() -> Response {
    (
        StatusCode::ACCEPTED,
        axum::Json(serde_json::json!({
            "status": "received",
            "message": "If the address is valid, a confirmation message is on its way."
        })),
    )
        .into_response()
}

/// `429` with the seconds to wait.
fn too_many(retry_after: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after.to_string())],
        "too many submissions",
    )
        .into_response()
}

/// Count this submission against the form's limits.
fn within_limits(form: &WebForm, headers: &HeaderMap) -> Result<(), u64> {
    let current = settings::current();
    let client = crate::rate_limit::client_key(
        None,
        header_text(headers, "x-forwarded-for"),
        current.trust_forwarded_for,
    );
    let now = Instant::now();
    let mut counts = counter()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    counts.hit(
        &format!("{}|{client}", form.slug),
        current.web_form_client_limit,
        CLIENT_SPAN,
        now,
    )?;
    counts.hit(&form.slug, current.web_form_limit, FORM_SPAN, now)
}

/// `POST /forms/{slug}` — see the module documentation.
#[debug_handler]
async fn submit(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    crate::json::Json(payload): crate::json::Json<FormPayload>,
) -> Response {
    let forms = settings::current().web_forms.clone();
    let (form, origin) = match admit(&forms, &slug, &headers) {
        Ok(found) => found,
        Err(refused) => return *refused,
    };
    if let Err(retry_after) = within_limits(form, &headers) {
        return with_cors(too_many(retry_after), &origin);
    }
    let response = match accept(&ctx, form, &payload).await {
        Ok(()) => received(),
        Err(error) => error.into_response(),
    };
    with_cors(response, &origin)
}

fn unavailable() -> Error {
    Error::CustomError(
        StatusCode::SERVICE_UNAVAILABLE,
        loco_rs::controller::ErrorDetail::new("unavailable", "please try again later"),
    )
}

/// Validate, then make or find the person, contact and lead, and ask for the confirmation.
async fn accept(ctx: &AppContext, form: &WebForm, payload: &FormPayload) -> Result<()> {
    if payload
        .website
        .as_deref()
        .is_some_and(|w| !w.trim().is_empty())
    {
        tracing::info!(form = %form.slug, "web form honeypot filled; nothing created");
        return Ok(());
    }
    let mut problems = Problems::new();
    if !rules::valid_name(&payload.name) {
        problems.push_field(
            "name",
            "is required, at most 200 characters, with no line breaks",
        );
    }
    if !rules::valid_email(payload.email.trim()) {
        problems.push_field("email", "is not a usable email address");
    }
    problems.finish()?;
    let name = payload.name.trim();
    let email = payload.email.trim();
    let person_ref = crate::clients::resolve_person(name, email)
        .await
        .map_err(|reason| {
            tracing::warn!(form = %form.slug, %reason, "web form: person service failed");
            unavailable()
        })?;
    let actor = format!("web_form:{}", form.slug);
    let txn = ctx.db.begin().await?;
    let contact = find_or_create_contact(&txn, &person_ref, name, &actor).await?;
    let lead_pid = find_or_create_lead(&txn, &contact, name, email, &actor).await?;
    let state = crate::consents::state_of(&txn, &contact, &form.purpose).await?;
    if state != "pending" && state != "granted" {
        apply_consent_in(
            &txn,
            contact.clone(),
            &form.purpose,
            "pending",
            &actor,
            Some(&actor),
        )
        .await?;
    }
    Audit::record(
        &txn,
        "lead",
        lead_pid,
        "web_form_submitted",
        Some(&actor),
        Some(serde_json::json!({ "form": form.slug, "purpose": form.purpose })),
    )
    .await?;
    let ask =
        state != "granted" && !confirmation_sent_lately(&txn, contact.pid, &form.purpose).await?;
    let submission = record_submission(&txn, form, contact.pid, lead_pid, ask).await?;
    txn.commit().await?;
    if ask && !send_confirmation(&ctx.db, form, &contact, name, email).await {
        clear_sent_mark(&ctx.db, submission).await;
    }
    Ok(())
}

async fn find_or_create_contact(
    txn: &DatabaseTransaction,
    person_ref: &str,
    name: &str,
    actor: &str,
) -> Result<contacts::Model> {
    if let Some(found) = contacts::Entity::find_live()
        .filter(contacts::Column::PersonRef.eq(person_ref))
        .one(txn)
        .await?
    {
        return Ok(found);
    }
    let row = contacts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(person_ref.to_string()),
        display_name: ActiveValue::set(name.to_string()),
        status: ActiveValue::set("active".to_string()),
        preferred_channel: ActiveValue::set("email".to_string().into()),
        marketing_consent: ActiveValue::set("never".to_string()),
        consent_changed_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Audit::record(txn, "contact", row.pid, "created", Some(actor), None).await?;
    crate::streaming::emit_on(
        txn,
        "contact",
        "created",
        &row.pid.to_string(),
        &row.display_name,
        Some(actor),
        None,
    )
    .await?;
    Ok(row)
}

/// The contact's open web lead, or a new one (not a second while one is open).
async fn find_or_create_lead(
    txn: &DatabaseTransaction,
    contact: &contacts::Model,
    name: &str,
    email: &str,
    actor: &str,
) -> Result<Uuid> {
    let open = leads::Entity::find_live()
        .filter(leads::Column::ContactPid.eq(contact.pid))
        .filter(leads::Column::Source.eq("web"))
        .filter(leads::Column::Status.is_in(["new", "contacted", "qualified"]))
        .one(txn)
        .await?;
    if let Some(lead) = open {
        return Ok(lead.pid);
    }
    // The lead holds the address's domain, which scoring uses, and no address.
    let (lead, _) = capture_lead(
        txn,
        &NewLead {
            source: "web",
            campaign_pid: None,
            contact_pid: Some(contact.pid),
            display_name: name,
            email: None,
            email_domain: rules::domain_of(email),
        },
        Some(actor),
    )
    .await?;
    Ok(lead.pid)
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// Whether a confirmation for this contact and purpose went out in the last hour.
async fn confirmation_sent_lately(
    txn: &DatabaseTransaction,
    contact: Uuid,
    purpose: &str,
) -> Result<bool> {
    Ok(txn
        .query_one_raw(stmt(
            "SELECT 1 AS one FROM form_submissions WHERE contact_pid = $1 AND purpose = $2 \
             AND confirmation_sent_at > now() - interval '1 hour' LIMIT 1",
            [contact.into(), purpose.into()],
        ))
        .await?
        .is_some())
}

async fn record_submission(
    txn: &DatabaseTransaction,
    form: &WebForm,
    contact: Uuid,
    lead: Uuid,
    sending: bool,
) -> Result<Uuid> {
    let pid = Uuid::new_v4();
    txn.execute_raw(stmt(
        "INSERT INTO form_submissions \
         (pid, form, contact_pid, lead_pid, purpose, sequence_pid, confirmation_sent_at) \
         VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $7 THEN now() END)",
        [
            pid.into(),
            form.slug.clone().into(),
            contact.into(),
            lead.into(),
            form.purpose.clone().into(),
            form.sequence.into(),
            sending.into(),
        ],
    ))
    .await?;
    Ok(pid)
}

/// A send that failed does not count against the hourly limit.
async fn clear_sent_mark(db: &sea_orm::DatabaseConnection, submission: Uuid) {
    if let Err(error) = db
        .execute_raw(stmt(
            "UPDATE form_submissions SET confirmation_sent_at = NULL WHERE pid = $1",
            [submission.into()],
        ))
        .await
    {
        tracing::warn!(%error, "could not clear the confirmation mark");
    }
}

/// Send the confirmation to the address in the request. It is not marketing and carries no
/// unsubscribe link. Returns whether it went out; a failure is logged and not shown.
async fn send_confirmation(
    db: &sea_orm::DatabaseConnection,
    form: &WebForm,
    contact: &contacts::Model,
    name: &str,
    email: &str,
) -> bool {
    let Some(signer) = crate::unsubscribe::current() else {
        tracing::warn!(form = %form.slug, "web form: no signed-link secret, so no confirmation");
        return false;
    };
    let Some(index) = tokens::PURPOSES
        .iter()
        .position(|p| *p == form.purpose)
        .and_then(|i| u8::try_from(i).ok())
    else {
        return false;
    };
    let expires = chrono::Utc::now() + chrono::Duration::days(CONFIRM_DAYS);
    let token = match crate::link_tokens::issue(
        db,
        crate::link_tokens::CONFIRM,
        contact.pid,
        index,
        expires,
    )
    .await
    {
        Ok(token) => token,
        Err(error) => {
            tracing::warn!(form = %form.slug, %error, "web form: could not make a confirmation link");
            return false;
        }
    };
    let link = signer.link("confirm", &token);
    let sender = mail::current();
    let message = mail::Message {
        to: email.to_string(),
        to_name: name.to_string(),
        subject: "Please confirm your subscription".to_string(),
        text: format!(
            "Hello {name},\n\n\
             Someone asked for this address to receive: {}.\n\
             To confirm, open this link and press the button:\n\n{link}\n\n\
             The link works for {CONFIRM_DAYS} days. If you did not ask for this, ignore this \
             message and nothing will be sent to you.\n",
            purpose_label(&form.purpose).to_lowercase()
        ),
        message_id: sender.new_message_id(),
        unsubscribe_url: None,
    };
    match sender.send(&message).await {
        Ok(_) => true,
        Err(error) => {
            tracing::warn!(form = %form.slug, error = %error.describe(), "web form: confirmation not sent");
            false
        }
    }
}

/// The visitor confirmed `purpose`: start the nurture sequences the forms they used asked for
/// and mark those submissions confirmed. Called inside the confirmation's transaction, after
/// the grant, so the consent gate the sequence sends under is already open.
pub(crate) async fn on_confirmed(
    txn: &DatabaseTransaction,
    contact: Uuid,
    purpose: &str,
) -> Result<()> {
    let rows = txn
        .query_all_raw(stmt(
            "SELECT DISTINCT sequence_pid FROM form_submissions WHERE contact_pid = $1 \
             AND purpose = $2 AND confirmed_at IS NULL AND sequence_pid IS NOT NULL",
            [contact.into(), purpose.into()],
        ))
        .await?;
    for row in rows {
        let sequence_pid: Uuid = row.try_get("", "sequence_pid")?;
        let Ok(sequence) = records::find_sequence(txn, sequence_pid).await else {
            continue;
        };
        if sequence.purpose != purpose {
            continue;
        }
        let already = txn
            .query_one_raw(stmt(
                "SELECT 1 AS one FROM nurture_enrollments WHERE contact_pid = $1 \
                 AND sequence_pid = $2 AND status = 'active' AND deleted_at IS NULL",
                [contact.into(), sequence_pid.into()],
            ))
            .await?;
        if already.is_none() {
            super::marketing::nurture::enrol_in(txn, &sequence, contact, Some("web_form")).await?;
        }
    }
    txn.execute_raw(stmt(
        "UPDATE form_submissions SET confirmed_at = now() WHERE contact_pid = $1 \
         AND purpose = $2 AND confirmed_at IS NULL",
        [contact.into(), purpose.into()],
    ))
    .await?;
    Ok(())
}
