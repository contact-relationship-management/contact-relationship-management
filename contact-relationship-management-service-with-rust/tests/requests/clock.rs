//! One clock for records (CRM-T221): the service's times follow the database's clock, however
//! far the host's is from it.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use contact_relationship_management::clock;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

/// The host's clock and the database's, on this machine, right now: the gap the service has to
/// make up. Printed so a run with the clocks apart can be told from one with them in step.
async fn host_gap(db: &sea_orm::DatabaseConnection) -> chrono::TimeDelta {
    let row = db
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT now() AS now",
        ))
        .await
        .unwrap()
        .unwrap();
    let database: chrono::DateTime<chrono::FixedOffset> = row.try_get("", "now").unwrap();
    let gap = database.to_utc() - chrono::Utc::now();
    println!("database minus host: {} ms", gap.num_milliseconds());
    gap
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn the_service_and_the_database_stamp_a_ticket_on_one_clock() {
    request::<App, _, _>(|request, ctx| async move {
        let gap = host_gap(&ctx.db).await;
        assert!(
            (clock::offset() - gap).abs() < chrono::TimeDelta::seconds(1),
            "measured {} ms, the gap is {} ms",
            clock::offset().num_milliseconds(),
            gap.num_milliseconds()
        );
        let made: Value = request
            .post("/api/tickets")
            .json(&json!({ "title": "One clock", "priority": "normal" }))
            .await
            .json();
        // `opened_at` is the service's, `created_at` the database's default (CRM-T220 saw them
        // 539 seconds apart).
        let row = ctx
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT extract(epoch FROM opened_at - created_at)::float8 AS apart \
                 FROM tickets WHERE pid = $1::uuid",
                [made["pid"].as_str().unwrap().into()],
            ))
            .await
            .unwrap()
            .unwrap();
        let apart: f64 = row.try_get("", "apart").unwrap();
        assert!(apart.abs() < 1.0, "opened_at is {apart} s from created_at");
    })
    .await;
}
