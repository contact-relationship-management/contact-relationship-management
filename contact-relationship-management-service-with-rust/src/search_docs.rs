//! The text expression searched in each table (CRM-R33), shared by the query that uses it and,
//! through the test below, the baseline's indexes over it, so the indexes are the ones the query
//! can use. Moved here from the retired `migration` crate (CRM-T232).

/// `(kind, table, title column, detail expression, text expression)`. The text expression is
/// folded (`unaccent`, lower case) and is exactly what the baseline indexes.
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

/// The scripts written without spaces between words, and the test for one, are the domain's
/// (CRM-T260): [`UNSPACED`] and [`is_unspaced`] live in `rules::search`.
pub use contact_relationship_management_domain::rules::search::{UNSPACED, is_unspaced};

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

/// The name of the baseline's function that adds a run of [`UNSPACED`] characters as its single
/// characters and overlapping pairs, as the query and the indexes call it.
pub const NGRAMS_FUNCTION: &str = "search_ngrams";

/// The full-text document of a text expression: [`NGRAMS_FUNCTION`] over it (CRM-T170).
#[must_use]
pub fn fts_doc(doc: &str) -> String {
    format!("{NGRAMS_FUNCTION}({doc})")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE: &str = include_str!("../migrations/20261110000000_baseline.sql");

    #[test]
    fn the_class_names_every_range() {
        let class = unspaced_class();
        assert!(class.starts_with("[\\u0E00-\\u0E7F"), "{class}");
        assert!(class.contains("\\U00020000-\\U000323AF"), "{class}");
        assert_eq!(class.matches('-').count(), UNSPACED.len());
        assert!(is_unspaced('芳') && is_unspaced('ส') && is_unspaced('민') && is_unspaced('か'));
        assert!(!is_unspaced('a') && !is_unspaced('ü') && !is_unspaced('प') && !is_unspaced('ن'));
        // Ranges are ordered and do not overlap.
        assert!(UNSPACED.windows(2).all(|w| w[0].1 < w[1].0));
        assert_eq!(fts_doc("lower(name)"), "search_ngrams(lower(name))");
    }

    /// The baseline's function splits the same characters the query does, and every searched table
    /// has both indexes. `pg_dump` rewrote the index expressions (casts, `public.`), so their text
    /// is not compared; the search request tests run the query against them.
    #[test]
    fn the_baseline_has_the_function_and_every_index_the_query_uses() {
        let function = format!("CREATE FUNCTION public.{NGRAMS_FUNCTION}(t text)");
        assert!(BASELINE.contains(&function), "{function}");
        assert!(BASELINE.contains(&format!("regexp_matches(t, E'{}+'", unspaced_class())));
        for (_, table, ..) in SOURCES {
            for suffix in ["search_fts", "search_trgm"] {
                let index = format!("CREATE INDEX {table}_{suffix} ON public.{table} USING gin");
                assert!(BASELINE.contains(&index), "{index}");
            }
            let fts = format!(
                "public.{table} USING gin (to_tsvector('simple'::regconfig, public.{NGRAMS_FUNCTION}("
            );
            assert!(BASELINE.contains(&fts), "{fts}");
        }
    }
}
