//! `Cache-Control: no-store` on every `/api/*` response.

use axum::{extract::Request, middleware::Next, response::Response};

/// Every `/api/*` response says it must not be stored by a cache: the bodies
/// hold personal data (GDS: never cache sensitive or user-specific data). A
/// handler that set its own policy keeps it.
pub(crate) async fn no_store_mw(req: Request, next: Next) -> Response {
    let api = req.uri().path().starts_with("/api/");
    let mut response = next.run(req).await;
    if api {
        response
            .headers_mut()
            .entry(axum::http::header::CACHE_CONTROL)
            .or_insert(axum::http::HeaderValue::from_static("no-store"));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::no_store_mw;
    use axum::{Router, body::Body, http::Request, routing::get};
    use tower::ServiceExt;

    async fn cache_control(path: &str, handler_policy: Option<&'static str>) -> Option<String> {
        let app = Router::new()
            .route(
                "/api/x",
                get(move || async move {
                    handler_policy.map_or_else(
                        || axum::response::Response::new(Body::from("ok")),
                        |p| {
                            let mut r = axum::response::Response::new(Body::from("ok"));
                            r.headers_mut().insert("cache-control", p.parse().unwrap());
                            r
                        },
                    )
                }),
            )
            .route("/_health", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(no_store_mw));
        let response = app
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        response
            .headers()
            .get("cache-control")
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[tokio::test]
    async fn api_responses_are_no_store() {
        assert_eq!(
            cache_control("/api/x", None).await.as_deref(),
            Some("no-store")
        );
    }

    #[tokio::test]
    async fn a_handlers_own_policy_is_kept() {
        assert_eq!(
            cache_control("/api/x", Some("max-age=5")).await.as_deref(),
            Some("max-age=5")
        );
    }

    #[tokio::test]
    async fn other_paths_are_untouched() {
        assert_eq!(cache_control("/_health", None).await, None);
    }
}
