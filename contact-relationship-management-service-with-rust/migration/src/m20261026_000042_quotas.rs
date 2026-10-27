//! Migration: quotas and commission plans (CRM-R59).
//!
//! A commission plan is a name and marginal tiers; a quota is an amount a worker is expected to
//! win in a period, in one currency, optionally paid under a plan. Both are kept by hand and
//! audited; the revenue they are measured against is read from the won deals.

use sea_orm_migration::prelude::*;

/// The quotas migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS commission_plans (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 tiers JSONB NOT NULL,
                 set_by VARCHAR NOT NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS commission_plans_name ON commission_plans (lower(name))",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS quotas (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 owner_ref VARCHAR NOT NULL,
                 period_from DATE NOT NULL,
                 period_to DATE NOT NULL,
                 currency CHAR(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
                 amount_minor BIGINT NOT NULL CHECK (amount_minor > 0),
                 plan_pid UUID NULL REFERENCES commission_plans (pid) ON DELETE RESTRICT,
                 set_by VARCHAR NOT NULL,
                 CHECK (period_from <= period_to),
                 UNIQUE (owner_ref, period_from, period_to, currency)
             )",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS quotas")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS commission_plans")
            .await?;
        Ok(())
    }
}
