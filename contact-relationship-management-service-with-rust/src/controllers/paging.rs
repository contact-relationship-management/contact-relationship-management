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
//!
//! A list the record-level policy filters row by row fills its page with
//! [`Page::fill`] (CRM-T326): it reads on, in batches, until the page holds a
//! page of rows the caller may see, so a page does not come back short and say
//! that rows were hidden. The next page starts after the last row returned. One
//! request reads at most [`SCAN_PAGES`] pages' worth of rows; a page that runs out
//! of that budget can be short, with a next link from where the reading stopped.
//! Time is not hidden: a page that skipped many rows is slower to answer.

use std::sync::OnceLock;

use std::future::Future;

use axa::cursor_pagination::{CursorKey, CursorLimits, CursorPage};

use crate::arith::OrOverflow;
use axa::prelude::*;
use axum::http::HeaderValue;
use axum::http::Uri;
use serde::Serialize;

/// The setting that holds the key cursors are signed with, 32 bytes or more.
pub const SECRET_VAR: &str = "CONTACT_RELATIONSHIP_MANAGEMENT_CURSOR_SECRET";

static KEY: OnceLock<CursorKey> = OnceLock::new();

/// A key for this process alone, for development and tests: its cursors stop working when the
/// process stops, and another replica refuses them.
fn process_key() -> Result<CursorKey, String> {
    let mut secret = Vec::with_capacity(32);
    secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    CursorKey::new(&secret).map_err(|e| format!("{SECRET_VAR}: {e}"))
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
        None => process_key(),
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
///
/// # Errors
///
/// When a key of its own cannot be made, which a 32-byte key never fails.
fn key() -> Result<&'static CursorKey> {
    if let Some(key) = KEY.get() {
        return Ok(key);
    }
    let made = process_key().map_err(|e| Error::string(&e))?;
    Ok(KEY.get_or_init(|| made))
}

/// How many pages' worth of rows one filled page may read (CRM-T326): enough that a caller who
/// may see one row in ten still gets full pages, and few enough that one who may see almost
/// nothing cannot make each request read a whole table.
pub const SCAN_PAGES: u64 = 10;

/// A page [`Page::fill`] filled: the rows the caller may see, and the next page's link.
#[derive(Debug)]
pub struct Filled<T> {
    /// At most the page's limit.
    pub rows: Vec<T>,
    next: Option<HeaderValue>,
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
        Self::with_key(key()?, uri, cap)
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

    /// Fill the page from `fetch(limit, offset)`, which reads rows in the list's order and gives
    /// `None` for each row the caller may not see, as it decides it. It reads again until the page
    /// holds a page of rows, the list ends, or [`SCAN_PAGES`] pages' worth have been read. With
    /// every row shown this is one read of [`Page::bounds`], as before.
    ///
    /// # Errors
    ///
    /// The first error `fetch` returns.
    pub async fn fill<T, F, Fut>(&self, fetch: F) -> Result<Filled<T>>
    where
        F: FnMut(i64, i64) -> Fut,
        Fut: Future<Output = Result<Vec<Option<T>>>>,
    {
        let budget = self
            .0
            .limit()
            .checked_mul(SCAN_PAGES)
            .or_overflow("scan budget")?;
        let filled = self.0.fill(budget, fetch, Option::is_some).await?;
        Ok(Filled {
            rows: filled.rows.into_iter().flatten().collect(),
            next: filled.next,
        })
    }

    /// The JSON array response to a filled page, with its `Link: rel="next"` when more may
    /// follow.
    ///
    /// # Errors
    ///
    /// When the body cannot be serialised.
    pub fn respond_filled<T: Serialize>(&self, filled: Filled<T>) -> Result<Response> {
        let mut response = format::json(filled.rows)?;
        if let Some(link) = filled.next {
            response
                .headers_mut()
                .insert(axum::http::header::LINK, link);
        }
        Ok(response)
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

    /// Walk a filled list from its first page, following each next link: the rows of every
    /// page, and how many reads each page took.
    async fn walk(table: &[u32], limit: u64, shown: fn(u32) -> bool) -> Vec<(Vec<u32>, usize)> {
        let mut pages = Vec::new();
        let mut uri = format!("/api/x?limit={limit}");
        loop {
            let at = page(&uri, 100).unwrap();
            let mut reads = 0;
            let filled = at
                .fill(|take, from| {
                    reads += 1;
                    let (from, take) = (
                        usize::try_from(from).unwrap(),
                        usize::try_from(take).unwrap(),
                    );
                    let rows: Vec<Option<u32>> = table
                        .iter()
                        .skip(from)
                        .take(take)
                        .map(|&row| shown(row).then_some(row))
                        .collect();
                    async move { Ok(rows) }
                })
                .await
                .unwrap();
            let rows = filled.rows.clone();
            let response = at.respond_filled(filled).unwrap();
            let more = response.headers().contains_key(axum::http::header::LINK);
            pages.push((rows, reads));
            if !more {
                return pages;
            }
            uri = next(&response);
        }
    }

    /// A list filtered row by row comes back in full pages, each row once, in order, and the
    /// last page has no next link (CRM-T326).
    #[tokio::test]
    async fn a_filtered_list_fills_its_pages_and_shows_every_row_once() {
        let table: Vec<u32> = (0..20).collect();
        let pages = walk(&table, 3, |row| row % 3 != 1).await;
        let rows: Vec<Vec<u32>> = pages.iter().map(|(rows, _)| rows.clone()).collect();
        assert_eq!(
            rows,
            [
                vec![0, 2, 3],
                vec![5, 6, 8],
                vec![9, 11, 12],
                vec![14, 15, 17],
                vec![18]
            ],
        );
    }

    /// With every row shown it is one read a page, as before.
    #[tokio::test]
    async fn an_unfiltered_list_reads_once_a_page() {
        let table: Vec<u32> = (0..7).collect();
        let pages = walk(&table, 3, |_| true).await;
        assert_eq!(
            pages,
            [(vec![0, 1, 2], 1), (vec![3, 4, 5], 1), (vec![6], 1)],
        );
    }

    /// A caller who may see almost nothing reads at most the budget a request, and gets a short
    /// page with a link on from where the reading stopped, so it still reaches every row.
    #[tokio::test]
    async fn a_page_that_spends_its_budget_comes_back_short_and_goes_on() {
        let table: Vec<u32> = (0..200).collect();
        let pages = walk(&table, 2, |row| row == 150).await;
        let found: Vec<u32> = pages.iter().flat_map(|(rows, _)| rows.clone()).collect();
        assert_eq!(found, [150]);
        // Each request reads ten pages of three rows at most: twenty rows, in batches of three.
        assert!(pages.iter().all(|(_, reads)| *reads <= 7), "{pages:?}");
        assert!(pages.len() > 1, "the budget made it stop and go on");
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
