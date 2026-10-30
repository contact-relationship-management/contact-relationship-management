//! Migration: the order of the worker service's hire and leave events (CRM-R93).
//!
//! `employment_last_events` keeps, per person and organization, the last hire or leave that was
//! applied: its kind, when WPM recorded it, the day it takes effect and its id. An event older
//! than that changes nothing, so a retried hire cannot undo the leave that followed it. A row is
//! written only for a person who is a contact, and the contact's erasure deletes it.

use sea_orm_migration::prelude::*;

/// The employment-order migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS employment_last_events (
                     person_ref VARCHAR NOT NULL,
                     organization_ref VARCHAR NOT NULL,
                     kind VARCHAR NOT NULL CHECK (kind IN ('hired', 'left')),
                     occurred_at TIMESTAMPTZ NOT NULL,
                     effective_on DATE NOT NULL,
                     event_id VARCHAR NOT NULL,
                     applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     PRIMARY KEY (person_ref, organization_ref)
                 )",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS employment_last_events")
            .await?;
        Ok(())
    }
}
