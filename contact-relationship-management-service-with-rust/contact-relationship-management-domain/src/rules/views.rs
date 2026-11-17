//! Saved views (CRM-R35): which lists have views, and what a view may filter, sort and show.
//!
//! Pure and DB-free. Every set is closed: a view is checked against it when it is stored.

use std::collections::BTreeMap;

use super::tokens;

/// The lists that have views.
pub const KINDS: &[&str] = &["contact", "account", "deal", "lead", "ticket"];
/// Most views one owner may keep.
pub const MAX_PER_OWNER: i64 = 50;
/// Longest name.
pub const MAX_NAME: usize = 80;
/// Most values in one clause.
const MAX_VALUES: usize = 20;

/// The columns of each list, which are also what it may be sorted by.
#[must_use]
pub fn columns(kind: &str) -> &'static [&'static str] {
    match kind {
        "contact" => &["display_name", "job_title", "status", "marketing_consent"],
        "account" => &["display_name", "tier", "industry"],
        "deal" => &[
            "name",
            "amount_minor",
            "currency",
            "state",
            "expected_close_on",
        ],
        "lead" => &["display_name", "source", "status", "score"],
        "ticket" => &["title", "priority", "status", "channel"],
        _ => &[],
    }
}

/// Whether `value` can be held by the filterable `field` of `kind`; `None` if the field is not
/// filterable.
#[must_use]
fn allows(kind: &str, field: &str, value: &str) -> Option<bool> {
    let token = |set: &[&str]| Some(tokens::is_token(set, value));
    match (kind, field) {
        ("contact" | "lead" | "ticket", "status") => token(match kind {
            "contact" => tokens::CONTACT_STATUSES,
            "lead" => tokens::LEAD_STATUSES,
            _ => tokens::TICKET_STATUSES,
        }),
        ("contact", "marketing_consent") => token(tokens::CONSENT_STATES),
        ("account", "tier") => token(tokens::ACCOUNT_TIERS),
        ("deal", "currency") => {
            Some(value.len() == 3 && value.bytes().all(|b| b.is_ascii_uppercase()))
        }
        ("deal", "state") => token(&["open", "won", "lost"]),
        ("lead", "source") => token(tokens::LEAD_SOURCES),
        ("ticket", "priority") => token(tokens::PRIORITIES),
        ("ticket", "channel") => token(tokens::TICKET_CHANNELS),
        _ => None,
    }
}

/// Check a whole view. Returns every problem, as `(field, message)`.
#[must_use]
pub fn check(
    kind: &str,
    filter: &BTreeMap<String, Vec<String>>,
    sort: Option<(&str, &str)>,
    shown: Option<&[String]>,
) -> Vec<(&'static str, String)> {
    let mut problems = Vec::new();
    if !KINDS.contains(&kind) {
        problems.push(("kind", format!("must be one of {KINDS:?}")));
        return problems;
    }
    for (field, values) in filter {
        if values.is_empty() || values.len() > MAX_VALUES {
            problems.push(("filter", format!("{field} needs 1 to {MAX_VALUES} values")));
            continue;
        }
        for value in values {
            match allows(kind, field, value) {
                None => {
                    problems.push((
                        "filter",
                        format!("{field} cannot be filtered on a {kind} list"),
                    ));
                    break;
                }
                Some(false) => {
                    problems.push(("filter", format!("{value:?} is not a value of {field}")));
                }
                Some(true) => {}
            }
        }
    }
    if let Some((field, direction)) = sort {
        if !columns(kind).contains(&field) {
            problems.push(("sort", format!("{field} is not a column of a {kind} list")));
        }
        if !matches!(direction, "asc" | "desc") {
            problems.push(("sort", "direction must be asc or desc".to_string()));
        }
    }
    if let Some(shown) = shown {
        if shown.is_empty() {
            problems.push(("columns", "needs at least one column".to_string()));
        }
        for column in shown {
            if !columns(kind).contains(&column.as_str()) {
                problems.push((
                    "columns",
                    format!("{column} is not a column of a {kind} list"),
                ));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, v)| {
                (
                    (*k).to_string(),
                    v.iter().map(ToString::to_string).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn a_good_view_has_no_problems() {
        let f = filter(&[("status", &["open", "pending"]), ("priority", &["high"])]);
        assert!(
            check(
                "ticket",
                &f,
                Some(("priority", "desc")),
                Some(&["title".to_string()])
            )
            .is_empty()
        );
        assert!(
            check(
                "deal",
                &filter(&[("currency", &["GBP"]), ("state", &["open"])]),
                None,
                None
            )
            .is_empty()
        );
        assert!(check("contact", &BTreeMap::new(), None, None).is_empty());
    }

    #[test]
    fn anything_outside_the_closed_sets_is_a_problem() {
        assert_eq!(check("campaign", &BTreeMap::new(), None, None).len(), 1);
        // A field of another list, an unknown value, an empty clause.
        assert_eq!(
            check("contact", &filter(&[("tier", &["customer"])]), None, None).len(),
            1
        );
        assert_eq!(
            check("ticket", &filter(&[("status", &["bogus"])]), None, None).len(),
            1
        );
        assert_eq!(
            check("lead", &filter(&[("source", &[])]), None, None).len(),
            1
        );
        assert_eq!(
            check("deal", &filter(&[("currency", &["gbp"])]), None, None).len(),
            1
        );
        // The preferred channel is masked data and cannot be filtered on.
        assert_eq!(
            check(
                "contact",
                &filter(&[("preferred_channel", &["email"])]),
                None,
                None
            )
            .len(),
            1
        );
        assert_eq!(
            check("lead", &BTreeMap::new(), Some(("name", "up")), None).len(),
            2
        );
        assert_eq!(check("account", &BTreeMap::new(), None, Some(&[])).len(), 1);
        assert_eq!(
            check(
                "account",
                &BTreeMap::new(),
                None,
                Some(&["salary".to_string()])
            )
            .len(),
            1
        );
    }
}
