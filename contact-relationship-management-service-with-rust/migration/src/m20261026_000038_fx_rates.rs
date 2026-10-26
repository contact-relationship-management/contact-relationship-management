//! Migration: exchange rates (CRM-R50).
//!
//! A rate is entered by hand: a pair of currencies, the day it takes effect, a rate held as an
//! integer number of billionths (so no floating point is ever involved), and where it came from.
//! There is one per pair and day.

use sea_orm_migration::prelude::*;

/// The exchange rates migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS fx_rates (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     from_currency CHAR(3) NOT NULL CHECK (from_currency ~ '^[A-Z]{3}$'),
                     to_currency CHAR(3) NOT NULL CHECK (to_currency ~ '^[A-Z]{3}$'),
                     rate_ppb BIGINT NOT NULL CHECK (rate_ppb > 0),
                     effective_on DATE NOT NULL,
                     source VARCHAR NOT NULL,
                     set_by VARCHAR NOT NULL,
                     CHECK (from_currency <> to_currency),
                     UNIQUE (from_currency, to_currency, effective_on)
                 )",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS fx_rates")
            .await?;
        Ok(())
    }
}
