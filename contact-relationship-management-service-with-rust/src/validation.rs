//! Payload validation → `422` (family convention). Bounded-input
//! rules (security invariant 3) plus the token vocabularies from
//! [`crate::rules::tokens`] and `EntityRef` URN shape checks.

use std::str::FromStr;

use entity_ref::{EntityRef, EntityType};

/// Maximum length of any free-text field (security invariant 3).
pub const MAX_TEXT_LEN: usize = 1024;
/// Maximum entries in any list field (e.g. deduction labels).
pub const MAX_ARRAY_LEN: usize = 256;
/// Maximum length of one list entry.
pub const MAX_ITEM_LEN: usize = 512;

/// One validation problem: what is wrong, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldProblem {
    /// A JSON Pointer (RFC 6901) to the member, such as `/display_name`. A
    /// member of a list is named without its index (`stages[].name` is
    /// `/stages/name`), because the check does not know which entry it is.
    /// `None` when the problem is about the request as a whole.
    pub pointer: Option<String>,
    /// The human-readable message.
    pub detail: String,
}

/// The JSON Pointer for a field as the API documents it: `a.b` is `/a/b`, and
/// a `[]` (any list entry) is dropped.
#[must_use]
pub fn pointer(field: &str) -> String {
    let path: Vec<&str> = field
        .split('.')
        .map(|part| part.trim_end_matches("[]"))
        .collect();
    format!("/{}", path.join("/"))
}

/// Accumulates human-readable validation problems, each with the member it is
/// about.
#[derive(Debug, Default)]
pub struct Problems(Vec<FieldProblem>);

impl Problems {
    /// Start an empty problem list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a problem about the request as a whole.
    pub fn push(&mut self, problem: impl Into<String>) {
        self.0.push(FieldProblem {
            pointer: None,
            detail: problem.into(),
        });
    }

    /// Record a problem about one member (`field`, as the API names it).
    pub fn push_field(&mut self, field: &str, problem: impl Into<String>) {
        self.0.push(FieldProblem {
            pointer: Some(pointer(field)),
            detail: problem.into(),
        });
    }

    /// `field` must be an ISO 4217 currency code the CRM knows (CRM-T168): never assumed, never
    /// any text.
    pub fn currency(&mut self, field: &str, value: &str) {
        if let Err(why) = crate::rules::currency::check(value) {
            self.push_field(field, why);
        }
    }

    /// `field` must be non-blank and within [`MAX_TEXT_LEN`].
    pub fn require_text(&mut self, field: &str, value: &str) {
        if value.trim().is_empty() {
            self.push_field(field, format!("{field} is required"));
        }
        self.cap_text(field, value);
    }

    /// `field`, when present, must be within [`MAX_TEXT_LEN`].
    pub fn cap_text(&mut self, field: &str, value: &str) {
        if value.len() > MAX_TEXT_LEN {
            self.push_field(field, format!("{field} exceeds {MAX_TEXT_LEN} characters"));
        }
    }

    /// Optional variant of [`Self::cap_text`].
    pub fn cap_opt(&mut self, field: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.cap_text(field, v);
        }
    }

    /// `field` must be a member of the closed token `set`.
    pub fn require_token(&mut self, field: &str, set: &[&str], value: &str) {
        if !crate::rules::tokens::is_token(set, value) {
            self.push_field(
                field,
                format!("{field} must be one of {set:?}, got {value:?}"),
            );
        }
    }

    /// Optional variant of [`Self::require_token`].
    pub fn token_opt(&mut self, field: &str, set: &[&str], value: Option<&str>) {
        if let Some(v) = value {
            self.require_token(field, set, v);
        }
    }

    /// `field` must be a well-formed `EntityRef` URN of `expected` type.
    pub fn require_ref(&mut self, field: &str, expected: EntityType, value: &str) {
        match EntityRef::from_str(value) {
            Ok(r) if r.entity_type == expected => {}
            Ok(r) => self.push_field(
                field,
                format!(
                    "{field} must reference a {expected:?}, got {:?}",
                    r.entity_type
                ),
            ),
            Err(e) => self.push_field(field, format!("{field} is not a valid EntityRef URN: {e}")),
        }
    }

    /// Optional variant of [`Self::require_ref`].
    pub fn ref_opt(&mut self, field: &str, expected: EntityType, value: Option<&str>) {
        if let Some(v) = value {
            self.require_ref(field, expected, v);
        }
    }

    /// `field` must name a CRM user (a rep, an agent, a manager): a well-formed `person:` URN, the
    /// sign-in subject (CRM-D18). The caller passes the value through
    /// [`crate::rules::user_ref::normalise`] first, so a legacy `worker:<id>` arrives here as
    /// `person:<id>`.
    pub fn require_user(&mut self, field: &str, value: &str) {
        self.require_ref(field, EntityType::Person, value);
    }

    /// Optional variant of [`Self::require_user`].
    pub fn user_opt(&mut self, field: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.require_user(field, v);
        }
    }

    /// A string list must respect the array and per-item caps, with no
    /// blank entries.
    pub fn cap_list(&mut self, field: &str, values: &[String]) {
        if values.len() > MAX_ARRAY_LEN {
            self.push_field(field, format!("{field} exceeds {MAX_ARRAY_LEN} entries"));
        }
        for v in values {
            if v.trim().is_empty() {
                self.push_field(field, format!("{field} entries must be non-blank"));
            }
            if v.len() > MAX_ITEM_LEN {
                self.push_field(
                    field,
                    format!("{field} entry exceeds {MAX_ITEM_LEN} characters"),
                );
            }
        }
    }

    /// The messages only (empty ⇒ valid).
    #[must_use]
    pub fn into_vec(self) -> Vec<String> {
        self.0.into_iter().map(|p| p.detail).collect()
    }

    /// The problems with the members they are about (empty ⇒ valid).
    #[must_use]
    pub fn into_problems(self) -> Vec<FieldProblem> {
        self.0
    }

    /// `Ok` when nothing was recorded; otherwise the `422` for the request:
    /// `description` joins the messages (what clients have always read), and
    /// `errors` lists each as `{ "pointer", "detail" }` (RFC 9457 §3.2), the
    /// `pointer` left out for a problem about the request as a whole.
    ///
    /// # Errors
    ///
    /// The `422` when any problem was recorded.
    pub fn finish(self) -> loco_rs::Result<()> {
        if self.0.is_empty() {
            return Ok(());
        }
        Err(crate::controllers::validation_error(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_rules() {
        let mut p = Problems::new();
        p.require_text("name", "Ward 7");
        assert!(p.into_vec().is_empty());
        let mut p = Problems::new();
        p.require_text("name", "   ");
        p.cap_text("note", &"x".repeat(MAX_TEXT_LEN + 1));
        let v = p.into_vec();
        assert_eq!(v.len(), 2);
        assert!(v[0].contains("required"));
        assert!(v[1].contains("exceeds"));
    }

    #[test]
    fn token_rules() {
        let mut p = Problems::new();
        p.require_token("status", crate::rules::tokens::LEAD_STATUSES, "new");
        p.token_opt("kind", crate::rules::tokens::ACTIVITY_KINDS, None);
        assert!(p.into_vec().is_empty());
        let mut p = Problems::new();
        p.require_token("status", crate::rules::tokens::LEAD_STATUSES, "won");
        assert_eq!(p.into_vec().len(), 1);
    }

    #[test]
    fn ref_rules() {
        let good = format!("person:{}", uuid::Uuid::new_v4());
        let mut p = Problems::new();
        p.require_ref("person_ref", EntityType::Person, &good);
        assert!(p.into_vec().is_empty());
        // Wrong type, malformed, and bad uuid each fail.
        let mut p = Problems::new();
        p.require_ref(
            "person_ref",
            EntityType::Person,
            &format!("worker:{}", uuid::Uuid::new_v4()),
        );
        p.require_ref("person_ref", EntityType::Person, "not-a-urn");
        p.require_ref("person_ref", EntityType::Person, "person:nope");
        assert_eq!(p.into_vec().len(), 3);
    }

    /// CRM-D18: a CRM user is a `person:` URN; a legacy `worker:` one is refused here unless the
    /// caller normalised it first, which is what every controller does.
    #[test]
    fn a_user_is_a_person_reference_and_worker_is_accepted_only_once_normalised() {
        let id = uuid::Uuid::new_v4();
        let mut p = Problems::new();
        p.require_user("owner_ref", &format!("person:{id}"));
        p.user_opt("owner_ref", None);
        p.require_user(
            "owner_ref",
            &crate::rules::user_ref::normalise(&format!("worker:{id}")),
        );
        assert!(p.into_vec().is_empty());
        let mut p = Problems::new();
        p.require_user("owner_ref", &format!("worker:{id}"));
        p.require_user("owner_ref", &format!("organization:{id}"));
        p.user_opt("owner_ref", Some("person:nope"));
        let v = p.into_problems();
        assert_eq!(v.len(), 3);
        assert!(v.iter().all(|f| f.pointer.as_deref() == Some("/owner_ref")));
        assert!(v[0].detail.contains("must reference a Person"), "{v:?}");
    }

    /// CRM-T25: `require_ref` exercises the wrong-type branch for
    /// `Worker` and `Organization` too, not just `Person`. (The
    /// controllers no longer pass `Worker`: a CRM user is a `person:`
    /// URN since CRM-D18, checked by `require_user`.)
    #[test]
    fn ref_rules_wrong_type_worker_and_organization() {
        // Worker: a well-formed ref of the *wrong* type is rejected...
        let mut p = Problems::new();
        p.require_ref(
            "rep_ref",
            EntityType::Worker,
            &format!("person:{}", uuid::Uuid::new_v4()),
        );
        let v = p.into_vec();
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("must reference a Worker"));
        // ...but the matching type is accepted.
        let mut p = Problems::new();
        p.require_ref(
            "rep_ref",
            EntityType::Worker,
            &format!("worker:{}", uuid::Uuid::new_v4()),
        );
        assert!(p.into_vec().is_empty());

        // Organization: same pattern.
        let mut p = Problems::new();
        p.require_ref(
            "account_ref",
            EntityType::Organization,
            &format!("person:{}", uuid::Uuid::new_v4()),
        );
        let v = p.into_vec();
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("must reference a Organization"));
        let mut p = Problems::new();
        p.require_ref(
            "account_ref",
            EntityType::Organization,
            &format!("organization:{}", uuid::Uuid::new_v4()),
        );
        assert!(p.into_vec().is_empty());
    }

    #[test]
    fn list_rules() {
        let mut p = Problems::new();
        p.cap_list("alerts", &["falls risk".to_string()]);
        assert!(p.into_vec().is_empty());
        let mut p = Problems::new();
        p.cap_list("alerts", &[String::new()]);
        assert_eq!(p.into_vec().len(), 1);
    }
}
