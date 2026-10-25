//! Migration: file attachments (CRM-R42).
//!
//! `attachments` holds the metadata of a file kept outside the database: whose it is, its cleaned
//! name, the content type its extension maps to, its size and SHA-256, and the id it is stored
//! under. The record is referred to by pid and kind, with no foreign key.

use sea_orm_migration::prelude::*;

/// The attachments migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and its index.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS attachments (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 subject_kind VARCHAR NOT NULL,
                 subject_pid UUID NOT NULL,
                 name VARCHAR NOT NULL,
                 content_type VARCHAR NOT NULL,
                 size_bytes BIGINT NOT NULL,
                 sha256 VARCHAR NOT NULL,
                 uploaded_by VARCHAR NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS attachments_subject ON attachments (subject_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS attachments")
            .await?;
        Ok(())
    }
}
