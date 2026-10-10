//! Migration: `break_glass_grants` (CRM-T44) — one row per emergency
//! elevation: who, why, until when, and the after-the-fact review.

use sea_orm_migration::prelude::*;

/// The break-glass migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS break_glass_grants ( \
               id BIGSERIAL PRIMARY KEY, \
               pid UUID NOT NULL UNIQUE, \
               subject TEXT NOT NULL, \
               reason TEXT NOT NULL, \
               granted_at TIMESTAMPTZ NOT NULL, \
               expires_at TIMESTAMPTZ NOT NULL, \
               reviewed_at TIMESTAMPTZ, \
               reviewed_by TEXT, \
               review_note TEXT)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS break_glass_grants_live \
             ON break_glass_grants (expires_at)",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS break_glass_grants")
            .await?;
        Ok(())
    }
}
