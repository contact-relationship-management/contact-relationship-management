//! An absent optional that means empty, said so (spec/rust-checked-arithmetic, RCA-P-3, RCA-IN-3).
//!
//! `Option::unwrap_or_default` is banned (`clippy.toml`): it reads as a hidden fallback, and it has
//! been used to turn a failure into an empty value. [`OrEmpty::or_empty`] is for the case where an
//! absent value really is empty, with no failure behind it: an optional text that was not given, a
//! header that was not sent, a list that has no entries yet. A value that failed to read is an error,
//! not an empty value; do not use this for one.

/// An absent value read as the empty one.
pub trait OrEmpty<T> {
    /// The value, or the empty one (`""`, an empty list, an empty map, `0`) when it is absent.
    fn or_empty(self) -> T;
}

impl<T: Default> OrEmpty<T> for Option<T> {
    #[expect(
        clippy::manual_unwrap_or_default,
        reason = "the one place that says an absent value is empty, so that Option::unwrap_or_default can be banned"
    )]
    fn or_empty(self) -> T {
        match self {
            Some(value) => value,
            None => T::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OrEmpty;

    #[test]
    fn an_absent_value_is_empty_and_a_present_one_is_kept() {
        assert_eq!(None::<String>.or_empty(), "");
        assert_eq!(Some("text".to_string()).or_empty(), "text");
        assert_eq!(None::<Vec<u8>>.or_empty(), Vec::<u8>::new());
        assert_eq!(Some(vec![1_u8]).or_empty(), [1]);
        assert_eq!(None::<&str>.or_empty(), "");
    }
}
