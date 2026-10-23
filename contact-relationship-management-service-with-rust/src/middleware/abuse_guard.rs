//! Per-client request limit and failed-authentication lockout.

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use loco_rs::Result;

use crate::{rate_limit, redis_store};

/// Abuse guard (CRM-T43): per-client request limit and a failed-auth
/// lockout, answering `429` with `Retry-After`. Runs outside the auth
/// middleware so it sees the `401`s the auth middleware produces.
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
    let key = {
        let header = |name: &str| req.headers().get(name).and_then(|v| v.to_str().ok());
        let trust_forwarded = crate::settings::current().trust_forwarded_for;
        rate_limit::client_key(
            header("authorization"),
            header("x-forwarded-for"),
            trust_forwarded,
        )
    };
    let now = std::time::Instant::now();
    let limit = u64::from(guard.requests.limit());
    let refused = |retry_after_secs: u64| {
        with_budget(
            (
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                [(
                    axum::http::header::RETRY_AFTER,
                    retry_after_secs.to_string(),
                )],
                "too many requests",
            )
                .into_response(),
            (limit > 0).then_some((limit, 0, retry_after_secs)),
        )
    };
    if let Some(shared) = &shared {
        // Shared counters (CRM_REDIS_URL): one count across all replicas.
        // A Redis error falls through to the in-process counters below.
        match shared_verdict(&guard, shared, &key).await {
            Ok(Err(retry_after_secs)) => return refused(retry_after_secs),
            Ok(Ok(budget)) => {
                let response = with_budget(next.run(req).await, budget);
                if response.status() == axum::http::StatusCode::UNAUTHORIZED
                    && !guard.failures.disabled()
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
    if let rate_limit::Decision::Limited { retry_after_secs } = guard.failures.blocked(&key, now) {
        return refused(retry_after_secs);
    }
    if let rate_limit::Decision::Limited { retry_after_secs } = guard.requests.hit(&key, now) {
        return refused(retry_after_secs);
    }
    let budget = guard
        .requests
        .budget(&key, now)
        .map(|(limit, remaining, reset)| (u64::from(limit), u64::from(remaining), reset));
    let response = with_budget(next.run(req).await, budget);
    if response.status() == axum::http::StatusCode::UNAUTHORIZED {
        let _ = guard.failures.hit(&key, now);
    }
    response
}

/// Tell the client its budget (`RateLimit-Limit`, `-Remaining`, `-Reset`; the
/// IETF draft names). The GDS standard asks services to warn clients before
/// they hit the limit.
fn with_budget(mut response: Response, budget: Option<(u64, u64, u64)>) -> Response {
    if let Some((limit, remaining, reset)) = budget {
        let headers = response.headers_mut();
        for (name, value) in [
            ("ratelimit-limit", limit),
            ("ratelimit-remaining", remaining),
            ("ratelimit-reset", reset),
        ] {
            headers.insert(name, axum::http::HeaderValue::from(value));
        }
    }
    response
}

/// A client's budget: `(limit, remaining, seconds until reset)`.
type Budget = Option<(u64, u64, u64)>;

/// The shared-counter decision: `Err(retry_after)` to refuse, otherwise the
/// client's remaining budget.
async fn shared_verdict(
    guard: &rate_limit::Guard,
    shared: &redis_store::Shared,
    key: &str,
) -> Result<std::result::Result<Budget, u64>, redis::RedisError> {
    if !guard.failures.disabled() {
        let (failed, ttl) = shared.peek(&format!("crm:fail:{key}")).await?;
        if failed >= u64::from(guard.failures.limit()) {
            return Ok(Err(ttl));
        }
    }
    if !guard.requests.disabled() {
        let limit = u64::from(guard.requests.limit());
        let (count, ttl) = shared.hit(&format!("crm:req:{key}"), 60).await?;
        if count > limit {
            return Ok(Err(ttl));
        }
        return Ok(Ok(Some((limit, limit - count, ttl))));
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
