//! Migration: lead imports (CRM-R34).
//!
//! One row per import: where the list came from and when it was obtained, the counts, and the
//! pid of every lead the import created (so it can be undone). It holds no name or address.

use sea_orm_migration::prelude::*;

/// The lead imports migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS lead_imports (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     created_by VARCHAR NULL,
                     source_note VARCHAR NOT NULL,
                     obtained_on DATE NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'running',
                     row_count INTEGER NOT NULL,
                     invalid_count INTEGER NOT NULL,
                     skipped_count INTEGER NOT NULL DEFAULT 0,
                     lead_pids JSONB NOT NULL DEFAULT '[]'::jsonb,
                     undone_at TIMESTAMPTZ NULL,
                     undone_count INTEGER NULL
                 )",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS lead_imports")
            .await?;
        Ok(())
    }
}
