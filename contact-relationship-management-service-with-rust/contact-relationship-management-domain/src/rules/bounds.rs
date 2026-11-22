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

#[cfg(test)]
mod tests {
    use super::within;

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
