//! The teams import from the worker service (CRM-R73, CRM-D17). DB-free.
//!
//! WPM lists its active workers; a team is made per department, and its members are the workers'
//! **`person:` references**, the family's name for a person (CRM keys its contacts and employments
//! by `person_ref`, and WPM's hire and leave events and overlap check use the same). A worker whose
//! `person_ref` is missing or not a `person:` URN joins no team and is counted; it never falls back
//! to a `worker:` reference.

use std::collections::BTreeMap;
use std::str::FromStr;

use entity_ref::{EntityRef, EntityType};

/// The workers sorted by department, ready to be made teams.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Grouped {
    /// Department (trimmed, as WPM spells it) to the distinct `person:` references in it, sorted.
    pub by_department: BTreeMap<String, Vec<String>>,
    /// Workers with a usable `person:` reference and no department.
    pub without_department: usize,
    /// Workers whose `person_ref` is missing or not a `person:` URN, whatever their department.
    pub without_person_ref: usize,
}

/// Whether `text` is a well-formed `person:` URN.
#[must_use]
pub fn is_person_ref(text: &str) -> bool {
    EntityRef::from_str(text).is_ok_and(|e| e.entity_type == EntityType::Person)
}

/// Group `(department, person_ref)` pairs, one per worker, into teams' members.
#[must_use]
pub fn group<'a>(workers: impl IntoIterator<Item = (&'a str, Option<&'a str>)>) -> Grouped {
    let mut out = Grouped::default();
    for (department, person_ref) in workers {
        let Some(person_ref) = person_ref.map(str::trim).filter(|r| is_person_ref(r)) else {
            out.without_person_ref += 1;
            continue;
        };
        let department = department.trim();
        if department.is_empty() {
            out.without_department += 1;
            continue;
        }
        out.by_department
            .entry(department.to_string())
            .or_default()
            .push(person_ref.to_string());
    }
    for members in out.by_department.values_mut() {
        members.sort();
        members.dedup();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "person:00000000-0000-0000-0000-00000000000a";
    const B: &str = "person:00000000-0000-0000-0000-00000000000b";
    const C: &str = "person:00000000-0000-0000-0000-00000000000c";

    #[test]
    fn members_are_person_references_sorted_and_once_each_per_department() {
        let g = group([
            ("Sales", Some(B)),
            ("Sales", Some(A)),
            ("Support", Some(C)),
            // The same person twice in one department (two employments) is one member.
            ("Sales", Some(A)),
            // And in two departments, a member of each.
            (" Support ", Some(A)),
        ]);
        assert_eq!(g.by_department["Sales"], [A, B]);
        assert_eq!(g.by_department["Support"], [A, C]);
        assert_eq!(g.by_department.len(), 2);
        assert_eq!((g.without_department, g.without_person_ref), (0, 0));
    }

    #[test]
    fn a_worker_without_a_person_reference_is_counted_and_never_becomes_a_worker_reference() {
        let worker = "worker:00000000-0000-0000-0000-00000000000a";
        let g = group([
            ("Sales", None),
            ("Sales", Some("")),
            ("Sales", Some("not-a-urn")),
            ("Sales", Some("person:nope")),
            ("Sales", Some(worker)),
            (
                "Sales",
                Some("organization:00000000-0000-0000-0000-00000000000a"),
            ),
            // No department and no person: counted once, as without a person.
            ("", None),
            ("Sales", Some(A)),
        ]);
        assert_eq!(g.by_department["Sales"], [A]);
        assert_eq!(g.without_person_ref, 7);
        assert_eq!(g.without_department, 0);
        assert!(
            g.by_department
                .values()
                .flatten()
                .all(|m| m.starts_with("person:"))
        );
    }

    #[test]
    fn a_worker_with_a_person_and_no_department_is_counted_apart() {
        let g = group([("", Some(A)), ("   ", Some(B)), ("Sales", Some(C))]);
        assert_eq!(g.without_department, 2);
        assert_eq!(g.without_person_ref, 0);
        assert_eq!(g.by_department.keys().collect::<Vec<_>>(), ["Sales"]);
    }

    #[test]
    fn a_person_reference_is_trimmed_and_only_person_urns_pass() {
        assert!(is_person_ref(A));
        assert!(!is_person_ref(
            "worker:00000000-0000-0000-0000-00000000000a"
        ));
        assert!(!is_person_ref(""));
        let g = group([(
            "Sales",
            Some(" person:00000000-0000-0000-0000-00000000000a "),
        )]);
        assert_eq!(g.by_department["Sales"], [A]);
    }

    #[test]
    fn nothing_in_is_nothing_out() {
        assert_eq!(group([]), Grouped::default());
    }
}
