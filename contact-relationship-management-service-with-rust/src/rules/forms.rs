//! Web form capture (CRM-R31): what counts as an address and a name.
//!
//! Pure and DB-free. The submission limits are `crate::rate_limit::Settable` (CRM-T219).

/// The longest address accepted (RFC 5321).
pub const MAX_EMAIL_LEN: usize = 254;
/// The longest name accepted.
pub const MAX_NAME_LEN: usize = 200;

/// Whether `email` is plausibly one mailbox: one `@`, a local part and a dotted domain, no
/// spaces, control characters or characters that would make it a list or a header. The
/// confirmation mail is the real test.
#[must_use]
pub fn valid_email(email: &str) -> bool {
    if email.is_empty() || email.len() > MAX_EMAIL_LEN {
        return false;
    }
    if email
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || "<>\"(),;:\\[]".contains(c))
    {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && !domain.contains('@')
        && domain.contains('.')
        && domain
            .split('.')
            .all(|label| !label.is_empty() && !label.starts_with('-') && !label.ends_with('-'))
}

/// The part of `email` after the `@`, lower-cased.
#[must_use]
pub fn domain_of(email: &str) -> Option<String> {
    email
        .rsplit_once('@')
        .map(|(_, domain)| domain.to_ascii_lowercase())
}

/// Whether `name` can be shown and put in a message: not blank, not too long, no control
/// characters (a newline in a name must not reach a mail header).
#[must_use]
pub fn valid_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name.chars().count() <= MAX_NAME_LEN && !name.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_one_mailbox_with_a_dotted_domain() {
        for good in ["a@b.co", "first.last+tag@sub.example.org", "ünï@exämple.de"] {
            assert!(valid_email(good), "{good}");
        }
        for bad in [
            "",
            "plain",
            "a@b",
            "@b.co",
            "a@@b.co",
            "a@b@c.co",
            "a b@c.co",
            "a@b.co\nBcc: x@y.z",
            "a,b@c.co",
            "<a@b.co>",
            "a@-b.co",
            "a@b..co",
        ] {
            assert!(!valid_email(bad), "{bad:?}");
        }
        assert!(!valid_email(&format!("{}@b.co", "a".repeat(65))));
        assert!(!valid_email(&format!("a@{}.co", "b".repeat(260))));
    }

    #[test]
    fn a_name_cannot_carry_a_line_break() {
        assert!(valid_name("Ada Lovelace"));
        assert!(!valid_name("  "));
        assert!(!valid_name("Ada\nBcc: x@y.z"));
        assert!(!valid_name(&"x".repeat(MAX_NAME_LEN + 1)));
    }

    #[test]
    fn the_domain_is_lower_cased() {
        assert_eq!(domain_of("Ada@Example.ORG").as_deref(), Some("example.org"));
        assert_eq!(domain_of("nope"), None);
    }
}
