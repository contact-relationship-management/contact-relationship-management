//! The baseline stands for the `SeaORM` history (CRM-D24, CRM-T229): it builds the same catalog,
//! a database the history built takes it without running it, and a partial history is refused.
//! Each test makes its own databases beside the test database and drops them after.

use contact_relationship_management::schema::{self, BASELINE};
use migration::{Migrator, MigratorTrait as _};
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

    /// Run the `SeaORM` migrations, all of them or the first `steps`.
    async fn history(&self, steps: Option<u32>) {
        let db = sea_orm::Database::connect(&self.url).await.unwrap();
        Migrator::up(&db, steps).await.unwrap();
        db.close().await.unwrap();
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
async fn the_baseline_builds_the_catalog_the_history_built() {
    let (history, baseline) = (Scratch::new().await, Scratch::new().await);
    history.history(None).await;
    let built = baseline.pool().await;
    schema::migrate(&built).await.unwrap();
    let old = history.pool().await;
    let (from_history, from_baseline) = (catalog(&old).await, catalog(&built).await);
    assert!(from_history.len() > 1_000, "{} lines", from_history.len());
    let only_history: Vec<_> = from_history
        .iter()
        .filter(|l| !from_baseline.contains(l))
        .collect();
    let only_baseline: Vec<_> = from_baseline
        .iter()
        .filter(|l| !from_history.contains(l))
        .collect();
    assert!(
        only_history.is_empty() && only_baseline.is_empty(),
        "only in the history: {only_history:#?}\nonly in the baseline: {only_baseline:#?}"
    );
    let ran = recorded(&built).await;
    assert_eq!(ran.len(), 1);
    assert_eq!(ran[0].0, BASELINE);
    assert!(ran[0].1 >= 0, "a new database runs the baseline");
    old.close().await;
    built.close().await;
    history.drop_now().await;
    baseline.drop_now().await;
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
