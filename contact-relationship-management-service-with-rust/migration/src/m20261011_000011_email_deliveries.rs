//! Migration: `email_deliveries` and campaign content (CRM-T39).
//!
//! One row per `(campaign, contact)` records what happened to one message:
//! it is the idempotency key for a send (a retry never mails a contact twice),
//! the per-recipient log subject access needs, and the state a retry resumes
//! from. It holds no address and no body, only the provider message id and a
//! short error. A campaign also gets an optional subject and body, since a real
//! message needs content.

use sea_orm_migration::prelude::*;

/// The email-deliveries migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and add the campaign columns.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE campaigns \
               ADD COLUMN IF NOT EXISTS subject VARCHAR NULL, \
               ADD COLUMN IF NOT EXISTS body TEXT NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS email_deliveries ( \
               created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, \
               updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, \
               id SERIAL PRIMARY KEY, \
               pid UUID NOT NULL UNIQUE, \
               campaign_pid UUID NOT NULL, \
               contact_pid UUID NOT NULL, \
               state VARCHAR NOT NULL DEFAULT 'pending' CHECK (state IN \
                 ('pending', 'sending', 'sent', 'failed', 'no_address', 'suppressed')), \
               attempts INTEGER NOT NULL DEFAULT 0, \
               message_id VARCHAR NULL, \
               error VARCHAR NULL, \
               sent_at TIMESTAMPTZ NULL, \
               UNIQUE (campaign_pid, contact_pid))",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS email_deliveries_contact \
             ON email_deliveries (contact_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the table and the campaign columns.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS email_deliveries")
            .await?;
        conn.execute_unprepared(
            "ALTER TABLE campaigns DROP COLUMN IF EXISTS subject, DROP COLUMN IF EXISTS body",
        )
        .await?;
        Ok(())
    }
}
