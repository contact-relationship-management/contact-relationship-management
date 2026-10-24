//! The text expression searched in each table (CRM-R33), shared by the migration that indexes it
//! and the service query that uses it, so the indexes are the ones the query can use.

/// `(kind, table, title column, detail expression, text expression)`. The text expression is
/// folded (`unaccent`, lower case) and is exactly what the migration indexes.
pub const SOURCES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "contact",
        "contacts",
        "display_name",
        "job_title",
        "immutable_unaccent(lower(display_name || ' ' || coalesce(job_title, '')))",
    ),
    (
        "account",
        "accounts",
        "display_name",
        "industry",
        "immutable_unaccent(lower(display_name || ' ' || coalesce(industry, '')))",
    ),
    (
        "lead",
        "leads",
        "display_name",
        "email_domain",
        "immutable_unaccent(lower(display_name || ' ' || coalesce(email_domain, '')))",
    ),
    (
        "deal",
        "deals",
        "name",
        "NULL::text",
        "immutable_unaccent(lower(name))",
    ),
    (
        "ticket",
        "tickets",
        "title",
        "status",
        "immutable_unaccent(lower(title))",
    ),
    (
        "campaign",
        "campaigns",
        "name",
        "status",
        "immutable_unaccent(lower(name))",
    ),
    (
        "article",
        "articles",
        "title",
        "status",
        "immutable_unaccent(lower(title || ' ' || coalesce(keywords, '') || ' ' || body))",
    ),
];

/// The statements that create the indexes the query uses.
#[must_use]
pub fn index_statements() -> Vec<String> {
    SOURCES
        .iter()
        .flat_map(|(_, table, _, _, doc)| {
            [
                format!(
                    "CREATE INDEX IF NOT EXISTS {table}_search_fts ON {table} \
                     USING gin (to_tsvector('simple', {doc}))"
                ),
                format!(
                    "CREATE INDEX IF NOT EXISTS {table}_search_trgm ON {table} \
                     USING gin (({doc}) gin_trgm_ops)"
                ),
            ]
        })
        .collect()
}
