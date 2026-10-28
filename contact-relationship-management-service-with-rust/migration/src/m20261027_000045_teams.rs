//! Migration: teams of workers, for rolling quota attainment up (CRM-R67).
//!
//! A team is a name and the workers on it, kept by hand and audited. It changes no quota and no
//! credit: it only decides which workers' standings are added together.

use sea_orm_migration::prelude::*;

/// The teams migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS teams (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 members JSONB NOT NULL,
                 set_by VARCHAR NOT NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS teams_name ON teams (lower(name))",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS teams")
            .await?;
        Ok(())
    }
}
