//! Idempotency keys for `POST` (CRM-T52).
//!
//! A client that sends `Idempotency-Key: <opaque>` on a `POST` gets
//! at-most-once execution per `(caller, key)`: a retry after a timeout replays
//! the first response instead of creating a second record.
//!
//! - Same key, same request, first attempt finished with a `2xx` → the stored
//!   response is replayed with `Idempotent-Replayed: true`.
//! - Same key, a **different** request → `422` (a reused key is a client bug).
//! - Same key while the first attempt is still running → `409` with
//!   `Retry-After: 1`.
//! - A first attempt that did not succeed (non-`2xx`) is forgotten, so the
//!   retry runs again; so is one that crashed (taken over after 2 minutes).
//! - Keys expire after 24 hours.
//!
//! The record is a Postgres row guarded by a primary key, so the guarantee
//! holds across replicas. Without the header nothing changes.

use sha2::{Digest, Sha256};

/// The request header carrying the key.
pub const HEADER: &str = "idempotency-key";
/// Longest accepted key.
pub const MAX_KEY_LEN: usize = 255;
/// Largest request or response body held for fingerprinting or replay.
pub const MAX_BODY: usize = 1024 * 1024;

/// A key is 1 to [`MAX_KEY_LEN`] visible ASCII characters (no spaces).
///
/// # Errors
///
/// A message naming what is wrong.
pub fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > MAX_KEY_LEN {
        return Err(format!(
            "Idempotency-Key must be 1 to {MAX_KEY_LEN} characters"
        ));
    }
    if !key.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        return Err("Idempotency-Key must be visible ASCII without spaces".to_string());
    }
    Ok(())
}

/// A stable digest of the request, so a reused key with different content is
/// caught.
#[must_use]
pub fn fingerprint(method: &str, path: &str, query: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for part in [method.as_bytes(), path.as_bytes(), query.as_bytes(), body] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

/// Whether a response is worth replaying: only successes are remembered, so a
/// retry of a failed attempt runs again.
#[must_use]
pub fn remember(status: u16) -> bool {
    (200..300).contains(&status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_bounded_visible_ascii() {
        assert!(validate_key("a1b2-c3").is_ok());
        assert!(validate_key(&"x".repeat(MAX_KEY_LEN)).is_ok());
        assert!(validate_key("").is_err());
        assert!(validate_key(&"x".repeat(MAX_KEY_LEN + 1)).is_err());
        assert!(validate_key("has space").is_err());
        assert!(validate_key("tab\there").is_err());
        assert!(validate_key("naïve").is_err());
    }

    #[test]
    fn fingerprint_distinguishes_every_part() {
        let base = fingerprint("POST", "/api/contacts", "", b"{}");
        assert_eq!(base, fingerprint("POST", "/api/contacts", "", b"{}"));
        assert_eq!(base.len(), 64);
        assert_ne!(base, fingerprint("POST", "/api/contacts", "", b"{ }"));
        assert_ne!(base, fingerprint("POST", "/api/accounts", "", b"{}"));
        assert_ne!(base, fingerprint("POST", "/api/contacts", "x=1", b"{}"));
        // Boundaries matter: moving a byte between parts changes the digest.
        assert_ne!(
            fingerprint("POST", "/a", "b", b""),
            fingerprint("POST", "/ab", "", b"")
        );
    }

    #[test]
    fn only_successes_are_remembered() {
        assert!(remember(200));
        assert!(remember(201));
        assert!(!remember(422));
        assert!(!remember(403));
        assert!(!remember(500));
    }
}
