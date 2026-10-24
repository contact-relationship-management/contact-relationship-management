//! The public routes outside `/api`: unsubscribe pages and the email webhooks.

use super::{
    AppContext, Audit, ColumnTrait, EntityTrait, IntoResponse, Path, QueryFilter, Response, Result,
    State, TransactionTrait, Uuid, apply_consent, apply_consent_in, contacts, debug_handler,
    deliveries, format, records, tokens,
};

/// A public page with the headers every one of them needs: nothing cached, no referrer, no
/// script, no styles, and any form may post only to this origin.
pub(super) fn public_page(status: axum::http::StatusCode, title: &str, body: &str) -> Response {
    use axum::http::{HeaderValue, header};
    use axum::response::IntoResponse;
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <meta name=\"robots\" content=\"noindex\"><title>{title}</title></head>\
         <body><main>{body}</main></body></html>"
    );
    let mut response = (status, html).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
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

/// The unsubscribe pages: a heading and, optionally, the button that posts to `form`.
pub(super) fn unsubscribe_page(
    status: axum::http::StatusCode,
    heading: &str,
    form: Option<&str>,
) -> Response {
    let form = form.map_or_else(String::new, |action| {
        format!(
            "<form method=\"post\" action=\"{action}\"><button type=\"submit\">\
             Unsubscribe</button></form>"
        )
    });
    public_page(status, "Unsubscribe", &format!("<h1>{heading}</h1>{form}"))
}

/// Check a link's token. The same answer for a forged, malformed or
/// unconfigured link, so nothing is learned from the difference.
pub(super) fn link_contact(token: &str) -> Option<(Uuid, Uuid)> {
    crate::unsubscribe::current()?.verify(token)
}

/// `GET /unsubscribe/{token}` — the page a recipient reaches from the link.
/// It changes nothing, because mail scanners follow links; the button posts.
#[debug_handler]
pub(super) async fn unsubscribe_page_get(Path(token): Path<String>) -> Response {
    match link_contact(&token) {
        Some(_) => unsubscribe_page(
            axum::http::StatusCode::OK,
            "Stop receiving marketing email?",
            Some(&format!("/unsubscribe/{token}")),
        ),
        None => unsubscribe_page(
            axum::http::StatusCode::NOT_FOUND,
            "This link is not valid.",
            None,
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
) -> Response {
    let Some((contact_pid, message_pid)) = link_contact(&token) else {
        return unsubscribe_page(
            axum::http::StatusCode::NOT_FOUND,
            "This link is not valid.",
            None,
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
        Ok(()) => unsubscribe_page(axum::http::StatusCode::OK, "You are unsubscribed.", None),
        Err(error) => {
            tracing::error!(%error, "unsubscribe failed");
            unsubscribe_page(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong. Please try again.",
                None,
            )
        }
    }
}

/// `POST /webhooks/email` body: one provider outcome.
#[derive(Debug, serde::Deserialize)]
pub(super) struct EmailEvent {
    /// The `Message-ID` we assigned (with angle brackets).
    message_id: String,
    /// `delivered`, `bounce_hard`, `bounce_soft` or `complaint`.
    event: String,
}

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
    let Some(secret) = crate::webhook::secret() else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string()
    };
    let now = chrono::Utc::now().timestamp();
    if !crate::webhook::verify(
        &secret,
        &header("x-crm-timestamp"),
        &header("x-crm-signature"),
        &body,
        now,
    ) {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }
    let Ok(event) = serde_json::from_slice::<EmailEvent>(&body) else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };
    if !["delivered", "bounce_hard", "bounce_soft", "complaint"].contains(&event.event.as_str()) {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    let txn = ctx.db.begin().await?;
    let result = apply_event(&txn, &event.message_id, &event.event).await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "result": result }))
}

/// Apply one provider outcome inside `txn`; returns what happened.
pub(super) async fn apply_event(
    txn: &sea_orm::DatabaseTransaction,
    message_id: &str,
    event_name: &str,
) -> Result<&'static str> {
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
fn invalid_link() -> Response {
    public_page(
        axum::http::StatusCode::NOT_FOUND,
        "Link not valid",
        "<h1>This link is not valid or has expired.</h1>",
    )
}

/// The purpose a confirmation token names, with the contact.
fn confirm_target(token: &str) -> Option<(Uuid, &'static str)> {
    let (contact, index) =
        crate::unsubscribe::current()?.verify_confirm(token, chrono::Utc::now().timestamp())?;
    Some((contact, tokens::PURPOSES.get(usize::from(index))?))
}

/// The words for a purpose on the public pages.
fn purpose_label(purpose: &str) -> &'static str {
    match purpose {
        "marketing" => "Marketing messages",
        "newsletter" => "The newsletter",
        "events" => "Invitations to events",
        "product_updates" => "Product updates",
        _ => "Messages",
    }
}

/// `GET /confirm/{token}` — the page a confirmation message links to. It changes nothing,
/// because mail scanners follow links; the button posts (CRM-R29).
#[debug_handler]
pub(super) async fn confirm_page_get(Path(token): Path<String>) -> Response {
    let Some((_, purpose)) = confirm_target(&token) else {
        return invalid_link();
    };
    public_page(
        axum::http::StatusCode::OK,
        "Confirm",
        &format!(
            "<h1>Confirm: {}</h1><form method=\"post\" action=\"/confirm/{token}\">\
             <button type=\"submit\">Yes, send me these</button></form>",
            purpose_label(purpose)
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
) -> Response {
    use axum::http::StatusCode;
    let Some((contact_pid, purpose)) = confirm_target(&token) else {
        return invalid_link();
    };
    let outcome: Result<Option<bool>> = async {
        let txn = ctx.db.begin().await?;
        let Ok(contact) = records::find_contact(&txn, contact_pid).await else {
            return Ok(None);
        };
        let state = crate::consents::state_of(&txn, &contact, purpose).await?;
        let result = match state.as_str() {
            "granted" => Some(true),
            "pending" => {
                apply_consent_in(&txn, contact, purpose, "granted", "double_opt_in", None).await?;
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
            "Confirmed",
            &format!(
                "<h1>Thank you. You will receive: {}.</h1>",
                purpose_label(purpose)
            ),
        ),
        Ok(None) => invalid_link(),
        Err(error) => {
            tracing::error!(%error, "confirmation failed");
            public_page(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Confirm",
                "<h1>Something went wrong. Please try again.</h1>",
            )
        }
    }
}

fn preferences_contact(token: &str) -> Option<Uuid> {
    crate::unsubscribe::current()?.verify_preferences(token, chrono::Utc::now().timestamp())
}

/// The preference page: each purpose with the choice that is not the current one.
async fn preferences_html(
    db: &sea_orm::DatabaseConnection,
    contact_pid: Uuid,
    token: &str,
) -> Result<Response> {
    use std::fmt::Write as _;
    let contact = records::find_contact(db, contact_pid).await?;
    let mut rows = String::new();
    for entry in crate::consents::all(db, &contact).await? {
        let (action, label) = if entry.state == "granted" {
            ("withdrawn", "Stop")
        } else {
            ("granted", "Start")
        };
        let status = match entry.state.as_str() {
            "granted" => "You receive these.",
            "pending" => "Waiting for you to confirm.",
            _ => "You do not receive these.",
        };
        let _ = write!(
            rows,
            "<li><strong>{}</strong> {status} \
             <form method=\"post\" action=\"/preferences/{token}\">\
             <input type=\"hidden\" name=\"purpose\" value=\"{}\">\
             <input type=\"hidden\" name=\"action\" value=\"{action}\">\
             <button type=\"submit\">{label}</button></form></li>",
            purpose_label(&entry.purpose),
            entry.purpose
        );
    }
    Ok(public_page(
        axum::http::StatusCode::OK,
        "Your preferences",
        &format!("<h1>What you would like to receive</h1><ul>{rows}</ul>"),
    ))
}

/// `GET /preferences/{token}` — what the person receives, with a button per purpose.
#[debug_handler]
pub(super) async fn preferences_page_get(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Response {
    let Some(contact) = preferences_contact(&token) else {
        return invalid_link();
    };
    preferences_html(&ctx.db, contact, &token)
        .await
        .unwrap_or_else(|_| invalid_link())
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
    axum::extract::Form(choice): axum::extract::Form<PreferenceChoice>,
) -> Response {
    let Some(contact_pid) = preferences_contact(&token) else {
        return invalid_link();
    };
    let Some(purpose) = tokens::PURPOSES.iter().find(|p| **p == choice.purpose) else {
        return public_page(
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            "Your preferences",
            "<h1>That choice is not available.</h1>",
        );
    };
    if !tokens::CONSENT_ACTIONS.contains(&choice.action.as_str()) {
        return public_page(
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            "Your preferences",
            "<h1>That choice is not available.</h1>",
        );
    }
    let outcome: Result<Response> = async {
        let contact = records::find_contact(&ctx.db, contact_pid).await?;
        if crate::consents::state_of(&ctx.db, &contact, purpose).await? != choice.action {
            apply_consent(
                &ctx.db,
                contact,
                purpose,
                &choice.action,
                "preference_page",
                None,
            )
            .await?;
        }
        preferences_html(&ctx.db, contact_pid, &token).await
    }
    .await;
    outcome.unwrap_or_else(|error| {
        tracing::error!(%error, "preference change failed");
        invalid_link()
    })
}
