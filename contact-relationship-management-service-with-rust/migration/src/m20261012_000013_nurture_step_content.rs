//! Migration: nurture steps carry message content (CRM-T39).
//!
//! A step so far held only a template reference, which was enough while sends
//! were simulated. A real send needs a subject and a plain-text body.

use sea_orm_migration::prelude::*;

/// The nurture-step-content migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE nurture_steps \
                   ADD COLUMN IF NOT EXISTS subject VARCHAR NULL, \
                   ADD COLUMN IF NOT EXISTS body TEXT NULL",
            )
            .await?;
        Ok(())
    }

    /// Drop the columns.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE nurture_steps DROP COLUMN IF EXISTS subject, \
                   DROP COLUMN IF EXISTS body",
            )
            .await?;
        Ok(())
    }
}
