//! Plain SQL (CRM-D24): each query is written out with `sqlx::query!` and checked against the
//! schema at build time, so a reviewer reads exactly what runs. While `SeaORM` is being retired,
//! both share one pool. A handler moves whole, because a transaction belongs to one library and
//! cannot span the two; [`Bridge`] carries a moved handler's transaction into a helper that has
//! not moved yet.

use async_trait::async_trait;
use axa::AppContext;
use sea_orm::sea_query::Values;
use sea_orm::{ConnectionTrait, DbBackend, DbErr, ExecResult, QueryResult, RuntimeErr, Statement};
use sea_query_sqlx::SqlxValues;
use sqlx::{AssertSqlSafe, Executor, PgConnection, PgPool, Postgres};
use tokio::sync::Mutex;

/// The connection pool, which is `SeaORM`'s own.
#[must_use]
pub fn pool(ctx: &AppContext) -> &PgPool {
    ctx.db.get_postgres_connection_pool()
}

/// A count or an offset as Postgres binds it. Past `i64::MAX` it saturates, which no table reaches.
#[must_use]
pub fn bound(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// A `sqlx` connection that `SeaORM` code can run on, so a moved handler can call a helper that
/// has not moved, inside its own transaction. It goes when the last helper has moved.
///
/// It lends only [`ConnectionTrait`]. A helper that begins a transaction or a savepoint through
/// `TransactionTrait` does not compile against it, which is the safe way for that to fail.
pub struct Bridge<'c> {
    // `ConnectionTrait` takes `&self`, and a connection runs one statement at a time.
    conn: Mutex<&'c mut PgConnection>,
}

impl<'c> Bridge<'c> {
    /// Lend `conn` to `SeaORM` code until this value is dropped.
    pub fn new(conn: &'c mut PgConnection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

/// The statement as `sqlx` runs it, with the values `SeaORM` built.
fn query(stmt: &Statement) -> sqlx::query::Query<'_, Postgres, SqlxValues> {
    let values = stmt.values.clone().unwrap_or(Values(Vec::new()));
    sqlx::query_with(AssertSqlSafe(stmt.sql.as_str()), SqlxValues(values))
}

/// The errors `SeaORM`'s own driver gives, so a helper handles them the same here.
fn exec_err(error: sqlx::Error) -> DbErr {
    DbErr::Exec(RuntimeErr::SqlxError(error.into()))
}

fn query_err(error: sqlx::Error) -> DbErr {
    DbErr::Query(RuntimeErr::SqlxError(error.into()))
}

#[async_trait]
impl ConnectionTrait for Bridge<'_> {
    fn get_database_backend(&self) -> DbBackend {
        DbBackend::Postgres
    }

    async fn execute_raw(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        let mut conn = self.conn.lock().await;
        query(&stmt)
            .execute(&mut **conn)
            .await
            .map(Into::into)
            .map_err(exec_err)
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        let mut conn = self.conn.lock().await;
        (&mut **conn)
            .execute(AssertSqlSafe(sql.to_owned()))
            .await
            .map(Into::into)
            .map_err(exec_err)
    }

    async fn query_one_raw(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        let mut conn = self.conn.lock().await;
        query(&stmt)
            .fetch_optional(&mut **conn)
            .await
            .map(|row| row.map(Into::into))
            .map_err(query_err)
    }

    async fn query_all_raw(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        let mut conn = self.conn.lock().await;
        query(&stmt)
            .fetch_all(&mut **conn)
            .await
            .map(|rows| rows.into_iter().map(Into::into).collect())
            .map_err(query_err)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::bound;
    use crate::rules::privacy::SOFT_DELETED_TABLES;

    /// Who may write a sealed column in SQL, and why. A write here must bind `EncryptedText`, or
    /// have a reason not to.
    const SEALED_WRITERS: &[(&str, &str)] = &[(
        "src/controllers/privacy.rs",
        "erasure writes a plaintext tombstone on purpose",
    )];

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

    #[test]
    fn a_bound_saturates() {
        assert_eq!(bound(0), 0);
        assert_eq!(bound(i64::MAX as u64), i64::MAX);
        assert_eq!(bound(u64::MAX), i64::MAX);
    }
}
