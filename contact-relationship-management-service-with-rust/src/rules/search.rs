//! Search (CRM-R33): turning what was typed into a safe full-text query.
//!
//! Pure and DB-free. Only letters and digits survive, so nothing typed can become tsquery syntax.

/// Fewest characters in a query.
pub const MIN_LEN: usize = 2;
/// Most characters in a query.
pub const MAX_LEN: usize = 100;
/// Most words used.
pub const MAX_WORDS: usize = 8;

/// The words of `query`: runs of letters and digits, lower-cased, at most [`MAX_WORDS`].
#[must_use]
pub fn words(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(MAX_WORDS)
        .map(str::to_lowercase)
        .collect()
}

/// The words as a `to_tsquery` string: each a prefix, all required. `None` for no words.
#[must_use]
pub fn tsquery(words: &[String]) -> Option<String> {
    if words.is_empty() {
        return None;
    }
    Some(
        words
            .iter()
            .map(|w| format!("{w}:*"))
            .collect::<Vec<_>>()
            .join(" & "),
    )
}

/// The words joined by a space, for trigram similarity.
#[must_use]
pub fn phrase(words: &[String]) -> String {
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_letters_and_digits_survive() {
        assert_eq!(words("  Ada  Lovelace "), ["ada", "lovelace"]);
        assert_eq!(words("o'brien & (x) | !y:*"), ["o", "brien", "x", "y"]);
        assert_eq!(words("Zoë 42"), ["zoë", "42"]);
        assert!(words("&|!:*()").is_empty());
    }

    #[test]
    fn a_query_is_prefix_words_all_required() {
        let w = words("ada love");
        assert_eq!(tsquery(&w).as_deref(), Some("ada:* & love:*"));
        assert_eq!(phrase(&w), "ada love");
        assert_eq!(tsquery(&[]), None);
    }

    #[test]
    fn long_queries_are_cut_to_eight_words() {
        assert_eq!(words("a b c d e f g h i j").len(), MAX_WORDS);
    }
}
