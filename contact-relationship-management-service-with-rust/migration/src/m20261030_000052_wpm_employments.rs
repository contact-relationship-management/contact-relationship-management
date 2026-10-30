//! Migration: the worker service's hire and leave events (CRM-R92).
//!
//! `wpm_events` keeps the id of each event received, so a retry is applied once; it holds no
//! personal data (an id, a kind and what was done). `contact_employments` says that a contact is
//! an employee of an organization, which pauses their marketing: the person and organization
//! references, the date and the event that said so. Nothing else, and no name.

use sea_orm_migration::prelude::*;

/// The WPM-events migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS wpm_events (
                 event_id VARCHAR PRIMARY KEY,
                 kind VARCHAR NOT NULL,
                 outcome VARCHAR NOT NULL,
                 received_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
             )",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS contact_employments (
                 contact_pid UUID NOT NULL,
                 person_ref VARCHAR NOT NULL,
                 organization_ref VARCHAR NOT NULL,
                 since DATE NOT NULL,
                 source_event_id VARCHAR NOT NULL,
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 PRIMARY KEY (contact_pid, organization_ref)
             )",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS contact_employments_person \
             ON contact_employments (person_ref)",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS contact_employments")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS wpm_events")
            .await?;
        Ok(())
    }
}
