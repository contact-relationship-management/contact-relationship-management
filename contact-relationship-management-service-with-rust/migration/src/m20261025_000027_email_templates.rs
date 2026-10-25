//! Migration: email templates with merge tokens (CRM-R36).
//!
//! A template is a name, a subject, a plain-text body and the template's own fallbacks. A campaign
//! and a nurture step may refer to one by `template_pid` instead of carrying their own subject and
//! body.

use sea_orm_migration::prelude::*;

/// The email templates migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table, its name rule and the references.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS email_templates (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 subject VARCHAR NOT NULL,
                 body TEXT NOT NULL,
                 fallbacks JSONB NOT NULL DEFAULT '{}'::jsonb
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS email_templates_name \
             ON email_templates (lower(name))",
        )
        .await?;
        for table in ["campaigns", "nurture_steps"] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS template_pid UUID NULL"
            ))
            .await?;
            conn.execute_unprepared(&format!(
                "CREATE INDEX IF NOT EXISTS {table}_template ON {table} (template_pid) \
                 WHERE template_pid IS NOT NULL"
            ))
            .await?;
        }
        Ok(())
    }

    /// Drop the references and the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in ["campaigns", "nurture_steps"] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP COLUMN IF EXISTS template_pid"
            ))
            .await?;
        }
        conn.execute_unprepared("DROP TABLE IF EXISTS email_templates")
            .await?;
        Ok(())
    }
}
