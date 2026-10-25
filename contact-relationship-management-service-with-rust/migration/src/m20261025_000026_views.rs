//! Migration: saved views (CRM-R35).
//!
//! A view is a name, the list it is for, a filter, an optional sort and optional columns, owned by
//! one worker and optionally shared. It holds no record data.

use sea_orm_migration::prelude::*;

/// The saved views migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and the per-owner name rule.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS saved_views (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 owner_ref VARCHAR NOT NULL,
                 kind VARCHAR NOT NULL,
                 name VARCHAR NOT NULL,
                 filter JSONB NOT NULL DEFAULT '{}'::jsonb,
                 sort_field VARCHAR NULL,
                 sort_direction VARCHAR NULL,
                 columns JSONB NULL,
                 shared BOOLEAN NOT NULL DEFAULT FALSE
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS saved_views_name \
             ON saved_views (owner_ref, kind, lower(name))",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS saved_views_shared ON saved_views (kind) WHERE shared",
        )
        .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS saved_views")
            .await?;
        Ok(())
    }
}
