//! Merging two contacts (CRM-R24): the consent rule, as pure code.
//!
//! A merge must never leave the survivor more permissive about marketing
//! consent than either record was. The states are ordered by how little they
//! allow; the result is the strictest of the two.

use super::tokens::CONSENT_STATES;

/// How restrictive a consent state is: higher is stricter. `withdrawn` (the
/// person said no) beats `never` (no one asked), which beats `granted`.
fn strictness(state: &str) -> u8 {
    match state {
        "withdrawn" => 2,
        "never" => 1,
        _ => 0,
    }
}

/// The consent state of a merged contact: the stricter of the two. An
/// unrecognised state is treated as `never`, never as a grant.
#[must_use]
pub fn stricter_consent<'a>(survivor: &'a str, merged: &'a str) -> &'a str {
    let known = |s: &'a str| {
        if CONSENT_STATES.contains(&s) {
            s
        } else {
            "never"
        }
    };
    let (a, b) = (known(survivor), known(merged));
    if strictness(b) > strictness(a) { b } else { a }
}

/// Keep the survivor's value; use the merged contact's only when the
/// survivor has none. Nothing the survivor holds is overwritten.
#[must_use]
pub fn fill<T>(survivor: Option<T>, merged: Option<T>) -> Option<T> {
    survivor.or(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stricter_state_wins_in_every_pair() {
        let cases = [
            ("granted", "granted", "granted"),
            ("granted", "withdrawn", "withdrawn"),
            ("withdrawn", "granted", "withdrawn"),
            ("granted", "never", "never"),
            ("never", "granted", "never"),
            ("never", "never", "never"),
            ("never", "withdrawn", "withdrawn"),
            ("withdrawn", "never", "withdrawn"),
            ("withdrawn", "withdrawn", "withdrawn"),
        ];
        for (survivor, merged, expected) in cases {
            assert_eq!(
                stricter_consent(survivor, merged),
                expected,
                "{survivor} + {merged}"
            );
        }
    }

    #[test]
    fn an_unknown_state_is_never_a_grant() {
        assert_eq!(stricter_consent("granted", "bogus"), "never");
        assert_eq!(stricter_consent("bogus", "granted"), "never");
    }

    #[test]
    fn fill_keeps_the_survivors_value() {
        assert_eq!(fill(Some(1), Some(2)), Some(1));
        assert_eq!(fill(None, Some(2)), Some(2));
        assert_eq!(fill::<i32>(None, None), None);
    }
}
