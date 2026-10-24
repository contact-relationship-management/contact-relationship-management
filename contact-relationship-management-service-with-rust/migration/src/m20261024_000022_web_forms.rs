//! Migration: web form submissions (CRM-R31).
//!
//! One row per accepted submission: which form, which contact and lead it made or found, the
//! purpose, the nurture sequence to enrol into once the visitor confirms, and when the
//! confirmation mail went out and when the visitor confirmed. It holds no name and no address.

use sea_orm_migration::prelude::*;

/// The web forms migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and the index the confirmation looks rows up by.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS form_submissions (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 form VARCHAR NOT NULL,
                 contact_pid UUID NOT NULL,
                 lead_pid UUID NOT NULL,
                 purpose VARCHAR NOT NULL,
                 sequence_pid UUID NULL,
                 confirmation_sent_at TIMESTAMPTZ NULL,
                 confirmed_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS form_submissions_contact \
             ON form_submissions (contact_pid, purpose)",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS form_submissions")
            .await?;
        Ok(())
    }
}
