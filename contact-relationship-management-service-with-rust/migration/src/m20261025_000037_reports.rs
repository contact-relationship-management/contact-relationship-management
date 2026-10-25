//! Migration: saved reports (CRM-R48).
//!
//! A report is a name and a specification (a kind of record, a grouping, a measure, filters),
//! owned by one worker and optionally shared, like a saved view. It holds no record data: running
//! it is the aggregate read of whoever runs it.

use sea_orm_migration::prelude::*;

/// The saved reports migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and the per-owner name rule.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS reports (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 owner_ref VARCHAR NOT NULL,
                 name VARCHAR NOT NULL,
                 spec JSONB NOT NULL,
                 shared BOOLEAN NOT NULL DEFAULT FALSE
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS reports_name ON reports (owner_ref, lower(name))",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS reports")
            .await?;
        Ok(())
    }
}
