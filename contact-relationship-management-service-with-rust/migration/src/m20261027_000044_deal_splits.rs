//! Migration: splitting a won deal between workers for quotas (CRM-R65).
//!
//! A deal with rows here is credited to those workers in those shares of 10000 instead of
//! wholly to its owner; a deal with none is credited to its owner as before.

use sea_orm_migration::prelude::*;

/// The deal-splits migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS deal_splits (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     id SERIAL PRIMARY KEY,
                     deal_pid UUID NOT NULL,
                     owner_ref VARCHAR NOT NULL,
                     share_bps INTEGER NOT NULL CHECK (share_bps BETWEEN 1 AND 10000),
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
            .execute_unprepared("DROP TABLE IF EXISTS deal_splits")
            .await?;
        Ok(())
    }
}
