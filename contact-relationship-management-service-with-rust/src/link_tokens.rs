//! Single-use link tokens, kept only as a hash (CRM-R79).
//!
//! The links a person is mailed to **confirm** a subscription or to open their **preference
//! page** carry a random token. The token is made here, shown once (in the link), and never stored:
//! the database keeps its SHA-256, so a leaked copy of the table opens nothing. A token **works
//! once**: it is spent by the request that acts on it, in the same transaction as the act, so a
//! failed act leaves it unspent. Looking at a page does not spend it, because mail scanners follow
//! links. A token expires, and expired and spent rows are deleted after a week.
//!
//! The unsubscribe link is deliberately not one of these: it must work from any old message, so it
//! is a signed, stateless token (see [`crate::unsubscribe`]).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A confirmation link (double opt-in); `detail` is the purpose's index.
pub const CONFIRM: &str = "confirm";
/// A preference-page link; `detail` is unused.
pub const PREFERENCES: &str = "preferences";

/// How long after it expires or is spent a token's row is kept, in days.
pub const KEEP_DAYS: i64 = 7;

/// What a token was made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The contact it names.
    pub contact: Uuid,
    /// The purpose index for a confirmation; `0` otherwise.
    pub detail: u8,
    /// When it stops working.
    pub expires_at: DateTime<Utc>,
}

/// A new random token: 256 bits from the operating system's generator, as 43 URL-safe characters.
#[must_use]
pub fn generate() -> String {
    let mut bytes = [0_u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

/// What the database keeps of a token: the lower-case hex of its SHA-256.
#[must_use]
pub fn hash(token: &str) -> String {
    use std::fmt::Write as _;
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(token.as_bytes()) {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Whether `token` could be one of ours, so a path segment of any other shape never reaches the
/// database.
fn well_formed(token: &str) -> bool {
    token.len() == 43
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn stmt(sql: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// Make a token of `kind` for `contact`, valid until `expires_at`, and return it. This is the only
/// time the token itself exists here.
///
/// # Errors
///
/// When the row cannot be written.
pub async fn issue<C: ConnectionTrait>(
    db: &C,
    kind: &str,
    contact: Uuid,
    detail: u8,
    expires_at: DateTime<Utc>,
) -> Result<String, sea_orm::DbErr> {
    let token = generate();
    db.execute_raw(stmt(
        "INSERT INTO link_tokens (token_hash, kind, contact_pid, detail, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
        [
            hash(&token).into(),
            kind.into(),
            contact.into(),
            i16::from(detail).into(),
            expires_at.into(),
        ],
    ))
    .await?;
    Ok(token)
}

fn target(row: &sea_orm::QueryResult) -> Result<Target, sea_orm::DbErr> {
    Ok(Target {
        contact: row.try_get("", "contact_pid")?,
        detail: u8::try_from(row.try_get::<i16>("", "detail")?).unwrap_or(0),
        expires_at: row.try_get("", "expires_at")?,
    })
}

/// What an unspent, unexpired token of `kind` names, **without spending it**: for the page a link
/// opens, which must change nothing.
///
/// # Errors
///
/// When the database cannot be read.
pub async fn peek<C: ConnectionTrait>(
    db: &C,
    kind: &str,
    token: &str,
) -> Result<Option<Target>, sea_orm::DbErr> {
    if !well_formed(token) {
        return Ok(None);
    }
    db.query_one_raw(stmt(
        "SELECT contact_pid, detail, expires_at FROM link_tokens \
         WHERE token_hash = $1 AND kind = $2 AND used_at IS NULL AND expires_at > now()",
        [hash(token).into(), kind.into()],
    ))
    .await?
    .as_ref()
    .map(target)
    .transpose()
}

/// Spend a token of `kind`: the one request that finds it unspent and unexpired gets what it names,
/// and every later one gets `None`. Call it inside the transaction of the act it allows.
///
/// # Errors
///
/// When the database cannot be written.
pub async fn consume<C: ConnectionTrait>(
    db: &C,
    kind: &str,
    token: &str,
) -> Result<Option<Target>, sea_orm::DbErr> {
    if !well_formed(token) {
        return Ok(None);
    }
    db.query_one_raw(stmt(
        "UPDATE link_tokens SET used_at = now() \
         WHERE token_hash = $1 AND kind = $2 AND used_at IS NULL AND expires_at > now() \
         RETURNING contact_pid, detail, expires_at",
        [hash(token).into(), kind.into()],
    ))
    .await?
    .as_ref()
    .map(target)
    .transpose()
}

/// Delete the rows of tokens that expired or were spent more than [`KEEP_DAYS`] ago.
///
/// # Errors
///
/// When the database cannot be written.
pub async fn sweep<C: ConnectionTrait>(db: &C) -> Result<u64, sea_orm::DbErr> {
    let done = db
        .execute_raw(stmt(
            "DELETE FROM link_tokens WHERE expires_at < now() - make_interval(days => $1::int) \
             OR used_at < now() - make_interval(days => $1::int)",
            [KEEP_DAYS.into()],
        ))
        .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_long_random_url_safe_and_never_repeats() {
        let a = generate();
        assert_eq!(a.len(), 43);
        assert!(well_formed(&a));
        assert_ne!(a, generate());
    }

    #[test]
    fn what_is_kept_is_a_hash_that_does_not_contain_the_token() {
        let token = generate();
        let kept = hash(&token);
        assert_eq!(kept.len(), 64);
        assert!(kept.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(!kept.contains(&token));
        assert_eq!(kept, hash(&token), "the same token always hashes the same");
        assert_ne!(kept, hash(&generate()));
        // A known value, so the hash cannot change by accident: SHA-256 of "abc".
        assert_eq!(
            hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn only_the_right_shape_reaches_the_database() {
        assert!(!well_formed(""));
        assert!(!well_formed("garbage"));
        assert!(!well_formed(&"a".repeat(44)));
        assert!(!well_formed(&format!("{}/", "a".repeat(42))));
        assert!(!well_formed(&format!("{}%", "a".repeat(42))));
        assert!(well_formed(
            &"a-_".repeat(15).chars().take(43).collect::<String>()
        ));
    }
}
