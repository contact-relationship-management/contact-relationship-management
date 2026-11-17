//! Tamper-evident audit chain (CRM-T31): each audit row carries the hash of
//! the row before it, so altering, deleting, or reordering any row breaks
//! every later link. Pure and DB-free.
//!
//! Limits, stated: a chain proves internal consistency. Deleting the newest
//! rows leaves a valid shorter chain, so operators should anchor the
//! reported `head` hash outside the database (log shipping, WORM storage).

use sha2::{Digest, Sha256};

/// The fields of one audit row that the hash covers.
#[derive(Debug, Clone)]
pub struct Link<'a> {
    /// Row id.
    pub id: i64,
    /// Hash of the previous row, empty for the first chained row.
    pub prev_hash: &'a str,
    /// Entity kind.
    pub entity: &'a str,
    /// Entity pid, as text.
    pub entity_pid: &'a str,
    /// Action name.
    pub action: &'a str,
    /// Actor, if any.
    pub actor: Option<&'a str>,
    /// Snapshot as compact JSON text, if any.
    pub snapshot: Option<&'a str>,
    /// Creation time in microseconds since the epoch.
    pub created_micros: i64,
}

fn put(hasher: &mut Sha256, field: &[u8]) {
    // Length-prefix every field so ("ab","c") and ("a","bc") differ.
    hasher.update((field.len() as u64).to_be_bytes());
    hasher.update(field);
}

/// SHA-256 over the length-prefixed fields, lowercase hex.
#[must_use]
pub fn hash(link: &Link<'_>) -> String {
    let mut hasher = Sha256::new();
    put(&mut hasher, &link.id.to_be_bytes());
    put(&mut hasher, link.prev_hash.as_bytes());
    put(&mut hasher, link.entity.as_bytes());
    put(&mut hasher, link.entity_pid.as_bytes());
    put(&mut hasher, link.action.as_bytes());
    put(&mut hasher, link.actor.map_or(&[][..], str::as_bytes));
    // Distinguish absent from empty.
    put(&mut hasher, &[u8::from(link.actor.is_some())]);
    put(&mut hasher, link.snapshot.map_or(&[][..], str::as_bytes));
    put(&mut hasher, &[u8::from(link.snapshot.is_some())]);
    put(&mut hasher, &link.created_micros.to_be_bytes());
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// One stored row as the verifier sees it.
#[derive(Debug, Clone)]
pub struct Stored<'a> {
    /// The hashed fields.
    pub link: Link<'a>,
    /// The stored hash, `None` for rows written before the chain existed.
    pub hash: Option<&'a str>,
}

/// The verifier's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// Rows examined.
    pub rows: usize,
    /// Rows written before the chain existed (leading, unchained).
    pub legacy_rows: usize,
    /// The newest verified hash, for external anchoring.
    pub head: Option<String>,
    /// The first broken row id and why, if any.
    pub first_break: Option<(i64, &'static str)>,
}

impl Verdict {
    /// Whether the chain is intact.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.first_break.is_none()
    }
}

/// Verify rows ordered by ascending id.
#[must_use]
pub fn verify<'a>(rows: impl IntoIterator<Item = Stored<'a>>) -> Verdict {
    let mut verdict = Verdict {
        rows: 0,
        legacy_rows: 0,
        head: None,
        first_break: None,
    };
    let mut expected_prev = String::new();
    let mut chained = false;
    for row in rows {
        verdict.rows += 1;
        let Some(stored) = row.hash else {
            if chained {
                verdict.first_break = Some((row.link.id, "unchained row after the chain began"));
                return verdict;
            }
            verdict.legacy_rows += 1;
            continue;
        };
        chained = true;
        if row.link.prev_hash != expected_prev {
            verdict.first_break = Some((
                row.link.id,
                "previous-hash link broken (row deleted or reordered)",
            ));
            return verdict;
        }
        if hash(&row.link) != stored {
            verdict.first_break = Some((row.link.id, "row content does not match its hash"));
            return verdict;
        }
        expected_prev = stored.to_string();
        verdict.head = Some(expected_prev.clone());
    }
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Row {
        id: i64,
        prev: String,
        action: String,
        hash: Option<String>,
    }

    fn link(row: &Row) -> Link<'_> {
        Link {
            id: row.id,
            prev_hash: &row.prev,
            entity: "deal",
            entity_pid: "p",
            action: &row.action,
            actor: Some("a"),
            snapshot: Some("{}"),
            created_micros: 1,
        }
    }

    fn chain(n: i64) -> Vec<Row> {
        let mut rows: Vec<Row> = Vec::new();
        for id in 1..=n {
            let prev = rows.last().and_then(|r| r.hash.clone()).unwrap_or_default();
            let mut row = Row {
                id,
                prev,
                action: format!("act{id}"),
                hash: None,
            };
            row.hash = Some(hash(&link(&row)));
            rows.push(row);
        }
        rows
    }

    fn run(rows: &[Row]) -> Verdict {
        verify(rows.iter().map(|r| Stored {
            link: link(r),
            hash: r.hash.as_deref(),
        }))
    }

    #[test]
    fn intact_chain_verifies_and_reports_head() {
        let rows = chain(3);
        let v = run(&rows);
        assert!(v.ok());
        assert_eq!(v.rows, 3);
        assert_eq!(v.head, rows[2].hash);
    }

    #[test]
    fn empty_chain_is_ok() {
        assert!(run(&[]).ok());
    }

    #[test]
    fn altered_row_is_caught_at_that_row() {
        let mut rows = chain(3);
        rows[1].action = "tampered".into();
        assert_eq!(run(&rows).first_break.map(|b| b.0), Some(2));
    }

    #[test]
    fn deleted_row_is_caught_at_the_next_row() {
        let mut rows = chain(4);
        rows.remove(1);
        assert_eq!(run(&rows).first_break.map(|b| b.0), Some(3));
    }

    #[test]
    fn reordered_rows_are_caught() {
        let mut rows = chain(3);
        rows.swap(1, 2);
        assert!(!run(&rows).ok());
    }

    #[test]
    fn leading_unchained_rows_are_legacy_not_a_break() {
        let mut rows = chain(2);
        rows.insert(
            0,
            Row {
                id: 0,
                prev: String::new(),
                action: "old".into(),
                hash: None,
            },
        );
        let v = run(&rows);
        assert!(v.ok());
        assert_eq!(v.legacy_rows, 1);
    }

    #[test]
    fn unchained_row_after_the_chain_began_is_a_break() {
        let mut rows = chain(2);
        rows.push(Row {
            id: 3,
            prev: String::new(),
            action: "x".into(),
            hash: None,
        });
        assert_eq!(run(&rows).first_break.map(|b| b.0), Some(3));
    }

    #[test]
    fn field_boundaries_are_unambiguous() {
        let a = Link {
            id: 1,
            prev_hash: "",
            entity: "ab",
            entity_pid: "c",
            action: "x",
            actor: None,
            snapshot: None,
            created_micros: 0,
        };
        let b = Link {
            entity: "a",
            entity_pid: "bc",
            ..a.clone()
        };
        assert_ne!(hash(&a), hash(&b));
        let some_empty = Link {
            actor: Some(""),
            ..a.clone()
        };
        assert_ne!(hash(&a), hash(&some_empty));
    }
}
