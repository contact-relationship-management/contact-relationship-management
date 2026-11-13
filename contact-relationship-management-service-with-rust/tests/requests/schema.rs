//! The baseline stands for the `SeaORM` history (CRM-D24, CRM-T229): a database the history built
//! takes it without running it, and a partial history is refused. Each test makes its own
//! databases beside the test database and drops them after.
//!
//! The `SeaORM` migrations went with the `migration` crate (CRM-T232), and with them the test that
//! the baseline builds the catalog they built (it passed at `29e9922`). A database they built is
//! made here as they left it: the baseline's schema, and `seaql_migrations` listing the history.

use contact_relationship_management::schema::{self, BASELINE, HISTORY};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

/// Everything a migration defines in `public`, one line each, sorted: columns with their place,
/// types,
/// nullability, defaults and identity; constraints; indexes; functions; triggers; views; enum
/// types; sequences; comments; and the extensions. The migrators' own tables are left out.
const CATALOG: &str = r"
WITH own AS (
    SELECT c.oid, c.relname FROM pg_class c
    WHERE c.relnamespace = 'public'::regnamespace AND c.relkind IN ('r', 'p', 'v', 'm')
      AND c.relname NOT IN ('seaql_migrations', '_sqlx_migrations')
)
SELECT line FROM (
    SELECT 'column ' || own.relname || '.' || a.attname
        || ' #' || row_number() OVER (PARTITION BY a.attrelid ORDER BY a.attnum)
        || ' ' || format_type(a.atttypid, a.atttypmod)
        || CASE WHEN a.attnotnull THEN ' not null' ELSE '' END
        || coalesce(' default ' || pg_get_expr(d.adbin, d.adrelid), '')
        || CASE WHEN a.attidentity <> '' THEN ' identity ' || a.attidentity::text ELSE '' END
        || CASE WHEN a.attgenerated <> '' THEN ' generated' ELSE '' END AS line
    FROM pg_attribute a JOIN own ON own.oid = a.attrelid
    LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
    WHERE a.attnum > 0 AND NOT a.attisdropped
    UNION ALL
    SELECT 'constraint ' || own.relname || ' ' || k.conname || ' ' || pg_get_constraintdef(k.oid)
    FROM pg_constraint k JOIN own ON own.oid = k.conrelid
    UNION ALL
    SELECT 'index ' || i.indexdef FROM pg_indexes i JOIN own ON own.relname = i.tablename
    WHERE i.schemaname = 'public'
    UNION ALL
    SELECT 'function ' || pg_get_functiondef(p.oid) FROM pg_proc p
    WHERE p.pronamespace = 'public'::regnamespace
      AND NOT EXISTS (SELECT 1 FROM pg_depend e WHERE e.objid = p.oid AND e.deptype = 'e')
    UNION ALL
    SELECT 'trigger ' || pg_get_triggerdef(t.oid) FROM pg_trigger t JOIN own ON own.oid = t.tgrelid
    WHERE NOT t.tgisinternal
    UNION ALL
    SELECT 'view ' || v.viewname || ' ' || v.definition FROM pg_views v WHERE v.schemaname = 'public'
    UNION ALL
    SELECT 'type ' || t.typname || ' ' || string_agg(e.enumlabel, ',' ORDER BY e.enumsortorder)
    FROM pg_type t JOIN pg_enum e ON e.enumtypid = t.oid
    WHERE t.typnamespace = 'public'::regnamespace GROUP BY t.typname
    UNION ALL
    SELECT 'sequence ' || s.sequencename || ' ' || s.data_type::text || ' ' || s.increment_by
    FROM pg_sequences s WHERE s.schemaname = 'public'
    UNION ALL
    SELECT 'comment ' || own.relname || coalesce('.' || a.attname, '') || ' ' || d.description
    FROM pg_description d JOIN own ON own.oid = d.objoid AND d.classoid = 'pg_class'::regclass
    LEFT JOIN pg_attribute a ON a.attrelid = own.oid AND a.attnum = d.objsubid AND d.objsubid > 0
    UNION ALL
    SELECT 'extension ' || x.extname FROM pg_extension x
) AS catalog
ORDER BY line";

/// A database of its own beside the test database, dropped by [`Scratch::drop_now`].
struct Scratch {
    admin: PgPool,
    name: String,
    url: String,
}

impl Scratch {
    async fn new() -> Self {
        let base = std::env::var("DB_URL").expect("DB_URL");
        let admin = PgPool::connect(&base).await.unwrap();
        let name = format!("crm_schema_{}", Uuid::new_v4().simple());
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&admin)
            .await
            .unwrap();
        let (head, query) = base
            .split_once('?')
            .map_or((base.as_str(), None), |(h, q)| (h, Some(q)));
        let (server, _) = head.rsplit_once('/').unwrap();
        let url = match query {
            Some(query) => format!("{server}/{name}?{query}"),
            None => format!("{server}/{name}"),
        };
        Self { admin, name, url }
    }

    async fn pool(&self) -> PgPool {
        PgPool::connect(&self.url).await.unwrap()
    }

    /// Make the database the `SeaORM` migrations left, all of them or the first `steps`: the
    /// baseline's schema (it is that schema, CRM-T229) and their `seaql_migrations` table.
    async fn history(&self, steps: Option<usize>) {
        let pool = self.pool().await;
        sqlx::raw_sql(include_str!("../../migrations/20261110000000_baseline.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE seaql_migrations (version varchar NOT NULL PRIMARY KEY, \
             applied_at bigint NOT NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        for version in &HISTORY[..steps.unwrap_or(HISTORY.len())] {
            sqlx::query("INSERT INTO seaql_migrations (version, applied_at) VALUES ($1, 0)")
                .bind(version)
                .execute(&pool)
                .await
                .unwrap();
        }
        pool.close().await;
    }

    async fn drop_now(self) {
        let sql = format!("DROP DATABASE {} WITH (FORCE)", self.name);
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&self.admin)
            .await
            .unwrap();
    }
}

async fn catalog(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(CATALOG).fetch_all(pool).await.unwrap()
}

/// What the baseline recorded: `(version, execution_time)`, `-1` for one skipped.
async fn recorded(pool: &PgPool) -> Vec<(i64, i64)> {
    sqlx::query_as("SELECT version, execution_time FROM _sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_database_the_history_built_takes_the_baseline_without_running_it() {
    let deployed = Scratch::new().await;
    deployed.history(None).await;
    let pool = deployed.pool().await;
    sqlx::raw_sql("INSERT INTO pipelines (pid, name) VALUES (gen_random_uuid(), 'kept')")
        .execute(&pool)
        .await
        .unwrap();
    let before = catalog(&pool).await;

    schema::migrate(&pool).await.unwrap();
    assert_eq!(
        recorded(&pool).await,
        vec![(BASELINE, -1)],
        "recorded, not run"
    );
    assert_eq!(catalog(&pool).await, before);
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM pipelines WHERE name = 'kept'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, 1);
    let history: i64 = sqlx::query_scalar("SELECT count(*) FROM seaql_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(history, 59, "the history is left where it is");

    // A second start, or a second replica, finds the baseline recorded and does nothing.
    schema::migrate(&pool).await.unwrap();
    assert_eq!(recorded(&pool).await, vec![(BASELINE, -1)]);
    pool.close().await;
    deployed.drop_now().await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_database_with_part_of_the_history_is_refused() {
    let behind = Scratch::new().await;
    behind.history(Some(58)).await;
    let pool = behind.pool().await;
    let refused = schema::migrate(&pool).await.unwrap_err().to_string();
    assert!(refused.contains("lacks 1 of the 59"), "{refused}");
    let recorded: bool =
        sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!recorded, "nothing is recorded for a refused database");
    pool.close().await;
    behind.drop_now().await;
}
