//! Words the matcher sets aside, by language (CRM-T169, CRM-R95): honorifics and name suffixes
//! that are not part of a name, and legal-form words that are not part of an organization's name.
//!
//! Each list is data, one row per language (an ISO 639-1 code, or the language a romanised form is
//! from). The matcher does not know a contact's language, so every row applies to every contact; a
//! word that is a name in some language is left out of the lists for that reason (`don`, `dona`).
//! Entries are written as people type them and folded ([`crate::normalize::fold`]) when the sets are
//! built. Scripts written without spaces carry their honorifics and legal forms joined to the name
//! (`王芳女士`, `株式会社山田`), so those are separate lists, stripped from the start or end of a word.
//! The lists were drafted by the AI assistant from general knowledge and were **not checked by a
//! speaker of each language**; a deployer adds to them through configuration only for nicknames.

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::normalize::fold;

/// One language's words.
pub(crate) type Row = (&'static str, &'static [&'static str]);

/// Titles written before a name.
pub(crate) const HONORIFICS: &[Row] = &[
    (
        "en",
        &[
            "mr", "mrs", "ms", "miss", "mx", "dr", "prof", "sir", "dame", "lord", "lady", "rev",
        ],
    ),
    ("cy", &["parch"]),
    ("de", &["herr", "frau", "fräulein", "dipl", "ing"]),
    ("fr", &["m", "mme", "mlle", "me", "pr"]),
    ("es", &["sr", "sra", "srta", "dra", "lic"]),
    ("pt", &["eng", "exmo", "exma"]),
    ("it", &["sig", "sigra", "dott", "dottssa", "avv"]),
    ("nl", &["dhr", "mevr", "ir", "drs"]),
    ("pl", &["pan", "pani"]),
    ("ru", &["господин", "госпожа"]),
    ("tr", &["bay", "bayan"]),
    (
        "ar",
        &["السيد", "السيدة", "الدكتور", "الدكتورة", "sheikh", "sayyid"],
    ),
    (
        "hi",
        &["श्री", "श्रीमती", "सुश्री", "डॉ", "shri", "smt", "kumari"],
    ),
];

/// Words written after a name that are not part of it.
pub(crate) const NAME_SUFFIXES: &[Row] = &[
    ("en", &["jr", "sr", "ii", "iii", "iv", "phd", "md", "esq"]),
    ("pt", &["filho", "neto", "junior"]),
    ("es", &["hijo"]),
    ("ja", &["san", "sama", "kun"]),
];

/// Honorifics joined to the end of a name in scripts written without spaces.
pub(crate) const JOINED_HONORIFICS: &[Row] = &[
    ("zh", &["先生", "女士", "小姐"]),
    ("ja", &["様", "さま", "さん", "殿"]),
    ("ko", &["님", "씨"]),
];

/// Legal-form words in an organization's name.
pub(crate) const LEGAL_WORDS: &[Row] = &[
    (
        "en",
        &[
            "inc",
            "incorporated",
            "llc",
            "ltd",
            "limited",
            "corp",
            "corporation",
            "co",
            "company",
            "plc",
            "llp",
            "lp",
            "the",
        ],
    ),
    ("de", &["gmbh", "ag", "kg", "ohg", "ug", "ev", "mbh"]),
    ("fr", &["sa", "sarl", "sas", "sasu", "eurl", "sci"]),
    ("es", &["sl", "slu", "sau"]),
    ("it", &["spa", "srl", "snc"]),
    ("pt", &["ltda", "eireli"]),
    ("nl", &["bv", "nv", "vof"]),
    ("sv", &["ab"]),
    ("fi", &["oy", "oyj"]),
    ("no", &["asa", "as"]),
    ("da", &["aps"]),
    ("pl", &["spolka"]),
    (
        "ru",
        &["ooo", "oao", "pao", "zao", "ооо", "оао", "пао", "зао"],
    ),
    ("ja", &["kk", "gk"]),
    ("hi", &["pvt", "private"]),
    ("ms", &["sdn", "bhd", "berhad", "pte"]),
    ("es-MX", &["cv", "sapi"]),
    ("tr", &["sti"]),
    ("id", &["tbk"]),
    ("en-AU", &["pty"]),
];

/// Legal forms joined to the start or end of a name in scripts written without spaces.
pub(crate) const JOINED_LEGAL_FORMS: &[Row] = &[
    (
        "zh",
        &[
            "股份有限公司",
            "有限责任公司",
            "有限責任公司",
            "有限公司",
            "集团",
            "集團",
        ],
    ),
    ("ja", &["株式会社", "有限会社", "合同会社"]),
    ("ko", &["주식회사", "유한회사"]),
];

fn folded(rows: &[Row]) -> HashSet<String> {
    rows.iter()
        .flat_map(|(_, words)| words.iter())
        .map(|w| fold(w))
        .filter(|w| !w.is_empty())
        .collect()
}

/// Joined forms, folded, longest first (so `股份有限公司` is tried before `有限公司`).
fn folded_longest_first(rows: &[Row]) -> Vec<String> {
    let mut out: Vec<String> = folded(rows).into_iter().collect();
    out.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));
    out
}

/// [`HONORIFICS`], folded.
pub(crate) static HONORIFIC_SET: LazyLock<HashSet<String>> = LazyLock::new(|| folded(HONORIFICS));
/// [`NAME_SUFFIXES`], folded.
pub(crate) static SUFFIX_SET: LazyLock<HashSet<String>> = LazyLock::new(|| folded(NAME_SUFFIXES));
/// [`LEGAL_WORDS`], folded.
pub(crate) static LEGAL_SET: LazyLock<HashSet<String>> = LazyLock::new(|| folded(LEGAL_WORDS));
/// [`JOINED_HONORIFICS`], folded, longest first.
pub(crate) static JOINED_HONORIFIC_LIST: LazyLock<Vec<String>> =
    LazyLock::new(|| folded_longest_first(JOINED_HONORIFICS));
/// [`JOINED_LEGAL_FORMS`], folded, longest first.
pub(crate) static JOINED_LEGAL_LIST: LazyLock<Vec<String>> =
    LazyLock::new(|| folded_longest_first(JOINED_LEGAL_FORMS));

/// `word` without a joined form from `forms` at its end (or, with `start`, its start too), when
/// something is left.
pub(crate) fn strip_joined(word: &str, forms: &[String], start: bool) -> String {
    for form in forms {
        if let Some(rest) = word.strip_suffix(form.as_str())
            && !rest.is_empty()
        {
            return rest.to_string();
        }
        if start
            && let Some(rest) = word.strip_prefix(form.as_str())
            && !rest.is_empty()
        {
            return rest.to_string();
        }
    }
    word.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn languages(rows: &[Row]) -> usize {
        rows.iter().filter(|(_, words)| !words.is_empty()).count()
    }

    #[test]
    fn every_list_holds_more_than_english() {
        for (name, rows) in [
            ("honorifics", HONORIFICS),
            ("suffixes", NAME_SUFFIXES),
            ("legal words", LEGAL_WORDS),
        ] {
            assert!(languages(rows) > 2, "{name}");
            assert!(rows.iter().any(|(l, _)| *l != "en"), "{name}");
        }
        // Non-Latin scripts are represented, not only romanised forms.
        assert!(HONORIFIC_SET.contains(&fold("السيد")));
        assert!(HONORIFIC_SET.contains(&fold("श्रीमती")));
        assert!(LEGAL_SET.contains(&fold("ООО")));
    }

    #[test]
    fn no_word_is_listed_twice_in_one_language_and_none_folds_to_nothing() {
        for rows in [
            HONORIFICS,
            NAME_SUFFIXES,
            LEGAL_WORDS,
            JOINED_HONORIFICS,
            JOINED_LEGAL_FORMS,
        ] {
            for (language, words) in rows {
                let mut seen = HashSet::new();
                for w in *words {
                    assert!(!fold(w).is_empty(), "{language}: {w}");
                    assert!(!fold(w).contains(' '), "{language}: {w} is one word");
                    assert!(seen.insert(fold(w)), "{language}: {w} twice");
                }
            }
        }
    }

    #[test]
    fn joined_forms_come_off_the_end_or_the_start_longest_first() {
        let legal = &*JOINED_LEGAL_LIST;
        assert_eq!(
            strip_joined(&fold("北京王芳科技股份有限公司"), legal, true),
            "北京王芳科技"
        );
        assert_eq!(strip_joined(&fold("株式会社山田"), legal, true), "山田");
        assert_eq!(
            strip_joined(&fold("有限公司"), legal, true),
            "有限公司",
            "never empty"
        );
        let titles = &*JOINED_HONORIFIC_LIST;
        assert_eq!(strip_joined("王芳女士", titles, false), "王芳");
        assert_eq!(strip_joined("山田様", titles, false), "山田");
        assert_eq!(
            strip_joined("様山田", titles, false),
            "様山田",
            "titles only at the end"
        );
    }
}
