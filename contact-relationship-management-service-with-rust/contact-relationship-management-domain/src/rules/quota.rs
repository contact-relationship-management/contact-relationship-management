//! Quotas and commission (CRM-R59): how much of a quota a rep's won revenue reached, and the
//! commission a plan pays on it.
//!
//! Pure and DB-free, and free of floating point. A plan is a list of **marginal** tiers, like tax
//! bands: each tier starts at a percentage of the quota and pays a rate, in basis points, on the
//! revenue that falls between its start and the next tier's start. The thresholds are whole minor
//! units of the quota (rounded down), so the bands tile exactly: no revenue is in two bands and
//! none is in neither.

use serde::{Deserialize, Serialize};

use super::analytics::{self, Ratio};
use super::fx::divide;

/// Most tiers a plan may have.
pub const MAX_TIERS: usize = 8;
/// Highest tier start, as a percentage of the quota.
pub const MAX_FROM_PERCENT: u32 = 1000;
/// Highest rate: all of it.
pub const MAX_RATE_BPS: u32 = 10_000;
/// Longest name.
pub const MAX_NAME: usize = 80;

/// One tier of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "CommissionTier")]
#[serde(deny_unknown_fields)]
pub struct Tier {
    /// Where the tier starts, as a percentage of the quota (the first is `0`).
    pub from_percent: u32,
    /// What it pays on the revenue in its band, in basis points (`500` is 5%).
    pub rate_bps: u32,
}

/// A plan's name, tidied.
///
/// # Errors
///
/// The reason, when it is empty, too long or has control characters.
pub fn name(raw: &str) -> Result<String, String> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() || text.chars().count() > MAX_NAME || text.chars().any(char::is_control) {
        Err(format!(
            "a plan needs a name of up to {MAX_NAME} characters"
        ))
    } else {
        Ok(text)
    }
}

/// Whether `tiers` make a plan.
///
/// # Errors
///
/// The reason, when there are none or too many, the first does not start at `0`, the starts do not
/// rise, a start is above [`MAX_FROM_PERCENT`] or a rate above [`MAX_RATE_BPS`].
pub fn check_tiers(tiers: &[Tier]) -> Result<(), String> {
    if tiers.is_empty() || tiers.len() > MAX_TIERS {
        return Err(format!("a plan has 1 to {MAX_TIERS} tiers"));
    }
    if tiers[0].from_percent != 0 {
        return Err("the first tier starts at 0".to_string());
    }
    if tiers
        .windows(2)
        .any(|p| p[1].from_percent <= p[0].from_percent)
    {
        return Err("each tier starts above the one before".to_string());
    }
    if tiers.iter().any(|t| t.from_percent > MAX_FROM_PERCENT) {
        return Err(format!("a tier starts at no more than {MAX_FROM_PERCENT}%"));
    }
    if tiers.iter().any(|t| t.rate_bps > MAX_RATE_BPS) {
        return Err(format!("a rate is at most {MAX_RATE_BPS} basis points"));
    }
    Ok(())
}

/// What one tier paid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(rename = "CommissionBand")]
pub struct Band {
    /// Where the band starts, as a percentage of the quota.
    pub from_percent: u32,
    /// Where it ends; `null` for the last, which has no end.
    pub to_percent: Option<u32>,
    /// The revenue that fell in it, in minor units.
    pub revenue_minor: i64,
    /// Its rate, in basis points.
    pub rate_bps: u32,
    /// What it paid, in minor units, rounded half to even.
    pub commission_minor: i64,
}

/// The bands of `tiers` over a quota and the revenue reached.
///
/// A band's revenue is the part of `revenue_minor` between its start and the next one's, where a
/// start is the quota times its percentage, rounded down. Each band's commission is rounded half to
/// even, then they add up. A negative revenue earns nothing.
#[must_use]
pub fn bands(quota_minor: i64, revenue_minor: i64, tiers: &[Tier]) -> Vec<Band> {
    let start = |percent: u32| i128::from(quota_minor) * i128::from(percent) / 100;
    let revenue = i128::from(revenue_minor.max(0));
    tiers
        .iter()
        .enumerate()
        .map(|(i, tier)| {
            let low = start(tier.from_percent);
            let next = tiers.get(i + 1);
            let in_band = match next {
                Some(next) => (revenue.min(start(next.from_percent)) - low).max(0),
                None => (revenue - low).max(0),
            };
            let paid = divide(in_band * i128::from(tier.rate_bps), 10_000);
            Band {
                from_percent: tier.from_percent,
                to_percent: next.map(|n| n.from_percent),
                revenue_minor: i64::try_from(in_band).unwrap_or(i64::MAX),
                rate_bps: tier.rate_bps,
                commission_minor: i64::try_from(paid).unwrap_or(i64::MAX),
            }
        })
        .collect()
}

/// What the bands paid in all.
#[must_use]
pub fn commission(bands: &[Band]) -> i64 {
    bands
        .iter()
        .fold(0_i64, |sum, b| sum.saturating_add(b.commission_minor))
}

/// Share `amount_minor` by `shares_bps` (which add up to 10000), to the minor unit (CRM-R65): each
/// share gets the floor of its proportion and the leftover minor units go one each to the largest
/// remainders, the earliest share first on a tie. The parts always add up to the amount.
#[must_use]
pub fn split(amount_minor: i64, shares_bps: &[u32]) -> Vec<i64> {
    let amount = i128::from(amount_minor);
    let exact: Vec<i128> = shares_bps.iter().map(|s| amount * i128::from(*s)).collect();
    let mut parts: Vec<i128> = exact.iter().map(|e| e.div_euclid(10_000)).collect();
    let mut left = amount - parts.iter().sum::<i128>();
    let mut order: Vec<usize> = (0..shares_bps.len()).collect();
    order.sort_by(|a, b| {
        exact[*b]
            .rem_euclid(10_000)
            .cmp(&exact[*a].rem_euclid(10_000))
            .then(a.cmp(b))
    });
    for i in order {
        if left <= 0 {
            break;
        }
        parts[i] += 1;
        left -= 1;
    }
    parts
        .into_iter()
        .map(|p| i64::try_from(p).unwrap_or(i64::MAX))
        .collect()
}

/// The share of a quota reached, with its numerator and denominator (`null` value on a zero quota).
#[must_use]
pub fn attainment(quota_minor: i64, revenue_minor: i64) -> Ratio {
    analytics::ratio(revenue_minor, quota_minor)
}

/// What is still to win to meet the quota; zero once it is met.
#[must_use]
pub fn remaining(quota_minor: i64, revenue_minor: i64) -> i64 {
    quota_minor.saturating_sub(revenue_minor).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A split adds up to the amount, leftovers going to the largest remainders, earliest first.
    #[test]
    fn a_split_adds_up_to_the_amount() {
        assert_eq!(split(10_000, &[6_000, 4_000]), vec![6_000, 4_000]);
        assert_eq!(split(100, &[3_333, 3_333, 3_334]), vec![33, 33, 34]);
        assert_eq!(split(1, &[5_000, 5_000]), vec![1, 0]);
        assert_eq!(split(2, &[3_333, 3_333, 3_334]), vec![1, 0, 1]);
        for amount in [0, 1, 7, 999, 123_457] {
            let parts = split(amount, &[1_234, 2_345, 6_421]);
            assert_eq!(parts.iter().sum::<i64>(), amount);
        }
    }

    fn plan(tiers: &[(u32, u32)]) -> Vec<Tier> {
        tiers
            .iter()
            .map(|&(from_percent, rate_bps)| Tier {
                from_percent,
                rate_bps,
            })
            .collect()
    }

    #[test]
    fn a_plan_is_checked() {
        assert!(check_tiers(&plan(&[(0, 500)])).is_ok());
        assert!(check_tiers(&plan(&[(0, 500), (100, 800), (150, 1200)])).is_ok());
        for bad in [
            plan(&[]),
            plan(&[(10, 500)]),
            plan(&[(0, 500), (0, 800)]),
            plan(&[(0, 500), (100, 800), (50, 900)]),
            plan(&[(0, 10_001)]),
            plan(&[(0, 500), (1001, 800)]),
        ] {
            assert!(check_tiers(&bad).is_err(), "{bad:?}");
        }
        let nine: Vec<Tier> = (0..9)
            .map(|i| Tier {
                from_percent: i * 10,
                rate_bps: 100,
            })
            .collect();
        assert!(check_tiers(&nine).is_err());
    }

    #[test]
    fn names_are_tidied_and_bounded() {
        assert_eq!(name("  Sales   2026 "), Ok("Sales 2026".to_string()));
        assert!(name("   ").is_err());
        assert!(name(&"x".repeat(MAX_NAME + 1)).is_err());
        assert!(name("a\u{7}b").is_err());
    }

    #[test]
    fn revenue_falls_into_bands_like_tax_bands() {
        // Quota 100,000; 5% up to the quota, 8% from the quota to 150%, 12% above.
        let tiers = plan(&[(0, 500), (100, 800), (150, 1200)]);
        // Under the quota: all in the first band.
        let b = bands(100_000, 60_000, &tiers);
        assert_eq!(
            b.iter().map(|b| b.revenue_minor).collect::<Vec<_>>(),
            [60_000, 0, 0]
        );
        assert_eq!(commission(&b), 3_000);
        // 130,000: 100,000 at 5% and 30,000 at 8%.
        let b = bands(100_000, 130_000, &tiers);
        assert_eq!(
            b.iter().map(|b| b.revenue_minor).collect::<Vec<_>>(),
            [100_000, 30_000, 0]
        );
        assert_eq!(commission(&b), 5_000 + 2_400);
        // 200,000: 100,000, 50,000 and 50,000.
        let b = bands(100_000, 200_000, &tiers);
        assert_eq!(commission(&b), 5_000 + 4_000 + 6_000);
        assert_eq!(b[2].to_percent, None);
        assert_eq!(b[0].to_percent, Some(100));
        // Nothing, or a loss, earns nothing.
        assert_eq!(commission(&bands(100_000, 0, &tiers)), 0);
        assert_eq!(commission(&bands(100_000, -5, &tiers)), 0);
    }

    #[test]
    fn the_bands_tile_the_revenue_exactly() {
        // A quota that does not divide evenly: thresholds are rounded down, and still no revenue
        // is counted twice or missed.
        let tiers = plan(&[(0, 100), (33, 200), (67, 300), (100, 400)]);
        for revenue in [0, 1, 332, 333, 334, 700, 1_000, 1_001, 5_000_000] {
            let b = bands(1_001, revenue, &tiers);
            assert_eq!(
                b.iter().map(|b| b.revenue_minor).sum::<i64>(),
                revenue,
                "{revenue}"
            );
        }
    }

    #[test]
    fn each_band_is_rounded_half_to_even() {
        // 5 minor units at 50% is 2.5, which rounds to 2; 7 at 50% is 3.5, which rounds to 4.
        assert_eq!(commission(&bands(1_000, 5, &plan(&[(0, 5_000)]))), 2);
        assert_eq!(commission(&bands(1_000, 7, &plan(&[(0, 5_000)]))), 4);
        // A rate of nothing pays nothing; all of it pays all of it.
        assert_eq!(commission(&bands(1_000, 600, &plan(&[(0, 0)]))), 0);
        assert_eq!(commission(&bands(1_000, 600, &plan(&[(0, 10_000)]))), 600);
    }

    #[test]
    fn a_huge_amount_saturates_rather_than_wrapping() {
        let b = bands(i64::MAX, i64::MAX, &plan(&[(0, 10_000)]));
        assert_eq!(commission(&b), i64::MAX);
    }

    #[test]
    fn attainment_is_a_ratio_and_the_gap_never_goes_negative() {
        let r = attainment(200_000, 150_000);
        assert_eq!((r.numerator, r.denominator), (150_000, 200_000));
        assert_eq!(r.value, Some(0.75));
        assert_eq!(attainment(0, 5).value, None);
        assert_eq!(remaining(200_000, 150_000), 50_000);
        assert_eq!(remaining(200_000, 250_000), 0);
    }
}
