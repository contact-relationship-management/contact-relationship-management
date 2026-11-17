//! How the CRM names one of its own users (a rep, an agent, a manager): by `person:` URN, the
//! family's sign-in subject (CRM-D18). DB-free.
//!
//! A PASETO's `sub` is the person id: WPM matches it as `person:<sub>` (its `/api/me`, its
//! organization memberships, `resource.person = $sub`) and PPM addresses it as `person:<sub>`. The
//! CRM used to write the same id as `worker:<sub>`, so deal owners, quotas, assignees and the rest
//! carried a person id under the worker prefix and never met the `person:` members the WPM teams
//! import stores (CRM-R73). Every field that names a CRM user now holds `person:<id>`.
//!
//! `worker:<id>` is still **accepted as input** for a deprecation period and rewritten to
//! `person:<id>` (same id), because in the CRM's API it always meant the sign-in subject. Answers
//! only ever say `person:`.

/// The prefix a CRM user's reference carries.
pub const PERSON: &str = "person:";
/// The prefix the CRM used before CRM-D18, accepted as input and rewritten.
pub const LEGACY: &str = "worker:";

/// The reference of the user whose sign-in subject is `sub`.
#[must_use]
pub fn of_subject(sub: &str) -> String {
    format!("{PERSON}{}", sub.trim())
}

/// `value` as the CRM stores a user reference: `worker:<id>` becomes `person:<id>`; anything else
/// is returned unchanged (and left to the URN check to accept or refuse). Surrounding blanks are
/// trimmed only when the prefix is rewritten, so a refusal still quotes what was sent.
#[must_use]
pub fn normalise(value: &str) -> String {
    match value.trim().strip_prefix(LEGACY) {
        Some(id) => format!("{PERSON}{id}"),
        None => value.to_string(),
    }
}

/// [`normalise`] for an optional field.
#[must_use]
pub fn normalise_opt(value: Option<String>) -> Option<String> {
    value.map(|v| normalise(&v))
}

/// [`normalise`] every entry of a list in place.
pub fn normalise_all(values: &mut [String]) {
    for v in values.iter_mut() {
        *v = normalise(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "3f2c6b7e-0000-4000-8000-000000000001";

    #[test]
    fn a_sign_in_subject_is_a_person_reference() {
        assert_eq!(of_subject(ID), format!("person:{ID}"));
        assert_eq!(of_subject(&format!(" {ID} ")), format!("person:{ID}"));
    }

    #[test]
    fn the_legacy_worker_prefix_becomes_person_with_the_same_id() {
        assert_eq!(normalise(&format!("worker:{ID}")), format!("person:{ID}"));
        assert_eq!(
            normalise(&format!("  worker:{ID} ")),
            format!("person:{ID}")
        );
    }

    #[test]
    fn anything_else_is_left_for_the_urn_check() {
        for v in [
            format!("person:{ID}"),
            format!("organization:{ID}"),
            "contact:x".to_string(),
            "Worker:x".to_string(),
            "workers:x".to_string(),
            String::new(),
        ] {
            assert_eq!(normalise(&v), v);
        }
    }

    #[test]
    fn options_and_lists_are_normalised_entry_by_entry() {
        assert_eq!(
            normalise_opt(Some(format!("worker:{ID}"))),
            Some(format!("person:{ID}"))
        );
        assert_eq!(normalise_opt(None), None);
        let mut list = vec![format!("worker:{ID}"), format!("person:{ID}"), "x".into()];
        normalise_all(&mut list);
        assert_eq!(
            list,
            [
                format!("person:{ID}"),
                format!("person:{ID}"),
                "x".to_string()
            ]
        );
    }
}
