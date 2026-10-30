//! Migration: an email template's language (CRM-T169).
//!
//! `email_templates.language` is the BCP 47 tag of the language a template is written in, checked
//! by the service (`rules::templates::language`). A token with no value takes the template's own
//! fallback, else the built-in one in that language. Existing templates keep `NULL`, which is the
//! English fallbacks they were written against.

use sea_orm_migration::prelude::*;

/// The template-language migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE email_templates ADD COLUMN IF NOT EXISTS language VARCHAR(35) NULL",
            )
            .await?;
        Ok(())
    }

    /// Drop the column.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE email_templates DROP COLUMN IF EXISTS language")
            .await?;
        Ok(())
    }
}
