//! Migration: search indexes (CRM-R33).
//!
//! Enables `unaccent` and `pg_trgm`, defines an immutable wrapper around `unaccent` (the stock
//! function is only stable, so it cannot be indexed), and indexes each searchable table twice:
//! full text and trigrams, over the expressions in [`crate::search_docs`].

use sea_orm_migration::prelude::*;

use crate::search_docs;

/// The search migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Extensions, the wrapper, then the indexes.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("CREATE EXTENSION IF NOT EXISTS unaccent")
            .await?;
        conn.execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm")
            .await?;
        // `unaccent` is STABLE only because its dictionary can be changed; the dictionary named
        // here is fixed, which is what makes the wrapper safe to call IMMUTABLE.
        conn.execute_unprepared(
            "CREATE OR REPLACE FUNCTION immutable_unaccent(text) RETURNS text \
             LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT \
             AS $$ SELECT public.unaccent('public.unaccent'::regdictionary, $1) $$",
        )
        .await?;
        for statement in search_docs::index_statements() {
            conn.execute_unprepared(&statement).await?;
        }
        Ok(())
    }

    /// Drop the indexes and the wrapper (the extensions stay: other things may use them).
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for (_, table, ..) in search_docs::SOURCES {
            conn.execute_unprepared(&format!("DROP INDEX IF EXISTS {table}_search_fts"))
                .await?;
            conn.execute_unprepared(&format!("DROP INDEX IF EXISTS {table}_search_trgm"))
                .await?;
        }
        conn.execute_unprepared("DROP FUNCTION IF EXISTS immutable_unaccent(text)")
            .await?;
        Ok(())
    }
}
