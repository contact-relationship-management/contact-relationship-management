//! Migration: the public knowledge base's daily counts (CRM-R57).
//!
//! Only counts are kept, per day and per article (the all-zero id is the portal as a whole): no
//! address, no visitor, and no text a visitor typed.

use sea_orm_migration::prelude::*;

/// The knowledge-base counts migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS kb_daily (
                     day DATE NOT NULL,
                     article_pid UUID NOT NULL,
                     views BIGINT NOT NULL DEFAULT 0 CHECK (views >= 0),
                     helpful_yes BIGINT NOT NULL DEFAULT 0 CHECK (helpful_yes >= 0),
                     helpful_no BIGINT NOT NULL DEFAULT 0 CHECK (helpful_no >= 0),
                     searches BIGINT NOT NULL DEFAULT 0 CHECK (searches >= 0),
                     searches_empty BIGINT NOT NULL DEFAULT 0 CHECK (searches_empty >= 0),
                     PRIMARY KEY (day, article_pid)
                 )",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS kb_daily")
            .await?;
        Ok(())
    }
}
