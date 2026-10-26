//! Exchange rates (CRM-R50): reading a rate, choosing the one in force on a day, and converting an
//! amount with it.
//!
//! Pure and DB-free, and free of floating point: a rate is a whole number of billionths, and an
//! amount is converted with integer arithmetic, rounded half to even.

use chrono::NaiveDate;

/// Billionths in one: a rate is held as `rate * 1_000_000_000`.
pub const ONE: i64 = 1_000_000_000;
/// Decimal places a rate may have.
const PLACES: usize = 9;
/// Largest rate (exclusive), as whole units.
const MAX_WHOLE: i64 = 1_000_000;
/// Longest source.
pub const MAX_SOURCE: usize = 80;

/// A currency code: three capital letters.
///
/// # Errors
///
/// The reason, when it is not.
pub fn currency(code: &str) -> Result<String, String> {
    if code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase()) {
        Ok(code.to_string())
    } else {
        Err("a currency is three capital letters, such as GBP".to_string())
    }
}

/// A source, tidied: it must say where the rate came from.
///
/// # Errors
///
/// The reason, when it is empty or too long.
pub fn source(raw: &str) -> Result<String, String> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() || text.chars().count() > MAX_SOURCE || text.chars().any(char::is_control) {
        Err(format!(
            "a rate needs a source in words, up to {MAX_SOURCE} characters"
        ))
    } else {
        Ok(text)
    }
}

/// A decimal rate such as `1.27` as billionths.
///
/// # Errors
///
/// The reason, when it is not a positive decimal below 1,000,000 with at most 9 places.
pub fn parse_rate(text: &str) -> Result<i64, String> {
    let bad = || {
        format!(
            "a rate is a positive decimal below {MAX_WHOLE} with at most {PLACES} decimal places"
        )
    };
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty()
        || !digits(whole)
        || !digits(fraction)
        || fraction.len() > PLACES
        || (text.contains('.') && fraction.is_empty())
    {
        return Err(bad());
    }
    let whole: i64 = whole.parse().map_err(|_| bad())?;
    if whole >= MAX_WHOLE {
        return Err(bad());
    }
    let padded = format!("{fraction:0<PLACES$}");
    let parts: i64 = padded.parse().map_err(|_| bad())?;
    let ppb = whole * ONE + parts;
    if ppb <= 0 { Err(bad()) } else { Ok(ppb) }
}

/// A rate in billionths as a decimal, trailing zeros dropped (`1270000000` is `1.27`).
#[must_use]
pub fn format_rate(ppb: i64) -> String {
    let (whole, parts) = (ppb / ONE, ppb % ONE);
    if parts == 0 {
        return whole.to_string();
    }
    let fraction = format!("{parts:09}");
    format!("{whole}.{}", fraction.trim_end_matches('0'))
}

/// One recorded rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rate {
    /// The currency being bought with.
    pub from: String,
    /// The currency it buys.
    pub to: String,
    /// Billionths of `to` per one `from`.
    pub ppb: i64,
    /// The day it takes effect.
    pub effective_on: NaiveDate,
    /// Where it came from.
    pub source: String,
}

/// A rate chosen for a conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picked {
    /// The recorded rate used.
    pub rate: Rate,
    /// Whether the conversion runs the other way, by dividing.
    pub inverted: bool,
}

/// The rate to turn `from` into `to` on `day`: the latest taking effect on or before it, the
/// pair's own or the inverse of the reverse pair's, the pair's own on a tie. `None` when there is
/// none (nothing later than `day` is ever used, and no third currency is gone through).
#[must_use]
pub fn pick(rates: &[Rate], from: &str, to: &str, day: NaiveDate) -> Option<Picked> {
    let latest = |a: &str, b: &str| {
        rates
            .iter()
            .filter(|r| r.from == a && r.to == b && r.effective_on <= day)
            .max_by_key(|r| r.effective_on)
    };
    match (latest(from, to), latest(to, from)) {
        (Some(own), Some(reverse)) if reverse.effective_on > own.effective_on => Some(Picked {
            rate: reverse.clone(),
            inverted: true,
        }),
        (Some(own), _) => Some(Picked {
            rate: own.clone(),
            inverted: false,
        }),
        (None, Some(reverse)) => Some(Picked {
            rate: reverse.clone(),
            inverted: true,
        }),
        (None, None) => None,
    }
}

/// `numerator / denominator` rounded half to even.
fn divide(numerator: i128, denominator: i128) -> i128 {
    let (quotient, remainder) = (numerator / denominator, numerator % denominator);
    let twice = remainder.abs() * 2;
    let step = if numerator < 0 { -1 } else { 1 };
    if twice > denominator || (twice == denominator && quotient % 2 != 0) {
        quotient + step
    } else {
        quotient
    }
}

/// `amount_minor` converted with a picked rate, to a minor unit, half to even.
#[must_use]
pub fn convert(amount_minor: i64, picked: &Picked) -> i64 {
    let (amount, ppb, one) = (
        i128::from(amount_minor),
        i128::from(picked.rate.ppb),
        i128::from(ONE),
    );
    let converted = if picked.inverted {
        divide(amount * one, ppb)
    } else {
        divide(amount * ppb, one)
    };
    i64::try_from(converted).unwrap_or(if converted < 0 { i64::MIN } else { i64::MAX })
}

/// Sums in one currency that could not be converted, because no rate was in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOut {
    /// The currency.
    pub currency: String,
    /// How many group sums were left out.
    pub groups: usize,
    /// Their total, in that currency's minor units.
    pub amount_minor: i64,
}

/// A report's rows after conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    /// `(group, value in the target currency)`, largest first, then by group.
    pub rows: Vec<(String, i64)>,
    /// The rate used for each currency that was converted, by currency.
    pub used: Vec<(String, Picked)>,
    /// What had no rate, by currency.
    pub left_out: Vec<LeftOut>,
}

/// Convert `(group, currency, sum)` rows into `target` with the rates in force on `day`.
///
/// Each sum is converted **once**, so the rounding is at most half a minor unit per group and
/// currency; a sum already in the target is kept as it is; a sum with no rate is left out and
/// counted, never dropped quietly and never taken at one-to-one. Groups then merge.
#[must_use]
pub fn convert_rows(
    rows: &[(String, String, i64)],
    target: &str,
    day: NaiveDate,
    rates: &[Rate],
) -> Converted {
    use std::collections::BTreeMap;
    let mut chosen: BTreeMap<&str, Option<Picked>> = BTreeMap::new();
    let mut merged: BTreeMap<&str, i64> = BTreeMap::new();
    let mut left: BTreeMap<&str, (usize, i64)> = BTreeMap::new();
    for (group, currency, value) in rows {
        let converted = if currency == target {
            Some(*value)
        } else {
            chosen
                .entry(currency.as_str())
                .or_insert_with(|| pick(rates, currency, target, day))
                .as_ref()
                .map(|picked| convert(*value, picked))
        };
        if let Some(value) = converted {
            let total = merged.entry(group.as_str()).or_default();
            *total = total.saturating_add(value);
        } else {
            let entry = left.entry(currency.as_str()).or_default();
            entry.0 += 1;
            entry.1 = entry.1.saturating_add(*value);
        }
    }
    let mut rows: Vec<(String, i64)> = merged
        .into_iter()
        .map(|(group, value)| (group.to_string(), value))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let used = chosen
        .into_iter()
        .filter_map(|(currency, picked)| picked.map(|p| (currency.to_string(), p)))
        .collect();
    let left_out = left
        .into_iter()
        .map(|(currency, (groups, amount_minor))| LeftOut {
            currency: currency.to_string(),
            groups,
            amount_minor,
        })
        .collect();
    Converted {
        rows,
        used,
        left_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }

    fn rate(from: &str, to: &str, text: &str, on: NaiveDate) -> Rate {
        Rate {
            from: from.to_string(),
            to: to.to_string(),
            ppb: parse_rate(text).unwrap(),
            effective_on: on,
            source: "test".to_string(),
        }
    }

    #[test]
    fn a_rate_is_read_exactly() {
        assert_eq!(parse_rate("1"), Ok(1_000_000_000));
        assert_eq!(parse_rate("1.27"), Ok(1_270_000_000));
        assert_eq!(parse_rate("0.000000001"), Ok(1));
        assert_eq!(parse_rate("999999.999999999"), Ok(999_999_999_999_999));
        for bad in [
            "",
            ".",
            "1.",
            ".5",
            "0",
            "0.0",
            "-1",
            "1e3",
            "1,5",
            " 1",
            "1.0000000001",
            "1000000",
            "abc",
            "1.2.3",
        ] {
            assert!(parse_rate(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn a_rate_reads_back_as_written_without_trailing_zeros() {
        for text in ["1", "1.27", "0.000000001", "0.5", "123.456789012"] {
            assert_eq!(format_rate(parse_rate(text).unwrap()), text);
        }
        assert_eq!(format_rate(parse_rate("1.2700").unwrap()), "1.27");
    }

    #[test]
    fn currencies_and_sources_are_checked() {
        assert!(currency("GBP").is_ok());
        for bad in ["gbp", "GB", "GBPP", "G2P", "", "£££"] {
            assert!(currency(bad).is_err(), "{bad:?}");
        }
        assert_eq!(
            source("  ECB   reference rate "),
            Ok("ECB reference rate".to_string())
        );
        assert!(source("   ").is_err());
        assert!(source(&"x".repeat(MAX_SOURCE)).is_ok());
        assert!(source(&"x".repeat(MAX_SOURCE + 1)).is_err());
        assert!(source("a\u{7}b").is_err());
    }

    #[test]
    fn the_latest_rate_on_or_before_the_day_is_chosen() {
        let rates = [
            rate("USD", "GBP", "0.78", day(1, 1)),
            rate("USD", "GBP", "0.80", day(3, 1)),
            rate("USD", "GBP", "0.90", day(9, 1)),
        ];
        let at = |d| pick(&rates, "USD", "GBP", d).map(|p| format_rate(p.rate.ppb));
        assert_eq!(at(day(2, 1)).as_deref(), Some("0.78"));
        assert_eq!(
            at(day(3, 1)).as_deref(),
            Some("0.8"),
            "the day itself counts"
        );
        assert_eq!(at(day(8, 31)).as_deref(), Some("0.8"));
        assert_eq!(at(day(12, 31)).as_deref(), Some("0.9"));
        assert!(
            at(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()).is_none(),
            "none before the first"
        );
    }

    #[test]
    fn the_reverse_pair_is_used_inverted_when_it_is_later_or_alone() {
        let alone = [rate("GBP", "USD", "1.25", day(1, 1))];
        let picked = pick(&alone, "USD", "GBP", day(6, 1)).unwrap();
        assert!(picked.inverted);
        // 125 USD at 1 GBP = 1.25 USD is 100 GBP.
        assert_eq!(convert(12_500, &picked), 10_000);

        let both = [
            rate("USD", "GBP", "0.80", day(1, 1)),
            rate("GBP", "USD", "1.25", day(4, 1)),
        ];
        assert!(
            pick(&both, "USD", "GBP", day(6, 1)).unwrap().inverted,
            "the later one wins"
        );
        assert!(
            !pick(&both, "USD", "GBP", day(2, 1)).unwrap().inverted,
            "the other is not yet in force"
        );

        let tie = [
            rate("USD", "GBP", "0.80", day(4, 1)),
            rate("GBP", "USD", "1.30", day(4, 1)),
        ];
        assert!(
            !pick(&tie, "USD", "GBP", day(4, 1)).unwrap().inverted,
            "a tie goes to the pair's own"
        );
    }

    #[test]
    fn there_is_no_conversion_through_a_third_currency_and_none_from_the_future() {
        let rates = [
            rate("USD", "EUR", "0.9", day(1, 1)),
            rate("EUR", "GBP", "0.85", day(1, 1)),
            rate("USD", "JPY", "150", day(9, 1)),
        ];
        assert!(pick(&rates, "USD", "GBP", day(6, 1)).is_none());
        assert!(pick(&rates, "USD", "JPY", day(6, 1)).is_none());
        assert!(pick(&rates, "USD", "USD", day(6, 1)).is_none());
    }

    #[test]
    fn conversion_rounds_half_to_even() {
        let at = |text: &str| Picked {
            rate: rate("A", "B", text, day(1, 1)),
            inverted: false,
        };
        assert_eq!(convert(10_000, &at("0.8")), 8_000);
        // 5 minor units at 0.5 is 2.5: to the even 2; 7 at 0.5 is 3.5: to the even 4.
        assert_eq!(convert(5, &at("0.5")), 2);
        assert_eq!(convert(7, &at("0.5")), 4);
        // Not at a half, it rounds to nearest.
        assert_eq!(convert(3, &at("0.4")), 1);
        assert_eq!(convert(4, &at("0.4")), 2);
        assert_eq!(convert(0, &at("1.5")), 0);
        assert_eq!(
            convert(-5, &at("0.5")),
            -2,
            "the same on the other side of zero"
        );
        assert_eq!(convert(-7, &at("0.5")), -4);
    }

    #[test]
    fn a_large_amount_does_not_overflow_the_arithmetic() {
        let big = Picked {
            rate: rate("A", "B", "999999.999999999", day(1, 1)),
            inverted: false,
        };
        assert_eq!(convert(i64::MAX, &big), i64::MAX, "it saturates, not wraps");
        let back = Picked {
            rate: rate("A", "B", "0.000000001", day(1, 1)),
            inverted: true,
        };
        assert_eq!(convert(i64::MAX, &back), i64::MAX);
        let ordinary = Picked {
            rate: rate("A", "B", "1.5", day(1, 1)),
            inverted: false,
        };
        assert_eq!(
            convert(9_000_000_000_000_000, &ordinary),
            13_500_000_000_000_000
        );
    }

    fn row(group: &str, currency: &str, value: i64) -> (String, String, i64) {
        (group.to_string(), currency.to_string(), value)
    }

    #[test]
    fn rows_are_converted_once_merged_and_what_has_no_rate_is_counted() {
        let rates = [
            rate("USD", "GBP", "0.8", day(1, 1)),
            rate("EUR", "GBP", "0.85", day(1, 1)),
        ];
        let rows = [
            row("open", "GBP", 10_000),
            row("open", "USD", 5_000),
            row("won", "EUR", 2_000),
            row("won", "JPY", 777),
            row("lost", "JPY", 223),
        ];
        let done = convert_rows(&rows, "GBP", day(6, 1), &rates);
        // open: 10,000 + 4,000; won: 1,700 (the yen left out); lost has nothing left.
        assert_eq!(
            done.rows,
            [("open".to_string(), 14_000), ("won".to_string(), 1_700)]
        );
        assert_eq!(
            done.left_out,
            [LeftOut {
                currency: "JPY".to_string(),
                groups: 2,
                amount_minor: 1_000
            }]
        );
        let used: Vec<&str> = done.used.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(
            used,
            ["EUR", "USD"],
            "by currency; GBP needed no rate, JPY had none"
        );
    }

    #[test]
    fn a_sum_already_in_the_target_is_never_touched_and_ties_sort_by_group() {
        let rows = [row("b", "GBP", 5), row("a", "GBP", 5), row("c", "GBP", 9)];
        let done = convert_rows(&rows, "GBP", day(6, 1), &[]);
        assert_eq!(
            done.rows,
            [
                ("c".to_string(), 9),
                ("a".to_string(), 5),
                ("b".to_string(), 5)
            ]
        );
        assert!(done.used.is_empty() && done.left_out.is_empty());
    }

    #[test]
    fn the_date_decides_which_rate_a_report_uses() {
        let rates = [
            rate("USD", "GBP", "0.80", day(1, 1)),
            rate("USD", "GBP", "0.90", day(6, 1)),
        ];
        let rows = [row("g", "USD", 1_000)];
        assert_eq!(
            convert_rows(&rows, "GBP", day(5, 31), &rates).rows[0].1,
            800
        );
        assert_eq!(convert_rows(&rows, "GBP", day(6, 1), &rates).rows[0].1, 900);
        let before = convert_rows(
            &rows,
            "GBP",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            &rates,
        );
        assert!(before.rows.is_empty());
        assert_eq!(
            before.left_out.len(),
            1,
            "before any rate, nothing is converted at one-to-one"
        );
    }
}
