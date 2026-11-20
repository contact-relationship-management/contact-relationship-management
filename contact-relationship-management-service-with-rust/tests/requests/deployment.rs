//! A database belongs to one deployment (CRM-R105, CRM-T282): the first start with
//! `CRM_DEPLOYMENT_ID` claims it, a start with another name or none is refused after, and the
//! claimed name cannot be changed or removed.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use contact_relationship_management::deployment::check;
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_first_named_start_claims_the_database_and_another_name_is_refused() {
    request::<App, _, _>(|_request, ctx| async move {
        let pool = contact_relationship_management::db::pool(&ctx);
        let held = || async {
            sqlx::query_scalar::<_, String>("SELECT name FROM deployment")
                .fetch_optional(pool)
                .await
                .unwrap()
        };

        // Unclaimed and unnamed: allowed, with a note, and nothing written.
        let note = check(pool, None).await.unwrap();
        assert!(note.is_some_and(|n| n.contains("not set")));
        assert_eq!(held().await, None);

        // The first named start claims it; the same name again matches.
        assert_eq!(check(pool, Some("customer-a")).await, Ok(None));
        assert_eq!(held().await.as_deref(), Some("customer-a"));
        assert_eq!(check(pool, Some("customer-a")).await, Ok(None));

        // Another name, or none, is refused, and the claim stands.
        let other = check(pool, Some("customer-b")).await.unwrap_err();
        assert!(
            other.contains("'customer-a'") && other.contains("'customer-b'"),
            "{other}"
        );
        let unnamed = check(pool, None).await.unwrap_err();
        assert!(unnamed.contains("not set"), "{unnamed}");
        assert_eq!(held().await.as_deref(), Some("customer-a"));

        // The row cannot be changed or removed, and a second one cannot be added.
        for statement in [
            "UPDATE deployment SET name = 'customer-b'",
            "DELETE FROM deployment",
            "INSERT INTO deployment (name) VALUES ('customer-b')",
        ] {
            let refused = sqlx::raw_sql(sqlx::AssertSqlSafe(statement))
                .execute(pool)
                .await;
            assert!(refused.is_err(), "{statement} was allowed");
        }
        assert_eq!(held().await.as_deref(), Some("customer-a"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_name_the_table_would_refuse_is_refused_before_it_is_written() {
    request::<App, _, _>(|_request, ctx| async move {
        let pool = contact_relationship_management::db::pool(&ctx);
        assert!(check(pool, Some("a b")).await.is_err());
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM deployment")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
        // The table's own check agrees with `valid`.
        assert!(
            sqlx::raw_sql("INSERT INTO deployment (name) VALUES ('a b')")
                .execute(pool)
                .await
                .is_err()
        );
    })
    .await;
}
