//! Migration: tags (CRM-R47).
//!
//! `tags` are the labels, unique by name in any case; `taggings` put a tag on a contact,
//! account, lead or deal, by pid and kind (no foreign key to the record, like attachments and
//! custom field values).

use sea_orm_migration::prelude::*;

/// The tags migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables and indexes.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS tags (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS tags_name ON tags (lower(name))",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS taggings (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id BIGSERIAL PRIMARY KEY,
                 tag_pid UUID NOT NULL,
                 subject_kind VARCHAR NOT NULL,
                 subject_pid UUID NOT NULL,
                 UNIQUE (tag_pid, subject_pid)
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS taggings_subject ON taggings (subject_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS taggings")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS tags").await?;
        Ok(())
    }
}
