//! Migration: what the sweep does with a ticket past its policy's wait limit (CRM-R69).
//!
//! `flag` (what it has always done), `resolve` it, or `escalate` it to a named worker.

use sea_orm_migration::prelude::*;

/// The wait-limit-action migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE sla_policies ADD COLUMN IF NOT EXISTS wait_limit_action VARCHAR NOT NULL DEFAULT 'flag' CHECK (wait_limit_action IN ('flag', 'resolve', 'escalate'));
                 ALTER TABLE sla_policies ADD COLUMN IF NOT EXISTS escalate_to VARCHAR NULL;",
            )
            .await?;
        Ok(())
    }

    /// Drop the columns.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE sla_policies DROP COLUMN IF EXISTS wait_limit_action;
                 ALTER TABLE sla_policies DROP COLUMN IF EXISTS escalate_to;",
            )
            .await?;
        Ok(())
    }
}
