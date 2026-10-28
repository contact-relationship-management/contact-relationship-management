//! Migration: single-use link tokens, kept only as a hash (CRM-R79).
//!
//! The confirmation link and the preference-page link are random tokens. The database holds the
//! SHA-256 of each, never the token, so a copy of the table cannot be used to confirm anything or
//! open a preference page; `used_at` makes a token work once.

use sea_orm_migration::prelude::*;

/// The link-tokens migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS link_tokens (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 token_hash CHAR(64) PRIMARY KEY,
                 kind VARCHAR NOT NULL CHECK (kind IN ('confirm', 'preferences')),
                 contact_pid UUID NOT NULL,
                 detail SMALLINT NOT NULL DEFAULT 0,
                 expires_at TIMESTAMPTZ NOT NULL,
                 used_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS link_tokens_expires ON link_tokens (expires_at)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS link_tokens_contact ON link_tokens (contact_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS link_tokens")
            .await?;
        Ok(())
    }
}
