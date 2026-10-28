//! Migration: the delivery idea opened in PPM for a won deal (CRM-R72).
//!
//! One row per deal, so opening it twice does not send it twice.

use sea_orm_migration::prelude::*;

/// The deal-deliveries migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS deal_deliveries (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     deal_pid UUID PRIMARY KEY,
                     ppm_idea_pid UUID NOT NULL,
                     opened_by VARCHAR NOT NULL
                 )",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS deal_deliveries")
            .await?;
        Ok(())
    }
}
