//! Which deployment a database belongs to (CRM-R105, CRM-T282).
//!
//! A hosted customer has its own database. `CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID` names the deployment; the first
//! start that has it writes it to the one-row `deployment` table, and every start after compares.
//! A start whose name differs from the database's is refused, so a configuration mistake cannot
//! point one customer's service at another customer's database. So is a start with no name
//! against a database that has one. Production refuses to start with no name at all (CRM-D26);
//! elsewhere, with neither, the start goes ahead and says so.

use sqlx::PgPool;

/// The longest name accepted; the table's check says the same.
pub const MAX_NAME: usize = 64;

/// Why a production start with no deployment name is refused (CRM-D26).
pub const MISSING_IN_PRODUCTION: &str = "refusing to start in production without \
    CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID (CRM-R105, CRM-D26): name this deployment, the \
    same on every replica, so a DB_URL that points at another customer's database stops the start";

/// Production needs a deployment name; elsewhere it may be left out.
///
/// # Errors
///
/// [`MISSING_IN_PRODUCTION`] when `production` and there is no name.
pub fn required(production: bool, configured: Option<&str>) -> Result<(), &'static str> {
    if production && configured.is_none() {
        Err(MISSING_IN_PRODUCTION)
    } else {
        Ok(())
    }
}

/// What a start should do, from the configured name and the one the database holds.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Neither names a deployment: go ahead, unchecked.
    Unchecked,
    /// The database names none yet: write this one, then go ahead.
    Claim(String),
    /// Both name the same deployment.
    Matches,
}

/// Whether `name` can name a deployment: 1 to 64 of ASCII letters, digits, `.`, `_` and `-`.
#[must_use]
pub fn valid(name: &str) -> bool {
    (1..=MAX_NAME).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Decide a start.
///
/// # Errors
///
/// Why the start is refused: a name that is not valid, a name that differs from the database's,
/// or no name against a database that has one.
pub fn decide(configured: Option<&str>, stored: Option<&str>) -> Result<Decision, String> {
    match (configured, stored) {
        (Some(name), _) if !valid(name) => Err(format!(
            "CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID must be 1 to {MAX_NAME} letters, digits, '.', '_' or '-'"
        )),
        (None, None) => Ok(Decision::Unchecked),
        (Some(name), None) => Ok(Decision::Claim(name.to_string())),
        (Some(name), Some(held)) if name == held => Ok(Decision::Matches),
        (Some(name), Some(held)) => Err(format!(
            "refusing to start: this database belongs to deployment '{held}', and \
             CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID is '{name}' (CRM-R105). Check DB_URL before anything else."
        )),
        (None, Some(held)) => Err(format!(
            "refusing to start: this database belongs to deployment '{held}', and \
             CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID is not set (CRM-R105). Set it to '{held}' if this is that deployment."
        )),
    }
}

/// `CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID`, trimmed; unset or blank is `None`.
#[must_use]
pub fn configured() -> Option<String> {
    std::env::var("CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Check the database against `configured` and claim it when it names no deployment yet.
/// `Ok(Some(note))` is a start that goes ahead unchecked; `Ok(None)` a checked one.
///
/// # Errors
///
/// Why the start is refused ([`decide`]), or a database error.
pub async fn check(
    pool: &PgPool,
    configured: Option<&str>,
) -> Result<Option<&'static str>, String> {
    let stored = held(pool).await?;
    match decide(configured, stored.as_deref())? {
        Decision::Unchecked => Ok(Some(
            "CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID is not set: nothing checks that this database belongs to this deployment",
        )),
        Decision::Matches => Ok(None),
        Decision::Claim(name) => {
            // Two replicas starting together may both get here: the first insert wins, and the
            // second start compares against whatever is there now.
            sqlx::query("INSERT INTO deployment (name) VALUES ($1) ON CONFLICT DO NOTHING")
                .bind(&name)
                .execute(pool)
                .await
                .map_err(|e| format!("could not record the deployment name: {e}"))?;
            let now = held(pool).await?;
            decide(Some(&name), now.as_deref()).map(|_| None)
        }
    }
}

/// The name the database holds, if any. A database not yet migrated has no table, so no name.
async fn held(pool: &PgPool) -> Result<Option<String>, String> {
    let exists: bool = sqlx::query_scalar("SELECT to_regclass('public.deployment') IS NOT NULL")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("could not read the deployment name: {e}"))?;
    if !exists {
        return Ok(None);
    }
    sqlx::query_scalar("SELECT name FROM deployment")
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("could not read the deployment name: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neither_naming_a_deployment_goes_ahead_unchecked() {
        assert_eq!(decide(None, None), Ok(Decision::Unchecked));
    }

    #[test]
    fn a_name_against_an_unclaimed_database_claims_it() {
        assert_eq!(
            decide(Some("acme-example"), None),
            Ok(Decision::Claim("acme-example".into()))
        );
    }

    #[test]
    fn the_same_name_matches() {
        assert_eq!(
            decide(Some("a.b_c-1"), Some("a.b_c-1")),
            Ok(Decision::Matches)
        );
    }

    #[test]
    fn another_deployments_database_is_refused_and_both_names_are_given() {
        let why = decide(Some("customer-b"), Some("customer-a")).unwrap_err();
        assert!(
            why.contains("'customer-a'") && why.contains("'customer-b'"),
            "{why}"
        );
    }

    #[test]
    fn no_name_against_a_claimed_database_is_refused() {
        let why = decide(None, Some("customer-a")).unwrap_err();
        assert!(
            why.contains("not set") && why.contains("'customer-a'"),
            "{why}"
        );
    }

    #[test]
    fn a_name_differing_only_in_case_is_another_deployment() {
        assert!(decide(Some("Customer-A"), Some("customer-a")).is_err());
    }

    #[test]
    fn names_are_short_and_plain() {
        assert!(valid("a"));
        assert!(valid(&"a".repeat(MAX_NAME)));
        for bad in ["", " a", "a b", "a/b", "ä", "a;drop"] {
            assert!(!valid(bad), "{bad:?}");
        }
        assert!(!valid(&"a".repeat(MAX_NAME + 1)));
        assert!(decide(Some("a b"), None).is_err());
        assert!(decide(Some("a b"), Some("a b")).is_err());
    }

    #[test]
    fn production_needs_a_name_and_elsewhere_it_may_be_left_out() {
        assert_eq!(required(true, None), Err(MISSING_IN_PRODUCTION));
        assert_eq!(required(true, Some("acme-example")), Ok(()));
        assert_eq!(required(false, None), Ok(()));
        assert!(MISSING_IN_PRODUCTION.contains("CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID"));
    }
}
