//! The public routes outside `/api`: unsubscribe pages and the email webhooks.

use super::{
    AppContext, Audit, ColumnTrait, EntityTrait, IntoResponse, Path, QueryFilter, Response, Result,
    State, TransactionTrait, Uuid, apply_consent, apply_consent_in, contacts, debug_handler,
    deliveries, format, records, tokens,
};

use axum::extract::Query;

use crate::rules::public_text::{self, Text};

/// `?lang=` on a public page: the language the reader chose from the page's language links.
#[derive(Debug, Default, serde::Deserialize)]
pub(super) struct PageLanguage {
    lang: Option<String>,
}

/// The language of a public page (CRM-T175): the address's `?lang=`, else the reader's
/// `Accept-Language`, else English ([`public_text::negotiate`]).
pub(super) fn page_text(chosen: &PageLanguage, headers: &axum::http::HeaderMap) -> &'static Text {
    let accept = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok());
    public_text::negotiate(chosen.lang.as_deref(), accept)
}

/// The page's links to the same page in each other language.
fn language_links(text: &Text, path: &str) -> String {
    use std::fmt::Write as _;
    let mut links = String::new();
    for other in public_text::TEXTS.iter().filter(|t| t.lang != text.lang) {
        let _ = write!(
            links,
            " <a href=\"{path}?lang={lang}\" hreflang=\"{lang}\" lang=\"{lang}\">{name}</a>",
            lang = other.lang,
            name = other.endonym
        );
    }
    format!(
        "<footer><nav aria-label=\"{label}\">{label}:{links}</nav></footer>",
        label = text.other_languages
    )
}

/// A public page with the headers every one of them needs: nothing cached, no referrer, no
/// script, no styles, and any form may post only to this origin. It is in `text`'s language and
/// direction, with links to the page at `path` in the others.
pub(super) fn public_page(
    status: axum::http::StatusCode,
    text: &Text,
    path: &str,
    title: &str,
    body: &str,
) -> Response {
    use axum::http::{HeaderValue, header};
    use axum::response::IntoResponse;
    let html = format!(
        "<!doctype html><html lang=\"{lang}\" dir=\"{dir}\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <meta name=\"robots\" content=\"noindex\"><title>{title}</title></head>\
         <body><main>{body}</main>{links}</body></html>",
        lang = text.lang,
        dir = text.dir,
        links = language_links(text, path)
    );
    let mut response = (status, html).into_response();
    // Kept as HTML even when it is an error: a person reads it, not a client (CRM-T175).
    response.extensions_mut().insert(crate::problem::PublicPage);
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    // The answer depends on the reader's language.
    headers.insert(header::VARY, HeaderValue::from_static("Accept-Language"));
    if let Ok(lang) = HeaderValue::from_str(text.lang) {
        headers.insert(header::CONTENT_LANGUAGE, lang);
    }
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'none'; form-action 'self'"),
    );
    response
}

/// The unsubscribe pages: a heading and, when `form`, the button that posts to the page itself.
pub(super) fn unsubscribe_page(
    status: axum::http::StatusCode,
    text: &Text,
    token: &str,
    heading: &str,
    form: bool,
) -> Response {
    let path = format!("/unsubscribe/{token}");
    let form = if form {
        format!(
            "<form method=\"post\" action=\"{path}?lang={lang}\"><button type=\"submit\">\
             {button}</button></form>",
            lang = text.lang,
            button = text.unsubscribe_button
        )
    } else {
        String::new()
    };
    public_page(
        status,
        text,
        &path,
        text.unsubscribe_title,
        &format!("<h1>{heading}</h1>{form}"),
    )
}

/// Check a link's token. The same answer for a forged, malformed or
/// unconfigured link, so nothing is learned from the difference.
pub(super) fn link_contact(token: &str) -> Option<(Uuid, Uuid)> {
    crate::unsubscribe::current()?.verify(token)
}

/// `GET /unsubscribe/{token}` — the page a recipient reaches from the link.
/// It changes nothing, because mail scanners follow links; the button posts.
#[debug_handler]
pub(super) async fn unsubscribe_page_get(
    Path(token): Path<String>,
    Query(chosen): Query<PageLanguage>,
    headers: axum::http::HeaderMap,
) -> Response {
    let text = page_text(&chosen, &headers);
    match link_contact(&token) {
        Some(_) => unsubscribe_page(
            axum::http::StatusCode::OK,
            text,
            &token,
            text.unsubscribe_ask,
            true,
        ),
        None => unsubscribe_page(
            axum::http::StatusCode::NOT_FOUND,
            text,
            &token,
            text.link_not_valid,
            false,
        ),
    }
}

/// `POST /unsubscribe/{token}` — withdraw marketing consent. This is also the
/// RFC 8058 one-click endpoint (`List-Unsubscribe-Post`), so the body is not
/// read. Idempotent: a repeat records nothing more. Public by design: the
/// signed token is the credential.
#[debug_handler]
pub(super) async fn unsubscribe_post(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
    Query(chosen): Query<PageLanguage>,
    headers: axum::http::HeaderMap,
) -> Response {
    let text = page_text(&chosen, &headers);
    let Some((contact_pid, message_pid)) = link_contact(&token) else {
        return unsubscribe_page(
            axum::http::StatusCode::NOT_FOUND,
            text,
            &token,
            text.link_not_valid,
            false,
        );
    };
    let outcome: Result<()> = async {
        let contact = contacts::Entity::find()
            .filter(contacts::Column::Pid.eq(contact_pid))
            .filter(contacts::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await?;
        // The link withdraws the purpose of the message it came from, and no other. A
        // contact already erased or already withdrawn needs nothing more; either way the
        // person gets the same confirmation.
        if let Some(contact) = contact {
            let purpose = crate::consents::purpose_of_message(&ctx.db, message_pid).await?;
            if crate::consents::state_of(&ctx.db, &contact, &purpose).await? != "withdrawn" {
                apply_consent(
                    &ctx.db,
                    contact,
                    &purpose,
                    "withdrawn",
                    "unsubscribe-link",
                    None,
                )
                .await?;
            }
        }
        Ok(())
    }
    .await;
    match outcome {
        Ok(()) => unsubscribe_page(
            axum::http::StatusCode::OK,
            text,
            &token,
            text.unsubscribed,
            false,
        ),
        Err(error) => {
            tracing::error!(%error, "unsubscribe failed");
            unsubscribe_page(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                text,
                &token,
                text.went_wrong,
                false,
            )
        }
    }
}

/// `POST /webhooks/email` body: one provider outcome.
#[derive(Debug, serde::Deserialize)]
pub(super) struct EmailEvent {
    /// The `Message-ID` we assigned (with angle brackets).
    message_id: String,
    /// `delivered`, `bounce_hard`, `bounce_soft`, `complaint`, `opened` or `clicked`.
    event: String,
}

/// The events a provider may report.
const EVENTS: [&str; 6] = [
    "delivered",
    "bounce_hard",
    "bounce_soft",
    "complaint",
    "opened",
    "clicked",
];

/// `POST /webhooks/email` — an outcome from the mail provider. Authenticated by
/// an HMAC signature, not a bearer token (see [`crate::webhook`]). A hard
/// bounce suppresses the contact's mailbox; a complaint also withdraws
/// marketing consent; `delivered` is counted on the campaign. Each
/// (message, event) is applied once, so a replay changes nothing. Not found
/// when no secret is configured.
#[debug_handler]
pub(super) async fn email_webhook(
    State(ctx): State<AppContext>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response> {
    use axum::http::StatusCode;
    match crate::webhook::signed_request(&headers, &body) {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(false) => return Ok(StatusCode::UNAUTHORIZED.into_response()),
        Some(true) => {}
    }
    let Ok(event) = serde_json::from_slice::<EmailEvent>(&body) else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };
    if !EVENTS.contains(&event.event.as_str()) {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    let txn = ctx.db.begin().await?;
    let result = apply_event(&txn, &event.message_id, &event.event).await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "result": result }))
}

/// Whether an open or click on this message may be recorded: the contact's consent for the
/// campaign's purpose is still `granted`, and their mailbox is not suppressed. A message that is not
/// ours is let through so the caller reports it as unknown.
async fn engagement_allowed(txn: &sea_orm::DatabaseTransaction, message_id: &str) -> Result<bool> {
    use sea_orm::ConnectionTrait;
    let Some((contact_pid, campaign_pid)) = deliveries::find_message(txn, message_id).await? else {
        return Ok(true);
    };
    let Some(contact) = contacts::Entity::find()
        .filter(contacts::Column::Pid.eq(contact_pid))
        .filter(contacts::Column::DeletedAt.is_null())
        .one(txn)
        .await?
    else {
        return Ok(false);
    };
    let purpose = txn
        .query_one_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT purpose FROM campaigns WHERE pid = $1",
            [campaign_pid.into()],
        ))
        .await?
        .map_or_else(
            || Ok(crate::rules::tokens::DEFAULT_PURPOSE.to_string()),
            |row| row.try_get::<String>("", "purpose"),
        )?;
    Ok(
        crate::consents::state_of(txn, &contact, &purpose).await? == "granted"
            && deliveries::suppression(txn, contact_pid).await?.is_none(),
    )
}

/// Apply one provider outcome inside `txn`; returns what happened.
pub(super) async fn apply_event(
    txn: &sea_orm::DatabaseTransaction,
    message_id: &str,
    event_name: &str,
) -> Result<&'static str> {
    // Engagement is recorded only while the contact still consents to what the campaign was sent
    // for: a person who has withdrawn is not profiled by what they open or click (CRM-R58).
    if matches!(event_name, "opened" | "clicked") && !engagement_allowed(txn, message_id).await? {
        return Ok("ignored");
    }
    let effect = deliveries::record_event(txn, message_id, event_name).await?;
    Ok(match effect {
        deliveries::EventEffect::UnknownMessage => "unknown_message",
        deliveries::EventEffect::Duplicate => "duplicate",
        deliveries::EventEffect::Recorded {
            contact_pid,
            campaign_pid,
        } => {
            match event_name {
                "delivered" => deliveries::count_delivered(txn, campaign_pid).await?,
                "opened" | "clicked" => {
                    deliveries::count_engagement(txn, campaign_pid, event_name).await?;
                }
                "bounce_hard" | "complaint" => {
                    deliveries::suppress(txn, contact_pid, event_name).await?;
                    Audit::record(
                        txn,
                        "contact",
                        contact_pid,
                        "email_suppressed",
                        None,
                        Some(serde_json::json!({ "reason": event_name })),
                    )
                    .await?;
                }
                _ => {}
            }
            if event_name == "complaint" {
                let contact = contacts::Entity::find()
                    .filter(contacts::Column::Pid.eq(contact_pid))
                    .filter(contacts::Column::DeletedAt.is_null())
                    .one(txn)
                    .await?;
                // A spam complaint is a no to everything.
                if let Some(contact) = contact {
                    for purpose in tokens::PURPOSES {
                        if crate::consents::state_of(txn, &contact, purpose).await? != "withdrawn" {
                            apply_consent_in(
                                txn,
                                contact.clone(),
                                purpose,
                                "withdrawn",
                                "complaint",
                                None,
                            )
                            .await?;
                        }
                    }
                }
            }
            "recorded"
        }
    })
}

/// Whether a provider callback is authentic. Mailgun signs the body with its
/// own scheme; every other provider carries `CRM_WEBHOOK_SECRET` as a bearer
/// token, a basic-auth password or `X-CRM-Webhook-Token`. `None` when the
/// credential for this provider is not configured (the route then does not
/// exist).
pub(super) fn callback_authentic(
    kind: crate::provider::Kind,
    headers: &axum::http::HeaderMap,
    body: &[u8],
) -> Option<bool> {
    if kind == crate::provider::Kind::Mailgun {
        let key = crate::provider_api::mailgun_signing_key()?;
        return Some(crate::provider_api::mailgun_verified(
            &key,
            body,
            chrono::Utc::now().timestamp(),
        ));
    }
    let secret = crate::webhook::secret()?;
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let given = crate::webhook::presented(header("authorization"), header("x-crm-webhook-token"));
    Some(crate::webhook::token_ok(&secret, &given))
}

/// `POST /webhooks/email/{provider}` — a callback in a vendor's own format
/// (see [`crate::provider`]): Brevo, Mailjet, SES (through SNS), Azure (through
/// Event Grid), Mailgun or `SendGrid`. Replays are possible where the vendor
/// sends no timestamp, but harmless (each message and event applies once).
/// Answers `200` for events it ignores, so the vendor does not retry them.
#[debug_handler]
pub(super) async fn provider_webhook(
    State(ctx): State<AppContext>,
    axum::extract::Path(provider): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response> {
    use crate::provider::Inbound;
    use axum::http::StatusCode;
    let Some(kind) = crate::provider::Kind::from_name(&provider) else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let Some(authentic) = callback_authentic(kind, &headers, &body) else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let parsed = crate::provider::inbound(kind, &body);
    // Event Grid's handshake only echoes a code it was given, and may arrive
    // without our credential, so it is answered before the check.
    if let Some(Inbound::Respond(answer)) = &parsed
        && kind == crate::provider::Kind::Azure
    {
        return format::json(answer.clone());
    }
    if !authentic {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }
    let outcomes = match parsed {
        None => return Ok(StatusCode::BAD_REQUEST.into_response()),
        Some(Inbound::Respond(_)) => Vec::new(),
        Some(Inbound::Subscribe(url)) => {
            let confirmed = crate::provider_cloud::subscribe_url_ok(&url)
                && reqwest::get(&url)
                    .await
                    .is_ok_and(|r| r.status().is_success());
            return format::json(serde_json::json!({ "confirmed": confirmed }));
        }
        Some(Inbound::Outcomes(outcomes)) => outcomes,
    };
    let txn = ctx.db.begin().await?;
    let mut recorded = 0_u32;
    for outcome in &outcomes {
        if apply_event(&txn, &outcome.message_id, outcome.event).await? == "recorded" {
            recorded += 1;
        }
    }
    txn.commit().await?;
    format::json(serde_json::json!({ "received": outcomes.len(), "recorded": recorded }))
}

/// What the confirm and preference pages say for a link that does not work.
fn invalid_link(text: &Text, path: &str) -> Response {
    public_page(
        axum::http::StatusCode::NOT_FOUND,
        text,
        path,
        text.link_title,
        &format!("<h1>{}</h1>", text.link_expired),
    )
}

/// The purpose a confirmation token names, with the contact, **without spending it** (CRM-R79).
async fn confirm_target(
    db: &sea_orm::DatabaseConnection,
    token: &str,
) -> Option<(Uuid, &'static str)> {
    let target = crate::link_tokens::peek(db, crate::link_tokens::CONFIRM, token)
        .await
        .ok()??;
    Some((
        target.contact,
        tokens::PURPOSES.get(usize::from(target.detail))?,
    ))
}

/// The English words for a purpose (the web form's own answer, which is not yet translated).
pub(crate) fn purpose_label(purpose: &str) -> &'static str {
    public_text::english().purpose(purpose)
}

/// `GET /confirm/{token}` — the page a confirmation message links to. It changes nothing,
/// because mail scanners follow links; the button posts (CRM-R29).
#[debug_handler]
pub(super) async fn confirm_page_get(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
    Query(chosen): Query<PageLanguage>,
    headers: axum::http::HeaderMap,
) -> Response {
    let text = page_text(&chosen, &headers);
    let path = format!("/confirm/{token}");
    let Some((_, purpose)) = confirm_target(&ctx.db, &token).await else {
        return invalid_link(text, &path);
    };
    public_page(
        axum::http::StatusCode::OK,
        text,
        &path,
        text.confirm_title,
        &format!(
            "<h1>{heading}</h1><form method=\"post\" action=\"{path}?lang={lang}\">\
             <button type=\"submit\">{button}</button></form>",
            heading = text
                .confirm_heading
                .replace("{purpose}", text.purpose(purpose)),
            lang = text.lang,
            button = text.confirm_button
        ),
    )
}

/// `POST /confirm/{token}` — move `pending` to `granted`. Only a `pending` purpose can be
/// confirmed: if the person withdrew since, the link no longer grants anything. Idempotent
/// once granted.
#[debug_handler]
pub(super) async fn confirm_post(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
    Query(chosen): Query<PageLanguage>,
    headers: axum::http::HeaderMap,
) -> Response {
    use axum::http::StatusCode;
    let text = page_text(&chosen, &headers);
    let path = format!("/confirm/{token}");
    // The token is spent in the same transaction as the confirmation, so a failure leaves it
    // usable and a second request, even a simultaneous one, finds it spent (CRM-R79).
    let mut purpose = "";
    let outcome: Result<Option<bool>> = async {
        let txn = ctx.db.begin().await?;
        let Some(spent) =
            crate::link_tokens::consume(&txn, crate::link_tokens::CONFIRM, &token).await?
        else {
            return Ok(None);
        };
        let Some(named) = tokens::PURPOSES.get(usize::from(spent.detail)) else {
            return Ok(None);
        };
        purpose = named;
        let contact_pid = spent.contact;
        let Ok(contact) = records::find_contact(&txn, contact_pid).await else {
            return Ok(None);
        };
        let state = crate::consents::state_of(&txn, &contact, purpose).await?;
        let result = match state.as_str() {
            "granted" => Some(true),
            "pending" => {
                apply_consent_in(&txn, contact, purpose, "granted", "double_opt_in", None).await?;
                // A web form may have asked for a nurture sequence to start on confirmation (CRM-R31).
                crate::controllers::forms::on_confirmed(&txn, contact_pid, purpose).await?;
                Some(true)
            }
            _ => None,
        };
        txn.commit().await?;
        Ok(result)
    }
    .await;
    match outcome {
        Ok(Some(_)) => public_page(
            StatusCode::OK,
            text,
            &path,
            text.confirmed_title,
            &format!(
                "<h1>{}</h1>",
                text.confirmed.replace("{purpose}", text.purpose(purpose))
            ),
        ),
        Ok(None) => invalid_link(text, &path),
        Err(error) => {
            tracing::error!(%error, "confirmation failed");
            public_page(
                StatusCode::INTERNAL_SERVER_ERROR,
                text,
                &path,
                text.confirm_title,
                &format!("<h1>{}</h1>", text.went_wrong),
            )
        }
    }
}

/// The contact a preference link names, **without spending it** (CRM-R79).
async fn preferences_contact(db: &sea_orm::DatabaseConnection, token: &str) -> Option<Uuid> {
    crate::link_tokens::peek(db, crate::link_tokens::PREFERENCES, token)
        .await
        .ok()?
        .map(|target| target.contact)
}

/// The preference page: each purpose with the choice that is not the current one.
async fn preferences_html(
    db: &sea_orm::DatabaseConnection,
    text: &Text,
    contact_pid: Uuid,
    token: &str,
) -> Result<Response> {
    use std::fmt::Write as _;
    let contact = records::find_contact(db, contact_pid).await?;
    let mut rows = String::new();
    for entry in crate::consents::all(db, &contact).await? {
        let (action, label) = if entry.state == "granted" {
            ("withdrawn", text.stop)
        } else {
            ("granted", text.start)
        };
        let status = match entry.state.as_str() {
            "granted" => text.receiving,
            "pending" => text.pending,
            _ => text.not_receiving,
        };
        let _ = write!(
            rows,
            "<li><strong>{}</strong> {status} \
             <form method=\"post\" action=\"/preferences/{token}?lang={lang}\">\
             <input type=\"hidden\" name=\"purpose\" value=\"{}\">\
             <input type=\"hidden\" name=\"action\" value=\"{action}\">\
             <button type=\"submit\">{label}</button></form></li>",
            text.purpose(&entry.purpose),
            entry.purpose,
            lang = text.lang
        );
    }
    Ok(public_page(
        axum::http::StatusCode::OK,
        text,
        &format!("/preferences/{token}"),
        text.preferences_title,
        &format!("<h1>{}</h1><ul>{rows}</ul>", text.preferences_heading),
    ))
}

/// `GET /preferences/{token}` — what the person receives, with a button per purpose.
#[debug_handler]
pub(super) async fn preferences_page_get(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
    Query(chosen): Query<PageLanguage>,
    headers: axum::http::HeaderMap,
) -> Response {
    let text = page_text(&chosen, &headers);
    let path = format!("/preferences/{token}");
    let Some(contact) = preferences_contact(&ctx.db, &token).await else {
        return invalid_link(text, &path);
    };
    preferences_html(&ctx.db, text, contact, &token)
        .await
        .unwrap_or_else(|_| invalid_link(text, &path))
}

/// `POST /preferences/{token}` form.
#[derive(Debug, serde::Deserialize)]
pub(super) struct PreferenceChoice {
    purpose: String,
    action: String,
}

/// `POST /preferences/{token}` — grant or withdraw one purpose. An ordinary consent event
/// with the source `preference_page`.
#[debug_handler]
pub(super) async fn preferences_post(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
    Query(chosen): Query<PageLanguage>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(choice): axum::extract::Form<PreferenceChoice>,
) -> Response {
    let text = page_text(&chosen, &headers);
    let path = format!("/preferences/{token}");
    let not_available = || {
        public_page(
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            text,
            &path,
            text.preferences_title,
            &format!("<h1>{}</h1>", text.not_available),
        )
    };
    let Some(purpose) = tokens::PURPOSES.iter().find(|p| **p == choice.purpose) else {
        return not_available();
    };
    if !tokens::CONSENT_ACTIONS.contains(&choice.action.as_str()) {
        return not_available();
    }
    // A choice is made with a link that works once: it is spent here, with the change, and the
    // page that answers carries a fresh one, for the next choice, that expires when the first
    // would have (CRM-R79).
    let outcome: Result<Response> = async {
        let txn = ctx.db.begin().await?;
        let Some(spent) =
            crate::link_tokens::consume(&txn, crate::link_tokens::PREFERENCES, &token).await?
        else {
            return Ok(invalid_link(text, &path));
        };
        let contact = records::find_contact(&txn, spent.contact).await?;
        if crate::consents::state_of(&txn, &contact, purpose).await? != choice.action {
            apply_consent_in(
                &txn,
                contact,
                purpose,
                &choice.action,
                "preference_page",
                None,
            )
            .await?;
        }
        let next = crate::link_tokens::issue(
            &txn,
            crate::link_tokens::PREFERENCES,
            spent.contact,
            0,
            spent.expires_at,
        )
        .await?;
        txn.commit().await?;
        preferences_html(&ctx.db, text, spent.contact, &next).await
    }
    .await;
    outcome.unwrap_or_else(|error| {
        tracing::error!(%error, "preference change failed");
        invalid_link(text, &path)
    })
}
