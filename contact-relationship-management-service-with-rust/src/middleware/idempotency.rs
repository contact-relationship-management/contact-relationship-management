//! `Idempotency-Key` replay for POSTs.

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{auth, idempotency_store, idempotency_store::Stored};

/// A JSON error from the idempotency layer.
fn idempotency_problem(status: axum::http::StatusCode, message: &str) -> Response {
    (
        status,
        axum::Json(serde_json::json!({ "error": "idempotency", "description": message })),
    )
        .into_response()
}

/// The store failed: refuse rather than run the request unprotected.
fn idempotency_unavailable(error: &sea_orm::DbErr) -> Response {
    tracing::error!(%error, "idempotency store failed");
    idempotency_problem(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "idempotency store unavailable",
    )
}

/// The answer to a request whose key an earlier request already holds: the
/// stored response, or the reason there is none yet.
fn answer_for_existing(existing: Option<&Stored>, fingerprint: &str) -> Response {
    use axum::http::{HeaderValue, StatusCode, header};
    let Some(stored) = existing else {
        // The first attempt failed and cleaned up between our two statements;
        // the client can retry at once.
        return idempotency_problem(
            StatusCode::CONFLICT,
            "the original request just finished without a stored result; retry",
        );
    };
    if stored.fingerprint != fingerprint {
        return idempotency_problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Idempotency-Key was already used with a different request",
        );
    }
    if stored.state != "done" {
        let mut response = idempotency_problem(
            StatusCode::CONFLICT,
            "the original request is still being processed; retry shortly",
        );
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
        return response;
    }
    let mut response = Response::new(axum::body::Body::from(stored.body.clone()));
    *response.status_mut() = u16::try_from(stored.status)
        .ok()
        .and_then(|s| StatusCode::from_u16(s).ok())
        .unwrap_or(StatusCode::OK);
    if let Some(value) = stored
        .content_type
        .as_deref()
        .and_then(|v| HeaderValue::from_str(v).ok())
    {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    response
        .headers_mut()
        .insert("idempotent-replayed", HeaderValue::from_static("true"));
    response
}

/// Idempotency keys (CRM-T52); see [`crate::idempotency`] for the contract.
/// Runs inside auth, so only authenticated callers reach it and the key is
/// scoped to the caller. Fails closed: a caller who asked for idempotency
/// never gets an unprotected execution because the store was unavailable.
pub(crate) async fn idempotency_mw(
    axum::extract::State(db): axum::extract::State<sea_orm::DatabaseConnection>,
    req: Request,
    next: Next,
) -> Response {
    use axum::http::{StatusCode, header};
    let wants = req.method() == axum::http::Method::POST
        && req.uri().path().starts_with("/api/")
        && req.headers().contains_key(crate::idempotency::HEADER);
    if !wants {
        return next.run(req).await;
    }
    let key = req
        .headers()
        .get(crate::idempotency::HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if let Err(message) = crate::idempotency::validate_key(&key) {
        return idempotency_problem(StatusCode::UNPROCESSABLE_ENTITY, &message);
    }
    let scope = auth::request_claims(req.headers())
        .await
        .map_or_else(|_| "anon".to_string(), |c| c.sub);
    let (parts, body) = req.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, crate::idempotency::MAX_BODY).await else {
        return idempotency_problem(StatusCode::PAYLOAD_TOO_LARGE, "request body too large");
    };
    let fingerprint = crate::idempotency::fingerprint(
        parts.method.as_str(),
        parts.uri.path(),
        parts.uri.query().unwrap_or(""),
        &bytes,
    );

    match idempotency_store::claim(&db, &scope, &key, &fingerprint).await {
        Ok(true) => {}
        Ok(false) => {
            return match idempotency_store::lookup(&db, &scope, &key).await {
                Ok(existing) => answer_for_existing(existing.as_ref(), &fingerprint),
                Err(error) => idempotency_unavailable(&error),
            };
        }
        Err(error) => return idempotency_unavailable(&error),
    }

    // We own the key: run the request, then record or forget the outcome.
    let response = next
        .run(Request::from_parts(parts, axum::body::Body::from(bytes)))
        .await;
    let (head, body) = response.into_parts();
    let body = axum::body::to_bytes(body, crate::idempotency::MAX_BODY)
        .await
        .ok();
    let text = body
        .as_ref()
        .and_then(|b| String::from_utf8(b.to_vec()).ok())
        .filter(|_| crate::idempotency::remember(head.status.as_u16()));
    let result = if let Some(text) = text {
        let content_type = head
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        idempotency_store::record(&db, &scope, &key, head.status.as_u16(), content_type, &text)
            .await
    } else {
        // Failed, oversized or not text: forget the key so a retry runs again.
        idempotency_store::forget(&db, &scope, &key).await
    };
    if let Err(error) = result {
        // The work is done but could not be recorded: a retry would repeat it.
        tracing::error!(%error, "idempotency result not recorded");
    }
    match body {
        Some(bytes) => Response::from_parts(head, axum::body::Body::from(bytes)),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::answer_for_existing;
    use crate::idempotency_store::Stored;
    use axum::http::StatusCode;

    fn stored(state: &str, fingerprint: &str) -> Stored {
        Stored {
            fingerprint: fingerprint.to_string(),
            state: state.to_string(),
            status: 201,
            content_type: Some("application/json".to_string()),
            body: "{\"id\":1}".to_string(),
        }
    }

    #[test]
    fn a_finished_request_is_replayed() {
        let response = answer_for_existing(Some(&stored("done", "f")), "f");
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["idempotent-replayed"], "true");
        assert_eq!(response.headers()["content-type"], "application/json");
    }

    #[test]
    fn a_different_request_under_the_same_key_is_refused() {
        let response = answer_for_existing(Some(&stored("done", "f")), "other");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn a_request_still_running_says_retry() {
        let response = answer_for_existing(Some(&stored("pending", "f")), "f");
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(response.headers()["retry-after"], "1");
    }

    #[test]
    fn a_vanished_original_says_retry_at_once() {
        assert_eq!(
            answer_for_existing(None, "f").status(),
            StatusCode::CONFLICT
        );
    }
}
