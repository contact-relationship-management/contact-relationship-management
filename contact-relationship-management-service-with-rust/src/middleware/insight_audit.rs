//! Insight read audit.

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::auth;

/// Insight read audit (CRM-T47 remainder): every successful `GET
/// /api/insights/*` writes a `insight_read` entry naming the view and the
/// caller. Fails closed — an unaudited disclosure is worse than an error.
pub(crate) async fn insight_audit_mw(
    axum::extract::State(db): axum::extract::State<sea_orm::DatabaseConnection>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let is_read = req.method() == axum::http::Method::GET && path.starts_with("/api/insights/");
    let actor = if is_read {
        auth::request_claims(req.headers())
            .await
            .ok()
            .map(|c| c.sub)
    } else {
        None
    };
    let response = next.run(req).await;
    if !is_read || !response.status().is_success() {
        return response;
    }
    let view = path.trim_start_matches("/api/insights/").to_string();
    match crate::models::audit_logs::Model::record_read(
        &db,
        "insight",
        uuid::Uuid::nil(),
        "insight_read",
        actor.as_deref(),
        Some(serde_json::json!({ "view": view })),
    )
    .await
    {
        Ok(_) => response,
        Err(error) => {
            tracing::error!(%error, "insight read audit failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
