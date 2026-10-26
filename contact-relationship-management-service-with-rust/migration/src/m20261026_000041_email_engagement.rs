//! Migration: opens and clicks as e-mail events (CRM-R58).
//!
//! `email_events` already holds each outcome a provider reports for a message, once. It now also
//! accepts `opened` and `clicked`, which the service records only when a provider reports them
//! (the service itself never turns tracking on) and the contact still consents.

use sea_orm_migration::prelude::*;

/// The e-mail engagement migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Widen the event check.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE email_events DROP CONSTRAINT IF EXISTS email_events_event_check;
                 ALTER TABLE email_events ADD CONSTRAINT email_events_event_check CHECK (event IN \
                   ('delivered', 'bounce_hard', 'bounce_soft', 'complaint', 'opened', 'clicked'));",
            )
            .await?;
        Ok(())
    }

    /// Remove the engagement events and narrow the check again.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DELETE FROM email_events WHERE event IN ('opened', 'clicked');
                 ALTER TABLE email_events DROP CONSTRAINT IF EXISTS email_events_event_check;
                 ALTER TABLE email_events ADD CONSTRAINT email_events_event_check CHECK (event IN \
                   ('delivered', 'bounce_hard', 'bounce_soft', 'complaint'));",
            )
            .await?;
        Ok(())
    }
}
