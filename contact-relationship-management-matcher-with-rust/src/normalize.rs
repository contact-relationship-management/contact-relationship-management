//! Turning free text into comparable forms.

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Lower-case, strip accents (`Siân` → `sian`), drop apostrophes
/// (`O'Brien` → `obrien`), turn every other non-alphanumeric run into one
/// space, and trim.
pub(crate) fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.nfd().filter(|c| !is_combining_mark(*c)) {
        if c == '\'' || c == '\u{2019}' {
            continue;
        }
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with(' ') && !out.is_empty() {
            out.push(' ');
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

const HONORIFICS: &[&str] = &["mr", "mrs", "ms", "miss", "mx", "dr", "prof", "sir", "dame"];
const NAME_SUFFIXES: &[&str] = &["jr", "sr", "ii", "iii", "iv", "phd", "md", "esq"];

/// A name split into the two parts the matcher compares, both folded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Name {
    /// First word of the given name.
    pub given: String,
    /// The whole family name.
    pub family: String,
}

/// Work out the given and family names of a contact, from the explicit
/// fields first and the display name for whatever is missing.
pub(crate) fn name_of(given: Option<&str>, family: Option<&str>, display: Option<&str>) -> Name {
    let first_word = |s: &str| fold(s).split(' ').next().unwrap_or("").to_string();
    let mut name = Name {
        given: given.map(first_word).unwrap_or_default(),
        family: family.map(fold).unwrap_or_default(),
    };
    if (name.given.is_empty() || name.family.is_empty())
        && let Some(display) = display
    {
        let from_display = split_display(display);
        if name.given.is_empty() {
            name.given = from_display.given;
        }
        if name.family.is_empty() {
            name.family = from_display.family;
        }
    }
    name
}

fn split_display(display: &str) -> Name {
    // "Ruiz, Ana" is family first.
    if let Some((family, given)) = display.split_once(',') {
        let given = strip_titles(&fold(given));
        return Name {
            given: given.first().cloned().unwrap_or_default(),
            family: strip_titles(&fold(family)).join(" "),
        };
    }
    let words = strip_titles(&fold(display));
    match words.as_slice() {
        [] => Name::default(),
        [only] => Name {
            given: only.clone(),
            family: String::new(),
        },
        [first, ..] => Name {
            given: first.clone(),
            // A particle starts the family name: "ana van der berg".
            family: words[family_start(&words)..].join(" "),
        },
    }
}

const PARTICLES: &[&str] = &[
    "van", "von", "de", "del", "della", "der", "den", "di", "da", "la", "le", "bin", "al",
];

/// Index where the family name starts: the last word, or the earliest
/// particle (`van`, `de`, …) after the first word.
fn family_start(words: &[String]) -> usize {
    words
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, w)| PARTICLES.contains(&w.as_str()))
        .map_or(words.len() - 1, |(i, _)| i)
}

fn strip_titles(folded: &str) -> Vec<String> {
    let mut words: Vec<String> = folded.split_whitespace().map(str::to_string).collect();
    while words.len() > 1 && HONORIFICS.contains(&words[0].as_str()) {
        words.remove(0);
    }
    while words.len() > 1 && NAME_SUFFIXES.contains(&words[words.len() - 1].as_str()) {
        words.pop();
    }
    words
}

/// Canonical form of an email address, or `None` if it is not one.
pub(crate) fn email(raw: &str, fold_gmail: bool) -> Option<String> {
    let trimmed = raw.trim();
    let trimmed = trimmed
        .strip_prefix("mailto:")
        .unwrap_or(trimmed)
        .trim()
        .to_lowercase();
    let (local, domain) = trimmed.rsplit_once('@')?;
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || trimmed.chars().any(char::is_whitespace)
    {
        return None;
    }
    let (local, domain) = if fold_gmail && (domain == "gmail.com" || domain == "googlemail.com") {
        let local = local.split('+').next().unwrap_or(local).replace('.', "");
        (local, "gmail.com".to_string())
    } else {
        (local.to_string(), domain.to_string())
    };
    if local.is_empty() {
        return None;
    }
    Some(format!("{local}@{domain}"))
}

/// A phone number reduced to digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Phone {
    pub digits: String,
    /// Starts with a country calling code.
    pub international: bool,
}

const MIN_PHONE_DIGITS: usize = 7;

/// Reduce a phone number to digits, dropping punctuation and an extension
/// (`x123`, `ext. 123`). Returns `None` when too few digits remain.
pub(crate) fn phone(raw: &str, default_code: Option<&str>) -> Option<Phone> {
    let lower = raw.trim().to_lowercase();
    let main = lower
        .find("ext")
        .or_else(|| lower.find('x'))
        .map_or(lower.as_str(), |i| &lower[..i]);
    let plus = main.trim_start().starts_with('+');
    let mut digits: String = main.chars().filter(char::is_ascii_digit).collect();
    let mut international = plus;
    if !plus && digits.starts_with("00") {
        digits.drain(..2);
        international = true;
    }
    if !international && let Some(code) = default_code {
        if code == "1" {
            if digits.len() == 10 {
                digits.insert(0, '1');
            }
        } else {
            if digits.starts_with('0') {
                digits.remove(0);
            }
            digits.insert_str(0, code);
        }
        international = true;
    }
    (digits.len() >= MIN_PHONE_DIGITS).then_some(Phone {
        digits,
        international,
    })
}

/// Whether two reduced phone numbers are the same line.
pub(crate) fn same_phone(a: &Phone, b: &Phone) -> bool {
    if a.international == b.international {
        return a.digits.trim_start_matches('0') == b.digits.trim_start_matches('0');
    }
    let (international, national) = if a.international { (a, b) } else { (b, a) };
    let national = national.digits.trim_start_matches('0');
    national.len() >= MIN_PHONE_DIGITS && international.digits.ends_with(national)
}

const LEGAL_WORDS: &[&str] = &[
    "inc",
    "incorporated",
    "llc",
    "ltd",
    "limited",
    "corp",
    "corporation",
    "co",
    "company",
    "gmbh",
    "ag",
    "plc",
    "sa",
    "bv",
    "nv",
    "srl",
    "pty",
    "llp",
    "lp",
    "the",
];

/// An organisation name without punctuation or legal-form words.
pub(crate) fn organization(raw: &str) -> String {
    let folded = fold(&raw.replace('&', " and "));
    let words: Vec<&str> = folded
        .split_whitespace()
        .filter(|w| !LEGAL_WORDS.contains(w))
        .collect();
    if words.is_empty() {
        folded
    } else {
        words.join(" ")
    }
}

/// A job title with common abbreviations spelled out.
pub(crate) fn job_title(raw: &str) -> String {
    fold(raw)
        .split_whitespace()
        .map(|w| match w {
            "vp" => "vice president",
            "svp" => "senior vice president",
            "ceo" => "chief executive officer",
            "cto" => "chief technology officer",
            "cfo" => "chief financial officer",
            "coo" => "chief operating officer",
            "cmo" => "chief marketing officer",
            "mgr" => "manager",
            "dir" => "director",
            "eng" => "engineer",
            "sr" => "senior",
            "jr" => "junior",
            "asst" => "assistant",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_case_accents_and_apostrophes() {
        assert_eq!(fold("  Siân  O'Brien-Smith "), "sian obrien smith");
        assert_eq!(fold("José"), "jose");
        assert_eq!(fold("   "), "");
    }

    #[test]
    fn names_come_from_fields_or_display() {
        let n = name_of(Some("Ana Maria"), Some("Ruiz"), None);
        assert_eq!((n.given.as_str(), n.family.as_str()), ("ana", "ruiz"));
        let n = name_of(None, None, Some("Dr Ana Ruiz Jr"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("ana", "ruiz"));
        let n = name_of(None, None, Some("Ruiz, Ana"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("ana", "ruiz"));
        let n = name_of(None, None, Some("Ana van der Berg"));
        assert_eq!(
            (n.given.as_str(), n.family.as_str()),
            ("ana", "van der berg")
        );
        // An explicit field wins over the display name.
        let n = name_of(Some("Anna"), None, Some("Ana Ruiz"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("anna", "ruiz"));
        // A single word is a given name.
        let n = name_of(None, None, Some("Cher"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("cher", ""));
    }

    #[test]
    fn emails() {
        assert_eq!(
            email(" Ana@Example.COM ", true).as_deref(),
            Some("ana@example.com")
        );
        assert_eq!(
            email("mailto:ana@example.com", true).as_deref(),
            Some("ana@example.com")
        );
        assert_eq!(
            email("a.n.a+news@gmail.com", true).as_deref(),
            Some("ana@gmail.com")
        );
        assert_eq!(
            email("a.n.a@googlemail.com", true).as_deref(),
            Some("ana@gmail.com")
        );
        assert_eq!(
            email("a.n.a@gmail.com", false).as_deref(),
            Some("a.n.a@gmail.com")
        );
        // Dots count outside Gmail.
        assert_eq!(
            email("a.n.a@example.com", true).as_deref(),
            Some("a.n.a@example.com")
        );
        for bad in [
            "",
            "ana",
            "@example.com",
            "ana@",
            "ana@localhost",
            "a b@x.com",
        ] {
            assert_eq!(email(bad, true), None, "{bad}");
        }
    }

    #[test]
    fn phones() {
        let p = |s| phone(s, None).unwrap();
        assert_eq!(p("+44 20 7946 0958").digits, "442079460958");
        assert_eq!(p("0044 20 7946 0958").digits, "442079460958");
        assert!(p("+1 (415) 555-0100 x22").international);
        assert_eq!(p("(415) 555-0100 ext. 9").digits, "4155550100");
        assert_eq!(phone("12345", None), None);
        assert_eq!(
            phone("020 7946 0958", Some("44")).unwrap().digits,
            "442079460958"
        );
        assert_eq!(
            phone("415 555 0100", Some("1")).unwrap().digits,
            "14155550100"
        );
    }

    #[test]
    fn phones_compare_across_formats() {
        let p = |s| phone(s, None).unwrap();
        assert!(same_phone(&p("+44 20 7946 0958"), &p("0044 2079 460958")));
        // National against international: the national digits end the other.
        assert!(same_phone(&p("+44 20 7946 0958"), &p("020 7946 0958")));
        assert!(!same_phone(&p("+44 20 7946 0958"), &p("020 7946 0959")));
        // Two different countries with overlapping digits do not collide.
        assert!(!same_phone(&p("+44 20 7946 0958"), &p("+49 20 7946 0958")));
    }

    #[test]
    fn organizations_and_titles() {
        assert_eq!(organization("The Acme Widgets, Inc."), "acme widgets");
        assert_eq!(organization("Smith & Sons Ltd"), "smith and sons");
        assert_eq!(organization("Inc"), "inc"); // never empty
        assert_eq!(job_title("Sr. Eng Mgr"), "senior engineer manager");
        assert_eq!(job_title("VP, Sales"), "vice president sales");
    }
}
