//! Migration: a contact may carry a family name (CRM-R37).
//!
//! The CRM keeps one display name per contact. The family name is optional, is never guessed from
//! the display name, and is what the `family_name` merge token (CRM-R36) fills in.

use sea_orm_migration::prelude::*;

/// The contact family name migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE contacts ADD COLUMN IF NOT EXISTS family_name VARCHAR NULL",
            )
            .await?;
        Ok(())
    }

    /// Drop the column.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE contacts DROP COLUMN IF EXISTS family_name")
            .await?;
        Ok(())
    }
}
