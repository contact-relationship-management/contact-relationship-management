//! Migration: business calendars for SLA clocks (CRM-R45).
//!
//! A calendar is a time zone, the working hours of each weekday, and a list of holidays. An SLA
//! policy may name one, and its deadlines are then counted in working time; with none they run
//! round the clock, as before.

use sea_orm_migration::prelude::*;

/// The business calendars migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and the reference.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS business_calendars (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 timezone VARCHAR NOT NULL,
                 week JSONB NOT NULL,
                 holidays JSONB NOT NULL DEFAULT '[]'::jsonb
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS business_calendars_name \
             ON business_calendars (lower(name))",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE sla_policies ADD COLUMN IF NOT EXISTS calendar_pid UUID NULL",
        )
        .await?;
        Ok(())
    }

    /// Drop the reference and the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("ALTER TABLE sla_policies DROP COLUMN IF EXISTS calendar_pid")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS business_calendars")
            .await?;
        Ok(())
    }
}
