//! Multi-touch attribution (CRM-R49): how the amount of a won deal is shared among the campaigns
//! that touched the buyer.
//!
//! Pure and DB-free. The shares of one deal add up to its amount exactly, to the minor unit.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Shortest window, in days.
pub const MIN_WINDOW_DAYS: i64 = 1;
/// Longest window, in days.
pub const MAX_WINDOW_DAYS: i64 = 365;
/// The window when none is asked for.
pub const DEFAULT_WINDOW_DAYS: i64 = 90;

/// How an amount is shared among touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// The whole amount to the earliest touch.
    First,
    /// The whole amount to the latest touch.
    Last,
    /// Equal shares, one per touch.
    Linear,
}

impl Model {
    /// The model called `name`.
    ///
    /// # Errors
    ///
    /// The reason, when there is no such model.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "first" => Ok(Self::First),
            "last" => Ok(Self::Last),
            "linear" => Ok(Self::Linear),
            _ => Err("the model is one of first, last, linear".to_string()),
        }
    }

    /// Its name, as asked for.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Last => "last",
            Self::Linear => "linear",
        }
    }
}

/// The window asked for, or the default.
///
/// # Errors
///
/// The reason, when it is outside 1 to 365 days.
pub fn window_days(asked: Option<i64>) -> Result<i64, String> {
    let days = asked.unwrap_or(DEFAULT_WINDOW_DAYS);
    if (MIN_WINDOW_DAYS..=MAX_WINDOW_DAYS).contains(&days) {
        Ok(days)
    } else {
        Err(format!(
            "the window is {MIN_WINDOW_DAYS} to {MAX_WINDOW_DAYS} days"
        ))
    }
}

/// One contact a campaign had with the buyer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Touch {
    /// The campaign.
    pub campaign: Uuid,
    /// When.
    pub at: DateTime<Utc>,
}

/// The credit each campaign earns for `amount_minor`, in campaign order.
///
/// The shares add up to `amount_minor` exactly: under `Linear`, the minor units left over by the
/// division go one each to the earliest touches. Touches are taken in time order (then campaign
/// id), so the answer does not depend on the order they are given in. No touches, or an amount
/// below zero, earn nothing.
#[must_use]
pub fn credit(model: Model, amount_minor: i64, touches: &[Touch]) -> Vec<(Uuid, i64)> {
    if touches.is_empty() || amount_minor < 0 {
        return Vec::new();
    }
    let mut ordered = touches.to_vec();
    ordered.sort_by_key(|t| (t.at, t.campaign));
    let mut shares: BTreeMap<Uuid, i64> = BTreeMap::new();
    match model {
        Model::First => *shares.entry(ordered[0].campaign).or_default() += amount_minor,
        Model::Last => {
            *shares
                .entry(ordered[ordered.len() - 1].campaign)
                .or_default() += amount_minor;
        }
        Model::Linear => {
            let n = i64::try_from(ordered.len()).unwrap_or(i64::MAX);
            let (each, left) = (amount_minor / n, amount_minor % n);
            for (i, touch) in ordered.iter().enumerate() {
                let extra = i64::from(i64::try_from(i).unwrap_or(i64::MAX) < left);
                *shares.entry(touch.campaign).or_default() += each + extra;
            }
        }
    }
    shares.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn touch(campaign: u128, day: u32) -> Touch {
        Touch {
            campaign: id(campaign),
            at: Utc.with_ymd_and_hms(2026, 6, day, 9, 0, 0).unwrap(),
        }
    }

    fn total(shares: &[(Uuid, i64)]) -> i64 {
        shares.iter().map(|(_, v)| v).sum()
    }

    #[test]
    fn first_and_last_give_everything_to_one_touch() {
        let touches = [touch(2, 10), touch(1, 3), touch(3, 20)];
        assert_eq!(credit(Model::First, 1000, &touches), [(id(1), 1000)]);
        assert_eq!(credit(Model::Last, 1000, &touches), [(id(3), 1000)]);
    }

    #[test]
    fn linear_splits_equally_and_by_touch_not_by_campaign() {
        // Campaign 1 touched twice, campaign 2 once: two thirds and one third.
        let touches = [touch(1, 3), touch(2, 5), touch(1, 9)];
        assert_eq!(
            credit(Model::Linear, 900, &touches),
            [(id(1), 600), (id(2), 300)]
        );
    }

    #[test]
    fn the_shares_always_add_up_to_the_amount() {
        let touches = [touch(1, 3), touch(2, 5), touch(3, 9)];
        for amount in [0, 1, 2, 3, 4, 100, 101, 102, 9_999_999] {
            for model in [Model::First, Model::Last, Model::Linear] {
                assert_eq!(total(&credit(model, amount, &touches)), amount, "{amount}");
            }
        }
    }

    #[test]
    fn the_left_over_minor_units_go_to_the_earliest_touches() {
        // 100 over three touches: 34, 33, 33, the extra unit to the earliest.
        let touches = [touch(3, 9), touch(1, 3), touch(2, 5)];
        assert_eq!(
            credit(Model::Linear, 100, &touches),
            [(id(1), 34), (id(2), 33), (id(3), 33)]
        );
        // 101 over three: 34, 34, 33.
        assert_eq!(
            credit(Model::Linear, 101, &touches),
            [(id(1), 34), (id(2), 34), (id(3), 33)]
        );
    }

    #[test]
    fn the_order_the_touches_arrive_in_changes_nothing() {
        let a = [touch(1, 3), touch(2, 5), touch(3, 9)];
        let b = [touch(3, 9), touch(1, 3), touch(2, 5)];
        for model in [Model::First, Model::Last, Model::Linear] {
            assert_eq!(credit(model, 1001, &a), credit(model, 1001, &b));
        }
    }

    #[test]
    fn touches_at_the_same_moment_tie_by_campaign() {
        let touches = [touch(5, 4), touch(2, 4)];
        assert_eq!(credit(Model::First, 10, &touches), [(id(2), 10)]);
        assert_eq!(credit(Model::Last, 10, &touches), [(id(5), 10)]);
    }

    #[test]
    fn nothing_to_credit_earns_nothing() {
        assert!(credit(Model::Linear, 100, &[]).is_empty());
        assert!(credit(Model::First, -5, &[touch(1, 1)]).is_empty());
        assert_eq!(credit(Model::Linear, 0, &[touch(1, 1)]), [(id(1), 0)]);
    }

    #[test]
    fn a_model_is_known_by_its_name() {
        for model in [Model::First, Model::Last, Model::Linear] {
            assert_eq!(Model::parse(model.name()), Ok(model));
        }
        assert!(Model::parse("time_decay").is_err());
        assert!(Model::parse("").is_err());
    }

    #[test]
    fn the_window_is_one_to_a_year_and_defaults_to_ninety_days() {
        assert_eq!(window_days(None), Ok(90));
        assert_eq!(window_days(Some(1)), Ok(1));
        assert_eq!(window_days(Some(365)), Ok(365));
        assert!(window_days(Some(0)).is_err());
        assert!(window_days(Some(366)).is_err());
        assert!(window_days(Some(-3)).is_err());
    }
}
