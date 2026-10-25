//! Migration: explainable workflow rules and their run log (CRM-R40).
//!
//! A workflow is a trigger, conditions in the segment filter language and a short closed list of
//! actions. Every run is a row of `workflow_runs` that says why it fired and what it did; the run
//! keeps the workflow's name, so deleting a workflow does not erase its history.

use sea_orm_migration::prelude::*;

/// The workflows migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables and indexes.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS workflows (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 enabled BOOLEAN NOT NULL DEFAULT TRUE,
                 trigger VARCHAR NOT NULL,
                 every_minutes INTEGER NULL,
                 conditions JSONB NOT NULL DEFAULT '{}'::jsonb,
                 actions JSONB NOT NULL,
                 scan_after_id INTEGER NOT NULL DEFAULT 0
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS workflows_name ON workflows (lower(name))",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS workflow_runs (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id BIGSERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 workflow_pid UUID NOT NULL,
                 workflow_name VARCHAR NOT NULL,
                 trigger VARCHAR NOT NULL,
                 subject_kind VARCHAR NOT NULL,
                 subject_pid UUID NOT NULL,
                 depth INTEGER NOT NULL,
                 outcome VARCHAR NOT NULL,
                 reason VARCHAR NOT NULL,
                 steps JSONB NOT NULL DEFAULT '[]'::jsonb
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS workflow_runs_by_workflow \
             ON workflow_runs (workflow_pid, created_at DESC)",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS workflow_runs")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS workflows")
            .await?;
        Ok(())
    }
}
