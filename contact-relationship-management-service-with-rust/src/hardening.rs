//! Start-up safety checks for regulated deployments (CRM-T30).
//!
//! The family default is `CRM_REQUIRE_AUTH` off so a developer can run the
//! service with no identity provider. That default must never reach
//! production unnoticed, so in the `production` environment the service
//! refuses to boot with enforcement off unless the operator says so
//! explicitly.

use crate::auth::parse_bool;

/// Why a production start was refused.
pub const INSECURE_REFUSAL: &str = "refusing to start in production with CRM_REQUIRE_AUTH off; \
    set CRM_REQUIRE_AUTH=1 (see spec/auth.md, activation runbook) or, to accept the risk \
    knowingly, CRM_ALLOW_INSECURE=1";

/// Decide whether a start is allowed. `Ok(Some(warning))` means allowed but
/// the operator overrode the safe default; `Ok(None)` means fully safe.
///
/// # Errors
///
/// [`INSECURE_REFUSAL`] when `production` is true, enforcement is off, and
/// the explicit override is absent.
pub fn production_start_check(
    production: bool,
    require_auth: bool,
    allow_insecure: bool,
) -> Result<Option<&'static str>, &'static str> {
    if !production || require_auth {
        return Ok(None);
    }
    if allow_insecure {
        return Ok(Some(
            "CRM_ALLOW_INSECURE is set: serving production traffic with authentication OFF",
        ));
    }
    Err(INSECURE_REFUSAL)
}

/// [`production_start_check`] over the process environment.
///
/// # Errors
///
/// As [`production_start_check`].
pub fn check_process(production: bool) -> Result<(), &'static str> {
    let allow = parse_bool(&std::env::var("CRM_ALLOW_INSECURE").unwrap_or_default());
    if let Some(warning) = production_start_check(production, crate::auth::require_auth(), allow)? {
        tracing::error!("{warning}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_production_is_never_blocked() {
        assert_eq!(production_start_check(false, false, false), Ok(None));
    }

    #[test]
    fn production_with_auth_is_safe() {
        assert_eq!(production_start_check(true, true, false), Ok(None));
    }

    #[test]
    fn production_without_auth_is_refused() {
        assert_eq!(
            production_start_check(true, false, false),
            Err(INSECURE_REFUSAL)
        );
    }

    #[test]
    fn explicit_override_allows_with_a_warning() {
        assert!(matches!(
            production_start_check(true, false, true),
            Ok(Some(_))
        ));
    }
}
