//! Which outside hosts a deployment may reach (CRM-R106, CRM-T283).
//!
//! A deployer with a residency rule lists the outside hosts the CRM may call. An entry is a host
//! name (`api.brevo.com`) or `*.` and a domain (`*.amazonaws.com`, which matches a name under
//! that domain but not the domain itself). Matching is on the whole name, case-insensitive and
//! ignoring a trailing dot, so `api.brevo.com.evil.example` is not `api.brevo.com`.

/// The longest list accepted.
pub const MAX_HOSTS: usize = 100;

/// Read a comma-separated list of hosts.
///
/// # Errors
///
/// An empty list, more than [`MAX_HOSTS`], or an entry that is not a host name or `*.` and a
/// domain (a scheme, a path, a port, white space inside, or a bare `*`).
pub fn parse_hosts(list: &str) -> Result<Vec<String>, String> {
    let mut hosts = Vec::new();
    for entry in list.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let host = normalise(entry);
        let name = host.strip_prefix("*.").unwrap_or(&host);
        if !is_host_name(name) {
            return Err(format!(
                "'{entry}' is not a host name or '*.' and a domain (no scheme, path or port)"
            ));
        }
        if !hosts.contains(&host) {
            hosts.push(host);
        }
    }
    if hosts.is_empty() {
        return Err("the list of outside hosts is empty".to_string());
    }
    if hosts.len() > MAX_HOSTS {
        return Err(format!("more than {MAX_HOSTS} outside hosts"));
    }
    Ok(hosts)
}

/// Whether `host` is one of `allowed`, or under a `*.` entry's domain.
#[must_use]
pub fn permits(allowed: &[String], host: &str) -> bool {
    let host = normalise(host);
    allowed.iter().any(|entry| match entry.strip_prefix("*.") {
        Some(domain) => host
            .strip_suffix(domain)
            .is_some_and(|rest| rest.len() > 1 && rest.ends_with('.')),
        None => *entry == host,
    })
}

/// The host of `address`, a URL (`https://api.brevo.com/v3`) or a bare host name (`smtp.example`).
#[must_use]
pub fn host_of(address: &str) -> Option<String> {
    let address = address.trim();
    if address.is_empty() {
        return None;
    }
    if address.contains("://") {
        return url::Url::parse(address)
            .ok()?
            .host_str()
            .map(|h| normalise(h.trim_start_matches('[').trim_end_matches(']')));
    }
    let name = address.split(':').next().unwrap_or(address);
    is_host_name(&normalise(name)).then(|| normalise(name))
}

fn normalise(host: &str) -> String {
    host.trim().trim_end_matches('.').to_ascii_lowercase()
}

/// Letters, digits, `-` and `.`, in labels of 1 to 63; an IP address passes as digits and dots.
fn is_host_name(name: &str) -> bool {
    (1..=253).contains(&name.len())
        && name.split('.').all(|label| {
            (1..=63).contains(&label.len())
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(text: &str) -> Vec<String> {
        parse_hosts(text).unwrap()
    }

    #[test]
    fn a_list_is_trimmed_lowered_and_without_repeats() {
        assert_eq!(
            list(" API.Brevo.com , *.amazonaws.com,api.brevo.com., "),
            ["api.brevo.com", "*.amazonaws.com"]
        );
    }

    #[test]
    fn entries_that_are_not_host_names_are_refused() {
        for bad in [
            "",
            " , ",
            "*",
            "https://api.brevo.com",
            "api.brevo.com/v3",
            "api.brevo.com:443",
            "a b.example",
            "*.",
            "a..b",
        ] {
            assert!(parse_hosts(bad).is_err(), "{bad:?}");
        }
        let many = (0..=MAX_HOSTS)
            .map(|i| format!("h{i}.example"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(parse_hosts(&many).is_err());
    }

    #[test]
    fn a_name_matches_itself_whole_and_in_any_case() {
        let allowed = list("api.brevo.com");
        assert!(permits(&allowed, "api.brevo.com"));
        assert!(permits(&allowed, "API.BREVO.COM."));
        assert!(!permits(&allowed, "api.brevo.com.evil.example"));
        assert!(!permits(&allowed, "evilapi.brevo.com"));
        assert!(!permits(&allowed, "brevo.com"));
    }

    #[test]
    fn a_wildcard_matches_names_under_its_domain_only() {
        let allowed = list("*.amazonaws.com");
        assert!(permits(&allowed, "email.eu-west-2.amazonaws.com"));
        assert!(!permits(&allowed, "amazonaws.com"));
        assert!(!permits(&allowed, "evilamazonaws.com"));
        assert!(!permits(&allowed, "amazonaws.com.evil.example"));
    }

    #[test]
    fn the_host_of_a_url_or_a_bare_name() {
        assert_eq!(
            host_of("https://API.brevo.com/v3/smtp"),
            Some("api.brevo.com".into())
        );
        assert_eq!(
            host_of("http://person.internal:8080/api"),
            Some("person.internal".into())
        );
        assert_eq!(
            host_of("smtp.example.test"),
            Some("smtp.example.test".into())
        );
        assert_eq!(
            host_of("smtp.example.test:587"),
            Some("smtp.example.test".into())
        );
        assert_eq!(
            host_of("https://[2001:db8::1]/x"),
            Some("2001:db8::1".into())
        );
        assert_eq!(host_of(""), None);
        assert_eq!(host_of("not a host"), None);
    }
}
