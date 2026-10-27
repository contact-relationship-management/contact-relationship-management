//! Axum controllers (loco `Routes`), one module per CRM module, plus
//! the shared error helpers.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;

use crate::validation::FieldProblem;

pub mod attachments;
pub mod attribution;
pub mod audits;
pub mod board;
pub mod break_glass;
pub mod calendars;
pub mod custom_fields;
pub mod dashboards;
pub mod docs;
pub mod engagement;
pub mod external_links;
pub mod forms;
pub mod fx_rates;
pub mod insights;
pub mod kb;
pub mod lead_imports;
pub mod marketing;
pub mod merge;
pub mod metrics;
pub mod notifications;
pub mod paging;
pub mod privacy;
pub mod quotas;
pub mod record_imports;
pub mod relationships;
pub mod reports;
pub mod sales;
pub mod search;
pub mod support;
pub mod tags;
pub mod templates;
pub mod unmerge;
pub mod views;
pub mod webhooks;
pub mod workflows;

/// A `{pid}` reference response: the record a `POST` created.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "Created")]
pub(crate) struct PidRef {
    #[schemars(with = "uuid::Uuid")]
    pub(crate) pid: String,
}

/// A state change: the status or stage to move to (`POST .../status`, `.../stage`).
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StatusPayload {
    pub(crate) to: String,
}

/// The single `{pid}` path parameter, parsed as a UUID: a value that is not
/// one is a `404`, like any other record that is not there. Replaces
/// `Path<String>` plus `records::parse_pid` in a handler.
#[derive(Clone, Copy, Debug)]
pub struct PidPath(pub uuid::Uuid);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for PidPath {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self> {
        let Path(raw) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| Error::NotFound)?;
        crate::models::records::parse_pid(&raw).map(Self)
    }
}

/// Map a non-empty problem list to the family's `422 Unprocessable
/// Entity` validation error (every problem in one response).
///
/// # Errors
///
/// Returns the `422` error when `problems` is non-empty.
pub fn ensure_valid(problems: &[String]) -> Result<()> {
    if problems.is_empty() {
        return Ok(());
    }
    Err(validation_error(
        &problems
            .iter()
            .map(|detail| FieldProblem {
                pointer: None,
                detail: detail.clone(),
            })
            .collect::<Vec<_>>(),
    ))
}

/// The `422` validation error for one problem (what `ensure_valid` returns
/// for a list of one).
#[must_use]
pub fn invalid(problem: &str) -> Error {
    validation_error(&[FieldProblem {
        pointer: None,
        detail: problem.to_string(),
    }])
}

/// A `422` for one problem about one request member (`field`, as the API
/// names it).
#[must_use]
pub fn invalid_field(field: &str, problem: &str) -> Error {
    validation_error(&[FieldProblem {
        pointer: Some(crate::validation::pointer(field)),
        detail: problem.to_string(),
    }])
}

/// The `422` for a list of problems: `description` joins the messages and
/// `errors` lists each with the member it is about.
#[must_use]
pub fn validation_error(problems: &[FieldProblem]) -> Error {
    let joined = problems
        .iter()
        .map(|p| p.detail.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    let mut detail = ErrorDetail::new("validation", joined);
    detail.errors = Some(serde_json::Value::Array(
        problems
            .iter()
            .map(|p| match &p.pointer {
                Some(pointer) => serde_json::json!({ "pointer": pointer, "detail": p.detail }),
                None => serde_json::json!({ "detail": p.detail }),
            })
            .collect(),
    ));
    Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, detail)
}

/// A `422` with a single message (state-machine refusals etc.).
#[must_use]
pub fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

/// Map a record-level authorization rejection (`(status, reason)`) to a
/// loco error response: `403` = policy denied; `401` = fail-safe when
/// claims are missing behind the guard.
#[must_use]
pub fn record_rejection((status, reason): (StatusCode, String)) -> Error {
    let code = if status == StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    Error::CustomError(status, ErrorDetail::new(code, &reason))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use axum_test::TestServer;

    async fn echo(PidPath(pid): PidPath) -> String {
        pid.to_string()
    }

    #[tokio::test]
    async fn pid_path_parses_a_uuid_and_answers_404_for_anything_else() {
        let server = TestServer::new(Router::new().route("/things/{pid}", get(echo)));
        let pid = uuid::Uuid::new_v4();
        let ok = server.get(&format!("/things/{pid}")).await;
        ok.assert_status_ok();
        ok.assert_text(pid.to_string());
        for bad in ["nope", "123", "00000000-0000-0000-0000-00000000000g"] {
            server
                .get(&format!("/things/{bad}"))
                .await
                .assert_status_not_found();
        }
    }
}
