//! Migration: task reminders and the notifications inbox (CRM-R32).
//!
//! Activities gain an assignee and a reminder time. `notifications` holds one row per
//! recipient, kind and cause; its uniqueness rule is what makes each fire once. A row the policy
//! would not let the recipient see is kept, marked `suppressed`, so it is not decided again.

use sea_orm_migration::prelude::*;

/// The notifications migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the activity columns and create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE activities ADD COLUMN IF NOT EXISTS assignee_ref VARCHAR NULL",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE activities ADD COLUMN IF NOT EXISTS remind_at TIMESTAMPTZ NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS activities_reminders ON activities (remind_at) \
             WHERE remind_at IS NOT NULL AND done = FALSE AND deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS notifications (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 recipient_ref VARCHAR NOT NULL,
                 kind VARCHAR NOT NULL,
                 subject_kind VARCHAR NOT NULL,
                 subject_pid UUID NOT NULL,
                 source_pid UUID NOT NULL,
                 source_key VARCHAR NOT NULL,
                 suppressed BOOLEAN NOT NULL DEFAULT FALSE,
                 read_at TIMESTAMPTZ NULL,
                 UNIQUE (recipient_ref, kind, source_pid, source_key)
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS notifications_inbox \
             ON notifications (recipient_ref, id DESC)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS notifications_subject ON notifications (subject_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop what `up` made.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS notifications")
            .await?;
        conn.execute_unprepared("DROP INDEX IF EXISTS activities_reminders")
            .await?;
        conn.execute_unprepared("ALTER TABLE activities DROP COLUMN IF EXISTS remind_at")
            .await?;
        conn.execute_unprepared("ALTER TABLE activities DROP COLUMN IF EXISTS assignee_ref")
            .await?;
        Ok(())
    }
}
