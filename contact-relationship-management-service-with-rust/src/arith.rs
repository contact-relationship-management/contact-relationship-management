//! Arithmetic failures and the tallies that have a declared ceiling (spec/rust-checked-arithmetic,
//! the project's declarations in spec/arithmetic/index.md).
//!
//! A figure that cannot be computed is an error that names the figure ([`overflow`], RCA-P-4) and
//! says nothing of its value (RCA-ERR-2, RCA-ERR-4); a tally that counts past its type is held at
//! the ceiling the declaration names ([`Tally`], TALLY-1).

use axa::prelude::*;

use crate::controllers::unprocessable;

/// A `422` for a figure that could not be worked out: it names the figure and never shows a number.
#[must_use]
pub fn overflow(figure: &str) -> Error {
    unprocessable(&format!("{figure} could not be calculated"))
}

/// A `422` for a figure the caller sent outside its legal range: it names the figure and the range,
/// and never echoes the value (RCA-ERR-2). The figure is refused, not moved to a bound.
#[must_use]
pub fn out_of_range(figure: &str, lo: i64, hi: i64) -> Error {
    unprocessable(&format!("{figure} must be from {lo} to {hi}"))
}

/// Counting that stops at the type's largest value rather than wrapping or stopping the process.
///
/// TALLY-1: a count of rows, events or attempts is the number of things seen; past the largest
/// value of its type it stays there. That is a valid value of a tally and no figure money rests
/// on, so the declared fallback is the type's `MAX`.
pub trait Tally: Sized {
    /// `self + by`, or the type's `MAX` when that does not fit.
    #[must_use]
    fn tally(self, by: Self) -> Self;
}

macro_rules! tally {
    ($($ty:ty),*) => {$(
        impl Tally for $ty {
            fn tally(self, by: Self) -> Self {
                self.checked_add(by).unwrap_or(Self::MAX) // TALLY-1
            }
        }
    )*};
}

tally!(u32, u64, usize, i32, i64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tally_stops_at_its_ceiling_and_never_wraps() {
        assert_eq!(250_u8.checked_add(5), Some(255));
        assert_eq!(1_u64.tally(2), 3);
        assert_eq!(u64::MAX.tally(1), u64::MAX);
        assert_eq!(i64::MAX.tally(1), i64::MAX);
        assert_eq!(usize::MAX.tally(usize::MAX), usize::MAX);
    }

    #[test]
    fn an_overflow_names_the_figure_and_shows_no_number() {
        let text = format!("{:?}", overflow("deal_total"));
        assert!(
            text.contains("deal_total could not be calculated"),
            "{text}"
        );
    }
}
