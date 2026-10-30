//! Migration: the organization's calendar (CRM-T171).
//!
//! `organization_settings` is one row: the organization's IANA time zone, which decides "today"
//! in the service and the months of reports, and the month its fiscal year starts, for fiscal
//! quarters and years. It starts at UTC and January, the values the service used before, and an
//! administrator changes them (`PUT /api/settings/organization`).

use sea_orm_migration::prelude::*;

/// The organization-settings migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and its one row.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS organization_settings (
                 id SMALLINT PRIMARY KEY CHECK (id = 1),
                 time_zone VARCHAR(64) NOT NULL DEFAULT 'UTC',
                 fiscal_year_start_month SMALLINT NOT NULL DEFAULT 1
                     CHECK (fiscal_year_start_month BETWEEN 1 AND 12),
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 set_by VARCHAR NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "INSERT INTO organization_settings (id) VALUES (1) ON CONFLICT (id) DO NOTHING",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS organization_settings")
            .await?;
        Ok(())
    }
}
