//! Migration: a contact may carry a given name (CRM-R38).
//!
//! The given name is optional and is what the `given_name` merge token (CRM-R36) fills in. When a
//! contact has none, the token falls back to the first word of the display name, as before.

use sea_orm_migration::prelude::*;

/// The contact given name migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE contacts ADD COLUMN IF NOT EXISTS given_name VARCHAR NULL",
            )
            .await?;
        Ok(())
    }

    /// Drop the column.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE contacts DROP COLUMN IF EXISTS given_name")
            .await?;
        Ok(())
    }
}
