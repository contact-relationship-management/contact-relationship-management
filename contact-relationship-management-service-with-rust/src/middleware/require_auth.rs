//! Blanket authentication: every request is checked against the policy before a handler runs.

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::auth;

/// Blanket auth-enforcement middleware: reads `CRM_REQUIRE_AUTH` per
/// request and delegates to [`auth::enforce_request`]. Off by
/// default — see `auth/` and `agents/share/security.md` §4.
pub(crate) async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let policy = auth::policy().current();
    match auth::enforce_request(auth::require_auth(), &method, &path, req.headers(), &policy).await
    {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}
