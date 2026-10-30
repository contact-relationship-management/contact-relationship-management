//! Migration: search finds part of a name written without spaces (CRM-T170).
//!
//! Chinese, Japanese, Korean names and Thai, Lao, Khmer and Myanmar text are written without a
//! space between words, so the `simple` parser made one word of a whole name and a part of it
//! (`芳` in `王芳`) was not found. This creates `search_ngrams(text)`, which adds each run of those
//! scripts' single characters and overlapping pairs to the text, and rebuilds each table's
//! full-text index over it ([`crate::search_docs::ngram_index_statements`]). The trigram indexes
//! are unchanged.

use sea_orm_migration::prelude::*;

use crate::search_docs;

/// The search n-grams migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// The function, then each full-text index again over it.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(&search_docs::ngrams_function())
            .await?;
        for (_, table, ..) in search_docs::SOURCES {
            conn.execute_unprepared(&format!("DROP INDEX IF EXISTS {table}_search_fts"))
                .await?;
        }
        for statement in search_docs::ngram_index_statements() {
            conn.execute_unprepared(&statement).await?;
        }
        Ok(())
    }

    /// The first full-text indexes again, then drop the function.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for (_, table, ..) in search_docs::SOURCES {
            conn.execute_unprepared(&format!("DROP INDEX IF EXISTS {table}_search_fts"))
                .await?;
        }
        for statement in search_docs::index_statements() {
            conn.execute_unprepared(&statement).await?;
        }
        conn.execute_unprepared(&format!(
            "DROP FUNCTION IF EXISTS {}(text)",
            search_docs::NGRAMS_FUNCTION
        ))
        .await?;
        Ok(())
    }
}
