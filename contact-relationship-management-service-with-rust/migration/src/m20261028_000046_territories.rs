//! Migration: territories of named accounts (CRM-R68).
//!
//! A territory is a name, an optional worker, and the accounts in it, named by the organization
//! they are (a deal reaches its territory through its account's `organization_ref`). An account is in at most one
//! territory, which the primary key on `territory_accounts` enforces.

use sea_orm_migration::prelude::*;

/// The territories migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS territories (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 owner_ref VARCHAR NULL,
                 set_by VARCHAR NOT NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS territories_name ON territories (lower(name))",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS territory_accounts (
                 organization_ref VARCHAR PRIMARY KEY,
                 territory_pid UUID NOT NULL REFERENCES territories (pid) ON DELETE CASCADE
             )",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS territory_accounts")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS territories")
            .await?;
        Ok(())
    }
}
