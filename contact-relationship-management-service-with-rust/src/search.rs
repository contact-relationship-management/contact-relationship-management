//! The SQL behind search (CRM-R33). The text expressions live in the migration crate next to the
//! indexes built on them.

pub use migration::search_docs::SOURCES;
use migration::search_docs::fts_doc;

/// The `UNION ALL` query. `$1` is the tsquery, `$2` the folded phrase, `$3` the limit, `$4` the
/// offset. Each branch can use its two indexes.
#[must_use]
pub fn query() -> String {
    let branches: Vec<String> = SOURCES
        .iter()
        .map(|(kind, table, title, detail, doc)| {
            // The full-text side reads the n-grams of unspaced scripts (CRM-T170); the trigram
            // side the text as it is.
            let fts = fts_doc(doc);
            format!(
                "(SELECT '{kind}' AS kind, pid, {title} AS title, {detail}::text AS detail, \
                 ts_rank(to_tsvector('simple', {fts}), to_tsquery('simple', immutable_unaccent($1))) \
                   + word_similarity($2, {doc}) AS score \
                 FROM {table} \
                 WHERE deleted_at IS NULL \
                   AND (to_tsvector('simple', {fts}) @@ to_tsquery('simple', immutable_unaccent($1)) \
                        OR $2 <% {doc}) \
                 ORDER BY score DESC, pid LIMIT $3 + $4)"
            )
        })
        .collect();
    format!(
        "SELECT kind, pid, title, detail, score FROM ({}) hits \
         ORDER BY score DESC, kind, pid LIMIT $3 OFFSET $4",
        branches.join(" UNION ALL ")
    )
}
