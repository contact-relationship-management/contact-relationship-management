//! Migration: provider outcome events and the email suppression list (CRM-T39).
//!
//! `email_events` records each outcome the provider reports for a message,
//! once (`UNIQUE (message_id, event)` makes a replayed webhook harmless).
//! `email_suppressions` holds contacts whose mailbox bounced hard or who
//! complained; the send path skips them. Both are keyed by identifiers, never
//! by address, because the CRM stores no addresses.

use sea_orm_migration::prelude::*;

/// The email-events migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create both tables.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS email_events ( \
               id SERIAL PRIMARY KEY, \
               message_id VARCHAR NOT NULL, \
               event VARCHAR NOT NULL CHECK (event IN \
                 ('delivered', 'bounce_hard', 'bounce_soft', 'complaint')), \
               received_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, \
               UNIQUE (message_id, event))",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS email_suppressions ( \
               contact_pid UUID PRIMARY KEY, \
               reason VARCHAR NOT NULL CHECK (reason IN ('bounce_hard', 'complaint')), \
               created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS email_deliveries_message \
             ON email_deliveries (message_id)",
        )
        .await?;
        Ok(())
    }

    /// Drop both tables.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP INDEX IF EXISTS email_deliveries_message")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS email_suppressions")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS email_events")
            .await?;
        Ok(())
    }
}
