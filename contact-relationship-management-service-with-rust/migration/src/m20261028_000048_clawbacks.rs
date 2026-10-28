//! Migration: clawbacks of a worker's credit on a won deal (CRM-R68).
//!
//! A row takes `amount_minor` back from what the worker is credited for the deal in quota
//! attainment; a worker with no row keeps the whole credit.

use sea_orm_migration::prelude::*;

/// The clawbacks migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS deal_clawbacks (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     id SERIAL PRIMARY KEY,
                     deal_pid UUID NOT NULL,
                     owner_ref VARCHAR NOT NULL,
                     amount_minor BIGINT NOT NULL CHECK (amount_minor > 0),
                     reason VARCHAR,
                     set_by VARCHAR NOT NULL,
                     UNIQUE (deal_pid, owner_ref)
                 )",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS deal_clawbacks")
            .await?;
        Ok(())
    }
}
