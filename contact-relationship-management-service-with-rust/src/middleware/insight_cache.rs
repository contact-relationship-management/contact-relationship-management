//! Redis-backed response cache for insight reads.

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use loco_rs::Result;

use crate::auth;

/// The generation counter every successful write bumps.
const CACHE_GEN_KEY: &str = "crm:cache:gen";
/// How long an insight response is cached.
const INSIGHT_TTL: std::time::Duration = std::time::Duration::from_secs(30);
/// The cache is an optimisation: a Redis that is down or frozen must cost a
/// request this long at most, never the pool's own timeout (CRM-R25).
const CACHE_WAIT: std::time::Duration = std::time::Duration::from_millis(750);

/// The cache key for an insight read: a hash of the caller's token (so masking
/// never crosses callers), their break-glass state, the time zone they name (their
/// "today" decides the follow-ups, hygiene and memberships lists, CRM-T179), the
/// current generation, and the path and query.
fn insight_key(
    token: &str,
    glass: bool,
    zone: &str,
    generation: i64,
    path: &str,
    query: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let digest =
        Sha256::digest(format!("{token}|{glass}|{zone}|{generation}|{path}?{query}").as_bytes());
    let hex = digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    format!("crm:insight:{hex}")
}

/// Make every cached insight stale: a write has happened.
async fn bump_cache_generation(cache: &loco_rs::cache::Cache) {
    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    match tokio::time::timeout(
        CACHE_WAIT,
        cache.insert_with_expiry(CACHE_GEN_KEY, &stamp, INSIGHT_TTL * 4),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::warn!(%error, "cache generation bump failed"),
        Err(_) => tracing::warn!("cache generation bump timed out"),
    }
}

/// Insight response cache (Loco's cache, Redis-backed). Sits inside auth and
/// the read audit, so every request is still authenticated and audited; only
/// the computation is skipped. The key carries the caller's token hash (so
/// masking never crosses callers), their break-glass state, the path and
/// query, and a generation bumped by every successful write, so a write is
/// visible at once. A policy reload is visible after at most the TTL.
pub(crate) async fn insight_cache_mw(
    axum::extract::State(cache): axum::extract::State<std::sync::Arc<loco_rs::cache::Cache>>,
    req: Request,
    next: Next,
) -> Response {
    const MAX_BODY: usize = 1024 * 1024;
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    if method != axum::http::Method::GET && path.starts_with("/api/") {
        let response = next.run(req).await;
        if response.status().is_success() {
            bump_cache_generation(&cache).await;
        }
        return response;
    }
    if method != axum::http::Method::GET
        || !path.starts_with("/api/insights/")
        || req
            .headers()
            .contains_key(axum::http::header::IF_NONE_MATCH)
    {
        return next.run(req).await;
    }
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let claims = auth::request_claims(req.headers()).await.ok();
    let glass = claims
        .as_ref()
        .is_some_and(|c| crate::break_glass::registry().active(&c.sub, chrono::Utc::now()));
    let generation: i64 = tokio::time::timeout(CACHE_WAIT, cache.get(CACHE_GEN_KEY))
        .await
        .ok()
        .and_then(Result::ok)
        .flatten()
        .unwrap_or_default();
    let zone = req
        .headers()
        .get(crate::rules::periods::USER_ZONE_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let key = insight_key(
        token,
        glass,
        zone,
        generation,
        &path,
        req.uri().query().unwrap_or(""),
    );
    if let Ok(Ok(Some(hit))) =
        tokio::time::timeout(CACHE_WAIT, cache.get::<CachedResponse>(&key)).await
    {
        return hit.into_response();
    }
    let response = next.run(req).await;
    if response.status() != axum::http::StatusCode::OK {
        return response;
    }
    let (parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, MAX_BODY).await else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        // Keep the response headers (ETag, cache control, content type) so a
        // cached read is indistinguishable from a computed one.
        let headers = parts
            .headers
            .iter()
            .filter(|(name, _)| *name != axum::http::header::CONTENT_LENGTH)
            .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
            .collect();
        let entry = CachedResponse { headers, text };
        match tokio::time::timeout(
            CACHE_WAIT,
            cache.insert_with_expiry(&key, &entry, INSIGHT_TTL),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "insight cache write failed"),
            Err(_) => tracing::warn!("insight cache write timed out"),
        }
    }
    Response::from_parts(parts, axum::body::Body::from(bytes))
}

/// A cached 200 response: its headers and body.
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedResponse {
    headers: Vec<(String, String)>,
    text: String,
}

impl CachedResponse {
    fn into_response(self) -> Response {
        let mut response = Response::new(axum::body::Body::from(self.text));
        for (name, value) in self.headers {
            if let (Ok(name), Ok(value)) = (
                axum::http::HeaderName::from_bytes(name.as_bytes()),
                axum::http::HeaderValue::from_str(&value),
            ) {
                response.headers_mut().append(name, value);
            }
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::insight_key;

    #[test]
    fn an_insight_key_depends_on_everything_that_changes_the_answer() {
        let base = insight_key("Bearer a", false, "", 1, "/api/insights/x", "days=7");
        assert_eq!(
            base,
            insight_key("Bearer a", false, "", 1, "/api/insights/x", "days=7"),
            "the same request gets the same key"
        );
        for other in [
            insight_key("Bearer b", false, "", 1, "/api/insights/x", "days=7"),
            insight_key("Bearer a", true, "", 1, "/api/insights/x", "days=7"),
            insight_key(
                "Bearer a",
                false,
                "Asia/Tokyo",
                1,
                "/api/insights/x",
                "days=7",
            ),
            insight_key("Bearer a", false, "", 2, "/api/insights/x", "days=7"),
            insight_key("Bearer a", false, "", 1, "/api/insights/y", "days=7"),
            insight_key("Bearer a", false, "", 1, "/api/insights/x", "days=30"),
        ] {
            assert_ne!(base, other);
        }
        assert!(base.starts_with("crm:insight:"));
        assert!(
            !base.contains("Bearer"),
            "the token is hashed, never stored"
        );
    }
}
