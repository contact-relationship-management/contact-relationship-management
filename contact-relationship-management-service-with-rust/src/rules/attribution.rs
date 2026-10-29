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
/// Under `TimeDecay`, a touch's weight halves for every this many days before the deal closed,
/// unless the caller chooses another (CRM-R89).
pub const HALF_LIFE_DAYS: i64 = 7;
/// Shortest half-life a caller may choose, in days. With the longest window (365 days) it makes at
/// most 92 halvings, inside the 100 a weight is held to, so a touch never weighs nothing.
pub const MIN_HALF_LIFE_DAYS: i64 = 4;
/// Longest half-life a caller may choose, in days.
pub const MAX_HALF_LIFE_DAYS: i64 = 90;
/// Longest "an open this soon after sending is a machine" a caller may set, in seconds (one day).
pub const MAX_MIN_OPEN_SECONDS: i64 = 86_400;
/// The share, in percent, the position model gives each of the first and last touches by default.
pub const END_PERCENT: i64 = 40;
/// The least a position model may give each end (CRM-R90).
pub const MIN_END_PERCENT: i64 = 1;
/// The most: two ends at 49% leave the middle touches 2%, so they are never given nothing.
pub const MAX_END_PERCENT: i64 = 49;

/// How an amount is shared among touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// The whole amount to the earliest touch.
    First,
    /// The whole amount to the latest touch.
    Last,
    /// Equal shares, one per touch.
    Linear,
    /// Weight halves for every [`HALF_LIFE_DAYS`] between the touch and the close, so recent
    /// touches earn more.
    TimeDecay,
    /// 40% to the first touch, 40% to the last, 20% shared equally among those between (one touch
    /// takes everything, two take half each).
    Position,
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
            "time_decay" => Ok(Self::TimeDecay),
            "position" => Ok(Self::Position),
            _ => Err("the model is one of first, last, linear, time_decay, position".to_string()),
        }
    }

    /// Its name, as asked for.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Last => "last",
            Self::Linear => "linear",
            Self::TimeDecay => "time_decay",
            Self::Position => "position",
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

/// The half-life asked for, or the default.
///
/// # Errors
///
/// The reason, when it is outside [`MIN_HALF_LIFE_DAYS`] to [`MAX_HALF_LIFE_DAYS`].
pub fn half_life_days(asked: Option<i64>) -> Result<i64, String> {
    let days = asked.unwrap_or(HALF_LIFE_DAYS);
    if (MIN_HALF_LIFE_DAYS..=MAX_HALF_LIFE_DAYS).contains(&days) {
        Ok(days)
    } else {
        Err(format!(
            "the half-life is {MIN_HALF_LIFE_DAYS} to {MAX_HALF_LIFE_DAYS} days"
        ))
    }
}

/// The share each end of the position model gets, or the default.
///
/// # Errors
///
/// The reason, when it is outside [`MIN_END_PERCENT`] to [`MAX_END_PERCENT`].
pub fn end_percent(asked: Option<i64>) -> Result<i64, String> {
    let percent = asked.unwrap_or(END_PERCENT);
    if (MIN_END_PERCENT..=MAX_END_PERCENT).contains(&percent) {
        Ok(percent)
    } else {
        Err(format!(
            "the share is {MIN_END_PERCENT} to {MAX_END_PERCENT} percent"
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

/// Share `amount` among `weights` in proportion, exactly: each gets its floor, and the minor units
/// left over go one each to the largest remainders, the earliest first on a tie. Weights are not
/// all zero (one is taken as one if they are).
fn apportion(amount: i64, weights: &[i128]) -> Vec<i64> {
    let total: i128 = weights.iter().sum();
    if total <= 0 {
        return apportion(amount, &vec![1; weights.len()]);
    }
    let amount = i128::from(amount);
    let mut shares: Vec<i128> = weights.iter().map(|w| amount * w / total).collect();
    let mut left = amount - shares.iter().sum::<i128>();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(amount * weights[i] % total), i));
    for i in order {
        if left == 0 {
            break;
        }
        shares[i] += 1;
        left -= 1;
    }
    shares
        .into_iter()
        .map(|s| i64::try_from(s).unwrap_or(i64::MAX))
        .collect()
}

/// A touch's weight under time decay: `2^-(age / half-life)`, held as an integer. Between whole
/// half-lives it falls in a straight line (so it is exact integer arithmetic, within about 6% of
/// the true curve); a touch from after the close counts as having just happened.
fn decay_weight(age_seconds: i64, half_life_days: i64) -> i128 {
    let half_life = i128::from(half_life_days) * 86_400;
    let age = i128::from(age_seconds.max(0));
    let (whole, part) = (age / half_life, age % half_life);
    // 2^100 leaves room for the 92 halvings of the longest window and the shortest half-life.
    let base: i128 = 1 << 100;
    let at_start = base >> whole.min(100);
    at_start * (2 * half_life - part) / (2 * half_life)
}

/// The credit each campaign earns for `amount_minor` of a deal closed at `closed_at`, in campaign
/// order.
///
/// The shares add up to `amount_minor` exactly: the minor units left over by a division go one each
/// to the largest remainders, the earliest touch first on a tie. Touches are taken in time order
/// (then campaign id), so the answer does not depend on the order they are given in. No touches, or
/// an amount below zero, earn nothing.
#[must_use]
pub fn credit(
    model: Model,
    amount_minor: i64,
    touches: &[Touch],
    closed_at: DateTime<Utc>,
) -> Vec<(Uuid, i64)> {
    credit_with(model, amount_minor, touches, closed_at, HALF_LIFE_DAYS)
}

/// [`credit`] with the time-decay half-life the caller chose (CRM-R89): [`HALF_LIFE_DAYS`] is the
/// default, and the half-life matters only to [`Model::TimeDecay`].
#[must_use]
pub fn credit_with(
    model: Model,
    amount_minor: i64,
    touches: &[Touch],
    closed_at: DateTime<Utc>,
    half_life_days: i64,
) -> Vec<(Uuid, i64)> {
    credit_tuned(
        model,
        amount_minor,
        touches,
        closed_at,
        half_life_days,
        END_PERCENT,
    )
}

/// [`credit_with`] with the position model's end share the caller chose (CRM-R90): each of the
/// first and last touches gets `end_percent` percent and the middle touches share the rest. It
/// matters only to [`Model::Position`].
#[must_use]
pub fn credit_tuned(
    model: Model,
    amount_minor: i64,
    touches: &[Touch],
    closed_at: DateTime<Utc>,
    half_life_days: i64,
    end_percent: i64,
) -> Vec<(Uuid, i64)> {
    if touches.is_empty() || amount_minor < 0 {
        return Vec::new();
    }
    let mut ordered = touches.to_vec();
    ordered.sort_by_key(|t| (t.at, t.campaign));
    let n = ordered.len();
    let weights: Vec<i128> = match model {
        Model::First => (0..n).map(|i| i128::from(i == 0)).collect(),
        Model::Last => (0..n).map(|i| i128::from(i == n - 1)).collect(),
        Model::Linear => vec![1; n],
        Model::TimeDecay => ordered
            .iter()
            .map(|t| decay_weight((closed_at - t.at).num_seconds(), half_life_days))
            .collect(),
        Model::Position => match n {
            1 | 2 => vec![1; n],
            _ => {
                // end : end : (100 - 2 end) shared by the middle, in whole numbers.
                let middle = i128::try_from(n - 2).unwrap_or(i128::MAX);
                let end = i128::from(end_percent);
                (0..n)
                    .map(|i| {
                        if i == 0 || i == n - 1 {
                            end * middle
                        } else {
                            100 - 2 * end
                        }
                    })
                    .collect()
            }
        },
    };
    let mut shares: BTreeMap<Uuid, i64> = BTreeMap::new();
    for ((touch, weight), share) in ordered
        .iter()
        .zip(&weights)
        .zip(apportion(amount_minor, &weights))
    {
        // A touch with no weight takes no part (first and last give all to one), but one that
        // has weight is listed even when its share rounds to nothing.
        if *weight > 0 {
            *shares.entry(touch.campaign).or_default() += share;
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

    const ALL: [Model; 5] = [
        Model::First,
        Model::Last,
        Model::Linear,
        Model::TimeDecay,
        Model::Position,
    ];

    /// The deal closed on 30 June at 09:00; the touches are earlier in June.
    fn closed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 30, 9, 0, 0).unwrap()
    }

    fn total(shares: &[(Uuid, i64)]) -> i64 {
        shares.iter().map(|(_, v)| v).sum()
    }

    #[test]
    fn first_and_last_give_everything_to_one_touch() {
        let touches = [touch(2, 10), touch(1, 3), touch(3, 20)];
        assert_eq!(
            credit(Model::First, 1000, &touches, closed()),
            [(id(1), 1000)]
        );
        assert_eq!(
            credit(Model::Last, 1000, &touches, closed()),
            [(id(3), 1000)]
        );
    }

    #[test]
    fn linear_splits_equally_and_by_touch_not_by_campaign() {
        // Campaign 1 touched twice, campaign 2 once: two thirds and one third.
        let touches = [touch(1, 3), touch(2, 5), touch(1, 9)];
        assert_eq!(
            credit(Model::Linear, 900, &touches, closed()),
            [(id(1), 600), (id(2), 300)]
        );
    }

    #[test]
    fn the_shares_always_add_up_to_the_amount() {
        let touches = [touch(1, 3), touch(2, 5), touch(3, 9)];
        for amount in [0, 1, 2, 3, 4, 100, 101, 102, 9_999_999] {
            for model in ALL {
                assert_eq!(
                    total(&credit(model, amount, &touches, closed())),
                    amount,
                    "{amount}"
                );
            }
        }
    }

    #[test]
    fn the_left_over_minor_units_go_to_the_earliest_touches() {
        // 100 over three touches: 34, 33, 33, the extra unit to the earliest.
        let touches = [touch(3, 9), touch(1, 3), touch(2, 5)];
        assert_eq!(
            credit(Model::Linear, 100, &touches, closed()),
            [(id(1), 34), (id(2), 33), (id(3), 33)]
        );
        // 101 over three: 34, 34, 33.
        assert_eq!(
            credit(Model::Linear, 101, &touches, closed()),
            [(id(1), 34), (id(2), 34), (id(3), 33)]
        );
    }

    #[test]
    fn the_order_the_touches_arrive_in_changes_nothing() {
        let a = [touch(1, 3), touch(2, 5), touch(3, 9)];
        let b = [touch(3, 9), touch(1, 3), touch(2, 5)];
        for model in ALL {
            assert_eq!(
                credit(model, 1001, &a, closed()),
                credit(model, 1001, &b, closed())
            );
        }
    }

    #[test]
    fn touches_at_the_same_moment_tie_by_campaign() {
        let touches = [touch(5, 4), touch(2, 4)];
        assert_eq!(credit(Model::First, 10, &touches, closed()), [(id(2), 10)]);
        assert_eq!(credit(Model::Last, 10, &touches, closed()), [(id(5), 10)]);
    }

    #[test]
    fn nothing_to_credit_earns_nothing() {
        assert!(credit(Model::Linear, 100, &[], closed()).is_empty());
        assert!(credit(Model::First, -5, &[touch(1, 1)], closed()).is_empty());
        assert_eq!(
            credit(Model::Linear, 0, &[touch(1, 1)], closed()),
            [(id(1), 0)]
        );
    }

    #[test]
    fn a_model_is_known_by_its_name() {
        for model in ALL {
            assert_eq!(Model::parse(model.name()), Ok(model));
        }
        assert!(Model::parse("time-decay").is_err());
        assert!(Model::parse("U").is_err());
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

    fn credited(shares: &[(Uuid, i64)], campaign: u128) -> i64 {
        shares
            .iter()
            .find(|(c, _)| *c == id(campaign))
            .map_or(0, |(_, v)| *v)
    }

    #[test]
    fn a_chosen_half_life_changes_time_decay_only_and_the_default_is_unchanged() {
        let closed = closed();
        let at = |days: i64, campaign: u128| Touch {
            campaign: id(campaign),
            at: closed - chrono::Duration::days(days),
        };
        let touches = [at(14, 1), at(0, 2)];
        // Seven days: 14 days is two half-lives, so 1 : 4 of 10,000.
        assert_eq!(
            credit_with(Model::TimeDecay, 10_000, &touches, closed, 7),
            [(id(1), 2_000), (id(2), 8_000)]
        );
        assert_eq!(
            credit(Model::TimeDecay, 10_000, &touches, closed),
            credit_with(Model::TimeDecay, 10_000, &touches, closed, HALF_LIFE_DAYS)
        );
        // Fourteen days: one half-life, so 1 : 2.
        assert_eq!(
            credit_with(Model::TimeDecay, 9_000, &touches, closed, 14),
            [(id(1), 3_000), (id(2), 6_000)]
        );
        // The other models do not look at it.
        for model in [Model::First, Model::Last, Model::Linear, Model::Position] {
            assert_eq!(
                credit_with(model, 10_000, &touches, closed, 90),
                credit(model, 10_000, &touches, closed)
            );
        }
        // The longest window at the shortest half-life still gives every touch some weight.
        let old = [at(365, 1), at(364, 2)];
        let shares = credit_with(Model::TimeDecay, 10_000, &old, closed, MIN_HALF_LIFE_DAYS);
        assert_eq!(shares.iter().map(|s| s.1).sum::<i64>(), 10_000);
        assert_eq!(shares.len(), 2, "both are listed");
    }

    #[test]
    fn the_half_life_is_four_to_ninety_days_and_seven_when_not_asked() {
        assert_eq!(half_life_days(None), Ok(7));
        assert_eq!(half_life_days(Some(4)), Ok(4));
        assert_eq!(half_life_days(Some(90)), Ok(90));
        assert!(half_life_days(Some(3)).is_err() && half_life_days(Some(91)).is_err());
        assert!(half_life_days(Some(0)).is_err() && half_life_days(Some(-7)).is_err());
    }

    #[test]
    fn time_decay_gives_recent_touches_more_and_halves_every_half_life() {
        // Touches exactly 0, 7 and 14 days before the close weigh 4 : 2 : 1 of 7.
        let closed = closed();
        let at = |days: i64, campaign: u128| Touch {
            campaign: id(campaign),
            at: closed - chrono::Duration::days(days),
        };
        let touches = [at(14, 1), at(7, 2), at(0, 3)];
        let shares = credit(Model::TimeDecay, 7000, &touches, closed);
        assert_eq!(credited(&shares, 3), 4000);
        assert_eq!(credited(&shares, 2), 2000);
        assert_eq!(credited(&shares, 1), 1000);
        // Between half-lives it falls steadily: 3.5 days is between 1 and a half.
        let steady = credit(
            Model::TimeDecay,
            10_000,
            &[at(0, 1), at(3, 2), at(7, 3)],
            closed,
        );
        assert!(credited(&steady, 1) > credited(&steady, 2));
        assert!(credited(&steady, 2) > credited(&steady, 3));
        // One touch takes everything however old it is.
        assert_eq!(
            credit(Model::TimeDecay, 500, &[at(300, 9)], closed),
            [(id(9), 500)]
        );
        // A touch after the close counts as just happened, not as a negative age.
        let after = Touch {
            campaign: id(4),
            at: closed + chrono::Duration::days(2),
        };
        assert_eq!(
            credit(Model::TimeDecay, 10, &[after], closed),
            [(id(4), 10)]
        );
    }

    #[test]
    fn a_chosen_end_share_moves_position_only_and_the_default_is_forty() {
        let touches = [touch(1, 3), touch(2, 10), touch(3, 20)];
        let at = |end| credit_tuned(Model::Position, 10_000, &touches, closed(), 7, end);
        assert_eq!(at(40), credit(Model::Position, 10_000, &touches, closed()));
        assert_eq!(at(30), [(id(1), 3_000), (id(2), 4_000), (id(3), 3_000)]);
        assert_eq!(at(49), [(id(1), 4_900), (id(2), 200), (id(3), 4_900)]);
        assert_eq!(
            credit_tuned(Model::Linear, 9_000, &touches, closed(), 7, 49),
            credit(Model::Linear, 9_000, &touches, closed()),
            "other models do not read it"
        );
        for end in MIN_END_PERCENT..=MAX_END_PERCENT {
            assert_eq!(total(&at(end)), 10_000, "exact at {end}");
        }
        assert_eq!(end_percent(None), Ok(40));
        assert!(end_percent(Some(0)).is_err() && end_percent(Some(50)).is_err());
    }

    #[test]
    fn position_gives_forty_forty_and_twenty_shared_between() {
        let one = [touch(1, 3)];
        assert_eq!(
            credit(Model::Position, 1000, &one, closed()),
            [(id(1), 1000)]
        );
        let two = [touch(1, 3), touch(2, 5)];
        assert_eq!(
            credit(Model::Position, 1000, &two, closed()),
            [(id(1), 500), (id(2), 500)]
        );
        let three = [touch(1, 3), touch(2, 5), touch(3, 9)];
        let shares = credit(Model::Position, 1000, &three, closed());
        assert_eq!(
            shares,
            [(id(1), 400), (id(2), 200), (id(3), 400)],
            "40, 20, 40"
        );
        // Four touches: the 20% is shared by the two in the middle.
        let four = [touch(1, 3), touch(2, 5), touch(3, 9), touch(4, 12)];
        let shares = credit(Model::Position, 1000, &four, closed());
        assert_eq!(
            shares,
            [(id(1), 400), (id(2), 100), (id(3), 100), (id(4), 400)]
        );
    }

    #[test]
    fn a_campaign_that_touched_twice_earns_both_shares_under_every_model() {
        let touches = [touch(1, 3), touch(2, 5), touch(1, 9)];
        for model in ALL {
            let shares = credit(model, 999, &touches, closed());
            assert_eq!(total(&shares), 999, "{model:?}");
            assert_eq!(
                shares.len(),
                if matches!(model, Model::First | Model::Last) {
                    1
                } else {
                    2
                }
            );
        }
    }

    #[test]
    fn the_leftover_minor_units_go_to_the_largest_remainders() {
        // 100 over a 40:20:40 split of three is exact; 101 leaves one unit, to the first on a tie.
        let touches = [touch(1, 3), touch(2, 5), touch(3, 9)];
        let shares = credit(Model::Position, 101, &touches, closed());
        assert_eq!(total(&shares), 101);
        assert_eq!(credited(&shares, 1), 41);
        assert_eq!(credited(&shares, 3), 40);
    }
}
