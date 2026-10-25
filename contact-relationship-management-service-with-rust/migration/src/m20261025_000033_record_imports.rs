//! Migration: contact and account imports from a CSV (CRM-R43).
//!
//! `record_imports` is the record of one import, like `lead_imports` (CRM-R34): where the list
//! came from, the counts, and the pid of every record it made, written as each batch commits so an
//! import that stops half way can still be undone.

use sea_orm_migration::prelude::*;

/// The record imports migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS record_imports (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     kind VARCHAR NOT NULL,
                     created_by VARCHAR NULL,
                     source_note VARCHAR NOT NULL,
                     obtained_on DATE NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'running',
                     row_count INTEGER NOT NULL,
                     invalid_count INTEGER NOT NULL,
                     skipped_count INTEGER NOT NULL,
                     failed_count INTEGER NOT NULL DEFAULT 0,
                     created_pids JSONB NOT NULL DEFAULT '[]'::jsonb,
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
            .execute_unprepared("DROP TABLE IF EXISTS record_imports")
            .await?;
        Ok(())
    }
}
