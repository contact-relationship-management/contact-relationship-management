//! Signature check for provider webhooks (CRM-T39).
//!
//! A webhook is authenticated by an HMAC-SHA256 over `"{timestamp}.{body}"`
//! with `CONTACT_RELATIONSHIP_MANAGEMENT_WEBHOOK_SECRET`, sent as `X-CRM-Timestamp` (Unix seconds) and
//! `X-CRM-Signature` (`sha256=<hex>`). A timestamp outside five minutes is
//! refused, so a captured request cannot be replayed later; within the window
//! a replay is harmless because each (message, event) is recorded once.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// How far the timestamp may be from now, in seconds.
pub const TOLERANCE_SECS: i64 = 300;

/// Shortest accepted secret.
pub const MIN_SECRET: usize = crate::settings::MIN_WEBHOOK_SECRET;

/// The configured secret, if long enough.
#[must_use]
pub fn secret() -> Option<String> {
    crate::settings::current().webhook_secret.clone()
}

/// `None` when the key is refused, which HMAC does for no length; a caller then fails closed.
fn tag(secret: &str, timestamp: &str, body: &[u8]) -> Option<Hmac<Sha256>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    Some(mac)
}

/// The signature header value for a request (used by tests and by a sender). Empty when the key
/// is refused, which no receiver accepts.
#[must_use]
pub fn sign(secret: &str, timestamp: &str, body: &[u8]) -> String {
    tag(secret, timestamp, body).map_or_else(String::new, |mac| {
        format!("sha256={}", hex(&mac.finalize().into_bytes()))
    })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Whether the request is authentic and fresh.
#[must_use]
pub fn verify(secret: &str, timestamp: &str, signature: &str, body: &[u8], now: i64) -> bool {
    let Ok(sent_at) = timestamp.trim().parse::<i64>() else {
        return false;
    };
    if now
        .checked_sub(sent_at)
        .and_then(i64::checked_abs)
        .is_none_or(|age| age > TOLERANCE_SECS)
    {
        return false;
    }
    let Some(given) = signature
        .strip_prefix("sha256=")
        .and_then(crate::rules::webhooks::unhex)
    else {
        return false;
    };
    tag(secret, timestamp.trim(), body).is_some_and(|mac| mac.verify_slice(&given).is_ok())
}

/// Whether a request to a signed webhook is authentic: `None` when no secret is configured (the
/// route then answers `404`), otherwise whether its `X-CRM-Timestamp` and `X-CRM-Signature` match.
#[must_use]
pub fn signed_request(headers: &axum::http::HeaderMap, body: &[u8]) -> Option<bool> {
    let secret = secret()?;
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
    };
    Some(verify(
        &secret,
        header("x-crm-timestamp"),
        header("x-crm-signature"),
        body,
        jiff::Timestamp::now().as_second(),
    ))
}

/// Every credential a request presents: a bearer token, a basic-auth
/// password, or `X-CRM-Webhook-Token`. Providers differ (Brevo can send a
/// bearer token or custom headers; Mailjet puts basic auth in the callback
/// URL), so the secret is accepted in any of them.
#[must_use]
pub fn presented(authorization: Option<&str>, token_header: Option<&str>) -> Vec<String> {
    use base64::Engine as _;
    let mut out = Vec::new();
    if let Some(value) = authorization {
        if let Some(token) = value.strip_prefix("Bearer ") {
            out.push(token.trim().to_string());
        } else if let Some(encoded) = value.strip_prefix("Basic ")
            && let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded.trim())
            && let Ok(pair) = String::from_utf8(bytes)
            && let Some((_, password)) = pair.split_once(':')
        {
            out.push(password.to_string());
        }
    }
    if let Some(value) = token_header {
        out.push(value.trim().to_string());
    }
    out
}

/// HMAC of a credential under the secret, so a comparison is over fixed-size
/// digests whatever the credential's length.
fn credential_mac(secret: &str, credential: &str) -> Option<Hmac<Sha256>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(b"token.");
    mac.update(credential.as_bytes());
    Some(mac)
}

/// Whether any presented credential is the secret (compared in constant time).
#[must_use]
pub fn token_ok(secret: &str, presented: &[String]) -> bool {
    let Some(expected) = credential_mac(secret, secret).map(|mac| mac.finalize().into_bytes())
    else {
        return false;
    };
    presented.iter().fold(false, |ok, candidate| {
        ok | credential_mac(secret, candidate)
            .is_some_and(|mac| mac.verify_slice(&expected).is_ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn a_good_signature_verifies() {
        let sig = sign(S, "1000", b"{}");
        assert!(verify(S, "1000", &sig, b"{}", 1000));
    }

    #[test]
    fn a_changed_body_secret_or_timestamp_fails() {
        let sig = sign(S, "1000", b"{}");
        assert!(!verify(S, "1000", &sig, b"{ }", 1000));
        assert!(!verify(&S.replace('0', "9"), "1000", &sig, b"{}", 1000));
        assert!(!verify(S, "1001", &sig, b"{}", 1001));
    }

    #[test]
    fn a_stale_or_future_request_is_refused() {
        let sig = sign(S, "1000", b"{}");
        assert!(!verify(S, "1000", &sig, b"{}", 1000 + TOLERANCE_SECS + 1));
        assert!(!verify(S, "1000", &sig, b"{}", 1000 - TOLERANCE_SECS - 1));
        assert!(verify(S, "1000", &sig, b"{}", 1000 + TOLERANCE_SECS));
    }

    #[test]
    fn a_token_is_read_from_bearer_basic_or_the_header() {
        use base64::Engine as _;
        let basic = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("mailjet:{S}"))
        );
        assert!(token_ok(S, &presented(Some(&format!("Bearer {S}")), None)));
        assert!(token_ok(S, &presented(Some(&basic), None)));
        assert!(token_ok(S, &presented(None, Some(S))));
        assert!(!token_ok(S, &presented(Some("Bearer nope"), Some("x"))));
        assert!(!token_ok(S, &presented(Some("Basic !!!"), None)));
        assert!(!token_ok(S, &presented(None, None)));
    }

    #[test]
    fn malformed_input_is_refused() {
        assert!(!verify(S, "x", "sha256=00", b"", 0));
        assert!(!verify(S, "0", "nope", b"", 0));
        assert!(!verify(S, "0", "sha256=zz", b"", 0));
        assert!(!verify(S, "0", "sha256=abc", b"", 0));
        assert!(!verify(S, "0", "sha256=é", b"", 0));
    }
}
