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
        let text = message(&self.0.body_text());
        let mut body = serde_json::json!({ "description": text });
        if status == StatusCode::UNPROCESSABLE_ENTITY {
            body["error"] = "validation".into();
            body["errors"] = serde_json::json!([match member(&text) {
                Some(pointer) => serde_json::json!({ "pointer": pointer, "detail": text }),
                None => serde_json::json!({ "detail": text }),
            }]);
        }
        (status, axum::Json(body)).into_response()
    }
}

/// The JSON Pointer of the member a serde message is about. The message starts
/// with the path serde was at (`name: …`, `stages[0].name: …`, or `.: …` for
/// the whole body); a `missing field` message names a member of that path.
/// `None` when the message names no path.
fn member(text: &str) -> Option<String> {
    let (path, rest) = text.split_once(": ")?;
    let is_path = !path.is_empty()
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'[' | b']'));
    if !is_path {
        return None;
    }
    let mut pointer = String::new();
    if path != "." {
        for part in path.split('.').filter(|p| !p.is_empty()) {
            // `stages[0]` is the member `stages`, then entry `0`.
            let mut pieces = part.split('[');
            pointer.push('/');
            pointer.push_str(pieces.next().unwrap_or_default());
            for index in pieces {
                pointer.push('/');
                pointer.push_str(index.trim_end_matches(']'));
            }
        }
    }
    if let Some(name) = rest
        .strip_prefix("missing field `")
        .and_then(|r| r.split_once('`'))
        .map(|(name, _)| name)
    {
        pointer.push('/');
        pointer.push_str(name);
    }
    (!pointer.is_empty()).then_some(pointer)
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

#[cfg(test)]
mod tests {
    use super::member;

    #[test]
    fn the_path_serde_was_at_becomes_a_pointer() {
        assert_eq!(
            member("dispaly_name: unknown field `dispaly_name`, expected one of `a`").as_deref(),
            Some("/dispaly_name")
        );
        assert_eq!(
            member("stages[2].probability_percent: invalid type: string \"x\", expected i32")
                .as_deref(),
            Some("/stages/2/probability_percent")
        );
        assert_eq!(member("tags[0]: invalid type").as_deref(), Some("/tags/0"));
    }

    #[test]
    fn a_missing_field_is_a_member_of_the_path() {
        assert_eq!(
            member(".: missing field `display_name` at line 1 column 2").as_deref(),
            Some("/display_name")
        );
        assert_eq!(
            member("stages[1]: missing field `name`").as_deref(),
            Some("/stages/1/name")
        );
    }

    #[test]
    fn a_message_without_a_path_gives_no_pointer() {
        assert_eq!(member("expected value at line 1 column 1"), None);
        assert_eq!(member("EOF while parsing: at line 1"), None);
        assert_eq!(member(".: expected a map"), None);
    }
}
