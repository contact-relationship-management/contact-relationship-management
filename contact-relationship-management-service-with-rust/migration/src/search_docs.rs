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

/// Scripts written without spaces between words (CRM-T170), as inclusive code-point ranges: Thai,
/// Lao, Myanmar, Khmer, Hangul syllables (Korean names are written without a space), Hiragana,
/// Katakana and its extensions, and the CJK ideographs (unified, extension A, compatibility, and
/// the supplementary planes' extensions). A run of these is indexed as its single characters and
/// overlapping pairs ([`NGRAMS_FUNCTION`]), so a part of a name such as `芳` in `王芳` is found.
/// Changing this table needs a new migration that recreates the function and the indexes.
pub const UNSPACED: &[(u32, u32)] = &[
    (0x0E00, 0x0E7F),
    (0x0E80, 0x0EFF),
    (0x1000, 0x109F),
    (0x1780, 0x17FF),
    (0x3040, 0x309F),
    (0x30A0, 0x30FF),
    (0x31F0, 0x31FF),
    (0x3400, 0x4DBF),
    (0x4E00, 0x9FFF),
    (0xAC00, 0xD7AF),
    (0xF900, 0xFAFF),
    (0xFF66, 0xFF9F),
    (0x20000, 0x323AF),
];

/// Whether `c` is in a script of [`UNSPACED`].
#[must_use]
pub fn is_unspaced(c: char) -> bool {
    let code = u32::from(c);
    UNSPACED.iter().any(|(lo, hi)| (*lo..=*hi).contains(&code))
}

/// [`UNSPACED`] as a PostgreSQL regular-expression bracket expression.
#[must_use]
pub fn unspaced_class() -> String {
    let escape = |code: u32| {
        if code > 0xFFFF {
            format!("\\U{code:08X}")
        } else {
            format!("\\u{code:04X}")
        }
    };
    let mut class = String::from("[");
    for (lo, hi) in UNSPACED {
        class.push_str(&escape(*lo));
        class.push('-');
        class.push_str(&escape(*hi));
    }
    class.push(']');
    class
}

/// The SQL that creates `search_ngrams(text)`: the text, then each run of [`UNSPACED`]
/// characters as its single characters and its overlapping pairs, space-separated, so the
/// `simple` text-search parser makes a word of each. The same splitting is done to a query by the
/// service's `rules::search::terms`. Immutable: it reads nothing but its argument.
#[must_use]
pub fn ngrams_function() -> String {
    format!(
        "CREATE OR REPLACE FUNCTION search_ngrams(t text) RETURNS text \
         LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE STRICT AS $$ \
         DECLARE extra text := ''; run text; n int; \
         BEGIN \
           FOR run IN SELECT (regexp_matches(t, E'{class}+', 'g'))[1] LOOP \
             n := char_length(run); \
             FOR i IN 1..n LOOP \
               extra := extra || ' ' || substr(run, i, 1); \
               IF i < n THEN extra := extra || ' ' || substr(run, i, 2); END IF; \
             END LOOP; \
           END LOOP; \
           RETURN t || extra; \
         END $$",
        class = unspaced_class()
    )
}

/// The name of [`ngrams_function`]'s function, as the query and the indexes call it.
pub const NGRAMS_FUNCTION: &str = "search_ngrams";

/// The full-text document of a text expression: [`NGRAMS_FUNCTION`] over it (CRM-T170).
#[must_use]
pub fn fts_doc(doc: &str) -> String {
    format!("{NGRAMS_FUNCTION}({doc})")
}

/// The full-text indexes over [`fts_doc`], replacing the first ones (CRM-T170).
#[must_use]
pub fn ngram_index_statements() -> Vec<String> {
    SOURCES
        .iter()
        .map(|(_, table, _, _, doc)| {
            format!(
                "CREATE INDEX IF NOT EXISTS {table}_search_fts ON {table} \
                 USING gin (to_tsvector('simple', {}))",
                fts_doc(doc)
            )
        })
        .collect()
}

/// The statements that create the indexes the query used first (migration `000024`); the
/// full-text ones are replaced by [`ngram_index_statements`] in migration `000055`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_class_names_every_range_and_the_function_uses_it() {
        let class = unspaced_class();
        assert!(class.starts_with("[\\u0E00-\\u0E7F"), "{class}");
        assert!(class.contains("\\U00020000-\\U000323AF"), "{class}");
        assert_eq!(class.matches('-').count(), UNSPACED.len());
        assert!(ngrams_function().contains(&class));
        assert!(is_unspaced('芳') && is_unspaced('ส') && is_unspaced('민') && is_unspaced('か'));
        assert!(!is_unspaced('a') && !is_unspaced('ü') && !is_unspaced('प') && !is_unspaced('ن'));
        // Ranges are ordered and do not overlap.
        assert!(UNSPACED.windows(2).all(|w| w[0].1 < w[1].0));
        assert_eq!(fts_doc("lower(name)"), "search_ngrams(lower(name))");
    }
}
