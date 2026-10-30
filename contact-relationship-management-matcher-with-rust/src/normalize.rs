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

use crate::words::{
    HONORIFIC_SET, JOINED_HONORIFIC_LIST, JOINED_LEGAL_LIST, LEGAL_SET, SUFFIX_SET, strip_joined,
};

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

/// The words of a folded name without honorifics before it and suffixes after it, in any of the
/// languages of [`crate::words`]; a title joined to the end of a word (`王芳女士`) comes off too.
fn strip_titles(folded: &str) -> Vec<String> {
    let mut words: Vec<String> = folded
        .split_whitespace()
        .map(|w| strip_joined(w, &JOINED_HONORIFIC_LIST, false))
        .collect();
    while words.len() > 1 && HONORIFIC_SET.contains(&words[0]) {
        words.remove(0);
    }
    while words.len() > 1 && SUFFIX_SET.contains(&words[words.len() - 1]) {
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

/// The zero of each Unicode decimal digit set a phone number may be typed in; the nine digits after
/// it are 1 to 9 (the same table as the workforce planning service's `rules::contact_details`,
/// WPM-T382). ASCII, and full width (folded by [`fold_width`]), are read before this: Arabic-Indic,
/// Extended Arabic-Indic (Persian, Urdu), N'Ko, Devanagari, Bengali, Gurmukhi, Gujarati, Oriya,
/// Tamil, Telugu, Kannada, Malayalam, Sinhala Lith, Thai, Lao, Tibetan, Myanmar, Myanmar Shan,
/// Khmer, Mongolian, Limbu, New Tai Lue, Balinese, Sundanese, Lepcha, Ol Chiki, Vai, Javanese,
/// Myanmar Tai Laing, Cham, Meetei Mayek.
pub(crate) const DIGIT_ZEROS: [u32; 31] = [
    0x0660, 0x06f0, 0x07c0, 0x0966, 0x09e6, 0x0a66, 0x0ae6, 0x0b66, 0x0be6, 0x0c66, 0x0ce6, 0x0d66,
    0x0de6, 0x0e50, 0x0ed0, 0x0f20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1b50, 0x1bb0,
    0x1c40, 0x1c50, 0xa620, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
];

/// A full-width form (`＋`, `８`, `（`) as its ASCII one, and the ideographic space as a space, as an
/// East Asian keyboard types them; any other character unchanged.
fn fold_width(c: char) -> char {
    match u32::from(c) {
        0x3000 => ' ',
        code @ 0xff01..=0xff5e => char::from_u32(code - 0xfee0).unwrap_or(c),
        _ => c,
    }
}

/// The ASCII digit of a decimal digit in ASCII, full width or any set in [`DIGIT_ZEROS`]; `None`
/// otherwise (a superscript or a Roman numeral is not a digit).
fn ascii_digit(c: char) -> Option<char> {
    let c = fold_width(c);
    if c.is_ascii_digit() {
        return Some(c);
    }
    let code = u32::from(c);
    DIGIT_ZEROS
        .iter()
        .find(|zero| (**zero..**zero + 10).contains(&code))
        .and_then(|zero| char::from_digit(code - zero, 10))
}

/// The trunk prefix a national number starts with, by country calling code, where the plan has
/// one: `Some("0")` for most, `Some("8")` (Russia and Kazakhstan, Belarus) or `Some("06")`
/// (Hungary) for a few, and `None` for a plan with no trunk prefix, where a leading `0` is part of
/// the number (Italy, San Marino and the Vatican; Spain, Portugal, Greece, Poland, Czechia, the
/// Nordic plans of Denmark, Norway and Iceland; Luxembourg, Malta, Cyprus, Estonia, Latvia, Monaco
/// and Andorra; Qatar, Kuwait, Bahrain and Oman; Singapore, Hong Kong and Macao; Mexico and the
/// Central American plans). From the ITU-T E.164 national numbering plans as summarised in public
/// references, **not checked against each regulator**. A code not listed is treated as having no
/// trunk prefix, so nothing is dropped that may be part of the number. The North American plan
/// (`1`) is handled on its own: a ten-digit national number gets the `1`.
pub(crate) const TRUNK_PREFIXES: &[(&str, Option<&str>)] = &[
    ("20", Some("0")),
    ("212", Some("0")),
    ("213", Some("0")),
    ("216", Some("0")),
    ("233", Some("0")),
    ("234", Some("0")),
    ("251", Some("0")),
    ("254", Some("0")),
    ("255", Some("0")),
    ("256", Some("0")),
    ("27", Some("0")),
    ("30", None),
    ("31", Some("0")),
    ("32", Some("0")),
    ("33", Some("0")),
    ("34", None),
    ("351", None),
    ("352", None),
    ("353", Some("0")),
    ("354", None),
    ("356", None),
    ("357", None),
    ("358", Some("0")),
    ("359", Some("0")),
    ("36", Some("06")),
    ("371", None),
    ("372", None),
    ("375", Some("8")),
    ("376", None),
    ("377", None),
    ("378", None),
    ("379", None),
    ("380", Some("0")),
    ("381", Some("0")),
    ("385", Some("0")),
    ("386", Some("0")),
    ("39", None),
    ("40", Some("0")),
    ("41", Some("0")),
    ("420", None),
    ("421", Some("0")),
    ("43", Some("0")),
    ("44", Some("0")),
    ("45", None),
    ("46", Some("0")),
    ("47", None),
    ("48", None),
    ("49", Some("0")),
    ("501", None),
    ("502", None),
    ("503", None),
    ("504", None),
    ("505", None),
    ("506", None),
    ("507", None),
    ("51", Some("0")),
    ("52", None),
    ("54", Some("0")),
    ("55", Some("0")),
    ("60", Some("0")),
    ("61", Some("0")),
    ("62", Some("0")),
    ("63", Some("0")),
    ("64", Some("0")),
    ("65", None),
    ("66", Some("0")),
    ("7", Some("8")),
    ("81", Some("0")),
    ("82", Some("0")),
    ("84", Some("0")),
    ("852", None),
    ("853", None),
    ("86", Some("0")),
    ("880", Some("0")),
    ("886", Some("0")),
    ("90", Some("0")),
    ("91", Some("0")),
    ("92", Some("0")),
    ("94", Some("0")),
    ("961", Some("0")),
    ("962", Some("0")),
    ("965", None),
    ("966", Some("0")),
    ("968", None),
    ("971", Some("0")),
    ("972", Some("0")),
    ("973", None),
    ("974", None),
    ("98", Some("0")),
];

/// The trunk prefix of the plan for `code`, if it has one (see [`TRUNK_PREFIXES`]).
pub(crate) fn trunk_prefix(code: &str) -> Option<&'static str> {
    TRUNK_PREFIXES
        .iter()
        .find(|(c, _)| *c == code)
        .and_then(|(_, trunk)| *trunk)
}

/// A phone number reduced to digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Phone {
    pub digits: String,
    /// Starts with a country calling code.
    pub international: bool,
}

const MIN_PHONE_DIGITS: usize = 7;

/// Reduce a phone number to ASCII digits, dropping punctuation and an extension
/// (`x123`, `ext. 123`). Digits may be typed in any Unicode decimal digit set in
/// [`DIGIT_ZEROS`] or in full width (`＋８１`). With a default calling code, a national number
/// loses its trunk prefix only where that code's plan has one ([`trunk_prefix`]): `020…` with `44`
/// drops the `0`, `06…` with `39` keeps it. Returns `None` when too few digits remain.
pub(crate) fn phone(raw: &str, default_code: Option<&str>) -> Option<Phone> {
    let lower: String = raw
        .trim()
        .chars()
        .map(fold_width)
        .collect::<String>()
        .to_lowercase();
    let main = lower
        .find("ext")
        .or_else(|| lower.find('x'))
        .map_or(lower.as_str(), |i| &lower[..i]);
    let plus = main.trim_start().starts_with('+');
    let mut digits: String = main.chars().filter_map(ascii_digit).collect();
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
            if let Some(trunk) = trunk_prefix(code)
                && let Some(rest) = digits.strip_prefix(trunk)
            {
                digits = rest.to_string();
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

/// An organisation name without punctuation or legal-form words, in any of the languages of
/// [`crate::words`]; a legal form joined to the start or end of a word (`株式会社`, `有限公司`)
/// comes off too.
pub(crate) fn organization(raw: &str) -> String {
    let folded = fold(&raw.replace('&', " and "));
    let joined: Vec<String> = folded
        .split_whitespace()
        .map(|w| strip_joined(w, &JOINED_LEGAL_LIST, true))
        .collect();
    let words: Vec<&str> = joined
        .iter()
        .map(String::as_str)
        .filter(|w| !LEGAL_SET.contains(*w))
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
        // Honorifics and suffixes in other languages (CRM-T169).
        let n = name_of(None, None, Some("Frau Anna Schmidt"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("anna", "schmidt"));
        let n = name_of(None, None, Some("Mme Claire Dubois"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("claire", "dubois"));
        let n = name_of(None, None, Some("Sra. Lucía Pérez"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("lucia", "perez"));
        let n = name_of(None, None, Some("João Silva Filho"));
        assert_eq!((n.given.as_str(), n.family.as_str()), ("joao", "silva"));
        let n = name_of(None, None, Some("श्रीमती प्रिया शर्मा"));
        assert_eq!(n.family, fold("शर्मा"));
        let n = name_of(None, None, Some("السيد نور حداد"));
        assert_eq!((n.given, n.family), (fold("نور"), fold("حداد")));
        let n = name_of(None, None, Some("王芳女士"));
        assert_eq!(n.given, "王芳");
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
    fn phone_digits_may_be_typed_in_any_script() {
        let p = |s| phone(s, None).unwrap().digits;
        assert_eq!(p("+٩٦٦ ٥٠ ١٢٣ ٤٥٦٧"), "966501234567"); // Arabic-Indic
        assert_eq!(p("+۹۲ ۳۰۰ ۱۲۳۴۵۶۷"), "923001234567"); // Extended Arabic-Indic
        assert_eq!(p("+९१ ९८७६५ ४३२१०"), "919876543210"); // Devanagari
        assert_eq!(p("+৮৮০ ১৭১১ ২৩৪৫৬৭"), "8801711234567"); // Bengali
        assert_eq!(p("＋８１ ９０-１２３４-５６７８"), "819012345678"); // full width
        assert_eq!(p("+66 ๘๑ ๒๓๔ ๕๖๗๘"), "66812345678"); // Thai digits after ASCII
        assert!(
            phone("+٩٦٦ ٥٠", None).is_none(),
            "five digits are too few in any script"
        );
        assert!(
            phone("²³⁴⁵⁶⁷⁸⁹", None).is_none(),
            "superscripts are not digits"
        );
        // The same line typed in two scripts is one line.
        assert!(same_phone(
            &phone("+٩٦٦ ٥٠ ١٢٣ ٤٥٦٧", None).unwrap(),
            &phone("+966 50 123 4567", None).unwrap()
        ));
        // Every set is ten consecutive code points that Unicode calls numeric.
        for zero in DIGIT_ZEROS {
            for d in 0..10 {
                let c = char::from_u32(zero + d).unwrap();
                assert!(c.is_numeric(), "{zero:#x} + {d}");
                assert_eq!(ascii_digit(c), char::from_digit(d, 10));
            }
        }
    }

    #[test]
    fn a_trunk_prefix_is_dropped_only_where_the_plan_has_one() {
        let national = |s, code| phone(s, Some(code)).unwrap().digits;
        // The UK, Germany, Japan, India: a 0 trunk prefix.
        assert_eq!(national("020 7946 0958", "44"), "442079460958");
        assert_eq!(national("030 123456", "49"), "4930123456");
        assert_eq!(national("090-1234-5678", "81"), "819012345678");
        assert_eq!(national("098765 43210", "91"), "919876543210");
        // Italy and Spain: no trunk prefix, the 0 is part of the number.
        assert_eq!(national("06 1234 5678", "39"), "390612345678");
        assert_eq!(national("912 345 678", "34"), "34912345678");
        // Russia: the trunk prefix is 8; Hungary: 06.
        assert_eq!(national("8 495 123 45 67", "7"), "74951234567");
        assert_eq!(national("06 1 234 5678", "36"), "3612345678");
        // A plan not in the table drops nothing.
        assert_eq!(national("0 123 4567", "999"), "99901234567");
        // A national Italian number matches the same number written internationally.
        assert!(same_phone(
            &phone("06 1234 5678", Some("39")).unwrap(),
            &phone("+39 06 1234 5678", None).unwrap()
        ));
        // The table is sorted (as text), holds digits only, and names each code once.
        let codes: Vec<&str> = TRUNK_PREFIXES.iter().map(|(c, _)| *c).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, codes, "sorted, each code once");
        assert!(codes.iter().all(|c| c.bytes().all(|b| b.is_ascii_digit())));
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
        // Other languages' legal forms (CRM-T169).
        assert_eq!(
            organization("Müller Maschinenbau GmbH & Co. KG"),
            "muller maschinenbau and"
        );
        assert_eq!(organization("Tata Consultancy Pvt Ltd"), "tata consultancy");
        assert_eq!(organization("Petronas Sdn Bhd"), "petronas");
        assert_eq!(organization("ООО Ромашка"), "ромашка");
        assert_eq!(organization("株式会社山田製作所"), "山田製作所");
        assert_eq!(organization("北京王芳科技有限公司"), "北京王芳科技");
        assert_eq!(organization("Kabushiki KK"), "kabushiki");
        assert_eq!(job_title("Sr. Eng Mgr"), "senior engineer manager");
        assert_eq!(job_title("VP, Sales"), "vice president sales");
    }
}
