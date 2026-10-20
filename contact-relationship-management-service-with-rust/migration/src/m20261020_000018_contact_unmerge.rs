//! Migration: undoing a merge (CRM-R26).
//!
//! The merge ledger gains `before`, what the merge changed beyond moving rows
//! (the consent each contact held, the fields it filled, the rows it dropped),
//! and `unmerged_at` / `unmerged_actor`, set when the merge is undone. A
//! contact can be merged again after it is unmerged, so the unique index on
//! `merged_pid` becomes a partial one over merges not yet undone. Merges made
//! before this migration have no `before` and cannot be undone.

use sea_orm_migration::prelude::*;

/// The contact-unmerge migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the undo columns and relax the uniqueness on the merged contact.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE contact_merges \
             ADD COLUMN IF NOT EXISTS before JSONB NULL, \
             ADD COLUMN IF NOT EXISTS unmerged_at TIMESTAMPTZ NULL, \
             ADD COLUMN IF NOT EXISTS unmerged_actor VARCHAR NULL",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE contact_merges DROP CONSTRAINT IF EXISTS contact_merges_merged_pid_key",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS contact_merges_merged_live \
             ON contact_merges (merged_pid) WHERE unmerged_at IS NULL",
        )
        .await?;
        Ok(())
    }

    /// Restore the original shape (rollback). Fails if a contact now has more
    /// than one ledger row, because the old unique constraint cannot hold.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("DROP INDEX IF EXISTS contact_merges_merged_live")
            .await?;
        conn.execute_unprepared(
            "ALTER TABLE contact_merges ADD CONSTRAINT contact_merges_merged_pid_key UNIQUE (merged_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE contact_merges DROP COLUMN IF EXISTS before, \
             DROP COLUMN IF EXISTS unmerged_at, DROP COLUMN IF EXISTS unmerged_actor",
        )
        .await?;
        Ok(())
    }
}
