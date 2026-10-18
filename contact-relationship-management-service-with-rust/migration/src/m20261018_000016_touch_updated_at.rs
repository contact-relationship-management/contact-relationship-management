//! Migration: keep `updated_at` true on every update.
//!
//! The ORM's `ActiveModel::update` writes only the columns it was given, so the
//! `updated_at` default (set at insert) never moved: an edited deal still said
//! it was last changed when it was created. A `BEFORE UPDATE` trigger sets it
//! to `now()` unless the statement sets it itself (the email deliveries job
//! does), so no handler can forget it. Append-only tables (audit log, consent
//! events, forecast snapshots, the outbox, the queue) are left alone.

use sea_orm_migration::prelude::*;

/// The tables whose rows are edited in place.
const TABLES: [&str; 18] = [
    "accounts",
    "activities",
    "articles",
    "campaigns",
    "contacts",
    "deals",
    "leads",
    "memberships",
    "nurture_enrollments",
    "nurture_sequences",
    "nurture_steps",
    "partnerships",
    "pipeline_stages",
    "pipelines",
    "segments",
    "sla_policies",
    "tickets",
    "working_groups",
];

/// The touch-`updated_at` migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the function and a trigger per table.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "CREATE OR REPLACE FUNCTION crm_touch_updated_at() RETURNS trigger AS $$ \
             BEGIN \
               IF NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at THEN \
                 NEW.updated_at := now(); \
               END IF; \
               RETURN NEW; \
             END; $$ LANGUAGE plpgsql",
        )
        .await?;
        for table in TABLES {
            db.execute_unprepared(&format!(
                "DROP TRIGGER IF EXISTS crm_touch_updated_at ON {table}"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "CREATE TRIGGER crm_touch_updated_at BEFORE UPDATE ON {table} \
                 FOR EACH ROW EXECUTE FUNCTION crm_touch_updated_at()"
            ))
            .await?;
        }
        Ok(())
    }

    /// Drop the triggers and the function.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for table in TABLES {
            db.execute_unprepared(&format!(
                "DROP TRIGGER IF EXISTS crm_touch_updated_at ON {table}"
            ))
            .await?;
        }
        db.execute_unprepared("DROP FUNCTION IF EXISTS crm_touch_updated_at()")
            .await?;
        Ok(())
    }
}
