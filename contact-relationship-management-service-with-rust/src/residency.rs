//! Where the deployment's data may go (CRM-R106, CRM-T283).
//!
//! `CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS` lists the outside hosts the service may call
//! ([`crate::rules::residency`] says what an entry is); `CONTACT_RELATIONSHIP_MANAGEMENT_DATA_REGION`
//! names the region, as a label for the assurance pack. With a list, a start whose configured
//! outside services name a host not on it is refused, and a webhook to such a host is refused at
//! registration and at each send. Without one, nothing is checked, as before.
//!
//! The database, Redis and the file store are the deployment's own infrastructure, not outside
//! services, so they are not checked here; where they are hosted is the supplier's.

use crate::rules::residency as rules;

/// The settings naming an outside service by URL that the start check reads.
pub const URL_SETTINGS: [&str; 6] = [
    "CONTACT_RELATIONSHIP_MANAGEMENT_PERSON_SERVICE_URL",
    "CONTACT_RELATIONSHIP_MANAGEMENT_ORGANIZATION_SERVICE_URL",
    "CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_URL",
    "CONTACT_RELATIONSHIP_MANAGEMENT_PPM_SERVICE_URL",
    "CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_URL",
    "CONTACT_RELATIONSHIP_MANAGEMENT_KEYCLOAK_URL",
];

fn setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The declared region, if any.
#[must_use]
pub fn region() -> Option<String> {
    setting("CONTACT_RELATIONSHIP_MANAGEMENT_DATA_REGION")
}

/// The declared outside hosts; `None` when there is no list.
///
/// # Errors
///
/// A list that does not read ([`rules::parse_hosts`]).
pub fn declared() -> Result<Option<Vec<String>>, String> {
    setting("CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS")
        .map(|list| {
            rules::parse_hosts(&list)
                .map_err(|e| format!("CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS: {e}"))
        })
        .transpose()
}

/// Whether the service may call `host`: yes when there is no list or it is on it; no for a list
/// that does not read, which the start check has already refused.
#[must_use]
pub fn permits(host: &str) -> bool {
    match declared() {
        Ok(None) => true,
        Ok(Some(hosts)) => rules::permits(&hosts, host),
        Err(_) => false,
    }
}

/// Each configured outside service: the setting that names it and its host.
#[must_use]
pub fn configured(mail: &crate::mail::Sender) -> Vec<(&'static str, String)> {
    let mut found: Vec<(&'static str, String)> = URL_SETTINGS
        .iter()
        .filter_map(|name| Some((*name, rules::host_of(&setting(name)?)?)))
        .collect();
    if let Some(host) = mail.outside_host() {
        found.push(("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE", host));
    }
    found
}

/// The start check: with a list, every configured outside service must be on it.
/// `Ok(Some(note))` is a start that goes ahead unchecked.
///
/// # Errors
///
/// A list that does not read, or the settings that name an undeclared host.
pub fn check_start(mail: &crate::mail::Sender) -> Result<Option<&'static str>, String> {
    let Some(hosts) = declared()? else {
        return Ok(Some(
            "CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS is not set: outside calls are not limited to a declared list",
        ));
    };
    let outside: Vec<String> = configured(mail)
        .into_iter()
        .filter(|(_, host)| !rules::permits(&hosts, host))
        .map(|(name, host)| format!("{name} ({host})"))
        .collect();
    if outside.is_empty() {
        Ok(None)
    } else {
        Err(format!(
            "refusing to start: these settings name a host that is not in \
             CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS (CRM-R106): {}",
            outside.join(", ")
        ))
    }
}
