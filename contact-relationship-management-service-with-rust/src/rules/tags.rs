//! Tags (CRM-R47): what a tag may be called, and how a record's set of tags is checked.
//!
//! Pure and DB-free. A tag is a short free label; two names that differ only in case or in the
//! spaces between words are the same tag.

use std::collections::BTreeSet;

/// The records that can carry tags.
pub const SUBJECTS: &[&str] = &["contact", "account", "lead", "deal"];
/// Longest tag name, in characters.
pub const MAX_NAME: usize = 40;
/// Most tags on one record.
pub const MAX_PER_RECORD: usize = 20;
/// Most tags in the system.
pub const MAX_TAGS: i64 = 500;

/// A tag name made tidy: leading and trailing space gone, runs of space made one.
///
/// # Errors
///
/// The reason, when the name is empty, too long or has a control character.
pub fn normalise(raw: &str) -> Result<String, String> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("a tag needs a name".to_string());
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!("a tag name is at most {MAX_NAME} characters"));
    }
    if name.chars().any(char::is_control) {
        return Err("a tag name cannot hold a control character".to_string());
    }
    Ok(name)
}

/// How two names are compared: the same tag whatever the case.
#[must_use]
pub fn key(name: &str) -> String {
    name.to_lowercase()
}

/// The set of names for one record, tidied and with repeats (in any case) taken out, keeping the
/// first spelling.
///
/// # Errors
///
/// The first name that cannot be a tag, or the reason there are too many.
pub fn set_of(names: &[String]) -> Result<Vec<String>, String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for raw in names {
        let name = normalise(raw)?;
        if seen.insert(key(&name)) {
            out.push(name);
        }
    }
    if out.len() > MAX_PER_RECORD {
        return Err(format!("a record has at most {MAX_PER_RECORD} tags"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_made_tidy_or_refused() {
        assert_eq!(normalise("  big   spender ").as_deref(), Ok("big spender"));
        assert!(normalise("   ").is_err());
        assert!(normalise(&"x".repeat(MAX_NAME + 1)).is_err());
        assert!(normalise(&"x".repeat(MAX_NAME)).is_ok());
        assert!(normalise("bad\u{7}tag").is_err());
    }

    #[test]
    fn names_that_differ_in_case_are_one_tag() {
        assert_eq!(key("VIP"), key("vip"));
        let set = set_of(&["VIP".into(), " vip ".into(), "Press".into()]).unwrap();
        assert_eq!(set, ["VIP", "Press"]);
    }

    #[test]
    fn a_record_has_a_limited_set() {
        let many: Vec<String> = (0..=MAX_PER_RECORD).map(|i| format!("tag {i}")).collect();
        assert!(set_of(&many).is_err());
        assert!(set_of(&many[..MAX_PER_RECORD]).is_ok());
        assert_eq!(set_of(&[]).unwrap(), Vec::<String>::new());
        assert!(set_of(&["ok".into(), String::new()]).is_err());
    }
}
