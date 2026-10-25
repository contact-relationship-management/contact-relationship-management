//! Migration: a lead has an owner, and a workflow remembers whose turn it is (CRM-R44).
//!
//! `leads.owner_ref` is the worker a lead is assigned to. `workflows.assign_cursor` counts the
//! leads a workflow has handed out, so a round-robin continues where it left off.

use sea_orm_migration::prelude::*;

/// The lead owner migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE leads ADD COLUMN IF NOT EXISTS owner_ref VARCHAR NULL",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE workflows ADD COLUMN IF NOT EXISTS assign_cursor BIGINT NOT NULL DEFAULT 0",
        )
        .await?;
        Ok(())
    }

    /// Drop the columns.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("ALTER TABLE workflows DROP COLUMN IF EXISTS assign_cursor")
            .await?;
        conn.execute_unprepared("ALTER TABLE leads DROP COLUMN IF EXISTS owner_ref")
            .await?;
        Ok(())
    }
}
