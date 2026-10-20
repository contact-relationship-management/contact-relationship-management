//! Migration: links to records in other systems (CRM-R27).
//!
//! One row per link: a contact, account, deal or lead, the system it is linked
//! in (`HubSpot`, `Salesforce`, `Attio`, …), the web address of the record there, and
//! an optional label and the system's own id. Nothing is synchronised.

use sea_orm_migration::prelude::*;

/// The external-links migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and its lookup index.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS external_links (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 subject_kind VARCHAR NOT NULL,
                 subject_pid UUID NOT NULL,
                 system VARCHAR NOT NULL,
                 url TEXT NOT NULL,
                 external_id VARCHAR NULL,
                 label VARCHAR NULL,
                 created_by VARCHAR NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS external_links_subject \
             ON external_links (subject_kind, subject_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the table (rollback).
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS external_links")
            .await?;
        Ok(())
    }
}
