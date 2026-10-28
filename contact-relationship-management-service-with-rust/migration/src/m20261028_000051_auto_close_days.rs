//! Migration: a policy may close a ticket that has stayed resolved for a number of days (CRM-R82).

use sea_orm_migration::prelude::*;

/// The auto-close-days migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE sla_policies ADD COLUMN IF NOT EXISTS auto_close_days INTEGER NULL CHECK (auto_close_days BETWEEN 1 AND 365);",
            )
            .await?;
        Ok(())
    }

    /// Drop the column.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE sla_policies DROP COLUMN IF EXISTS auto_close_days;")
            .await?;
        Ok(())
    }
}
