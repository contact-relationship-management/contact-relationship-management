//! Migration: SLA clocks that pause while a ticket waits for the customer (CRM-R55).
//!
//! A ticket records when it was paused and, for each deadline, how much paused time counted
//! against it, so a deadline can always be derived again from the opening time, the target and the
//! time spent waiting.

use sea_orm_migration::prelude::*;

/// The ticket pauses migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE tickets ADD COLUMN IF NOT EXISTS paused_at TIMESTAMPTZ NULL;
                 ALTER TABLE tickets ADD COLUMN IF NOT EXISTS first_response_paused_seconds BIGINT NOT NULL DEFAULT 0 CHECK (first_response_paused_seconds >= 0);
                 ALTER TABLE tickets ADD COLUMN IF NOT EXISTS resolution_paused_seconds BIGINT NOT NULL DEFAULT 0 CHECK (resolution_paused_seconds >= 0);",
            )
            .await?;
        Ok(())
    }

    /// Drop the columns.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE tickets DROP COLUMN IF EXISTS paused_at;
                 ALTER TABLE tickets DROP COLUMN IF EXISTS first_response_paused_seconds;
                 ALTER TABLE tickets DROP COLUMN IF EXISTS resolution_paused_seconds;",
            )
            .await?;
        Ok(())
    }
}
