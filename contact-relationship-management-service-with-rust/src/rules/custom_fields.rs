//! Typed custom fields (CRM-R41): what a field may be, and what a value may be for it.
//!
//! Pure and DB-free. Every set is closed, and a value is checked against its field's kind when it
//! is stored, so a number field never holds "soon".

use serde_json::Value;

/// The records that can carry custom fields.
pub const ENTITIES: &[&str] = &["contact", "account", "lead", "deal"];
/// What a field holds.
pub const KINDS: &[&str] = &["text", "number", "date", "choice"];
/// How a field is treated, which decides what masking, storage, subject access and erasure do:
///
/// - `business`: operational data. Shown to masked callers, kept after an erasure.
/// - `personal`: personal data. Masked for masked callers, included in a subject-access export,
///   deleted when the contact is erased.
/// - `sensitive`: as `personal`, and sealed with the field key in the database.
pub const SENSITIVITIES: &[&str] = &["business", "personal", "sensitive"];
/// Most fields on one kind of record.
pub const MAX_PER_ENTITY: i64 = 30;
/// Longest label, key and choice.
pub const MAX_LABEL: usize = 80;
/// Longest text value.
pub const MAX_TEXT: usize = 500;
/// Most choices in a choice field.
pub const MAX_CHOICES: usize = 20;

/// Whether `key` is a lower-case name: a letter, then letters, digits and underscores, at most 40.
#[must_use]
pub fn is_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && key.len() <= 40
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// What would stop this field being saved, as `(member, message)` pairs.
#[must_use]
pub fn check_def(
    entity: &str,
    key: &str,
    label: &str,
    kind: &str,
    choices: &[String],
    sensitivity: &str,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut add = |member: &str, message: String| out.push((member.to_string(), message));
    if !ENTITIES.contains(&entity) {
        add("entity", format!("must be one of {}", ENTITIES.join(", ")));
    }
    if !is_key(key) {
        add(
            "key",
            "must be lower-case letters, digits and underscores, starting with a letter (40 at most)"
                .to_string(),
        );
    }
    if label.trim().is_empty() || label.chars().count() > MAX_LABEL {
        add("label", format!("must be 1 to {MAX_LABEL} characters"));
    }
    if !KINDS.contains(&kind) {
        add("kind", format!("must be one of {}", KINDS.join(", ")));
    }
    if !SENSITIVITIES.contains(&sensitivity) {
        add(
            "sensitivity",
            format!("must be one of {}", SENSITIVITIES.join(", ")),
        );
    }
    out.extend(check_choices(kind, choices));
    out
}

/// What is wrong with the choices for a field of this kind.
#[must_use]
pub fn check_choices(kind: &str, choices: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if kind == "choice" {
        let mut seen = std::collections::BTreeSet::new();
        if choices.is_empty() || choices.len() > MAX_CHOICES {
            out.push((
                "choices".to_string(),
                format!("needs 1 to {MAX_CHOICES} choices"),
            ));
        }
        for choice in choices {
            if choice.trim().is_empty() || choice.chars().count() > MAX_LABEL {
                out.push((
                    "choices".to_string(),
                    format!("each choice must be 1 to {MAX_LABEL} characters"),
                ));
                break;
            }
            if !seen.insert(choice.to_lowercase()) {
                out.push(("choices".to_string(), format!("{choice} appears twice")));
                break;
            }
        }
    } else if !choices.is_empty() {
        out.push((
            "choices".to_string(),
            "only a choice field has choices".to_string(),
        ));
    }
    out
}

/// A value checked against its field: the text to store, or why it is refused.
///
/// # Errors
///
/// The reason, in words, when the value does not fit the kind.
pub fn check_value(kind: &str, choices: &[String], value: &Value) -> Result<String, String> {
    match (kind, value) {
        ("text", Value::String(s)) => {
            if s.chars().count() > MAX_TEXT {
                Err(format!("is longer than {MAX_TEXT} characters"))
            } else if s.trim().is_empty() {
                Err("is empty (send null to clear it)".to_string())
            } else {
                Ok(s.clone())
            }
        }
        ("number", Value::Number(n)) => n
            .as_f64()
            .filter(|f| f.is_finite())
            .map(|_| n.to_string())
            .ok_or_else(|| "is not a finite number".to_string()),
        ("date", Value::String(s)) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(|d| d.to_string())
            .map_err(|_| "must be a date written YYYY-MM-DD".to_string()),
        ("choice", Value::String(s)) => choices
            .iter()
            .find(|c| c.as_str() == s)
            .cloned()
            .ok_or_else(|| format!("must be one of {}", choices.join(", "))),
        ("text" | "date" | "choice", _) => Err("must be a string".to_string()),
        ("number", _) => Err("must be a number".to_string()),
        _ => Err("this field has an unknown kind".to_string()),
    }
}

/// A stored value as JSON again: a number field gives a number, the rest a string.
#[must_use]
pub fn to_json(kind: &str, stored: &str) -> Value {
    if kind == "number"
        && let Ok(n) = stored.parse::<serde_json::Number>()
    {
        return Value::Number(n);
    }
    Value::String(stored.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn problems(kind: &str, choices: &[&str]) -> Vec<(String, String)> {
        let choices: Vec<String> = choices.iter().map(ToString::to_string).collect();
        check_def(
            "contact",
            "tier_note",
            "Tier note",
            kind,
            &choices,
            "business",
        )
    }

    #[test]
    fn a_good_field_has_no_problems() {
        assert!(problems("text", &[]).is_empty());
        assert!(problems("choice", &["gold", "silver"]).is_empty());
    }

    #[test]
    fn keys_are_lower_case_names() {
        assert!(is_key("renewal_date"));
        assert!(!is_key("Renewal"));
        assert!(!is_key("1st"));
        assert!(!is_key("has space"));
        assert!(!is_key(""));
        assert!(!is_key(&"a".repeat(41)));
    }

    #[test]
    fn a_field_is_checked_member_by_member() {
        let found = check_def("lorry", "Bad Key", " ", "blob", &[], "secret");
        let members: Vec<&str> = found.iter().map(|(m, _)| m.as_str()).collect();
        assert_eq!(members, ["entity", "key", "label", "kind", "sensitivity"]);
    }

    #[test]
    fn choices_belong_to_choice_fields_only() {
        assert!(!problems("choice", &[]).is_empty());
        assert!(!problems("choice", &["a", "A"]).is_empty());
        assert!(!problems("text", &["a"]).is_empty());
        let many: Vec<String> = (0..=MAX_CHOICES).map(|i| i.to_string()).collect();
        assert!(!check_choices("choice", &many).is_empty());
    }

    #[test]
    fn values_must_fit_the_kind() {
        let choices = vec!["gold".to_string(), "silver".to_string()];
        assert_eq!(
            check_value("text", &[], &json!("hello")).as_deref(),
            Ok("hello")
        );
        assert!(check_value("text", &[], &json!("  ")).is_err());
        assert!(check_value("text", &[], &json!(5)).is_err());
        assert!(check_value("text", &[], &json!("x".repeat(MAX_TEXT + 1))).is_err());
        assert_eq!(
            check_value("number", &[], &json!(12.5)).as_deref(),
            Ok("12.5")
        );
        assert!(check_value("number", &[], &json!("12")).is_err());
        assert_eq!(
            check_value("date", &[], &json!("2026-10-25")).as_deref(),
            Ok("2026-10-25")
        );
        assert!(check_value("date", &[], &json!("25/10/2026")).is_err());
        assert!(check_value("date", &[], &json!("2026-02-30")).is_err());
        assert_eq!(
            check_value("choice", &choices, &json!("gold")).as_deref(),
            Ok("gold")
        );
        assert!(check_value("choice", &choices, &json!("bronze")).is_err());
    }

    #[test]
    fn a_stored_value_comes_back_in_its_kind() {
        assert_eq!(to_json("number", "12.5"), json!(12.5));
        assert_eq!(to_json("number", "7"), json!(7));
        assert_eq!(to_json("text", "7"), json!("7"));
        assert_eq!(to_json("date", "2026-10-25"), json!("2026-10-25"));
    }
}
