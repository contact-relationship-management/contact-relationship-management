//! Plain SQL (CRM-D24): each query is written out with `sqlx::query!` and checked against the
//! schema at build time, so a reviewer reads exactly what runs.

use axa::AppContext;
use sqlx::PgPool;

/// The connection pool: axa's own, which its `sqlx` feature makes a `PgPool` (CRM-T232).
#[must_use]
pub fn pool(ctx: &AppContext) -> &PgPool {
    &ctx.db
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::rules::privacy::SOFT_DELETED_TABLES;

    /// Who may write a sealed column in SQL, and why. A write here must bind `EncryptedText`, or
    /// have a reason not to.
    const SEALED_WRITERS: &[(&str, &str)] = &[
        (
            "src/controllers/board.rs",
            "an activity's edit binds `EncryptedText`, which seals the summary",
        ),
        (
            "src/controllers/engagement.rs",
            "a stakeholder declaration's `UPDATE contacts` returns the row, sealed channel \
             included, and writes no sealed column",
        ),
        (
            "src/controllers/forms.rs",
            "a form's new contact binds `EncryptedText`, which seals the channel",
        ),
        (
            "src/controllers/marketing/campaign.rs",
            "a campaign's touch activity binds `EncryptedText`, which seals the summary",
        ),
        (
            "src/controllers/marketing/nurture.rs",
            "a nurture step's touch activity binds `EncryptedText`, which seals the summary",
        ),
        (
            "src/controllers/privacy.rs",
            "erasure writes a plaintext tombstone on purpose",
        ),
        (
            "src/controllers/record_imports.rs",
            "an imported contact binds `EncryptedText`, which seals the channel",
        ),
        (
            "src/controllers/relationships.rs",
            "a contact's channel and an activity's summary bind `EncryptedText`, which seals them",
        ),
        (
            "src/controllers/sales.rs",
            "a converted lead's new contact binds `EncryptedText`, which seals the channel",
        ),
        (
            "src/tasks/seed.rs",
            "a seeded contact's channel binds `EncryptedText`, which seals it",
        ),
        (
            "src/workflows.rs",
            "a task's summary and a set channel bind `EncryptedText`, which seals them",
        ),
    ];

    fn lower_words(sql: &str) -> Vec<String> {
        sql.split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect()
    }

    /// The soft-deleted tables `sql` reads or updates without a word about `deleted_at`.
    fn forgets_deleted_rows(sql: &str) -> Vec<String> {
        let words = lower_words(sql);
        if words.iter().any(|word| word == "deleted_at") {
            return Vec::new();
        }
        words
            .windows(2)
            .filter(|pair| matches!(pair[0].as_str(), "from" | "join" | "update"))
            .map(|pair| pair[1].clone())
            .filter(|table| SOFT_DELETED_TABLES.contains(&table.as_str()))
            .collect()
    }

    /// Whether this Rust source has SQL that writes `contacts.preferred_channel` or
    /// `activities.summary`. It sees SQL whose table is written out, in capitals; a name built at
    /// run time, as in `rotate_field_keys`, is not seen.
    fn writes_sealed_column(source: &str) -> bool {
        [
            ("INSERT INTO contacts", "preferred_channel"),
            ("UPDATE contacts", "preferred_channel"),
            ("INSERT INTO activities", "summary"),
            ("UPDATE activities", "summary"),
        ]
        .iter()
        .any(|(statement, column)| {
            source.match_indices(statement).any(|(at, _)| {
                let rest = &source[at..];
                // To the end of the string literal.
                rest[..rest.find('"').unwrap_or(rest.len())].contains(column)
            })
        })
    }

    fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    #[test]
    fn the_checks_below_can_fail() {
        assert_eq!(
            forgets_deleted_rows("SELECT id FROM contacts c JOIN deals d ON true"),
            ["contacts", "deals"]
        );
        assert_eq!(
            forgets_deleted_rows("UPDATE tickets SET x = 1"),
            ["tickets"]
        );
        assert!(
            forgets_deleted_rows("SELECT id FROM contacts WHERE deleted_at IS NULL").is_empty()
        );
        assert!(forgets_deleted_rows("INSERT INTO contacts (id) VALUES (1)").is_empty());
        assert!(forgets_deleted_rows("SELECT hash FROM audit_logs").is_empty());
        assert!(writes_sealed_column(
            r#""UPDATE activities SET summary = $1""#
        ));
        assert!(writes_sealed_column(
            "\"INSERT INTO contacts (pid, \\\n preferred_channel) VALUES ($1)\""
        ));
        assert!(!writes_sealed_column(
            r#""UPDATE activities SET done = $1" "summary""#
        ));
    }

    /// `find_live` could not forget the filter; plain SQL can. A query that means to see deleted
    /// rows names `deleted_at` anyway, or this list needs a reasoned exception.
    #[test]
    fn no_checked_query_forgets_deleted_rows() {
        let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join(".sqlx");
        let mut queries = 0;
        for entry in std::fs::read_dir(cache).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            let cached: serde_json::Value = serde_json::from_str(&text).unwrap();
            let sql = cached["query"].as_str().unwrap();
            assert_eq!(forgets_deleted_rows(sql), Vec::<String>::new(), "{sql}");
            queries += 1;
        }
        assert!(queries > 0, "the cache is empty, so nothing was checked");
    }

    #[test]
    fn only_the_listed_modules_write_a_sealed_column_in_sql() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        rust_files(&root.join("src"), &mut files);
        assert!(!files.is_empty());
        let mut writers: Vec<String> = files
            .iter()
            .filter(|path| !path.ends_with("src/db.rs"))
            .filter(|path| writes_sealed_column(&std::fs::read_to_string(path).unwrap()))
            .map(|path| path.strip_prefix(root).unwrap().display().to_string())
            .collect();
        writers.sort();
        let listed: Vec<&str> = SEALED_WRITERS.iter().map(|(path, _)| *path).collect();
        assert_eq!(writers, listed);
    }
}
