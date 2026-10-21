//! The request-body extractor. Loco's own `Json` throws the parser's message
//! away, so a body with a misspelt member got a bare "Unprocessable Entity".
//! This one keeps the same shape (`Json(payload)`, `Json(value)` as a
//! response) and says what was wrong with the body: which member is unknown,
//! which is missing, which has the wrong type. The names it quotes are the
//! API's own, already public in the `OpenAPI` document.

use axum::extract::{FromRequest, Request, rejection::JsonRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A JSON body, in or out.
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// The body is not what the endpoint takes.
#[derive(Debug)]
pub struct Rejected(JsonRejection);

impl IntoResponse for Rejected {
    fn into_response(self) -> Response {
        // `422` for a body that is JSON but not this shape; axum's own
        // `400` (bad syntax), `413` and `415` are kept.
        let status = self.0.status();
        // Only the `422` gets our own code; for the others the problem layer
        // derives it from the status (`payload_too_large`, …).
        let mut body = serde_json::json!({ "description": message(&self.0.body_text()) });
        if status == StatusCode::UNPROCESSABLE_ENTITY {
            body["error"] = "validation".into();
        }
        (status, axum::Json(body)).into_response()
    }
}

/// axum's text minus its prefix: `unknown field `x`, expected one of …`.
fn message(body_text: &str) -> String {
    body_text
        .split_once(": ")
        .map_or(body_text, |(_, rest)| rest)
        .to_string()
}

impl<S, T> FromRequest<S> for Json<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = Rejected;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        axum::Json::<T>::from_request(req, state)
            .await
            .map(|axum::Json(value)| Self(value))
            .map_err(Rejected)
    }
}
