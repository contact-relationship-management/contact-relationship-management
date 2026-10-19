//! Migration: merging two contacts (CRM-R24).
//!
//! `contacts.merged_into_pid` points a merged-away contact at the one that
//! survived. `contact_merges` is the merge ledger: one row per merge, holding
//! the opaque ids of every row that moved, so a mistaken merge can be repaired
//! by hand and an unmerge can be built later. The ids are not personal data.
//! The ledger is append-only.

use sea_orm_migration::prelude::*;

/// The contact-merges migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the pointer column and create the ledger.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE contacts ADD COLUMN IF NOT EXISTS merged_into_pid UUID NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS contact_merges (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 survivor_pid UUID NOT NULL,
                 merged_pid UUID NOT NULL UNIQUE,
                 merged_person_ref VARCHAR NOT NULL,
                 actor VARCHAR NULL,
                 moved JSONB NOT NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS contact_merges_survivor ON contact_merges (survivor_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the ledger and the column (rollback).
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS contact_merges")
            .await?;
        conn.execute_unprepared("ALTER TABLE contacts DROP COLUMN IF EXISTS merged_into_pid")
            .await?;
        Ok(())
    }
}
