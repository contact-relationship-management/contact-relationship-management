//! Per-client request limit and failed-authentication lockout.

use axa::ratelimit::Verdict;
use axum::{extract::Request, middleware::Next, response::Response};
use std::net::IpAddr;

use crate::{auth, rate_limit, redis_store};

/// Abuse guard (CRM-T43): per-client request limit and a failed-auth
/// lockout, answering `429` with `Retry-After`. Runs outside the auth
/// middleware so it sees the `401`s the auth middleware produces, and inside
/// `axa`'s stack so it sees the client address that stack resolves.
///
/// A token counts only once it has verified, which costs a signature check
/// here as well as in the guard behind this one. Counting by the bearer value
/// would give each made-up token a limit of its own (CRM-T217).
pub(crate) async fn abuse_guard_mw(
    axum::extract::State((guard, shared)): axum::extract::State<(
        std::sync::Arc<rate_limit::Guard>,
        Option<redis_store::Shared>,
    )>,
    req: Request,
    next: Next,
) -> Response {
    // Orchestrator probes must never be throttled or locked out: a refused
    // probe would restart a healthy pod.
    if matches!(req.uri().path(), "/_health" | "/_ping" | "/_readiness") {
        return next.run(req).await;
    }
    if guard.requests.is_none() && guard.failures.is_none() {
        return next.run(req).await;
    }
    let verified = auth::request_claims(req.headers()).await.ok();
    let key = rate_limit::client_key(
        verified.as_ref().map(|claims| claims.sub.as_str()),
        req.extensions().get::<IpAddr>().copied(),
    );
    if let Some(shared) = &shared {
        // Shared counters (CRM_REDIS_URL): one count across all replicas.
        // A Redis error falls through to the in-process counters below.
        match shared_verdict(&guard, shared, &key).await {
            Ok(Err(refused)) => return refused.refusal(),
            Ok(Ok(budget)) => {
                let mut response = next.run(req).await;
                if let Some(budget) = budget {
                    budget.stamp(&mut response);
                }
                if response.status() == axum::http::StatusCode::UNAUTHORIZED
                    && guard.failures.is_some()
                    && let Err(error) = shared
                        .hit(&format!("crm:fail:{key}"), guard.failure_window_secs())
                        .await
                {
                    tracing::warn!(%error, "shared lockout counter failed");
                }
                return response;
            }
            Err(error) => tracing::warn!(%error, "shared rate limit failed; using local counters"),
        }
    }
    // The lockout only looks; a `401` below is what counts against it.
    if let Some(failures) = &guard.failures {
        let verdict = failures.peek(&key);
        if !verdict.allowed {
            return verdict.refusal();
        }
    }
    let budget = match &guard.requests {
        Some(requests) => {
            let verdict = requests.check(&key);
            if !verdict.allowed {
                return verdict.refusal();
            }
            Some(verdict)
        }
        None => None,
    };
    let mut response = next.run(req).await;
    if let Some(budget) = budget {
        // `RateLimit-Limit`, `-Remaining`, `-Reset` (the IETF draft names): the GDS standard
        // asks services to warn clients before they hit the limit.
        budget.stamp(&mut response);
    }
    if response.status() == axum::http::StatusCode::UNAUTHORIZED
        && let Some(failures) = &guard.failures
    {
        let _ = failures.check(&key);
    }
    response
}

/// A verdict from the shared counters, in axa's shape, so a refusal and a budget are answered
/// the same whichever counted them.
fn verdict(allowed: bool, limit: u32, remaining: u64, reset_seconds: u64) -> Verdict {
    Verdict {
        allowed,
        limit,
        remaining: u32::try_from(remaining).unwrap_or(u32::MAX),
        reset_seconds,
    }
}

/// The shared-counter decision: `Err` with the refusal, otherwise the client's remaining
/// budget, `None` when the request limit is off.
async fn shared_verdict(
    guard: &rate_limit::Guard,
    shared: &redis_store::Shared,
    key: &str,
) -> Result<std::result::Result<Option<Verdict>, Verdict>, redis::RedisError> {
    if guard.failures.is_some() {
        let limit = guard.failure_limit();
        let (failed, ttl) = shared.peek(&format!("crm:fail:{key}")).await?;
        if failed >= u64::from(limit) {
            return Ok(Err(verdict(false, limit, 0, ttl)));
        }
    }
    if guard.requests.is_some() {
        let limit = guard.requests_per_minute();
        let (count, ttl) = shared.hit(&format!("crm:req:{key}"), 60).await?;
        if count > u64::from(limit) {
            return Ok(Err(verdict(false, limit, 0, ttl)));
        }
        return Ok(Ok(Some(verdict(
            true,
            limit,
            u64::from(limit) - count,
            ttl,
        ))));
    }
    Ok(Ok(None))
}

#[cfg(test)]
mod tests {
    use super::abuse_guard_mw;
    use crate::rate_limit::Guard;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    fn app(guard: Guard) -> Router {
        Router::new()
            .route("/api/x", get(|| async { "ok" }))
            .route("/api/denied", get(|| async { StatusCode::UNAUTHORIZED }))
            .route("/_health", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                (Arc::new(guard), None),
                abuse_guard_mw,
            ))
    }

    async fn status(app: &Router, path: &str) -> axum::http::Response<Body> {
        app.clone()
            .oneshot(
                Request::get(path)
                    .header("authorization", "Bearer t")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn requests_over_the_limit_get_429_with_retry_after() {
        let app = app(Guard::new(2, 0, 60));
        let first = status(&app, "/api/x").await;
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(first.headers()["ratelimit-limit"], "2");
        assert_eq!(first.headers()["ratelimit-remaining"], "1");
        assert_eq!(status(&app, "/api/x").await.status(), StatusCode::OK);
        let third = status(&app, "/api/x").await;
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(third.headers().contains_key("retry-after"));
        assert_eq!(third.headers()["ratelimit-remaining"], "0");
    }

    #[tokio::test]
    async fn health_probes_are_never_throttled() {
        let app = app(Guard::new(1, 0, 60));
        for _ in 0..5 {
            assert_eq!(status(&app, "/_health").await.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn repeated_401s_lock_the_client_out() {
        let app = app(Guard::new(0, 2, 60));
        assert_eq!(
            status(&app, "/api/denied").await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status(&app, "/api/denied").await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status(&app, "/api/x").await.status(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    /// A request from `address`, with whatever else the caller chose to send.
    fn from(address: &str, headers: &[(&str, &str)]) -> Request<Body> {
        let mut request = Request::get("/api/x");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let mut request = request.body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(address.parse::<std::net::IpAddr>().unwrap());
        request
    }

    #[tokio::test]
    async fn neither_a_made_up_token_nor_a_forged_address_buys_a_fresh_limit() {
        let app = app(Guard::new(2, 0, 60));
        for attempt in 0..2 {
            let token = format!("Bearer forged-{attempt}");
            let forwarded = format!("192.0.2.{attempt}");
            let headers = [
                ("authorization", token.as_str()),
                ("x-forwarded-for", forwarded.as_str()),
            ];
            let answer = app
                .clone()
                .oneshot(from("203.0.113.9", &headers))
                .await
                .unwrap();
            assert_eq!(answer.status(), StatusCode::OK);
        }
        let headers = [
            ("authorization", "Bearer forged-2"),
            ("x-forwarded-for", "192.0.2.2"),
        ];
        let answer = app
            .clone()
            .oneshot(from("203.0.113.9", &headers))
            .await
            .unwrap();
        assert_eq!(answer.status(), StatusCode::TOO_MANY_REQUESTS);
        // Another client still has its own.
        let answer = app.oneshot(from("203.0.113.10", &[])).await.unwrap();
        assert_eq!(answer.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn limits_of_zero_switch_the_guard_off() {
        let app = app(Guard::new(0, 0, 60));
        for _ in 0..5 {
            let r = status(&app, "/api/x").await;
            assert_eq!(r.status(), StatusCode::OK);
            assert!(!r.headers().contains_key("ratelimit-limit"));
        }
    }
}
