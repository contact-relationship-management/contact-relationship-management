//! Start-up safety checks for regulated deployments (CRM-T30).
//!
//! The family default is `CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH` off so a developer can run the
//! service with no identity provider. That default must never reach
//! production unnoticed, so in the `production` environment the service
//! refuses to boot with enforcement off unless the operator says so
//! explicitly.

/// Why a production start was refused.
pub const INSECURE_REFUSAL: &str = "refusing to start in production with CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH off; \
    set CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH=1 (see spec/auth.md, activation runbook) or, to accept the risk \
    knowingly, CONTACT_RELATIONSHIP_MANAGEMENT_ALLOW_INSECURE=1";

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
            "CONTACT_RELATIONSHIP_MANAGEMENT_ALLOW_INSECURE is set: serving production traffic with authentication OFF",
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
    let allow = crate::settings::current().allow_insecure;
    if let Some(warning) = production_start_check(production, crate::auth::require_auth(), allow)? {
        tracing::error!("{warning}");
    }
    Ok(())
}

/// Why a start was refused when keys were supplied but none could be loaded.
pub const UNUSABLE_FIELD_KEYS: &str = "CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS or CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS_FILE is set \
    but no keyring could be loaded from it (an invalid key list or an unreadable file); refusing to start rather than \
    store plaintext";

/// Why a start was refused when encryption is required and no keys were supplied.
pub const MISSING_FIELD_KEYS: &str = "CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_FIELD_ENCRYPTION is on but no valid \
    CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS / CONTACT_RELATIONSHIP_MANAGEMENT_FIELD_KEYS_FILE is configured";

/// Whether the field-encryption keys are as the operator asked (CRM-T41). Keys that were
/// `supplied` but did not give a keyring always stop the start: the operator meant to encrypt,
/// and a typo must not turn into plaintext at rest. With
/// `CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_FIELD_ENCRYPTION` on (`required`), no keys at all also
/// stop it.
///
/// # Errors
///
/// [`UNUSABLE_FIELD_KEYS`] or [`MISSING_FIELD_KEYS`].
pub fn field_encryption_check(
    required: bool,
    supplied: bool,
    configured: bool,
) -> Result<(), &'static str> {
    match (configured, supplied, required) {
        (true, _, _) | (false, false, false) => Ok(()),
        (false, true, _) => Err(UNUSABLE_FIELD_KEYS),
        (false, false, true) => Err(MISSING_FIELD_KEYS),
    }
}

/// [`field_encryption_check`] over the process environment and keyring.
///
/// # Errors
///
/// As [`field_encryption_check`].
pub fn check_field_encryption() -> Result<(), &'static str> {
    let settings = crate::settings::current();
    field_encryption_check(
        settings.require_field_encryption,
        settings.field_keys.is_some()
            || settings
                .field_keys_file
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty()),
        crate::crypto::current().is_some(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn field_encryption_is_required_only_when_asked() {
        use super::field_encryption_check as check;
        // (required, supplied, configured)
        assert!(
            check(false, false, false).is_ok(),
            "no keys, none asked for"
        );
        assert!(check(true, true, true).is_ok());
        assert!(check(false, true, true).is_ok());
        assert_eq!(check(true, false, false), Err(MISSING_FIELD_KEYS));
    }

    /// Keys that were given but could not be loaded stop the start whether or not encryption is
    /// required: before, the service logged an error and stored plaintext.
    #[test]
    fn supplied_keys_that_do_not_load_are_refused() {
        use super::field_encryption_check as check;
        assert_eq!(check(false, true, false), Err(UNUSABLE_FIELD_KEYS));
        assert_eq!(check(true, true, false), Err(UNUSABLE_FIELD_KEYS));
    }

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
