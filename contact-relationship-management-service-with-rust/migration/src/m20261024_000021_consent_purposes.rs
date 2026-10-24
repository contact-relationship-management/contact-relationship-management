//! Migration: consent per purpose (CRM-R29).
//!
//! `marketing_consent` on `contacts` stays the state of the original purpose (`marketing`), so
//! nothing that reads it changes and nothing needs copying. `contact_consents` holds the state
//! of every other purpose, one row per contact and purpose (no row = `never`). Consent events
//! gain the purpose they were about (every existing event is a `marketing` event), and a
//! campaign or nurture sequence names the purpose it is sent under.

use sea_orm_migration::prelude::*;

/// The consent-purposes migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and add the purpose columns.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS contact_consents (
                 contact_pid UUID NOT NULL,
                 purpose VARCHAR NOT NULL,
                 state VARCHAR NOT NULL,
                 changed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 PRIMARY KEY (contact_pid, purpose)
             )",
        )
        .await?;
        for table in ["consent_events", "campaigns", "nurture_sequences"] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS purpose VARCHAR NOT NULL DEFAULT 'marketing'"
            ))
            .await?;
        }
        Ok(())
    }

    /// Drop the table and the columns (rollback). Consent held for other purposes is lost.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in ["consent_events", "campaigns", "nurture_sequences"] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP COLUMN IF EXISTS purpose"
            ))
            .await?;
        }
        conn.execute_unprepared("DROP TABLE IF EXISTS contact_consents")
            .await?;
        Ok(())
    }
}
