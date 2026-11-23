//! A figure's legal range, checked and never clamped (spec/rust-checked-arithmetic, RCA-INT-6).
//!
//! An out-of-range figure is not pulled to the nearest bound. [`within`] says whether it is in
//! range, and the caller recovers with the fallback its declaration names (`spec/arithmetic/index.md`)
//! by writing `within(..).unwrap_or(FALLBACK)`, citing the declaration's ID.

/// `n` when it lies in `range`, else `None`.
#[must_use]
pub fn within<T: PartialOrd>(n: T, range: std::ops::RangeInclusive<T>) -> Option<T> {
    range.contains(&n).then_some(n)
}

/// A length as a `u64`. A `usize` is at most 64 bits on every supported target, so this is the length
/// itself; on a target where it were wider, the declared fallback is the largest `u64` (LEN-1).
#[must_use]
pub fn len_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX) // LEN-1
}

/// A length or a constant as an `i64`, for a database bind or a count shown. A `usize` is at most 64
/// bits on every supported target, so this is the value itself; on a wider one, the declared fallback
/// is the largest `i64` (LEN-1).
#[must_use]
pub fn len_i64(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX) // LEN-1
}

#[cfg(test)]
mod tests {
    use super::within;

    #[test]
    fn a_length_is_itself_as_an_i64() {
        assert_eq!(super::len_i64(0), 0);
        assert_eq!(super::len_i64(41), 41);
    }

    #[test]
    fn a_length_is_itself_as_a_u64() {
        assert_eq!(super::len_u64(0), 0);
        assert_eq!(super::len_u64(41), 41);
    }

    #[test]
    fn a_figure_is_in_range_or_none_never_moved_to_a_bound() {
        assert_eq!(within(5, 1..=10), Some(5));
        assert_eq!(within(1, 1..=10), Some(1));
        assert_eq!(within(10, 1..=10), Some(10));
        assert_eq!(within(0, 1..=10), None);
        assert_eq!(within(11, 1..=10), None);
        assert_eq!(within(u64::MAX, 1..=10), None);
    }
}
