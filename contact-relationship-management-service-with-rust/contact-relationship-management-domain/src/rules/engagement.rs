//! Pure rules for the stakeholder-engagement features: declared
//! stakeholder typing, recorded sentiment, and the
//! innovation-partnership lifecycle. Declared means declared — no
//! rule here infers a role, a grid position, or a sentiment.

/// Stakeholder roles a contact or account may be **declared** as.
pub const STAKEHOLDER_ROLES: &[&str] = &[
    "customer",
    "partner",
    "regulator",
    "sponsor",
    "community",
    "media",
    "member",
];

/// Recorded interaction sentiments.
pub const SENTIMENTS: &[&str] = &["positive", "neutral", "negative"];

/// Innovation-partnership kinds.
pub const PARTNERSHIP_KINDS: &[&str] = &["university", "startup", "vendor", "accelerator", "other"];

/// Partnership stages, in order.
pub const PARTNERSHIP_STAGES: &[&str] = &["scouting", "pilot", "scaled", "retired"];

/// Membership statuses.
pub const MEMBERSHIP_STATUSES: &[&str] = &["active", "lapsed"];

/// A power–interest score must sit on the declared 1–5 scale.
#[must_use]
pub fn valid_grid_score(score: i32) -> bool {
    (1..=5).contains(&score)
}

/// Why a partnership stage change is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PartnershipError {
    /// The target is not a stage.
    #[error("unknown stage `{to}` (stages: {PARTNERSHIP_STAGES:?})")]
    UnknownStage {
        /// What was asked for.
        to: String,
    },
    /// A retired partnership is final.
    #[error("a retired partnership does not transition")]
    Retired,
    /// Not one step forward, and not a retirement.
    #[error("illegal transition `{from}` → `{to}` (forward one step, or retire)")]
    Illegal {
        /// Where it is.
        from: String,
        /// Where it was asked to go.
        to: String,
    },
}

/// The partnership lifecycle: forward one step at a time
/// (scouting → pilot → scaled), and `retired` reachable from any
/// live stage. Everything else refuses with a reason.
///
/// # Errors
///
/// [`PartnershipError`] naming the refusal.
pub fn partnership_transition(current: &str, to: &str) -> Result<(), PartnershipError> {
    if !PARTNERSHIP_STAGES.contains(&to) {
        return Err(PartnershipError::UnknownStage { to: to.to_string() });
    }
    if current == "retired" {
        return Err(PartnershipError::Retired);
    }
    if to == "retired" {
        return Ok(());
    }
    let current_idx = PARTNERSHIP_STAGES.iter().position(|s| *s == current);
    let to_idx = PARTNERSHIP_STAGES.iter().position(|s| *s == to);
    match (current_idx, to_idx) {
        (Some(c), Some(t)) if t == c + 1 => Ok(()),
        _ => Err(PartnershipError::Illegal {
            from: current.to_string(),
            to: to.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_scores_are_one_to_five() {
        assert!(valid_grid_score(1));
        assert!(valid_grid_score(5));
        assert!(!valid_grid_score(0));
        assert!(!valid_grid_score(6));
    }

    #[test]
    fn partnership_lifecycle_is_forward_or_retire() {
        assert!(partnership_transition("scouting", "pilot").is_ok());
        assert!(partnership_transition("pilot", "scaled").is_ok());
        assert!(partnership_transition("scouting", "retired").is_ok());
        assert!(
            matches!(
                partnership_transition("scouting", "scaled"),
                Err(PartnershipError::Illegal { .. })
            ),
            "no skipping"
        );
        assert!(
            partnership_transition("pilot", "scouting").is_err(),
            "no going back"
        );
        assert!(
            partnership_transition("retired", "pilot") == Err(PartnershipError::Retired),
            "retired is terminal"
        );
        assert!(
            matches!(
                partnership_transition("pilot", "sideways"),
                Err(PartnershipError::UnknownStage { .. })
            ),
            "unknown refused"
        );
    }
}
