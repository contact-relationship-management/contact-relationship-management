//! RFC 9457 problem details for every error response (CRM-T54).
//!
//! The service's errors are produced in several places: the controllers
//! (`{"error", "description"}` through axa), the auth and abuse middleware
//! (plain text), the API-version check, and the idempotency layer. One
//! outermost middleware turns every `4xx`/`5xx` into
//! `application/problem+json` so a client parses one shape.
//!
//! The members `error` (a stable machine code) and `description` (the human
//! message) are kept next to the standard `type`, `title`, `status`, `detail`
//! and `instance`, so a client written against the earlier body keeps working;
//! only the media type changed. Members a source adds (for example
//! `supported` on a refused API version) pass through unchanged.
//!
//! The document is [`axa::problem::Problem`]. What is this service's own is reading it from
//! whatever body a source sent, the `urn:crm:problem:` types, and the two earlier members.

use axa::problem::{PROBLEM_JSON, Problem};
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value, json};

/// Largest error body read for conversion; an error body is a sentence.
const MAX_ERROR_BODY: usize = 64 * 1024;

/// Machine code for a status that arrived without one (`not_found`, …).
fn code_for(status: StatusCode) -> String {
    status
        .canonical_reason()
        .unwrap_or("error")
        .to_ascii_lowercase()
        .replace(' ', "_")
}

/// Whether `text` is a `snake_case` machine code.
fn is_code(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// A problem-type URI: `about:blank` when the status says it all, otherwise a
/// stable URN for the machine code.
fn type_for(code: &str, default_code: &str) -> String {
    if code == default_code {
        "about:blank".to_string()
    } else {
        format!("urn:crm:problem:{}", code.replace('_', "-"))
    }
}

/// Build the problem document for `status` from whatever body the source sent.
#[must_use]
pub fn document(status: StatusCode, instance: &str, body: &[u8]) -> Problem {
    let default_code = code_for(status);
    let mut members = Map::new();
    let mut code = default_code.clone();
    let mut detail = None;
    if let Ok(Value::Object(source)) = serde_json::from_slice::<Value>(body) {
        for (key, value) in source {
            match (key.as_str(), &value) {
                // axa labels some statuses with a reason phrase ("Bad Request"
                // for a 413); only a snake_case code is a stable machine code.
                ("error", Value::String(text)) => {
                    if is_code(text) {
                        code.clone_from(text);
                    }
                }
                ("description", Value::String(text)) => detail = Some(text.clone()),
                // The standard members are ours to set.
                ("type" | "title" | "status" | "detail" | "instance", _) => {}
                _ => {
                    members.insert(key, value);
                }
            }
        }
    } else {
        let text = String::from_utf8_lossy(body).trim().to_string();
        if !text.is_empty() {
            detail = Some(text);
        }
    }
    let problem = Problem::new(status);
    let detail = detail.unwrap_or_else(|| problem.title.clone());
    let problem = problem
        .with_type(type_for(&code, &default_code))
        .with_detail(detail.clone())
        .with_instance(instance)
        .with_extension("error", json!(code))
        .with_extension("description", json!(detail));
    members.into_iter().fold(problem, |problem, (name, value)| {
        problem.with_extension(&name, value)
    })
}

/// Marks a response as a public HTML page for a person in a browser (the unsubscribe, confirm and
/// preference pages), which [`problem_mw`] leaves as it is (CRM-T175).
#[derive(Debug, Clone, Copy)]
pub struct PublicPage;

/// A problem's `title` in the request's language, set by a source that answers in it (the web
/// form, CRM-T182); without it the title is the status's English reason phrase. The `error`
/// code is never translated.
#[derive(Debug, Clone, Copy)]
pub struct Title(pub &'static str);

/// Rewrite every error response as a problem document, except a [`PublicPage`].
pub async fn problem_mw(req: Request, next: Next) -> Response {
    let instance = req.uri().path().to_string();
    let response = next.run(req).await;
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }
    // A public page (unsubscribe, confirm, preferences) answers a person in a browser, so its
    // HTML is kept: turned into JSON, the person would read a problem document (CRM-T175).
    if response.extensions().get::<PublicPage>().is_some() {
        return response;
    }
    let title = response.extensions().get::<Title>().copied();
    let (mut parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, MAX_ERROR_BODY)
        .await
        .unwrap_or_default();
    let mut problem = document(status, &instance, &bytes);
    if let Some(Title(title)) = title {
        problem = problem.with_title(title);
    }
    // Only the body is taken: the source's own headers, such as `Retry-After`, stay.
    let body = problem.into_response().into_body();
    parts
        .headers
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(PROBLEM_JSON));
    parts.headers.remove(header::CONTENT_LENGTH);
    if status == StatusCode::UNAUTHORIZED && !parts.headers.contains_key(header::WWW_AUTHENTICATE) {
        // RFC 9110 §15.5.2: a 401 names the scheme that would be accepted.
        parts
            .headers
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    Response::from_parts(parts, body)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::routing::get;
    use axum::{Router, middleware};
    use tower::ServiceExt;

    use super::*;

    /// The document as a client reads it.
    fn document(status: StatusCode, instance: &str, body: &[u8]) -> Value {
        serde_json::to_value(super::document(status, instance, body)).unwrap()
    }

    /// `route`'s answer after [`problem_mw`], as the application would send it.
    async fn through_the_middleware(route: Router) -> (Response, Value) {
        let response = route
            .layer(middleware::from_fn(problem_mw))
            .oneshot(Request::get("/x").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, MAX_ERROR_BODY).await.unwrap();
        // The source's length would cut the longer document short.
        if let Some(length) = parts.headers.get(header::CONTENT_LENGTH) {
            assert_eq!(length.to_str().unwrap(), bytes.len().to_string());
        }
        let value = serde_json::from_slice(&bytes).unwrap();
        (Response::from_parts(parts, Body::empty()), value)
    }

    #[tokio::test]
    async fn the_source_keeps_its_status_and_headers() {
        let route = Router::new().route(
            "/x",
            get(|| async {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(header::RETRY_AFTER, "7"), (header::CONTENT_LENGTH, "17")],
                    "too many requests",
                )
            }),
        );
        let (response, document) = through_the_middleware(route).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "7");
        assert_eq!(response.headers()[header::CONTENT_TYPE], PROBLEM_JSON);
        assert_eq!(document["detail"], "too many requests");
        assert_eq!(document["instance"], "/x");
    }

    #[tokio::test]
    async fn a_set_title_replaces_the_reason_phrase_and_a_401_names_its_scheme() {
        let route = Router::new().route(
            "/x",
            get(|| async {
                let mut response = StatusCode::UNAUTHORIZED.into_response();
                response.extensions_mut().insert(Title("Non autorisé"));
                response
            }),
        );
        let (response, document) = through_the_middleware(route).await;
        assert_eq!(document["title"], "Non autorisé");
        assert_eq!(document["error"], "unauthorized");
        assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
    }

    #[test]
    fn a_source_member_with_the_wrong_type_still_replaces_the_earlier_member() {
        let doc = document(StatusCode::CONFLICT, "/x", br#"{"error":{"code":7}}"#);
        assert_eq!(doc["error"], json!({ "code": 7 }));
        assert_eq!(doc["type"], "about:blank");
    }

    #[test]
    fn a_reason_phrase_is_not_taken_for_a_machine_code() {
        let doc = document(
            StatusCode::PAYLOAD_TOO_LARGE,
            "/api/x",
            br#"{"error":"Bad Request","description":"Payload Too Large"}"#,
        );
        assert_eq!(doc["error"], "payload_too_large");
        assert_eq!(doc["type"], "about:blank");
        assert_eq!(doc["detail"], "Payload Too Large");
    }

    #[test]
    fn an_axa_error_keeps_its_code_and_message() {
        let body = br#"{"error":"validation","description":"name is required"}"#;
        let doc = document(StatusCode::UNPROCESSABLE_ENTITY, "/api/contacts", body);
        assert_eq!(doc["type"], "urn:crm:problem:validation");
        assert_eq!(doc["title"], "Unprocessable Entity");
        assert_eq!(doc["status"], 422);
        assert_eq!(doc["detail"], "name is required");
        assert_eq!(doc["instance"], "/api/contacts");
        assert_eq!(doc["error"], "validation");
        assert_eq!(doc["description"], "name is required");
    }

    #[test]
    fn plain_text_becomes_the_detail_with_a_blank_type() {
        let doc = document(
            StatusCode::TOO_MANY_REQUESTS,
            "/api/deals",
            b"too many requests",
        );
        assert_eq!(doc["type"], "about:blank");
        assert_eq!(doc["detail"], "too many requests");
        assert_eq!(doc["error"], "too_many_requests");
    }

    #[test]
    fn an_empty_body_falls_back_to_the_reason_phrase() {
        let doc = document(StatusCode::NOT_FOUND, "/api/nope", b"");
        assert_eq!(doc["type"], "about:blank");
        assert_eq!(doc["detail"], "Not Found");
        assert_eq!(doc["error"], "not_found");
    }

    #[test]
    fn extra_members_pass_through_and_standard_ones_are_not_overridden() {
        let body = br#"{"error":"unsupported_api_version","requested":"9.9",
            "supported":["1.0"],"status":200,"title":"x"}"#;
        let doc = document(StatusCode::NOT_ACCEPTABLE, "/api/contacts", body);
        assert_eq!(doc["status"], 406);
        assert_eq!(doc["title"], "Not Acceptable");
        assert_eq!(doc["requested"], "9.9");
        assert_eq!(doc["supported"], json!(["1.0"]));
        assert_eq!(doc["type"], "urn:crm:problem:unsupported-api-version");
    }

    #[test]
    fn every_document_has_the_standard_and_legacy_members() {
        for status in [
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED,
            StatusCode::CONFLICT,
            StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            let doc = document(status, "/x", b"");
            for key in [
                "type",
                "title",
                "status",
                "detail",
                "instance",
                "error",
                "description",
            ] {
                assert!(doc.get(key).is_some(), "{status} lacks {key}");
            }
        }
    }
}
