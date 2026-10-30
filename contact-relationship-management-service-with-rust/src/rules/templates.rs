//! Email templates and their merge tokens (CRM-R36): parsing, checking and rendering, DB-free.
//!
//! A template's text holds `{{token}}` marks from a closed set. Rendering turns a template and a
//! contact's values into the plain text that is mailed and an HTML form in which every value is
//! escaped; a missing value becomes the token's fallback.
//!
//! **Any language (CRM-T169, CRM-R95).** A template may name the language it is written in (a BCP
//! 47 tag). A missing value takes the template's own fallback, else the built-in fallback in the
//! template's language ([`FALLBACKS`]: the UI's eight languages, machine drafts but English). A
//! template in a language with no built-in fallbacks must give its own for every token it uses, so
//! no English word is put into, say, a Japanese message. A template that names no language keeps
//! the English fallbacks it was written against. `given_name` is the given name on file only: it
//! is never taken from the display name, whose first word is the family name in many orders.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The token that carries the signed unsubscribe link. Every body must contain it.
pub const UNSUBSCRIBE: &str = "unsubscribe_url";

/// One merge token.
#[derive(Debug, Clone, Copy)]
pub struct Token {
    /// How it is written between the braces.
    pub name: &'static str,
    /// What it stands for.
    pub description: &'static str,
}

/// The closed set of tokens that take a value from the contact, in the order of [`FALLBACKS`].
pub const TOKENS: &[Token] = &[
    Token {
        name: "given_name",
        description: "The contact's given name, when one is recorded (never taken from the display name)",
    },
    Token {
        name: "name",
        description: "The contact's full name",
    },
    Token {
        name: "family_name",
        description: "The contact's family name, when one is recorded",
    },
    Token {
        name: "account_name",
        description: "The name of the contact's account",
    },
    Token {
        name: "rep_name",
        description: "The name of the contact's owner, or the account's",
    },
    Token {
        name: "job_title",
        description: "The contact's job title",
    },
];

/// The language of a template that names none: the one its built-in fallbacks were written in.
pub const UNNAMED_LANGUAGE: &str = "en";

/// The built-in fallback of each token in [`TOKENS`] order, by primary language subtag: the UI's
/// eight languages. All but English are machine drafts by the AI assistant, not reviewed by a
/// native speaker; a template sets its own to replace any of them.
pub const FALLBACKS: &[(&str, [&str; 6])] = &[
    (
        "ar",
        [
            "عميلنا العزيز",
            "عميلنا العزيز",
            "العميل",
            "مؤسستكم",
            "الفريق",
            "دوركم",
        ],
    ),
    (
        "cy",
        [
            "gyfaill",
            "gyfaill",
            "cwsmer",
            "eich sefydliad",
            "y tîm",
            "eich rôl",
        ],
    ),
    (
        "de",
        [
            "zusammen",
            "zusammen",
            "Kundschaft",
            "Ihre Organisation",
            "das Team",
            "Ihre Funktion",
        ],
    ),
    (
        "en",
        [
            "there",
            "there",
            "customer",
            "your organisation",
            "the team",
            "your role",
        ],
    ),
    (
        "es",
        [
            "a todos",
            "a todos",
            "cliente",
            "su organización",
            "el equipo",
            "su puesto",
        ],
    ),
    (
        "fr",
        [
            "à tous",
            "à tous",
            "client",
            "votre organisation",
            "l’équipe",
            "votre fonction",
        ],
    ),
    (
        "hi",
        ["ग्राहक", "ग्राहक", "ग्राहक", "आपका संगठन", "टीम", "आपकी भूमिका"],
    ),
    ("zh", ["您好", "您好", "客户", "贵机构", "团队", "您的职位"]),
];

/// Longest language tag.
pub const MAX_LANGUAGE: usize = 35;

/// A template's language tag, trimmed: a BCP 47 shape (a primary subtag of 2 or 3 letters, then
/// subtags of 1 to 8 letters or digits), such as `ja`, `pt-BR` or `zh-Hant`.
///
/// # Errors
///
/// The reason, when it is not that shape.
pub fn language(tag: &str) -> Result<String, String> {
    let tag = tag.trim();
    let mut parts = tag.split('-');
    let primary_ok = parts
        .next()
        .is_some_and(|p| (2..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphabetic()));
    let rest_ok =
        parts.all(|p| (1..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric()));
    if primary_ok && rest_ok && tag.len() <= MAX_LANGUAGE {
        Ok(tag.to_string())
    } else {
        Err("is a BCP 47 language tag such as ja, pt-BR or zh-Hant".to_string())
    }
}

/// The built-in fallbacks for `language` (its primary subtag), or for a template with no
/// language [`UNNAMED_LANGUAGE`]'s; `None` for a language that has none.
#[must_use]
pub fn fallbacks_in(language: Option<&str>) -> Option<&'static [&'static str; 6]> {
    let primary = language
        .unwrap_or(UNNAMED_LANGUAGE)
        .split('-')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    FALLBACKS
        .iter()
        .find(|(code, _)| *code == primary)
        .map(|(_, texts)| texts)
}

/// The built-in fallback of `token` in `language`, if there is one.
#[must_use]
pub fn built_in_fallback(token: &str, language: Option<&str>) -> Option<&'static str> {
    let index = TOKENS.iter().position(|t| t.name == token)?;
    fallbacks_in(language).map(|texts| texts[index])
}

/// Longest subject a template may have.
pub const MAX_SUBJECT: usize = 300;
/// Longest body a template may have.
pub const MAX_BODY: usize = 20_000;
/// Longest template name.
pub const MAX_NAME: usize = 100;
/// Longest fallback text.
pub const MAX_FALLBACK: usize = 100;

/// A piece of a parsed template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Text kept as written.
    Text(String),
    /// A token to fill.
    Token(String),
}

/// Why a template's text cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A `{{` with no `}}` after it.
    Unclosed,
    /// A token that is not in the set.
    Unknown(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unclosed => write!(f, "has a {{{{ with no closing }}}}"),
            Self::Unknown(name) => {
                write!(f, "uses {{{{{name}}}}}, which is not a merge token")
            }
        }
    }
}

fn known(name: &str) -> bool {
    name == UNSUBSCRIBE || TOKENS.iter().any(|t| t.name == name)
}

/// Split `source` into text and tokens.
///
/// # Errors
///
/// [`ParseError::Unclosed`] for a `{{` with no `}}`; [`ParseError::Unknown`] for a name outside
/// the set.
pub fn parse(source: &str) -> Result<Vec<Segment>, ParseError> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        if start > 0 {
            out.push(Segment::Text(rest[..start].to_string()));
        }
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or(ParseError::Unclosed)?;
        let name = after[..end].trim();
        if !known(name) {
            return Err(ParseError::Unknown(name.to_string()));
        }
        out.push(Segment::Token(name.to_string()));
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        out.push(Segment::Text(rest.to_string()));
    }
    Ok(out)
}

fn has_unsubscribe(segments: &[Segment]) -> bool {
    segments
        .iter()
        .any(|s| matches!(s, Segment::Token(t) if t == UNSUBSCRIBE))
}

/// Everything wrong with a template, as `(field, message)` pairs; empty when it can be saved.
#[must_use]
pub fn check(
    subject: &str,
    body: &str,
    fallbacks: &BTreeMap<String, String>,
    language_tag: Option<&str>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut add = |field: &str, message: String| out.push((field.to_string(), message));
    let mut language_ok = true;
    if let Some(tag) = language_tag
        && let Err(why) = language(tag)
    {
        add("language", why);
        language_ok = false;
    }
    if subject.trim().is_empty() {
        add("subject", "is required".to_string());
    } else if subject.contains(['\r', '\n']) {
        add("subject", "must be a single line".to_string());
    } else if subject.chars().count() > MAX_SUBJECT {
        add(
            "subject",
            format!("is longer than {MAX_SUBJECT} characters"),
        );
    }
    match parse(subject) {
        Ok(segments) if has_unsubscribe(&segments) => add(
            "subject",
            format!("cannot contain {{{{{UNSUBSCRIBE}}}}}; it belongs in the body"),
        ),
        Ok(_) => {}
        Err(error) => add("subject", error.to_string()),
    }
    if body.trim().is_empty() {
        add("body", "is required".to_string());
    } else if body.chars().count() > MAX_BODY {
        add("body", format!("is longer than {MAX_BODY} characters"));
    }
    match parse(body) {
        Ok(segments) if !has_unsubscribe(&segments) => add(
            "body",
            format!("must contain {{{{{UNSUBSCRIBE}}}}}, the link a recipient uses to unsubscribe"),
        ),
        Ok(_) => {}
        Err(error) => add("body", error.to_string()),
    }
    // A language with no built-in fallbacks: each token used needs the template's own.
    if language_ok && fallbacks_in(language_tag).is_none() {
        let used: Vec<String> = [subject, body]
            .iter()
            .filter_map(|text| parse(text).ok())
            .flatten()
            .filter_map(|segment| match segment {
                Segment::Token(name) if name != UNSUBSCRIBE => Some(name),
                _ => None,
            })
            .collect();
        let mut asked = Vec::new();
        for token in used {
            if !fallbacks.contains_key(&token) && !asked.contains(&token) {
                add(
                    &format!("fallbacks/{token}"),
                    format!(
                        "is required: there is no built-in fallback in {}",
                        language_tag.unwrap_or_default().trim()
                    ),
                );
                asked.push(token);
            }
        }
    }
    for (token, text) in fallbacks {
        let field = format!("fallbacks/{token}");
        if !TOKENS.iter().any(|t| t.name == token) {
            add(&field, "is not a token that takes a fallback".to_string());
        } else if text.trim().is_empty() {
            add(&field, "is required".to_string());
        } else if text.contains(['\r', '\n']) || text.chars().count() > MAX_FALLBACK {
            add(
                &field,
                format!("must be one line of at most {MAX_FALLBACK} characters"),
            );
        }
    }
    out
}

/// A value made safe to put in a message: control characters become a space and the ends are
/// trimmed. `None` when nothing is left, which means the value is missing.
#[must_use]
pub fn clean(value: &str) -> Option<String> {
    let flat: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let flat = flat.trim();
    (!flat.is_empty()).then(|| flat.to_string())
}

/// `text` with the characters that mean something in HTML replaced by entities.
#[must_use]
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// A template filled in for one recipient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The subject, one line.
    pub subject: String,
    /// The plain text that is mailed.
    pub text: String,
    /// The body as HTML, every value escaped.
    pub html: String,
    /// The tokens that had no value and fell back, in the order they first appeared.
    pub fell_back: Vec<String>,
}

/// What to put in a message for one recipient.
#[derive(Debug, Clone, Copy)]
pub struct Fill<'a> {
    /// The contact's values by token name; a token not here, or empty, is missing.
    pub values: &'a BTreeMap<String, String>,
    /// The template's own fallbacks by token name.
    pub fallbacks: &'a BTreeMap<String, String>,
    /// The template's language, for the built-in fallbacks; `None` is [`UNNAMED_LANGUAGE`].
    pub language: Option<&'a str>,
    /// The signed unsubscribe link.
    pub unsubscribe_url: &'a str,
}

impl Fill<'_> {
    /// The text for `token` and whether it is the fallback.
    fn text_for(&self, token: &str) -> (String, bool) {
        if let Some(value) = self.values.get(token).and_then(|v| clean(v)) {
            return (value, false);
        }
        let own = self.fallbacks.get(token).and_then(|v| clean(v));
        let declared = built_in_fallback(token, self.language).unwrap_or_default();
        (own.unwrap_or_else(|| declared.to_string()), true)
    }
}

/// Fill a parsed template.
fn fill(segments: &[Segment], with: &Fill<'_>, fell_back: &mut Vec<String>) -> (String, String) {
    let mut text = String::new();
    let mut html = String::new();
    for segment in segments {
        match segment {
            Segment::Text(literal) => {
                text.push_str(literal);
                html.push_str(&escape_html(literal));
            }
            Segment::Token(name) if name == UNSUBSCRIBE => {
                text.push_str(with.unsubscribe_url);
                let url = escape_html(with.unsubscribe_url);
                let _ = write!(html, "<a href=\"{url}\">{url}</a>");
            }
            Segment::Token(name) => {
                let (value, missing) = with.text_for(name);
                if missing && !fell_back.contains(name) {
                    fell_back.push(name.clone());
                }
                html.push_str(&escape_html(&value));
                text.push_str(&value);
            }
        }
    }
    (text, html)
}

/// Render a template for one recipient. A text that cannot be parsed renders as itself, which
/// cannot happen for a saved template (it was checked when saved).
#[must_use]
pub fn render(subject: &str, body: &str, with: &Fill<'_>) -> Rendered {
    let mut fell_back = Vec::new();
    let (subject_text, _) = fill(
        &parse(subject).unwrap_or_else(|_| vec![Segment::Text(subject.to_string())]),
        with,
        &mut fell_back,
    );
    let (text, html) = fill(
        &parse(body).unwrap_or_else(|_| vec![Segment::Text(body.to_string())]),
        with,
        &mut fell_back,
    );
    Rendered {
        subject: subject_text,
        text,
        html: html.replace('\n', "<br>\n"),
        fell_back,
    }
}

/// The contact the preview is rendered for. Never a real person.
#[must_use]
pub fn synthetic_values() -> BTreeMap<String, String> {
    [
        ("given_name", "Alex"),
        ("name", "Alex Example"),
        ("family_name", "Example"),
        ("account_name", "Example Ltd"),
        ("rep_name", "Sam Rep"),
        ("job_title", "Operations Lead"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    fn fill_with<'a>(
        values: &'a BTreeMap<String, String>,
        fallbacks: &'a BTreeMap<String, String>,
    ) -> Fill<'a> {
        Fill {
            values,
            fallbacks,
            language: None,
            unsubscribe_url: "https://crm.example.org/unsubscribe/abc",
        }
    }

    #[test]
    fn tokens_are_found_with_or_without_spaces() {
        let parsed = parse("Hi {{given_name}}, {{ account_name }}!").unwrap();
        assert_eq!(
            parsed,
            vec![
                Segment::Text("Hi ".into()),
                Segment::Token("given_name".into()),
                Segment::Text(", ".into()),
                Segment::Token("account_name".into()),
                Segment::Text("!".into()),
            ]
        );
    }

    #[test]
    fn an_unknown_or_unclosed_token_is_refused() {
        assert_eq!(
            parse("Hi {{nickname}}"),
            Err(ParseError::Unknown("nickname".into()))
        );
        assert_eq!(parse("Hi {{given_name"), Err(ParseError::Unclosed));
        assert!(parse("a single { brace } is text").is_ok());
    }

    #[test]
    fn a_value_is_html_escaped_in_the_html_and_left_alone_in_the_text() {
        let v = values(&[("given_name", "<script>alert(1)</script> & \"Co\"")]);
        let out = render(
            "Hi {{given_name}}",
            "Dear {{given_name}}\n{{unsubscribe_url}}",
            &fill_with(&v, &none()),
        );
        assert!(out.text.contains("<script>alert(1)</script> & \"Co\""));
        assert!(!out.html.contains("<script>"));
        assert!(
            out.html
                .contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;Co&quot;")
        );
        assert!(
            out.html
                .contains("<br>\n<a href=\"https://crm.example.org/unsubscribe/abc\">")
        );
    }

    #[test]
    fn the_template_s_own_text_is_escaped_too() {
        let out = render(
            "s",
            "1 < 2 & {{unsubscribe_url}}",
            &fill_with(&none(), &none()),
        );
        assert!(out.html.starts_with("1 &lt; 2 &amp; "));
    }

    #[test]
    fn a_missing_value_falls_back_to_the_declared_text() {
        let out = render(
            "Hello {{given_name}}",
            "From {{rep_name}} at {{account_name}} {{unsubscribe_url}}",
            &fill_with(
                &values(&[("account_name", "   ")]),
                &values(&[("rep_name", "Pat")]),
            ),
        );
        assert_eq!(out.subject, "Hello there");
        assert!(out.text.starts_with("From Pat at your organisation "));
        assert_eq!(
            out.fell_back,
            vec!["given_name", "rep_name", "account_name"]
        );
    }

    #[test]
    fn a_value_cannot_add_a_line_to_the_subject() {
        let v = values(&[("given_name", "Eve\r\nBcc: x@example.org")]);
        let out = render(
            "Hi {{given_name}}",
            "{{unsubscribe_url}}",
            &fill_with(&v, &none()),
        );
        assert!(!out.subject.contains(['\r', '\n']));
        assert_eq!(out.subject, "Hi Eve  Bcc: x@example.org");
    }

    #[test]
    fn a_body_without_the_unsubscribe_link_is_refused() {
        let problems = check("Hello", "Buy now", &none(), None);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].0, "body");
        assert!(problems[0].1.contains("unsubscribe_url"));
        assert!(check("Hello", "Bye {{ unsubscribe_url }}", &none(), None).is_empty());
    }

    #[test]
    fn the_subject_cannot_carry_the_link_and_fallbacks_are_checked() {
        let bad = values(&[
            ("unsubscribe_url", "x"),
            ("given_name", " "),
            ("name", "a\nb"),
        ]);
        let problems = check("Out {{unsubscribe_url}}", "{{unsubscribe_url}}", &bad, None);
        let fields: Vec<&str> = problems.iter().map(|(f, _)| f.as_str()).collect();
        assert!(fields.contains(&"subject"));
        assert!(fields.contains(&"fallbacks/unsubscribe_url"));
        assert!(fields.contains(&"fallbacks/given_name"));
        assert!(fields.contains(&"fallbacks/name"));
    }

    #[test]
    fn fallbacks_are_in_the_template_s_language() {
        let empty = none();
        let mut with = fill_with(&empty, &empty);
        with.language = Some("de-AT");
        let out = render(
            "Hallo {{given_name}}",
            "{{account_name}} {{unsubscribe_url}}",
            &with,
        );
        assert_eq!(out.subject, "Hallo zusammen");
        assert!(out.text.starts_with("Ihre Organisation "));
        with.language = Some("zh-Hans");
        assert_eq!(
            render("{{given_name}}", "{{unsubscribe_url}}", &with).subject,
            "您好"
        );
        // No language: the English the old templates were written against.
        with.language = None;
        assert_eq!(
            render("Hi {{given_name}}", "{{unsubscribe_url}}", &with).subject,
            "Hi there"
        );
    }

    #[test]
    fn every_language_has_a_fallback_for_every_token_and_none_is_english_but_english() {
        let english = fallbacks_in(Some("en")).unwrap();
        assert_eq!(TOKENS.len(), english.len());
        for (code, texts) in FALLBACKS {
            assert!(language(code).is_ok(), "{code}");
            assert!(
                texts
                    .iter()
                    .all(|t| !t.trim().is_empty() && t.chars().count() <= MAX_FALLBACK)
            );
            if *code != "en" {
                assert!(
                    texts.iter().all(|t| !english.contains(t)),
                    "{code} has English"
                );
            }
        }
        assert!(
            FALLBACKS.windows(2).all(|w| w[0].0 < w[1].0),
            "sorted, once each"
        );
    }

    #[test]
    fn a_language_with_no_built_in_fallbacks_needs_the_template_s_own() {
        let problems = check(
            "{{given_name}} 様",
            "{{name}} {{given_name}} {{unsubscribe_url}}",
            &none(),
            Some("ja"),
        );
        let fields: Vec<&str> = problems.iter().map(|(f, _)| f.as_str()).collect();
        assert_eq!(fields, ["fallbacks/given_name", "fallbacks/name"]);
        assert!(problems[0].1.contains("ja"));
        let own = values(&[("given_name", "お客"), ("name", "お客様")]);
        assert!(
            check(
                "{{given_name}} 様",
                "{{name}} {{unsubscribe_url}}",
                &own,
                Some("ja")
            )
            .is_empty()
        );
        let empty = none();
        let mut with = fill_with(&empty, &own);
        with.language = Some("ja");
        assert_eq!(
            render("{{given_name}}様", "{{unsubscribe_url}}", &with).subject,
            "お客様"
        );
        // A built-in language, or none, needs nothing more.
        assert!(check("{{given_name}}", "{{unsubscribe_url}}", &none(), Some("hi")).is_empty());
        assert!(check("{{given_name}}", "{{unsubscribe_url}}", &none(), None).is_empty());
    }

    #[test]
    fn a_language_tag_has_the_bcp_47_shape() {
        for ok in ["ja", "pt-BR", "zh-Hant", "yue", "es-419", " cy "] {
            assert!(language(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "j",
            "english",
            "pt_BR",
            "zh-",
            "de-AT-toolongsubtag",
            "日本",
        ] {
            assert!(language(bad).is_err(), "{bad}");
        }
        assert_eq!(
            check("s", "{{unsubscribe_url}}", &none(), Some("x"))[0].0,
            "language"
        );
    }
}
