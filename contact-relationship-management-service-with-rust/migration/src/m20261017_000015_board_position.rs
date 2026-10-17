//! Migration: a stored card order for the lead and ticket boards.
//!
//! Deals already carry `kanban_position`. Leads and tickets get the same idea
//! under a clearer name: the 0-based place of the card among the cards in its
//! column (lead status, ticket status). Existing rows start at 0, so the
//! earlier ordering (score for leads, id for tickets) holds until someone
//! reorders a column.

use sea_orm_migration::prelude::*;

/// The board-position migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for table in ["leads", "tickets"] {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS board_position INTEGER NOT NULL DEFAULT 0"
            ))
            .await?;
        }
        Ok(())
    }

    /// Drop them.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for table in ["leads", "tickets"] {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP COLUMN IF EXISTS board_position"
            ))
            .await?;
        }
        Ok(())
    }
}
