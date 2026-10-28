//! The personal-data inventory and retention schedule (`spec/compliance/personal-data-inventory.md`)
//! describes every table the service has, and says `swept` of exactly the tables the retention
//! sweep deletes from. A new table cannot be added without being described there.

use std::collections::{BTreeMap, BTreeSet};

use contact_relationship_management::app::App;
use contact_relationship_management::models::deliveries::MAX_ATTEMPTS;
use contact_relationship_management::rules::privacy::{SOFT_DELETED_TABLES, log_rules};
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;

const CLASSES: [&str; 6] = ["swept", "aged", "evidence", "ttl", "manual", "gap"];

/// `table -> retention class`, from the inventory table: the rows with five cells, the first and
/// fourth of which are in backticks.
fn inventory() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../spec/compliance/personal-data-inventory.md"
    ))
    .expect("the inventory is in the spec");
    let mut rows = BTreeMap::new();
    for line in text.lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() != 7 {
            continue; // the schedule's three-cell rows
        }
        let unquote = |cell: &str| cell.trim_matches('`').to_string();
        let was = rows.insert(unquote(cells[1]), unquote(cells[4]));
        assert!(was.is_none(), "{} is listed twice", cells[1]);
    }
    rows
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn every_table_is_in_the_inventory_and_the_swept_ones_are_those_the_sweep_deletes_from() {
    request::<App, _, _>(|_request, ctx| async move {
        let listed = inventory();
        let tables: BTreeSet<String> = ctx
            .db
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT table_name FROM information_schema.tables \
                 WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
            ))
            .await
            .unwrap()
            .iter()
            .map(|row| row.try_get::<String>("", "table_name").unwrap())
            .collect();

        let missing: Vec<_> = tables.iter().filter(|t| !listed.contains_key(*t)).collect();
        assert!(
            missing.is_empty(),
            "tables with no inventory row: {missing:?}"
        );
        let gone: Vec<_> = listed.keys().filter(|t| !tables.contains(*t)).collect();
        assert!(
            gone.is_empty(),
            "inventory rows for tables that do not exist: {gone:?}"
        );

        let unknown: Vec<_> = listed
            .values()
            .filter(|c| !CLASSES.contains(&c.as_str()))
            .collect();
        assert!(
            unknown.is_empty(),
            "retention classes that are not in the schedule: {unknown:?}"
        );

        let swept: BTreeSet<&str> = listed
            .iter()
            .filter(|(_, class)| *class == "swept")
            .map(|(table, _)| table.as_str())
            .collect();
        let deleted_from: BTreeSet<&str> = SOFT_DELETED_TABLES.iter().copied().collect();
        assert_eq!(
            swept, deleted_from,
            "`swept` must be exactly what the sweep deletes from"
        );

        // `aged` is exactly the logs the sweep prunes by age (CRM-R81).
        let aged: BTreeSet<&str> = listed
            .iter()
            .filter(|(_, class)| *class == "aged")
            .map(|(table, _)| table.as_str())
            .collect();
        let rules = log_rules(MAX_ATTEMPTS);
        let pruned: BTreeSet<&str> = rules.iter().map(|rule| rule.table).collect();
        assert_eq!(
            aged, pruned,
            "`aged` must be exactly what the sweep prunes by age"
        );

        // Every table the sweep reaches has the column it deletes by.
        for table in SOFT_DELETED_TABLES {
            ctx.db
                .query_one_raw(Statement::from_string(
                    DatabaseBackend::Postgres,
                    format!("SELECT count(deleted_at) FROM {table}"),
                ))
                .await
                .unwrap_or_else(|e| panic!("{table} has no deleted_at: {e}"));
        }
    })
    .await;
}
