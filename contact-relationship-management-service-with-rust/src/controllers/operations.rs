//! Operational facts for an assurance review (CRM-R106, CRM-T283). Admin only: the routes are
//! under `/api/operations`, an [`crate::auth::policy::ADMIN_ONLY_PREFIXES`] entry.

use crate::rules::absent::OrEmpty;
use axa::prelude::*;
use serde::Serialize;

/// One configured outside service.
#[derive(Debug, Serialize)]
struct Outside {
    /// The setting that names it.
    setting: &'static str,
    host: String,
    /// Whether the declared list allows it; always true when there is no list.
    allowed: bool,
}

/// The answer of `GET /api/operations/residency`.
#[derive(Debug, Serialize)]
struct Residency {
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_DATA_REGION`, if set.
    region: Option<String>,
    /// Whether outside calls are limited to a declared list.
    limited: bool,
    /// `CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS`, as read.
    declared_hosts: Vec<String>,
    /// Each outside service this process is configured to call.
    configured: Vec<Outside>,
    /// What the list does not cover.
    not_covered: &'static str,
}

/// `GET /api/operations/residency`: the declared region and outside hosts, and each configured
/// outside service with whether the list allows it.
#[debug_handler]
async fn residency() -> Result<Response> {
    let declared = crate::residency::declared().map_err(|e| Error::string(&e))?;
    let mail = crate::mail::current();
    let configured = crate::residency::configured(&mail)
        .into_iter()
        .map(|(setting, host)| Outside {
            allowed: crate::residency::permits(&host),
            setting,
            host,
        })
        .collect();
    format::json(Residency {
        region: crate::residency::region(),
        limited: declared.is_some(),
        declared_hosts: declared.or_empty(),
        configured,
        not_covered: "the database, Redis and the file store (the deployment's own \
                      infrastructure), and webhook addresses, which are checked when \
                      registered and at each send; where any of them is hosted is the \
                      supplier's",
    })
}

/// The operations routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/operations")
        .add("/residency", get(residency))
}
