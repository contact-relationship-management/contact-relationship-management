//! Migration: outbound webhooks (CRM-R28).
//!
//! `webhook_subscriptions` holds an address, the event patterns it wants and the signing
//! secret (sealed with the field keyring; the previous one keeps signing for a while
//! after a rotation). `webhook_deliveries` holds one row per subscription per event, with
//! its attempts. `event_outbox.webhook_fanned_out_at` marks the outbox rows already turned
//! into deliveries. A delivery stores no contact value: the body is built from the outbox
//! row's ids when it is sent.

use sea_orm_migration::prelude::*;

/// The webhooks migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables, their indexes and the outbox marker.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS webhook_subscriptions (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 url TEXT NOT NULL,
                 events JSONB NOT NULL,
                 secret TEXT NOT NULL,
                 previous_secret TEXT NULL,
                 previous_secret_expires_at TIMESTAMPTZ NULL,
                 enabled BOOLEAN NOT NULL DEFAULT TRUE,
                 disabled_reason VARCHAR NULL,
                 disabled_at TIMESTAMPTZ NULL,
                 created_by VARCHAR NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS webhook_deliveries (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 subscription_id INTEGER NOT NULL REFERENCES webhook_subscriptions (id),
                 event_id UUID NOT NULL,
                 entity_pid UUID NOT NULL,
                 status VARCHAR NOT NULL DEFAULT 'pending',
                 attempts INTEGER NOT NULL DEFAULT 0,
                 next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 last_status INTEGER NULL,
                 last_error VARCHAR NULL,
                 delivered_at TIMESTAMPTZ NULL,
                 UNIQUE (subscription_id, event_id)
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS webhook_deliveries_due \
             ON webhook_deliveries (next_attempt_at) WHERE status = 'pending'",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS webhook_deliveries_entity \
             ON webhook_deliveries (entity_pid) WHERE status = 'pending'",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE event_outbox ADD COLUMN IF NOT EXISTS webhook_fanned_out_at TIMESTAMPTZ NULL",
        )
        .await?;
        // Rows written before this migration are not replayed to new subscriptions.
        conn.execute_unprepared(
            "UPDATE event_outbox SET webhook_fanned_out_at = CURRENT_TIMESTAMP \
             WHERE webhook_fanned_out_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS event_outbox_unfanned \
             ON event_outbox (id) WHERE webhook_fanned_out_at IS NULL",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables and the marker (rollback).
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS webhook_deliveries")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS webhook_subscriptions")
            .await?;
        conn.execute_unprepared("DROP INDEX IF EXISTS event_outbox_unfanned")
            .await?;
        conn.execute_unprepared(
            "ALTER TABLE event_outbox DROP COLUMN IF EXISTS webhook_fanned_out_at",
        )
        .await?;
        Ok(())
    }
}
