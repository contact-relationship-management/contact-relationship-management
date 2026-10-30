//! Search (CRM-R33): turning what was typed into a safe full-text query.
//!
//! Pure and DB-free. Only letters, digits and the marks that belong to them survive, so nothing
//! typed can become tsquery syntax.
//!
//! **Every script (CRM-T170).** A word keeps its combining marks, so a Devanagari or Thai word is
//! not cut at a vowel sign. Scripts written without spaces between words (Chinese, Japanese,
//! Korean names, Thai, Lao, Khmer, Myanmar: `migration::search_docs::UNSPACED`) are searched by
//! character n-grams, the method of the CJK analysers of Lucene and Elasticsearch: the index holds
//! each run's single characters and overlapping pairs (the `search_ngrams` SQL function), and a
//! query's run becomes the same pairs, all required ([`terms`]). A segmenter (ICU word breaks, as
//! `Intl.Segmenter` does in a browser) was not chosen: PostgreSQL has none built in, an extension
//! such as `pg_bigm` or `zhparser` would be one more thing every deployer must install, and a
//! dictionary segmenter splits a name it does not know unpredictably, where pairs find any part
//! of two characters or more and a single ideograph.

use migration::search_docs::is_unspaced;
use unicode_normalization::char::is_combining_mark;

/// Fewest characters in a query.
pub const MIN_LEN: usize = 2;
/// Most characters in a query.
pub const MAX_LEN: usize = 100;
/// Most words used.
pub const MAX_WORDS: usize = 8;
/// Most query terms after n-grams are made.
pub const MAX_TERMS: usize = 16;

/// Whether `c` is a character a word is made of: a letter, a digit or a combining mark.
fn in_word(c: char) -> bool {
    c.is_alphanumeric() || is_combining_mark(c)
}

/// Whether `c` is an ideograph or kana, one of which can be a whole given name (`芳`), so a
/// query of one such character is long enough.
#[must_use]
pub fn is_ideographic(c: char) -> bool {
    matches!(u32::from(c), 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

/// Whether `typed` (trimmed) is long enough to search: [`MIN_LEN`] characters, or one ideograph.
#[must_use]
pub fn long_enough(typed: &str) -> bool {
    let mut chars = typed.chars();
    match (chars.next(), chars.next()) {
        (Some(only), None) => is_ideographic(only),
        (Some(_), Some(_)) => typed.chars().count() >= MIN_LEN,
        _ => false,
    }
}

/// The words of `query`: runs of letters, digits and their marks, lower-cased, at most
/// [`MAX_WORDS`]. A leading mark with no letter before it is dropped.
#[must_use]
pub fn words(query: &str) -> Vec<String> {
    query
        .split(|c: char| !in_word(c))
        .map(|w| w.trim_start_matches(is_combining_mark))
        .filter(|w| !w.is_empty())
        .take(MAX_WORDS)
        .map(str::to_lowercase)
        .collect()
}

/// The terms a word is searched by: the word, or for a word wholly in an unspaced script the
/// overlapping pairs of its characters (one character stays one term), as the index holds them.
#[must_use]
pub fn terms(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < 2 || !chars.iter().copied().all(is_unspaced) {
        return vec![word.to_string()];
    }
    chars.windows(2).map(|pair| pair.iter().collect()).collect()
}

/// The words as a `to_tsquery` string: each term a prefix, all required, at most [`MAX_TERMS`].
/// `None` for no words.
#[must_use]
pub fn tsquery(words: &[String]) -> Option<String> {
    if words.is_empty() {
        return None;
    }
    let mut all: Vec<String> = Vec::new();
    for term in words.iter().flat_map(|w| terms(w)) {
        if !all.contains(&term) {
            all.push(term);
        }
    }
    Some(
        all.iter()
            .take(MAX_TERMS)
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
    fn a_word_keeps_its_marks_in_every_script() {
        assert_eq!(words("प्रिया वेंकटरमन"), ["प्रिया", "वेंकटरमन"]);
        assert_eq!(words("สมศรี ใจดี"), ["สมศรี", "ใจดี"]);
        assert_eq!(words("نور حداد"), ["نور", "حداد"]);
        assert_eq!(
            words("\u{301}abc"),
            ["abc"],
            "a stray leading mark is dropped"
        );
    }

    #[test]
    fn unspaced_scripts_are_searched_by_pairs() {
        assert_eq!(terms("王芳"), ["王芳"]);
        assert_eq!(terms("王芳科技"), ["王芳", "芳科", "科技"]);
        assert_eq!(terms("芳"), ["芳"]);
        assert_eq!(terms("たなか"), ["たな", "なか"]);
        assert_eq!(terms("สมศรี"), ["สม", "มศ", "ศร", "รี"]);
        assert_eq!(terms("민준"), ["민준"]);
        assert_eq!(terms("lovelace"), ["lovelace"]);
        // A word mixing scripts is kept whole.
        assert_eq!(terms("abc王芳"), ["abc王芳"]);
        assert_eq!(
            tsquery(&words("芳科技")).as_deref(),
            Some("芳科:* & 科技:*")
        );
        assert_eq!(
            tsquery(&words("芳 芳")).as_deref(),
            Some("芳:*"),
            "once each"
        );
        let long = "王".repeat(40);
        assert_eq!(tsquery(&words(&long)).as_deref(), Some("王王:*"));
        let varied: String = ('\u{4e00}'..='\u{4e30}').collect();
        assert_eq!(
            tsquery(&words(&varied)).unwrap().matches(" & ").count(),
            MAX_TERMS - 1
        );
    }

    #[test]
    fn one_ideograph_is_long_enough_and_one_letter_is_not() {
        assert!(long_enough("芳"));
        assert!(long_enough("か"));
        assert!(!long_enough("a"));
        assert!(!long_enough("ส"), "a Thai letter alone is not a word");
        assert!(long_enough("ab"));
        assert!(!long_enough(""));
    }

    #[test]
    fn long_queries_are_cut_to_eight_words() {
        assert_eq!(words("a b c d e f g h i j").len(), MAX_WORDS);
    }
}
