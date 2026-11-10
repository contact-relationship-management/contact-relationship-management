//! [`parse_pid`]. The finders are in [`super::rows`] on `sqlx` (CRM-D24, CRM-T216).

use axa::prelude::*;
use uuid::Uuid;

/// Parse a path `pid` string to a [`Uuid`], mapping failure to `404`.
///
/// # Errors
///
/// [`Error::NotFound`] when `pid` is not a UUID.
pub fn parse_pid(pid: &str) -> Result<Uuid> {
    Uuid::parse_str(pid).map_err(|_| Error::NotFound)
}
