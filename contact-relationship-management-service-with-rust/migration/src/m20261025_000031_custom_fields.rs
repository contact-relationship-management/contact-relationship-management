//! Migration: typed custom fields (CRM-R41).
//!
//! `custom_field_defs` declares a field on contacts, accounts, leads or deals: its key, kind and
//! sensitivity. `custom_field_values` holds one value per field and record, as text (sealed in
//! the database when the field is `sensitive`). Values refer to a record by pid and kind, with no
//! foreign key, like notifications and external links, so they never block a record's removal.

use sea_orm_migration::prelude::*;

/// The custom fields migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables and indexes.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS custom_field_defs (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 entity VARCHAR NOT NULL,
                 key VARCHAR NOT NULL,
                 label VARCHAR NOT NULL,
                 kind VARCHAR NOT NULL,
                 choices JSONB NOT NULL DEFAULT '[]'::jsonb,
                 sensitivity VARCHAR NOT NULL,
                 position INTEGER NOT NULL DEFAULT 0,
                 UNIQUE (entity, key)
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS custom_field_values (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id BIGSERIAL PRIMARY KEY,
                 field_pid UUID NOT NULL,
                 subject_pid UUID NOT NULL,
                 value TEXT NOT NULL,
                 UNIQUE (field_pid, subject_pid)
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS custom_field_values_subject \
             ON custom_field_values (subject_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS custom_field_values")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS custom_field_defs")
            .await?;
        Ok(())
    }
}
