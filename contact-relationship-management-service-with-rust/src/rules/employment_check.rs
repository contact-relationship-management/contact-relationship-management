//! The employment check against the worker service (CRM-R94, WPM-R208). DB-free and clock-free.
//!
//! WPM answers, for up to [`BATCH`] `person:` references the caller already holds, whether each is a
//! worker of one organization today: `POST /api/bridges/overlap` with `{organization_ref,
//! person_refs}`, answered `{organization_ref, results: [{person_ref, worker}]}`. This module plans
//! the batches, reads an answer strictly, and says what one answer means for a contact; the
//! controller only sends and applies.

use std::collections::BTreeSet;
use std::str::FromStr;

use entity_ref::{EntityRef, EntityType};

/// The most references one call to WPM may carry (WPM's `OVERLAP_MAX`).
pub const BATCH: usize = 500;

/// The batches to send for `refs`: each distinct well-formed `person:` reference once, in sorted
/// order, at most [`BATCH`] a batch. Anything else (an erased contact's tombstone is a `person:`
/// reference and is left out by the caller; a malformed one is left out here) is not sent, since
/// WPM refuses a whole batch for one bad reference.
#[must_use]
pub fn batches(refs: impl IntoIterator<Item = String>, tombstone: &str) -> Vec<Vec<String>> {
    let distinct: BTreeSet<String> = refs
        .into_iter()
        .filter(|r| r != tombstone)
        .filter(|r| EntityRef::from_str(r).is_ok_and(|e| e.entity_type == EntityType::Person))
        .collect();
    let all: Vec<String> = distinct.into_iter().collect();
    all.chunks(BATCH).map(<[String]>::to_vec).collect()
}

/// The references in `asked` that WPM's `answer` says are workers of `organization_ref` today.
///
/// # Errors
///
/// A short reason, naming no reference, when the answer is not one to `asked`: another
/// organization, no `results` list, an entry without a `person_ref` string or a `worker` boolean, a
/// reference that was not asked about, or one answered twice. A bad answer records nothing.
pub fn workers_in(
    asked: &[String],
    organization_ref: &str,
    answer: &serde_json::Value,
) -> Result<BTreeSet<String>, String> {
    if answer
        .get("organization_ref")
        .and_then(serde_json::Value::as_str)
        != Some(organization_ref)
    {
        return Err("the worker service answered for another organization".to_string());
    }
    let results = answer
        .get("results")
        .and_then(serde_json::Value::as_array)
        .ok_or("the worker service answer has no results")?;
    let asked: BTreeSet<&str> = asked.iter().map(String::as_str).collect();
    let mut seen = BTreeSet::new();
    let mut workers = BTreeSet::new();
    for entry in results {
        let person_ref = entry
            .get("person_ref")
            .and_then(serde_json::Value::as_str)
            .ok_or("the worker service answer has an entry without a person_ref")?;
        let worker = entry
            .get("worker")
            .and_then(serde_json::Value::as_bool)
            .ok_or("the worker service answer has an entry without a yes or no")?;
        if !asked.contains(person_ref) {
            return Err("the worker service answered about someone not asked about".to_string());
        }
        if !seen.insert(person_ref) {
            return Err("the worker service answered about someone twice".to_string());
        }
        if worker {
            workers.insert(person_ref.to_string());
        }
    }
    Ok(workers)
}

/// The last hire or leave applied for a person by the organization (CRM-R93), as far as the
/// check needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastEvent {
    /// No event has been applied, or its record has been pruned.
    None,
    /// The last applied was a hire.
    Hired,
    /// The last applied was a leave.
    Left,
}

/// What one answer means for the contacts carrying the person (CRM-R94).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Mark them employees of the organization, as a hire event would.
    Record,
    /// Change nothing. Carries the reason.
    Nothing(&'static str),
}

/// What WPM's `worker` answer means, given the last event applied for the person.
///
/// - `false` changes nothing (`not_a_worker`): the check **never ends an employment**, since one
///   recorded from a hire event is ended by its leave event, and a "no" today may be a hire whose
///   first day has not come (CRM-D15 pauses from the hire's arrival).
/// - `true` records the employment, unless the last event applied was a leave (`left_by_event`):
///   WPM says "worker" until a leave's day comes, while the CRM lifts the pause when the leave
///   arrives (CRM-D15), and the event is the more specific word.
#[must_use]
pub const fn decide(worker: bool, last: LastEvent) -> Action {
    match (worker, last) {
        (false, _) => Action::Nothing("not_a_worker"),
        (true, LastEvent::Left) => Action::Nothing("left_by_event"),
        (true, LastEvent::None | LastEvent::Hired) => Action::Record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ORG: &str = "organization:0c4f1e2a-0000-4000-8000-0000000000aa";
    const TOMB: &str = "person:00000000-0000-0000-0000-000000000000";

    fn person(n: usize) -> String {
        format!("person:0c4f1e2a-0000-4000-8000-{n:012}")
    }

    #[test]
    fn references_go_once_each_sorted_in_batches_of_at_most_five_hundred() {
        assert!(batches(Vec::new(), TOMB).is_empty(), "nothing to ask");
        let one = batches(vec![person(2), person(1), person(2)], TOMB);
        assert_eq!(one, vec![vec![person(1), person(2)]]);
        let full = batches((0..BATCH).map(person), TOMB);
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].len(), BATCH);
        let over = batches((0..=BATCH).map(person), TOMB);
        assert_eq!(over.iter().map(Vec::len).collect::<Vec<_>>(), [BATCH, 1]);
        let many = batches((0..1_234).rev().map(person), TOMB);
        assert_eq!(
            many.iter().map(Vec::len).collect::<Vec<_>>(),
            [500, 500, 234]
        );
        assert_eq!(many[0][0], person(0));
        assert_eq!(many[2][233], person(1_233));
    }

    #[test]
    fn only_well_formed_person_references_are_sent_and_never_the_tombstone() {
        let sent = batches(
            vec![
                person(1),
                TOMB.to_string(),
                ORG.to_string(),
                "worker:0c4f1e2a-0000-4000-8000-000000000001".to_string(),
                "Ada Lovelace".to_string(),
                "person:not-a-uuid".to_string(),
            ],
            TOMB,
        );
        assert_eq!(sent, vec![vec![person(1)]]);
    }

    fn answer(results: serde_json::Value) -> serde_json::Value {
        let mut body = json!({ "organization_ref": ORG });
        body["results"] = results;
        body
    }

    #[test]
    fn an_answer_gives_the_workers_among_those_asked() {
        let asked = vec![person(1), person(2), person(3)];
        let got = workers_in(
            &asked,
            ORG,
            &answer(json!([
                { "person_ref": person(1), "worker": true },
                { "person_ref": person(2), "worker": false },
                { "person_ref": person(3), "worker": true },
            ])),
        )
        .unwrap();
        assert_eq!(got, BTreeSet::from([person(1), person(3)]));
        // An answer may leave someone out: they are not a worker as far as the CRM knows.
        let partial = workers_in(&asked, ORG, &answer(json!([]))).unwrap();
        assert!(partial.is_empty());
    }

    #[test]
    fn an_answer_that_is_not_to_the_question_records_nothing() {
        let asked = vec![person(1)];
        let bad = |body: serde_json::Value| workers_in(&asked, ORG, &body);
        assert!(bad(json!({ "organization_ref": "organization:other", "results": [] })).is_err());
        assert!(bad(json!({ "results": [] })).is_err());
        assert!(bad(json!({ "organization_ref": ORG })).is_err());
        assert!(bad(json!({ "organization_ref": ORG, "results": {} })).is_err());
        assert!(bad(answer(json!([{ "person_ref": person(1) }]))).is_err());
        assert!(
            bad(answer(
                json!([{ "person_ref": person(1), "worker": "yes" }])
            ))
            .is_err()
        );
        assert!(bad(answer(json!([{ "worker": true }]))).is_err());
        let stranger = bad(answer(json!([{ "person_ref": person(9), "worker": true }])));
        assert!(stranger.is_err());
        assert!(
            !stranger.unwrap_err().contains("person:"),
            "a reason names no one"
        );
        assert!(
            bad(answer(json!([
                { "person_ref": person(1), "worker": true },
                { "person_ref": person(1), "worker": false },
            ])))
            .is_err()
        );
    }

    #[test]
    fn a_no_never_ends_an_employment_and_a_yes_does_not_undo_a_leave() {
        for last in [LastEvent::None, LastEvent::Hired, LastEvent::Left] {
            assert_eq!(decide(false, last), Action::Nothing("not_a_worker"));
        }
        assert_eq!(decide(true, LastEvent::None), Action::Record);
        assert_eq!(decide(true, LastEvent::Hired), Action::Record);
        assert_eq!(
            decide(true, LastEvent::Left),
            Action::Nothing("left_by_event")
        );
    }
}
