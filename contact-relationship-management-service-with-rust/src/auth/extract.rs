//! The bearer-token extractors, [`AuthUser`] and [`MaybeAuthUser`].

#[cfg(feature = "keycloak")]
use super::keycloak_verifier;
use super::{AUTHORIZATION, Claims, FromRequestParts, HeaderMap, Parts, StatusCode};
#[cfg(feature = "paseto")]
use super::{Verifier, verifier};

/// Extract and verify the bearer token from request headers. Pure (the
/// verifier is passed in), so it is unit-testable without the global.
///
/// # Errors
///
/// `401` when the `Authorization` header is missing, is not a bearer
/// token, or the token fails PASETO signature / issuer / audience / expiry
/// verification.
#[cfg(feature = "paseto")]
pub fn bearer_claims(
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<Claims, (StatusCode, String)> {
    let token = bearer_token(headers)?;
    verifier
        .verify(token)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
}

/// The bearer token from the `Authorization` header, trimmed.
///
/// # Errors
///
/// `401` when the header is missing or is not a bearer token.
pub(super) fn bearer_token(headers: &HeaderMap) -> Result<&str, (StatusCode, String)> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing authorization header".to_string(),
        ))?;
    header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .map(str::trim)
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "expected bearer token".to_string(),
        ))
}

/// Authenticate a request through every credential type this build
/// accepts, against the process-wide verifiers. A token beginning `v4.` is
/// a PASETO; anything else is offered to Keycloak when configured. Each
/// path fails closed with `401`.
///
/// # Errors
///
/// `401` when the header is missing or not a bearer token, the token fails
/// verification, or no verifier in this build accepts its type.
// `async` for the Keycloak path (JWKS refresh); a PASETO-only build never awaits.
#[allow(clippy::unused_async)]
pub async fn request_claims(headers: &HeaderMap) -> Result<Claims, (StatusCode, String)> {
    let token = bearer_token(headers)?;
    #[cfg(feature = "keycloak")]
    if !token.starts_with("v4.")
        && let Some(keycloak) = keycloak_verifier()
    {
        return keycloak
            .verify(token)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()));
    }
    #[cfg(feature = "paseto")]
    {
        verifier()
            .current()
            .verify(token)
            .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
    }
    #[cfg(not(feature = "paseto"))]
    {
        let _ = token;
        Err((
            StatusCode::UNAUTHORIZED,
            "no verifier accepts this credential".to_string(),
        ))
    }
}

/// A request whose bearer token passed PASETO signature / issuer /
/// audience / expiry verification. The wrapped [`Claims`] identify the
/// caller (`sub` is the user `pid`). Taking this argument makes a handler
/// require authentication.
pub struct AuthUser(pub Claims);

/// Extracting an [`AuthUser`] verifies the request's bearer token against
/// the process-wide [`verifier`]; a missing/invalid token rejects with
/// `401` before the handler runs, so the type is the "require auth" gate.
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // The live verifiers, not a boot snapshot: a rotated key set must
        // reach the extractors and the guard together.
        request_claims(&parts.headers).await.map(AuthUser)
    }
}

/// Like [`AuthUser`], but never rejects: yields `Some(claims)` when a
/// valid bearer token is present and `None` otherwise. Handlers use it to
/// stamp the caller identity (e.g. the audit `actor`) without requiring
/// authentication on the route.
pub struct MaybeAuthUser(pub Option<Claims>);

impl MaybeAuthUser {
    /// The caller's `sub` (user `pid`) if a valid token was presented.
    #[must_use]
    pub fn actor(&self) -> Option<&str> {
        self.0.as_ref().map(|c| c.sub.as_str())
    }

    /// The verified [`Claims`] if a valid token was presented, for the
    /// record-level authorization pass ([`authorize_record`]).
    #[must_use]
    pub fn claims(&self) -> Option<&Claims> {
        self.0.as_ref()
    }
}

/// Extracting a [`MaybeAuthUser`] never rejects: a valid token yields
/// `Some(claims)`, anything else `None`. Handlers use it to opportunistically
/// stamp the caller identity without requiring authentication.
impl<S: Send + Sync> FromRequestParts<S> for MaybeAuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(request_claims(&parts.headers).await.ok()))
    }
}
