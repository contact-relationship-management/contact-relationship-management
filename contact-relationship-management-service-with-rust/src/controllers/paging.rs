//! Pagination for list endpoints (CRM-T51, CRM-T223).
//!
//! Every list used to cut its result at a fixed cap and say nothing. Now a
//! list takes `?limit=` (1 to the endpoint's cap, default the cap) and an
//! opaque `?cursor=`, and when more rows exist the response carries
//! `Link: <...>; rel="next"`. The body stays a bare JSON array, so an existing
//! client keeps working until it passes the cap, and then it is told rather
//! than truncated.
//!
//! The cursor is `axa::cursor_pagination`'s: an offset signed with the
//! service's key over the path and every filter, so a caller cannot make one
//! up, edit one, or carry one to another list or filter. A row inserted or
//! removed between two page fetches can make a later page skip or repeat one
//! row; every order here ends in `id` so the order itself is stable. There is
//! deliberately no total count: a total over stored rows would include rows a
//! caller's record-level policy hides.

use std::sync::OnceLock;

use axa::cursor_pagination::{CursorKey, CursorLimits, CursorPage};
use axa::prelude::*;
use axum::http::Uri;
use serde::Serialize;

/// The setting that holds the key cursors are signed with, 32 bytes or more.
pub const SECRET_VAR: &str = "CRM_CURSOR_SECRET";

static KEY: OnceLock<CursorKey> = OnceLock::new();

/// A key for this process alone, for development and tests: its cursors stop working when the
/// process stops, and another replica refuses them.
fn process_key() -> CursorKey {
    let mut secret = Vec::with_capacity(32);
    secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    CursorKey::new(&secret).unwrap_or_else(|_| unreachable!("32 bytes is long enough"))
}

/// The cursor key from `secret`. In production a key is required, because replicas must
/// accept each other's cursors.
///
/// # Errors
///
/// A message when production has no key, or a key is shorter than 32 bytes.
pub fn key_from(secret: Option<&str>, production: bool) -> Result<CursorKey, String> {
    match secret {
        Some(secret) => CursorKey::new(secret.as_bytes()).map_err(|e| format!("{SECRET_VAR}: {e}")),
        None if production => Err(format!(
            "{SECRET_VAR} is required in production: it signs the list cursors, and every \
             replica must hold the same one"
        )),
        None => Ok(process_key()),
    }
}

/// Read [`SECRET_VAR`] at start-up. A second call in one process keeps the first key.
///
/// # Errors
///
/// As [`key_from`].
pub fn init(production: bool) -> Result<(), String> {
    let secret = std::env::var(SECRET_VAR).ok();
    let key = key_from(secret.as_deref(), production)?;
    let _ = KEY.set(key);
    Ok(())
}

/// The installed key. A process that never ran [`init`], such as a unit test, gets a key of
/// its own; a server always runs it, and production refuses to start without a setting.
fn key() -> &'static CursorKey {
    KEY.get_or_init(process_key)
}

/// A validated page request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page(CursorPage);

impl Page {
    /// Read `limit` and `cursor` from the request's `uri`, against the endpoint's `cap`.
    ///
    /// # Errors
    ///
    /// `422` for a `limit` outside `1..=cap`, not a number or given twice, or a cursor this
    /// service did not sign for this path and these filters. The `detail` quotes nothing the
    /// caller sent.
    pub fn new(uri: &Uri, cap: u64) -> Result<Self> {
        Self::with_key(key(), uri, cap)
    }

    fn with_key(key: &CursorKey, uri: &Uri, cap: u64) -> Result<Self> {
        let limits = CursorLimits {
            default_limit: cap,
            max_limit: cap,
        };
        Ok(Self(CursorPage::new(key, limits, uri)?))
    }

    /// `LIMIT` and `OFFSET` as `sqlx` binds them, the limit one more than the page: the probe
    /// row [`Page::split`] drops. A cursor past every row saturates (CRM-D24): an empty page,
    /// not a failed query.
    #[must_use]
    pub fn bounds(&self) -> (i64, i64) {
        self.0.bounds()
    }

    /// Drop the probe row; the flag says whether more rows follow.
    #[must_use]
    pub fn split<T>(&self, rows: Vec<T>) -> (Vec<T>, bool) {
        self.0.split(rows)
    }

    /// The JSON array response, with `Link: rel="next"` when `more`. `body` is what is left of
    /// the page after [`Page::split`] and any filtering the caller may not see past.
    ///
    /// # Errors
    ///
    /// When the body cannot be serialised.
    pub fn respond<T: Serialize>(&self, body: T, more: bool) -> Result<Response> {
        let mut response = format::json(body)?;
        if more && let Some(link) = self.0.next_link() {
            response
                .headers_mut()
                .insert(axum::http::header::LINK, link);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> CursorKey {
        CursorKey::new(b"a test secret of thirty-two byte").unwrap()
    }

    fn page(uri: &str, cap: u64) -> Result<Page> {
        Page::with_key(&key(), &uri.parse().unwrap(), cap)
    }

    /// The path and query of the next link, to send back.
    fn next(response: &Response) -> String {
        let link = response.headers()[axum::http::header::LINK]
            .to_str()
            .unwrap();
        link.strip_prefix('<')
            .unwrap()
            .split('>')
            .next()
            .unwrap()
            .to_string()
    }

    #[test]
    fn defaults_to_the_cap_and_rejects_out_of_range() {
        assert_eq!(page("/api/x", 100).unwrap().bounds(), (101, 0));
        for bad in ["limit=0", "limit=101", "limit=two", "limit=1&limit=2"] {
            assert!(page(&format!("/api/x?{bad}"), 100).is_err(), "{bad}");
        }
        assert_eq!(page("/api/x?limit=1", 100).unwrap().bounds(), (2, 0));
    }

    #[test]
    fn a_next_cursor_moves_on_and_keeps_the_filters() {
        let first = page("/api/tickets?status=open&limit=2", 10).unwrap();
        let response = first.respond(vec![1, 2], true).unwrap();
        let link = next(&response);
        assert!(
            link.starts_with("/api/tickets?status=open&limit=2&cursor="),
            "{link}"
        );
        assert_eq!(page(&link, 10).unwrap().bounds(), (3, 2));
        assert!(
            first
                .respond(vec![1], false)
                .unwrap()
                .headers()
                .get(axum::http::header::LINK)
                .is_none()
        );
    }

    #[test]
    fn a_cursor_is_refused_when_made_up_edited_moved_or_signed_by_another_key() {
        let link = next(
            &page("/api/tickets?status=open&limit=2", 10)
                .unwrap()
                .respond(vec![1, 2], true)
                .unwrap(),
        );
        let cursor = link.split("cursor=").nth(1).unwrap();
        // The old unsigned form, `o:<offset>`.
        assert!(page("/api/tickets?status=open&cursor=bzoyNTA", 10).is_err());
        let mut edited = cursor.to_string();
        let last = if edited.ends_with('A') { 'B' } else { 'A' };
        edited.pop();
        edited.push(last);
        assert!(page(&format!("/api/tickets?status=open&cursor={edited}"), 10).is_err());
        assert!(page(&format!("/api/tickets?status=closed&cursor={cursor}"), 10).is_err());
        assert!(page(&format!("/api/deals?status=open&cursor={cursor}"), 10).is_err());
        let other = CursorKey::new(b"another secret, thirty-two bytes").unwrap();
        assert!(Page::with_key(&other, &link.parse().unwrap(), 10).is_err());
        // The page size may change between pages.
        assert!(
            page(
                &format!("/api/tickets?status=open&limit=5&cursor={cursor}"),
                10
            )
            .is_ok()
        );
    }

    #[test]
    fn split_reports_whether_more_rows_follow() {
        let page = page("/api/x?limit=2", 10).unwrap();
        assert_eq!(page.split(vec![1, 2, 3]), (vec![1, 2], true));
        assert_eq!(page.split(vec![1, 2]), (vec![1, 2], false));
        assert_eq!(page.split(Vec::<i32>::new()), (vec![], false));
    }

    #[test]
    fn production_needs_a_key_of_32_bytes_and_elsewhere_one_is_made() {
        assert!(key_from(None, true).unwrap_err().contains(SECRET_VAR));
        assert!(key_from(Some("too short"), false).is_err());
        assert!(key_from(Some("too short"), true).is_err());
        assert!(key_from(Some("a production secret, 32 bytes ok"), true).is_ok());
        assert!(key_from(None, false).is_ok());
    }

    #[test]
    fn the_error_quotes_nothing_the_caller_sent() {
        let Err(error) = page("/api/x?cursor=SECRETVALUE", 10) else {
            panic!("a made-up cursor was accepted");
        };
        assert!(!format!("{error:?}").contains("SECRETVALUE"));
    }
}
