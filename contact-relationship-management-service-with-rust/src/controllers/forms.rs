//! Web form lead capture (CRM-R31): `POST /forms/{slug}`.
//!
//! The form is configured (`CRM_WEB_FORMS`), the caller's `Origin` must be on its list, and
//! each form is rate limited per client and in all. The address goes to the person service
//! (the CRM stores none); the CRM keeps a contact for the returned person, a `source=web`
//! lead, and the consent purpose as `pending` until the visitor confirms by the link in the
//! one mail sent. A nurture sequence the form names is started on that confirmation, not
//! before. Every accepted or ignored submission gets the same answer.

use std::net::IpAddr;
use std::num::NonZeroU32;

use axa::client::ClientAddr;
use axa::extract::Path;
use axa::prelude::*;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::marketing::consent::apply_consent_in;
use super::sales::{NewLead, capture_lead};
use crate::db;
use crate::mail;
use crate::models::_entities::contacts;
use crate::models::audit_logs::Model as Audit;
use crate::models::encrypted::EncryptedText;
use crate::models::rows;
use crate::rate_limit::Settable;
use crate::rules::forms as rules;
use crate::rules::public_text::{self, Text};
use crate::rules::tokens;
use crate::settings::{self, WebForm};
use crate::validation::Problems;

/// How long a visitor has to confirm.
const CONFIRM_DAYS: i64 = 7;
/// Submissions per client per form, over ten minutes.
static PER_CLIENT: Settable = Settable::new(NonZeroU32::new(600).unwrap());
/// Submissions per form, over an hour.
static PER_FORM: Settable = Settable::new(NonZeroU32::new(3600).unwrap());

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
    /// The language of the page the form is on (a BCP 47 tag, CRM-T180): the confirmation mail
    /// and this answer are in it when it is one of the public pages' languages, else in the
    /// request's `Accept-Language`, else English. Used for this request only and never stored.
    #[serde(default)]
    language: Option<String>,
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

/// The form and the origin it was posted from, or the response that refuses the request (a
/// `403` in `text`'s language).
fn admit<'a>(
    forms: &'a [WebForm],
    slug: &str,
    headers: &HeaderMap,
    text: &Text,
) -> Result<(&'a WebForm, String), Box<Response>> {
    let Some(form) = forms.iter().find(|f| f.slug == slug) else {
        return Err(Box::new(StatusCode::NOT_FOUND.into_response()));
    };
    match header_text(headers, "origin") {
        Some(origin) if form.allows(origin) => Ok((form, origin.trim().to_string())),
        _ => Err(Box::new(refusal(
            StatusCode::FORBIDDEN,
            "forbidden",
            text.form_refused,
            text,
        ))),
    }
}

/// Mark an answer as in `text`'s language: `Content-Language`, `Vary: Accept-Language`, and for
/// a refusal the problem document's title (CRM-T182).
fn in_language(mut response: Response, text: &Text) -> Response {
    let title = match response.status() {
        StatusCode::UNPROCESSABLE_ENTITY => Some(text.form_invalid_title),
        StatusCode::FORBIDDEN => Some(text.form_refused_title),
        StatusCode::TOO_MANY_REQUESTS => Some(text.form_busy_title),
        StatusCode::SERVICE_UNAVAILABLE => Some(text.form_unavailable_title),
        _ => None,
    };
    if let Some(title) = title {
        response
            .extensions_mut()
            .insert(crate::problem::Title(title));
    }
    let headers = response.headers_mut();
    headers.insert(header::VARY, HeaderValue::from_static("accept-language"));
    if let Ok(lang) = HeaderValue::from_str(text.lang) {
        headers.insert(header::CONTENT_LANGUAGE, lang);
    }
    response
}

/// A refusal in `text`'s language. `code` is the problem document's `error`, the same in every
/// language, as it was before the answers were translated.
fn refusal(status: StatusCode, code: &str, message: &str, text: &Text) -> Response {
    in_language(
        (
            status,
            axum::Json(serde_json::json!({ "error": code, "description": message })),
        )
            .into_response(),
        text,
    )
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
    // A preflight has no body: its refusal follows `Accept-Language` alone.
    let text = public_text::negotiate(None, header_text(&headers, "accept-language"));
    match admit(&forms, &slug, &headers, text) {
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

/// The one answer to every submission accepted or ignored, in the request's language.
fn received(text: &Text) -> Response {
    in_language(
        (
            StatusCode::ACCEPTED,
            axum::Json(serde_json::json!({
                "status": "received",
                "message": text.form_received,
            })),
        )
            .into_response(),
        text,
    )
}

/// The language of the mail and the answer (CRM-T180): the form page's `language`, else the
/// request's `Accept-Language`, else English. The contact's language is not stored (CRM-R29):
/// the request that asks for the mail says it.
fn language_of(payload: &FormPayload, headers: &HeaderMap) -> &'static Text {
    let accept = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok());
    public_text::negotiate(payload.language.as_deref(), accept)
}

/// `429` with the seconds to wait, in `text`'s language.
fn too_many(retry_after: u64, text: &Text) -> Response {
    let mut response = refusal(
        StatusCode::TOO_MANY_REQUESTS,
        "too_many_requests",
        text.form_busy,
        text,
    );
    if let Ok(seconds) = HeaderValue::from_str(&retry_after.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, seconds);
    }
    response
}

/// Count this submission against the form's limits.
fn within_limits(form: &WebForm, address: Option<IpAddr>) -> Result<(), u64> {
    let current = settings::current();
    let client = crate::rate_limit::client_key(None, address);
    PER_CLIENT.check(current.web_form_client_limit, (form.slug.as_str(), client))?;
    PER_FORM.check(current.web_form_limit, form.slug.as_str())
}

/// `POST /forms/{slug}` — see the module documentation.
#[debug_handler]
async fn submit(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    ClientAddr(address): ClientAddr,
    headers: HeaderMap,
    crate::json::Json(payload): crate::json::Json<FormPayload>,
) -> Response {
    let forms = settings::current().web_forms.clone();
    // Every answer, refusals too, is in the request's language (CRM-T180, CRM-T182).
    let text = language_of(&payload, &headers);
    let (form, origin) = match admit(&forms, &slug, &headers, text) {
        Ok(found) => found,
        Err(refused) => return *refused,
    };
    if let Err(retry_after) = within_limits(form, address) {
        return with_cors(too_many(retry_after, text), &origin);
    }
    let response = match accept(&ctx, form, &payload, text).await {
        Ok(()) => received(text),
        // A `422` and a `503` are in `text`'s words; any other error (a `500`) is not, so it is
        // not marked as in that language.
        Err(error) => {
            let refused = error.into_response();
            match refused.status() {
                StatusCode::UNPROCESSABLE_ENTITY | StatusCode::SERVICE_UNAVAILABLE => {
                    in_language(refused, text)
                }
                _ => refused,
            }
        }
    };
    with_cors(response, &origin)
}

fn unavailable(text: &Text) -> Error {
    Error::CustomError(
        StatusCode::SERVICE_UNAVAILABLE,
        axa::error::ErrorDetail::new("unavailable", text.form_unavailable),
    )
}

/// Validate, then make or find the person, contact and lead, and ask for the confirmation.
async fn accept(
    ctx: &AppContext,
    form: &WebForm,
    payload: &FormPayload,
    text: &'static Text,
) -> Result<()> {
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
        problems.push_field("name", text.form_name_invalid);
    }
    if !rules::valid_email(payload.email.trim()) {
        problems.push_field("email", text.form_email_invalid);
    }
    problems.finish()?;
    let name = payload.name.trim();
    let email = payload.email.trim();
    let person_ref = crate::clients::resolve_person(name, email)
        .await
        .map_err(|reason| {
            tracing::warn!(form = %form.slug, %reason, "web form: person service failed");
            unavailable(text)
        })?;
    let actor = format!("web_form:{}", form.slug);
    let pool = db::pool(ctx);
    let mut txn = pool.begin().await?;
    let contact = find_or_create_contact(&mut txn, &person_ref, name, &actor).await?;
    let lead_pid = find_or_create_lead(&mut txn, &contact, name, email, &actor).await?;
    // The consent helpers are still on `SeaORM`; `Bridge` runs them in this transaction.
    let state = crate::consents::state_of(&mut *txn, &contact, &form.purpose).await?;
    if state != "pending" && state != "granted" {
        apply_consent_in(
            &mut txn,
            contact.clone(),
            &form.purpose,
            "pending",
            &actor,
            Some(&actor),
        )
        .await?;
    }
    Audit::record_on(
        &mut txn,
        "lead",
        lead_pid,
        "web_form_submitted",
        Some(&actor),
        Some(serde_json::json!({ "form": form.slug, "purpose": form.purpose })),
    )
    .await?;
    let ask = state != "granted"
        && !confirmation_sent_lately(&mut txn, contact.pid, &form.purpose).await?;
    let submission = record_submission(&mut txn, form, contact.pid, lead_pid, ask).await?;
    txn.commit().await?;
    if ask && !send_confirmation(pool, form, &contact, (name, email), text).await {
        clear_sent_mark(pool, submission).await;
    }
    Ok(())
}

async fn find_or_create_contact(
    txn: &mut PgConnection,
    person_ref: &str,
    name: &str,
    actor: &str,
) -> Result<contacts::Model> {
    if let Some(found) = rows::contact_by_person_ref(&mut *txn, person_ref).await? {
        return Ok(found);
    }
    // `preferred_channel` is sealed: binding `EncryptedText` seals it (CRM-T41).
    let row = sqlx::query_as!(
        contacts::Model,
        "INSERT INTO contacts (pid, person_ref, display_name, status, preferred_channel, \
            marketing_consent) VALUES ($1, $2, $3, 'active', $4, 'never') \
            RETURNING created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
            status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
            \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
            DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
            given_name, consent_jurisdiction",
        Uuid::new_v4(),
        person_ref,
        name,
        EncryptedText::from("email") as _,
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(txn, "contact", row.pid, "created", Some(actor), None).await?;
    crate::streaming::emit_in(
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
    txn: &mut PgConnection,
    contact: &contacts::Model,
    name: &str,
    email: &str,
    actor: &str,
) -> Result<Uuid> {
    let open = sqlx::query_scalar!(
        "SELECT pid FROM leads WHERE deleted_at IS NULL AND contact_pid = $1 AND source = 'web' \
         AND status IN ('new', 'contacted', 'qualified') ORDER BY id LIMIT 1",
        contact.pid,
    )
    .fetch_optional(&mut *txn)
    .await?;
    if let Some(lead) = open {
        return Ok(lead);
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

/// Whether a confirmation for this contact and purpose went out in the last hour.
async fn confirmation_sent_lately(
    txn: &mut PgConnection,
    contact: Uuid,
    purpose: &str,
) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM form_submissions WHERE contact_pid = $1 AND purpose = $2 \
         AND confirmation_sent_at > now() - interval '1 hour') AS \"sent!\"",
        contact,
        purpose,
    )
    .fetch_one(txn)
    .await?)
}

async fn record_submission(
    txn: &mut PgConnection,
    form: &WebForm,
    contact: Uuid,
    lead: Uuid,
    sending: bool,
) -> Result<Uuid> {
    let pid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO form_submissions \
         (pid, form, contact_pid, lead_pid, purpose, sequence_pid, confirmation_sent_at) \
         VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $7 THEN now() END)",
        pid,
        form.slug,
        contact,
        lead,
        form.purpose,
        form.sequence,
        sending,
    )
    .execute(txn)
    .await?;
    Ok(pid)
}

/// A send that failed does not count against the hourly limit.
async fn clear_sent_mark(pool: &PgPool, submission: Uuid) {
    if let Err(error) = sqlx::query!(
        "UPDATE form_submissions SET confirmation_sent_at = NULL WHERE pid = $1",
        submission,
    )
    .execute(pool)
    .await
    {
        tracing::warn!(%error, "could not clear the confirmation mark");
    }
}

/// Send the confirmation to the address in the request, in `text`'s language, with a link that
/// opens the confirmation page in the same language (`?lang=`; CRM-T180). It is not marketing
/// and carries no unsubscribe link. Returns whether it went out; a failure is logged and not
/// shown.
async fn send_confirmation(
    pool: &PgPool,
    form: &WebForm,
    contact: &contacts::Model,
    (name, email): (&str, &str),
    text: &Text,
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
    let expires = crate::clock::now() + chrono::Duration::days(CONFIRM_DAYS);
    let mut conn = match pool.acquire().await {
        Ok(conn) => conn,
        Err(error) => {
            tracing::warn!(form = %form.slug, %error, "web form: could not make a confirmation link");
            return false;
        }
    };
    let token = match crate::link_tokens::issue(
        &mut *conn,
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
    let link = format!("{}?lang={}", signer.link("confirm", &token), text.lang);
    let (subject, body) =
        public_text::confirmation_mail(text, name, &form.purpose, &link, CONFIRM_DAYS);
    let sender = mail::current();
    let message = mail::Message {
        to: email.to_string(),
        to_name: name.to_string(),
        subject,
        text: body,
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
    txn: &mut PgConnection,
    contact: Uuid,
    purpose: &str,
) -> Result<()> {
    let sequences = sqlx::query_scalar!(
        "SELECT DISTINCT sequence_pid AS \"sequence_pid!\" FROM form_submissions \
         WHERE contact_pid = $1 AND purpose = $2 AND confirmed_at IS NULL \
         AND sequence_pid IS NOT NULL",
        contact,
        purpose,
    )
    .fetch_all(&mut *txn)
    .await?;
    for sequence_pid in sequences {
        let Some(sequence) = rows::sequence(&mut *txn, sequence_pid).await? else {
            continue;
        };
        if sequence.purpose != purpose {
            continue;
        }
        let already = sqlx::query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM nurture_enrollments WHERE contact_pid = $1 \
             AND sequence_pid = $2 AND status = 'active' AND deleted_at IS NULL) AS \"found!\"",
            contact,
            sequence_pid,
        )
        .fetch_one(&mut *txn)
        .await?;
        if !already {
            super::marketing::nurture::enrol_in(&mut *txn, &sequence, contact, Some("web_form"))
                .await?;
        }
    }
    sqlx::query!(
        "UPDATE form_submissions SET confirmed_at = now() WHERE contact_pid = $1 \
         AND purpose = $2 AND confirmed_at IS NULL",
        contact,
        purpose,
    )
    .execute(&mut *txn)
    .await?;
    Ok(())
}
