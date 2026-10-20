//! The public routes outside `/api`: unsubscribe pages and the email webhooks.

use super::{
    AppContext, Audit, ColumnTrait, EntityTrait, IntoResponse, Path, QueryFilter, Response, Result,
    State, TransactionTrait, Uuid, apply_consent, apply_consent_in, contacts, debug_handler,
    deliveries, format,
};

/// The page headers for the public unsubscribe pages: nothing cached, no
/// referrer, no script, and the form may post only to this origin.
pub(super) fn unsubscribe_page(
    status: axum::http::StatusCode,
    heading: &str,
    form: Option<&str>,
) -> Response {
    use axum::http::{HeaderValue, header};
    use axum::response::IntoResponse;
    let form = form.map_or_else(String::new, |action| {
        format!(
            "<form method=\"post\" action=\"{action}\"><button type=\"submit\">\
             Unsubscribe</button></form>"
        )
    });
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <meta name=\"robots\" content=\"noindex\"><title>Unsubscribe</title></head>\
         <body><main><h1>{heading}</h1>{form}</main></body></html>"
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

/// Check a link's token. The same answer for a forged, malformed or
/// unconfigured link, so nothing is learned from the difference.
pub(super) fn link_contact(token: &str) -> Option<Uuid> {
    crate::unsubscribe::current()?
        .verify(token)
        .map(|(contact, _campaign)| contact)
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
    let Some(contact_pid) = link_contact(&token) else {
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
        // A contact already erased or already withdrawn needs nothing more;
        // either way the person gets the same confirmation.
        if let Some(contact) = contact.filter(|c| c.marketing_consent != "withdrawn") {
            apply_consent(&ctx.db, contact, "withdrawn", "unsubscribe-link", None).await?;
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
                if let Some(contact) = contact.filter(|c| c.marketing_consent != "withdrawn") {
                    apply_consent_in(txn, contact, "withdrawn", "complaint", None).await?;
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
