//! Pagination for list endpoints (CRM-T51).
//!
//! Every list used to cut its result at a fixed cap and say nothing. Now a
//! list takes `?limit=` (1 to the endpoint's cap, default the cap) and an
//! opaque `?cursor=`, and when more rows exist the response carries
//! `Link: <...>; rel="next"`. The body stays a bare JSON array, so an existing
//! client keeps working until it passes the cap, and then it is told rather
//! than truncated.
//!
//! The cursor is an offset, so a row inserted or removed between two page
//! fetches can make a later page skip or repeat one row; every order here
//! ends in `id` so the order itself is stable. There is deliberately no
//! total count: a total over stored rows would include rows a caller's
//! record-level policy hides.

use axum::http::{StatusCode, Uri};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, QuerySelect, Select};
use serde::{Deserialize, Serialize};

/// The `limit` and `cursor` query parameters, taken alongside an
/// endpoint's own filters (a second `Query` extractor).
#[derive(Debug, Default, Deserialize)]
pub struct PageParams {
    /// Rows wanted, 1 to the endpoint cap.
    #[serde(default)]
    pub limit: Option<u64>,
    /// Opaque position from a previous `Link: rel="next"`.
    #[serde(default)]
    pub cursor: Option<String>,
}

/// A validated page request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    limit: u64,
    offset: u64,
}

fn bad(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

/// Encode an offset as an opaque cursor.
#[must_use]
pub fn encode_cursor(offset: u64) -> String {
    URL_SAFE_NO_PAD.encode(format!("o:{offset}"))
}

fn decode_cursor(cursor: &str) -> Option<u64> {
    let bytes = URL_SAFE_NO_PAD.decode(cursor).ok()?;
    String::from_utf8(bytes)
        .ok()?
        .strip_prefix("o:")?
        .parse()
        .ok()
}

impl Page {
    /// Validate `params` against the endpoint's `cap`.
    ///
    /// # Errors
    ///
    /// `422` for a `limit` outside `1..=cap` or a cursor that is not one of
    /// ours.
    pub fn new(params: &PageParams, cap: u64) -> Result<Self> {
        let limit = params.limit.unwrap_or(cap);
        if !(1..=cap).contains(&limit) {
            return Err(bad(&format!("limit must be between 1 and {cap}")));
        }
        let offset = match params.cursor.as_deref() {
            None | Some("") => 0,
            Some(cursor) => decode_cursor(cursor).ok_or_else(|| bad("cursor is not valid"))?,
        };
        Ok(Self { limit, offset })
    }

    /// Restrict `select` to this page, fetching one extra row to learn
    /// whether another page exists.
    #[must_use]
    pub fn apply<E: EntityTrait>(&self, select: Select<E>) -> Select<E> {
        select.offset(self.offset).limit(self.limit + 1)
    }

    /// Drop the probe row; the flag says whether more rows follow.
    #[must_use]
    pub fn split<T>(&self, mut rows: Vec<T>) -> (Vec<T>, bool) {
        let more = u64::try_from(rows.len()).unwrap_or(u64::MAX) > self.limit;
        rows.truncate(usize::try_from(self.limit).unwrap_or(usize::MAX));
        (rows, more)
    }

    /// The JSON array response, with `Link: rel="next"` when `more`.
    ///
    /// # Errors
    ///
    /// When the body cannot be serialised.
    pub fn respond<T: Serialize>(&self, body: T, more: bool, uri: &Uri) -> Result<Response> {
        let mut response = format::json(body)?;
        if more {
            let kept: Vec<&str> = uri
                .query()
                .unwrap_or("")
                .split('&')
                .filter(|part| !part.is_empty() && !part.starts_with("cursor="))
                .collect();
            let next = encode_cursor(self.offset + self.limit);
            let query = if kept.is_empty() {
                format!("cursor={next}")
            } else {
                format!("{}&cursor={next}", kept.join("&"))
            };
            if let Ok(value) = axum::http::HeaderValue::from_str(&format!(
                "<{}?{query}>; rel=\"next\"",
                uri.path()
            )) {
                response
                    .headers_mut()
                    .insert(axum::http::header::LINK, value);
            }
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(limit: Option<u64>, cursor: Option<&str>) -> PageParams {
        PageParams {
            limit,
            cursor: cursor.map(str::to_string),
        }
    }

    #[test]
    fn defaults_to_the_cap_and_rejects_out_of_range() {
        let page = Page::new(&params(None, None), 100).unwrap();
        assert_eq!((page.limit, page.offset), (100, 0));
        assert!(Page::new(&params(Some(0), None), 100).is_err());
        assert!(Page::new(&params(Some(101), None), 100).is_err());
        assert!(Page::new(&params(Some(1), None), 100).is_ok());
    }

    #[test]
    fn cursors_round_trip_and_junk_is_refused() {
        let cursor = encode_cursor(250);
        let page = Page::new(&params(Some(50), Some(&cursor)), 100).unwrap();
        assert_eq!(page.offset, 250);
        assert!(Page::new(&params(None, Some("not a cursor")), 100).is_err());
        assert!(Page::new(&params(None, Some(&URL_SAFE_NO_PAD.encode("x:1"))), 100).is_err());
    }

    #[test]
    fn split_reports_whether_more_rows_follow() {
        let page = Page::new(&params(Some(2), None), 10).unwrap();
        assert_eq!(page.split(vec![1, 2, 3]), (vec![1, 2], true));
        assert_eq!(page.split(vec![1, 2]), (vec![1, 2], false));
        assert_eq!(page.split(Vec::<i32>::new()), (vec![], false));
    }

    #[test]
    fn next_link_keeps_filters_and_replaces_the_cursor() {
        let page = Page::new(&params(Some(2), Some(&encode_cursor(2))), 10).unwrap();
        let uri: Uri = format!(
            "/api/tickets?status=open&limit=2&cursor={}",
            encode_cursor(2)
        )
        .parse()
        .unwrap();
        let response = page.respond(vec![1, 2], true, &uri).unwrap();
        let link = response.headers()[axum::http::header::LINK]
            .to_str()
            .unwrap();
        assert_eq!(
            link,
            format!(
                "</api/tickets?status=open&limit=2&cursor={}>; rel=\"next\"",
                encode_cursor(4)
            )
        );
        let none = page.respond(vec![1], false, &uri).unwrap();
        assert!(none.headers().get(axum::http::header::LINK).is_none());
    }
}
