//! Migration: `idempotency_keys` (CRM-T52) — one row per
//! `(caller, Idempotency-Key)`: the request fingerprint and, once the first
//! attempt succeeds, the response to replay.

use sea_orm_migration::prelude::*;

/// The idempotency-keys migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS idempotency_keys ( \
                   scope TEXT NOT NULL, \
                   key TEXT NOT NULL, \
                   fingerprint TEXT NOT NULL, \
                   state TEXT NOT NULL CHECK (state IN ('in_progress', 'done')), \
                   status INTEGER, \
                   content_type TEXT, \
                   body TEXT, \
                   created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
                   PRIMARY KEY (scope, key))",
            )
            .await?;
        m.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idempotency_keys_age \
                 ON idempotency_keys (created_at)",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS idempotency_keys")
            .await?;
        Ok(())
    }
}
