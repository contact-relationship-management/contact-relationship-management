//! Response security headers (CRM-T33), set on every response in production. The values are the
//! ones production has always sent. Inside `axa`'s stack, whose defaults yield to a header already set.

use axum::{
    Router,
    extract::Request,
    http::{HeaderName, HeaderValue, header},
    middleware::{Next, from_fn},
    response::Response,
};

const HEADERS: [(HeaderName, &str); 8] = [
    (
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self' https:; font-src 'self' https: data:; img-src 'self' https: data:; \
         object-src 'none'; script-src https:; style-src 'self' https: 'unsafe-inline'",
    ),
    (
        header::STRICT_TRANSPORT_SECURITY,
        "max-age=63072000; includeSubDomains",
    ),
    (header::REFERRER_POLICY, "no-referrer"),
    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    (header::X_FRAME_OPTIONS, "sameorigin"),
    (header::X_XSS_PROTECTION, "0"),
    (HeaderName::from_static("x-download-options"), "noopen"),
    (
        HeaderName::from_static("x-permitted-cross-domain-policies"),
        "none",
    ),
];

async fn set_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in &HEADERS {
        response
            .headers_mut()
            .insert(name.clone(), HeaderValue::from_static(value));
    }
    response
}

/// Add the headers to everything `router` serves, its fallback included.
pub fn install(router: Router) -> Router {
    router.layer(from_fn(set_headers))
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, routing::get};
    use tower::ServiceExt as _;

    use super::*;

    async fn headers_of(path: &str) -> axum::http::HeaderMap {
        let router = Router::new().route(
            "/replaced",
            get(|| async { ([(header::X_FRAME_OPTIONS, "ALLOW")], "ok") }),
        );
        let request = Request::builder().uri(path).body(Body::empty()).unwrap();
        install(router)
            .oneshot(request)
            .await
            .unwrap()
            .into_parts()
            .0
            .headers
    }

    #[tokio::test]
    async fn every_header_is_sent_with_its_value() {
        let headers = headers_of("/replaced").await;
        for (name, value) in &HEADERS {
            assert_eq!(headers.get(name).unwrap().to_str().unwrap(), *value);
        }
    }

    #[tokio::test]
    async fn the_policy_and_the_two_year_transport_rule_are_the_ones_promised() {
        let headers = headers_of("/replaced").await;
        let policy = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
        assert!(policy.contains("object-src 'none'"), "{policy}");
        assert_eq!(
            headers[header::STRICT_TRANSPORT_SECURITY],
            "max-age=63072000; includeSubDomains"
        );
    }

    #[tokio::test]
    async fn a_handler_cannot_loosen_them() {
        let headers = headers_of("/replaced").await;
        assert_eq!(headers[header::X_FRAME_OPTIONS], "sameorigin");
    }

    /// `axa` has its own value for several of these in production, and only sets one that is absent.
    #[tokio::test]
    async fn they_win_over_the_defaults_of_axa_s_stack_around_them() {
        let http = axa::config::Http {
            body_limit: 1024,
            compression: false,
            timeout_seconds: 0,
            timeout_overrides: Vec::new(),
            cors_origins: Vec::new(),
            trusted_proxies: Vec::new(),
            hsts: true,
            hsts_max_age: 1,
            csp: Some("default-src 'none'".to_string()),
            permissions_policy: Some("camera=()".to_string()),
        };
        let router = Router::new().route("/x", get(|| async { "ok" }));
        let request = Request::builder().uri("/x").body(Body::empty()).unwrap();
        let response = axa::middleware::apply(install(router), &http)
            .unwrap()
            .oneshot(request)
            .await
            .unwrap();
        for (name, value) in &HEADERS {
            let sent = response.headers().get(name).unwrap().to_str().unwrap();
            assert_eq!(sent, *value);
        }
    }

    #[tokio::test]
    async fn a_page_that_does_not_exist_carries_them_too() {
        let headers = headers_of("/nothing-here").await;
        assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
    }
}
