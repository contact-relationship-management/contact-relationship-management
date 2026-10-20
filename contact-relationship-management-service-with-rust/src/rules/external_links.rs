//! Links to records in other systems (CRM-R27): which systems, and which web
//! addresses are acceptable for each.
//!
//! A link is only a link: nothing is fetched or synchronised. What we guard is
//! what a person is sent to when they click it, so the address must be `https`,
//! carry no embedded credentials, and, for a named system, be on that system's
//! own domains.

/// The systems a link can point at: `(token, domain suffixes)`. `other` has no
/// suffix list: any `https` address is accepted.
pub const SYSTEMS: [(&str, &[&str]); 6] = [
    ("hubspot", &["hubspot.com"]),
    (
        "salesforce",
        &["salesforce.com", "force.com", "salesforce-setup.com"],
    ),
    ("attio", &["attio.com"]),
    ("pipedrive", &["pipedrive.com"]),
    ("dynamics", &["dynamics.com"]),
    ("other", &[]),
];

/// The system tokens, for validation messages.
pub const SYSTEM_TOKENS: [&str; 6] = [
    "hubspot",
    "salesforce",
    "attio",
    "pipedrive",
    "dynamics",
    "other",
];

/// The kinds of record a link can hang off.
pub const SUBJECT_KINDS: [&str; 4] = ["contact", "account", "deal", "lead"];

/// The longest address accepted.
pub const MAX_URL_LEN: usize = 2048;

/// Check `address` for `system`. Returns the address to store (as the URL
/// parser wrote it), or what is wrong.
///
/// # Errors
///
/// The address is not a valid `https` URL, carries credentials, is too long, or
/// is not on a domain of the named system.
pub fn check_url(system: &str, address: &str) -> Result<String, String> {
    let address = address.trim();
    if address.len() > MAX_URL_LEN {
        return Err(format!("url exceeds {MAX_URL_LEN} characters"));
    }
    let parsed =
        url::Url::parse(address).map_err(|_| "url is not a valid web address".to_string())?;
    if parsed.scheme() != "https" {
        return Err("url must start with https://".to_string());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("url must not contain a user name or password".to_string());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "url has no host".to_string())?
        .to_ascii_lowercase();
    let suffixes = SYSTEMS
        .iter()
        .find(|(token, _)| *token == system)
        .map(|(_, suffixes)| *suffixes)
        .ok_or_else(|| format!("system must be one of {SYSTEM_TOKENS:?}"))?;
    if !suffixes.is_empty()
        && !suffixes
            .iter()
            .any(|s| host == *s || host.ends_with(&format!(".{s}")))
    {
        return Err(format!(
            "a {system} link must be on one of: {}",
            suffixes.join(", ")
        ));
    }
    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_systems_must_be_on_their_own_domains() {
        assert!(check_url("hubspot", "https://app.hubspot.com/contacts/1/record/0-1/2").is_ok());
        assert!(check_url("hubspot", "https://app-eu1.hubspot.com/contacts/1").is_ok());
        assert!(
            check_url(
                "salesforce",
                "https://acme.lightning.force.com/lightning/r/Contact/003/view"
            )
            .is_ok()
        );
        assert!(check_url("salesforce", "https://acme.my.salesforce.com/003").is_ok());
        assert!(check_url("attio", "https://app.attio.com/acme/people/1").is_ok());
        assert!(check_url("hubspot", "https://hubspot.com.evil.example/x").is_err());
        assert!(check_url("hubspot", "https://evilhubspot.com/x").is_err());
        assert!(check_url("attio", "https://app.hubspot.com/x").is_err());
    }

    #[test]
    fn other_accepts_any_https_and_nothing_else() {
        assert!(check_url("other", "https://crm.example.org/people/9").is_ok());
        assert!(check_url("other", "http://crm.example.org/people/9").is_err());
        assert!(check_url("other", "javascript:alert(1)").is_err());
        assert!(check_url("other", "data:text/html,x").is_err());
        assert!(check_url("other", "https://user:pw@crm.example.org/").is_err());
        assert!(check_url("other", "not a url").is_err());
        assert!(check_url("nonsense", "https://crm.example.org/").is_err());
    }

    #[test]
    fn the_address_is_bounded_and_trimmed() {
        assert!(
            check_url(
                "other",
                &format!("https://x.example/{}", "a".repeat(MAX_URL_LEN))
            )
            .is_err()
        );
        assert_eq!(
            check_url("attio", "  https://app.attio.com/a  ").unwrap(),
            "https://app.attio.com/a"
        );
    }

    #[test]
    fn every_token_has_a_row() {
        let tokens: Vec<&str> = SYSTEMS.iter().map(|(t, _)| *t).collect();
        assert_eq!(tokens, SYSTEM_TOKENS);
    }
}
