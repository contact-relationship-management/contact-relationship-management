//! Migration: make `audit_logs` tamper-evident (CRM-T31). Adds the
//! `prev_hash` / `hash` chain columns and, on `PostgreSQL`, a trigger that
//! makes the table append-only (no `DELETE`; an `UPDATE` may only fill a
//! still-empty `hash`). The trigger is defence in depth: a database owner can
//! still drop it, which is why the hash chain exists and why operators should
//! anchor the chain head outside the database.

use sea_orm_migration::prelude::*;

/// The audit-chain migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the chain columns and the append-only trigger.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS prev_hash text, \
             ADD COLUMN IF NOT EXISTS hash text",
        )
        .await?;
        if m.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            db.execute_unprepared(
                "CREATE OR REPLACE FUNCTION audit_logs_append_only() RETURNS trigger AS $$ \
                 BEGIN \
                   IF TG_OP = 'DELETE' THEN \
                     RAISE EXCEPTION 'audit_logs is append-only'; \
                   END IF; \
                   IF OLD.hash IS NULL AND NEW.hash IS NOT NULL \
                      AND (OLD.id, OLD.entity, OLD.entity_pid, OLD.action, OLD.actor, \
                           OLD.snapshot, OLD.prev_hash, OLD.created_at) \
                          IS NOT DISTINCT FROM \
                          (NEW.id, NEW.entity, NEW.entity_pid, NEW.action, NEW.actor, \
                           NEW.snapshot, NEW.prev_hash, NEW.created_at) THEN \
                     RETURN NEW; \
                   END IF; \
                   RAISE EXCEPTION 'audit_logs is append-only'; \
                 END; $$ LANGUAGE plpgsql",
            )
            .await?;
            db.execute_unprepared("DROP TRIGGER IF EXISTS audit_logs_append_only ON audit_logs")
                .await?;
            db.execute_unprepared(
                "CREATE TRIGGER audit_logs_append_only BEFORE UPDATE OR DELETE ON audit_logs \
                 FOR EACH ROW EXECUTE FUNCTION audit_logs_append_only()",
            )
            .await?;
        }
        Ok(())
    }

    /// Remove the trigger and the chain columns.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        if m.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            db.execute_unprepared("DROP TRIGGER IF EXISTS audit_logs_append_only ON audit_logs")
                .await?;
            db.execute_unprepared("DROP FUNCTION IF EXISTS audit_logs_append_only()")
                .await?;
        }
        db.execute_unprepared(
            "ALTER TABLE audit_logs DROP COLUMN IF EXISTS hash, DROP COLUMN IF EXISTS prev_hash",
        )
        .await?;
        Ok(())
    }
}
