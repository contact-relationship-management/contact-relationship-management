//! Derived KPI arithmetic (CRM-R13, CRM-D4): forecast, ROI, CLV, and
//! win rate — minor units, per-currency honesty (mixed currencies
//! never silently sum), overflow-refusing, ratios reported with their
//! numerator/denominator and `null` on a zero denominator.

use std::collections::BTreeMap;

/// One open deal's forecast inputs.
#[derive(Debug, Clone)]
pub struct OpenDeal {
    /// Deal amount, minor units.
    pub amount_minor: i64,
    /// ISO-4217 currency.
    pub currency: String,
    /// The stage's win probability, 0–100.
    pub probability_percent: i32,
}

/// Stage-weighted forecast per currency: `Σ amount × p / 100`.
///
/// # Errors
///
/// On arithmetic overflow.
pub fn forecast_by_currency(deals: &[OpenDeal]) -> Result<BTreeMap<String, i64>, String> {
    let mut totals: BTreeMap<String, i64> = BTreeMap::new();
    for deal in deals {
        let weighted = deal
            .amount_minor
            .checked_mul(i64::from(deal.probability_percent.clamp(0, 100)))
            .map(|v| v / 100)
            .ok_or_else(|| "forecast arithmetic overflows".to_string())?;
        let slot = totals.entry(deal.currency.clone()).or_insert(0);
        *slot = slot
            .checked_add(weighted)
            .ok_or_else(|| "forecast sum overflows".to_string())?;
    }
    Ok(totals)
}

/// A ratio reported honestly: numerator, denominator, and the value
/// (`None` when the denominator is zero — never 0% or 100%).
#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct Ratio {
    /// The numerator.
    pub numerator: i64,
    /// The denominator.
    pub denominator: i64,
    /// `numerator / denominator`, or `None` on a zero denominator.
    pub value: Option<f64>,
}

/// Build a [`Ratio`].
#[must_use]
#[allow(clippy::cast_precision_loss)] // display-only ratio
pub fn ratio(numerator: i64, denominator: i64) -> Ratio {
    Ratio {
        numerator,
        denominator,
        value: if denominator == 0 {
            None
        } else {
            Some(numerator as f64 / denominator as f64)
        },
    }
}

/// Win rate = won / (won + lost) over closed deals.
#[must_use]
pub fn win_rate(won: i64, lost: i64) -> Ratio {
    ratio(won, won + lost)
}

/// Campaign ROI = (attributed won revenue − cost) / cost, per the
/// campaign's currency. Zero cost ⇒ `value: None` with the absolute
/// figures alongside (CRM-R8).
#[must_use]
pub fn roi(attributed_won_revenue_minor: i64, cost_minor: i64) -> Ratio {
    ratio(
        attributed_won_revenue_minor.saturating_sub(cost_minor),
        cost_minor,
    )
}

/// CLV per account: Σ won-deal amounts per currency.
///
/// # Errors
///
/// On arithmetic overflow.
pub fn clv_by_currency(won: &[(i64, String)]) -> Result<BTreeMap<String, i64>, String> {
    let mut totals: BTreeMap<String, i64> = BTreeMap::new();
    for (amount, currency) in won {
        let slot = totals.entry(currency.clone()).or_insert(0);
        *slot = slot
            .checked_add(*amount)
            .ok_or_else(|| "clv sum overflows".to_string())?;
    }
    Ok(totals)
}

/// Most years a projection may look ahead.
pub const MAX_YEARS: u32 = 20;
/// Highest margin, retention or discount, in basis points (100%).
pub const MAX_BPS: u32 = 10_000;

/// `amount × bps / 10000`, rounded half to even (CRM-R60).
///
/// # Errors
///
/// On arithmetic overflow.
pub fn margin(amount_minor: i64, bps: u32) -> Result<i64, String> {
    let scaled = super::fx::divide(i128::from(amount_minor) * i128::from(bps), 10_000);
    i64::try_from(scaled).map_err(|_| "margin overflows".to_string())
}

/// The share of the year before that came again in the latest year, in basis points of it,
/// rounded half to even and held at 100% (CRM-R61): history is never read as growth. `None`
/// when nothing was won in the year before, because then there is nothing to measure against.
///
/// # Errors
///
/// On arithmetic overflow.
pub fn retention_from_history(current_minor: i64, prior_minor: i64) -> Result<Option<u32>, String> {
    if prior_minor <= 0 {
        return Ok(None);
    }
    let scaled = super::fx::divide(
        i128::from(current_minor.max(0)) * i128::from(MAX_BPS),
        i128::from(prior_minor),
    );
    Ok(Some(
        u32::try_from(scaled.min(i128::from(MAX_BPS)))
            .map_err(|_| "retention overflows".to_string())?,
    ))
}

/// What a year's revenue is still expected to bring over the next `years`, discounted
/// (CRM-R60): year `t` is year `t-1` times `retention / (1 + discount)`, each year rounded half
/// to even, and the years are added. Integer arithmetic, no floating point.
///
/// # Errors
///
/// On arithmetic overflow.
pub fn projected(
    annual_minor: i64,
    retention_bps: u32,
    discount_bps: u32,
    years: u32,
) -> Result<i64, String> {
    let (mut year, mut total) = (i128::from(annual_minor), 0_i128);
    let denominator = i128::from(MAX_BPS) + i128::from(discount_bps);
    for _ in 0..years {
        year = super::fx::divide(year * i128::from(retention_bps), denominator);
        total += year;
    }
    i64::try_from(total).map_err(|_| "projection overflows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deal(amount: i64, currency: &str, p: i32) -> OpenDeal {
        OpenDeal {
            amount_minor: amount,
            currency: currency.to_string(),
            probability_percent: p,
        }
    }

    /// Retention is the latest year over the year before, held at 100%, with no basis when the
    /// year before is empty (CRM-R61).
    #[test]
    fn retention_comes_from_the_year_before() {
        assert_eq!(retention_from_history(6_000, 8_000).unwrap(), Some(7_500));
        assert_eq!(retention_from_history(1, 3).unwrap(), Some(3_333));
        assert_eq!(retention_from_history(20_000, 8_000).unwrap(), Some(10_000));
        assert_eq!(retention_from_history(0, 8_000).unwrap(), Some(0));
        assert_eq!(retention_from_history(5, 0).unwrap(), None);
    }

    /// Weighted sums group per currency and never merge currencies.
    #[test]
    fn forecast_groups_per_currency() {
        let totals = forecast_by_currency(&[
            deal(100_000, "GBP", 50),
            deal(200_000, "GBP", 25),
            deal(300_000, "USD", 100),
        ])
        .unwrap();
        assert_eq!(totals["GBP"], 100_000);
        assert_eq!(totals["USD"], 300_000);
        assert_eq!(totals.len(), 2);
        assert!(forecast_by_currency(&[deal(i64::MAX, "GBP", 99)]).is_err());
    }

    /// Ratios carry their parts; zero denominators yield None.
    #[test]
    fn honest_ratios() {
        let rate = win_rate(3, 1);
        assert_eq!(rate.numerator, 3);
        assert_eq!(rate.denominator, 4);
        assert!((rate.value.unwrap() - 0.75).abs() < f64::EPSILON);
        assert_eq!(win_rate(0, 0).value, None);
    }

    /// ROI: profitable, loss-making, and free campaigns.
    #[test]
    fn roi_shapes() {
        let profitable = roi(300_000, 100_000);
        assert!((profitable.value.unwrap() - 2.0).abs() < f64::EPSILON);
        let loss = roi(50_000, 100_000);
        assert!((loss.value.unwrap() + 0.5).abs() < f64::EPSILON);
        let free = roi(50_000, 0);
        assert_eq!(free.value, None);
        assert_eq!(free.numerator, 50_000);
    }

    /// CLV sums per currency and refuses overflow.
    #[test]
    fn clv_sums() {
        let totals =
            clv_by_currency(&[(100, "GBP".into()), (200, "GBP".into()), (5, "EUR".into())])
                .unwrap();
        assert_eq!(totals["GBP"], 300);
        assert_eq!(totals["EUR"], 5);
        assert!(clv_by_currency(&[(i64::MAX, "GBP".into()), (1, "GBP".into())]).is_err());
    }

    /// Margin and projection round half to even and discount exactly.
    #[test]
    fn margin_and_projection() {
        assert_eq!(margin(10_000, 2_500), Ok(2_500));
        assert_eq!(margin(1, 5_000), Ok(0)); // 0.5 -> 0
        assert_eq!(margin(3, 5_000), Ok(2)); // 1.5 -> 2
        // No discount and full retention: the year again, each year.
        assert_eq!(projected(1_000, 10_000, 0, 3), Ok(3_000));
        // 50% retention: 500 + 250 + 125.
        assert_eq!(projected(1_000, 5_000, 0, 3), Ok(875));
        // 10% discount on full retention: 1000/1.1 = 909, then 909/1.1 = 826.
        assert_eq!(projected(1_000, 10_000, 1_000, 2), Ok(909 + 826));
        assert_eq!(projected(1_000, 0, 0, 5), Ok(0));
        assert!(projected(i64::MAX, 10_000, 0, 2).is_err());
    }
}
