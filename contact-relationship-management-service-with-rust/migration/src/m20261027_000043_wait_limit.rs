//! Migration: a limit on how long a ticket may wait for the customer (CRM-R64).
//!
//! A policy may name the most minutes a ticket of its priority waits in `pending`; the sweep
//! records the moment a ticket passes it, once, in `wait_limit_exceeded_at`.

use sea_orm_migration::prelude::*;

/// The wait-limit migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE sla_policies ADD COLUMN IF NOT EXISTS max_wait_minutes INTEGER NULL CHECK (max_wait_minutes > 0);
                 ALTER TABLE tickets ADD COLUMN IF NOT EXISTS wait_limit_exceeded_at TIMESTAMPTZ NULL;",
            )
            .await?;
        Ok(())
    }

    /// Drop the columns.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE sla_policies DROP COLUMN IF EXISTS max_wait_minutes;
                 ALTER TABLE tickets DROP COLUMN IF EXISTS wait_limit_exceeded_at;",
            )
            .await?;
        Ok(())
    }
}
