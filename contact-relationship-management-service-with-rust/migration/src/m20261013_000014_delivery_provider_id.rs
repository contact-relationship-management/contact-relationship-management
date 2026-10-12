//! Migration: a delivery row can carry the provider's own message id (CRM-T39).
//!
//! Brevo, Mailjet and SES give our id back on events; Azure Communication
//! Services does not, so its events are matched on the id it returned at send.

use sea_orm_migration::prelude::*;

/// The delivery-provider-id migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column and its index.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE email_deliveries ADD COLUMN IF NOT EXISTS provider_id VARCHAR NULL",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS email_deliveries_provider \
               ON email_deliveries (provider_id)",
        )
        .await?;
        Ok(())
    }

    /// Drop them.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared("DROP INDEX IF EXISTS email_deliveries_provider")
            .await?;
        db.execute_unprepared("ALTER TABLE email_deliveries DROP COLUMN IF EXISTS provider_id")
            .await?;
        Ok(())
    }
}
