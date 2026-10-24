//! Outbound webhooks (CRM-R28): which addresses may be called, which events a
//! subscription wants, how a delivery is signed, and when a failed one is tried again.
//!
//! Pure and DB-free. The network, the clock and the database are the controller's.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// The longest address accepted.
pub const MAX_URL_LEN: usize = 2048;
/// The most event patterns one subscription can carry.
pub const MAX_PATTERNS: usize = 20;
/// Attempts before a delivery is dead (the first send counts as one).
pub const MAX_ATTEMPTS: i32 = 8;
/// How long the previous secret keeps signing after a rotation.
pub const ROTATION_GRACE_HOURS: i64 = 24;

/// Seconds to wait after the `n`th failed attempt (1-based) before the next one, or
/// `None` when that was the last attempt and the delivery is dead.
#[must_use]
pub fn retry_delay_seconds(failed_attempts: i32) -> Option<i64> {
    const DELAYS: [i64; 7] = [30, 120, 600, 3_600, 21_600, 43_200, 86_400];
    if failed_attempts >= MAX_ATTEMPTS {
        return None;
    }
    let index = usize::try_from(failed_attempts.max(1) - 1).unwrap_or(0);
    Some(DELAYS[index.min(DELAYS.len() - 1)])
}

/// Whether `ip` is an address on the public internet. Everything that reaches
/// into a network (private, loopback, link-local, shared, documentation,
/// multicast, reserved, unspecified) is refused, and so is an IPv6 address that
/// wraps one of those IPv4 addresses.
#[must_use]
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || a == 0
        || (a == 100 && (64..=127).contains(&b)) // shared address space (carrier NAT)
        || (a == 192 && b == 0 && c == 0) // IETF protocol assignments
        || (a == 192 && b == 0 && c == 2) // documentation
        || (a == 198 && (b == 18 || b == 19)) // benchmarking
        || (a == 198 && b == 51 && c == 100) // documentation
        || (a == 203 && b == 0 && c == 113) // documentation
        || a >= 240) // reserved
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    let s = ip.segments();
    // 64:ff9b::/96 translates to an IPv4 address; judge that address.
    if s[0] == 0x64 && s[1] == 0xff9b && s[2..6].iter().all(|x| *x == 0) {
        let [hi, lo] = [s[6].to_be_bytes(), s[7].to_be_bytes()];
        return is_public_v4(Ipv4Addr::new(hi[0], hi[1], lo[0], lo[1]));
    }
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (s[0] & 0xfe00) == 0xfc00 // unique local
        || (s[0] & 0xffc0) == 0xfe80 // link-local
        || (s[0] == 0x2001 && s[1] == 0x0db8)) // documentation
}

/// Check a subscription address. Returns the address to store, or what is wrong.
/// An IP literal must be public; a name is resolved and checked at each send
/// (see the controller), because the answer can change.
///
/// # Errors
///
/// Not a valid `https` URL, too long, carries credentials, has no host, or its
/// host is an IP address that is not public.
pub fn check_target(address: &str) -> Result<String, String> {
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
    match parsed.host() {
        None => return Err("url has no host".to_string()),
        Some(url::Host::Ipv4(ip)) if !is_public_v4(ip) => {
            return Err("url must not point at a private or reserved address".to_string());
        }
        Some(url::Host::Ipv6(ip)) if !is_public_v6(ip) => {
            return Err("url must not point at a private or reserved address".to_string());
        }
        Some(url::Host::Domain(name)) if is_local_name(name) => {
            return Err("url must not point at a local name".to_string());
        }
        Some(_) => {}
    }
    Ok(parsed.to_string())
}

/// Names that never leave the machine or the network, whatever they resolve to.
#[allow(clippy::case_sensitive_file_extension_comparisons)] // host names, not files
fn is_local_name(name: &str) -> bool {
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    name == "localhost"
        || name.ends_with(".localhost")
        || name.ends_with(".local")
        || name.ends_with(".internal")
        || !name.contains('.')
}

/// Check and normalise the event patterns: `*`, `entity.*`, `*.kind` or `entity.kind`.
///
/// # Errors
///
/// None given, too many, or one that is not of those shapes.
pub fn check_patterns(patterns: &[String]) -> Result<Vec<String>, String> {
    if patterns.is_empty() {
        return Err("at least one event pattern is required".to_string());
    }
    if patterns.len() > MAX_PATTERNS {
        return Err(format!("at most {MAX_PATTERNS} event patterns"));
    }
    let mut out: Vec<String> = Vec::new();
    for pattern in patterns {
        let pattern = pattern.trim().to_ascii_lowercase();
        let word = |part: &str| {
            part == "*"
                || (!part.is_empty()
                    && part.len() <= 40
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
        };
        let ok = pattern == "*"
            || pattern
                .split_once('.')
                .is_some_and(|(entity, kind)| word(entity) && word(kind));
        if !ok {
            return Err(format!(
                "event pattern '{pattern}' must be '*', 'entity.*', '*.kind' or 'entity.kind'"
            ));
        }
        if !out.contains(&pattern) {
            out.push(pattern);
        }
    }
    Ok(out)
}

/// Whether `pattern` (already checked) selects the event `entity.kind`.
#[must_use]
pub fn pattern_matches(pattern: &str, entity: &str, kind: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    pattern
        .split_once('.')
        .is_some_and(|(e, k)| (e == "*" || e == entity) && (k == "*" || k == kind))
}

/// Whether any of `patterns` selects the event.
#[must_use]
pub fn wants(patterns: &[String], entity: &str, kind: &str) -> bool {
    patterns.iter().any(|p| pattern_matches(p, entity, kind))
}

/// A new signing secret: `whsec_` and 32 random bytes in base64, the format the
/// Standard Webhooks libraries expect.
#[must_use]
pub fn new_secret() -> String {
    let mut bytes = [0_u8; 32];
    bytes[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    format!("whsec_{}", STANDARD.encode(bytes))
}

/// One signature, `v1,<base64>`: HMAC-SHA256 over `id.timestamp.body` keyed by the
/// secret's bytes (the part after `whsec_`).
///
/// # Errors
///
/// The secret is not `whsec_` and base64.
pub fn sign(secret: &str, id: &str, timestamp: i64, body: &str) -> Result<String, String> {
    let key = STANDARD
        .decode(secret.strip_prefix("whsec_").unwrap_or(secret))
        .map_err(|_| "signing secret is not valid base64".to_string())?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).map_err(|e| e.to_string())?;
    mac.update(format!("{id}.{timestamp}.{body}").as_bytes());
    Ok(format!(
        "v1,{}",
        STANDARD.encode(mac.finalize().into_bytes())
    ))
}

/// The `webhook-signature` header: one signature per secret that is still valid,
/// separated by spaces.
///
/// # Errors
///
/// A secret is not valid, or none was given.
pub fn signature_header(
    secrets: &[&str],
    id: &str,
    timestamp: i64,
    body: &str,
) -> Result<String, String> {
    if secrets.is_empty() {
        return Err("no signing secret".to_string());
    }
    secrets
        .iter()
        .map(|secret| sign(secret, id, timestamp, body))
        .collect::<Result<Vec<_>, _>>()
        .map(|all| all.join(" "))
}

/// The body of a delivery: ids and the time, never a contact value.
#[must_use]
pub fn body(
    event_id: uuid::Uuid,
    entity: &str,
    kind: &str,
    pid: uuid::Uuid,
    occurred_at: chrono::DateTime<chrono::Utc>,
) -> String {
    serde_json::json!({
        "id": event_id,
        "type": format!("{entity}.{kind}"),
        "timestamp": occurred_at,
        "data": { "entity": entity, "kind": kind, "pid": pid },
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn private_loopback_link_local_and_reserved_addresses_are_refused() {
        for bad in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.169.254", // cloud metadata
            "0.0.0.0",
            "100.64.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "240.0.0.1",
            "198.18.0.1",
            "192.0.2.5",
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "fd12:3456::1",
            "ff02::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
            "64:ff9b::7f00:1", // NAT64 wrapping 127.0.0.1
        ] {
            assert!(!is_public_ip(ip(bad)), "{bad} must be refused");
        }
    }

    #[test]
    fn public_addresses_are_accepted() {
        for good in [
            "8.8.8.8",
            "1.1.1.1",
            "93.184.216.34",
            "2606:4700:4700::1111",
            "172.32.0.1",
        ] {
            assert!(is_public_ip(ip(good)), "{good} must be accepted");
        }
    }

    #[test]
    fn a_target_must_be_a_plain_https_address_to_a_public_host() {
        assert!(check_target("https://hooks.example.com/crm").is_ok());
        assert!(check_target("https://8.8.8.8/x").is_ok());
        for bad in [
            "http://hooks.example.com/",
            "ftp://hooks.example.com/",
            "https://user:pw@hooks.example.com/",
            "https://127.0.0.1/",
            "https://[::1]/",
            "https://169.254.169.254/latest/meta-data",
            "https://[::ffff:10.0.0.1]/",
            "https://localhost/",
            "https://db.internal/",
            "https://printer.local/",
            "https://intranet/",
            "https://2130706433/", // 127.0.0.1 as one number: the parser reads it as an IP
            "https://0x7f.1/",
            "not a url",
            "",
        ] {
            assert!(check_target(bad).is_err(), "{bad} must be refused");
        }
        let long = format!("https://example.com/{}", "a".repeat(MAX_URL_LEN));
        assert!(check_target(&long).is_err());
    }

    #[test]
    fn patterns_are_checked_and_matched() {
        let ok = check_patterns(&[
            "Contact.Created".into(),
            "deal.*".into(),
            "contact.created".into(),
        ])
        .unwrap();
        assert_eq!(ok, vec!["contact.created", "deal.*"]);
        assert!(check_patterns(&[]).is_err());
        for bad in [
            "contact",
            "contact.",
            ".created",
            "a.b.c",
            "con tact.x",
            "contact.*x",
            "",
        ] {
            assert!(check_patterns(&[bad.into()]).is_err(), "{bad}");
        }
        assert!(check_patterns(&vec!["a.b".to_string(); 21]).is_err());
        assert!(pattern_matches("*", "deal", "opened"));
        assert!(pattern_matches("deal.*", "deal", "opened"));
        assert!(!pattern_matches("deal.*", "lead", "opened"));
        assert!(pattern_matches("*.created", "lead", "created"));
        assert!(!pattern_matches("*.created", "lead", "opened"));
        assert!(pattern_matches("deal.opened", "deal", "opened"));
        assert!(!pattern_matches("deal.opened", "deal", "won"));
        assert!(wants(&["lead.*".into(), "deal.won".into()], "deal", "won"));
        assert!(!wants(&["lead.*".into()], "deal", "won"));
    }

    /// The worked example from the Standard Webhooks specification's reference
    /// implementation: the signature must match what any receiving library computes.
    #[test]
    fn the_signature_is_the_standard_webhooks_one() {
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let got = sign(
            secret,
            "msg_p5jXN8AQM9LWM0D4loKWxJek",
            1_614_265_330,
            r#"{"test": 2432232314}"#,
        )
        .unwrap();
        assert_eq!(got, "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=");
    }

    #[test]
    fn a_rotating_secret_signs_twice_and_each_signature_differs() {
        let (a, b) = (new_secret(), new_secret());
        assert!(a.starts_with("whsec_") && a != b);
        let header = signature_header(&[&a, &b], "evt", 1, "{}").unwrap();
        let parts: Vec<&str> = header.split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|p| p.starts_with("v1,")));
        assert_ne!(parts[0], parts[1]);
        assert_eq!(parts[0], sign(&a, "evt", 1, "{}").unwrap());
        assert!(signature_header(&[], "evt", 1, "{}").is_err());
        assert!(sign("whsec_!!!", "evt", 1, "{}").is_err());
    }

    #[test]
    fn the_timestamp_and_the_body_are_signed() {
        let s = new_secret();
        let base = sign(&s, "e", 1, "{}").unwrap();
        assert_ne!(base, sign(&s, "e", 2, "{}").unwrap());
        assert_ne!(base, sign(&s, "e", 1, "{ }").unwrap());
        assert_ne!(base, sign(&s, "f", 1, "{}").unwrap());
    }

    #[test]
    fn retries_back_off_and_then_the_delivery_is_dead() {
        let delays: Vec<Option<i64>> = (1..=MAX_ATTEMPTS).map(retry_delay_seconds).collect();
        assert_eq!(
            delays,
            vec![
                Some(30),
                Some(120),
                Some(600),
                Some(3_600),
                Some(21_600),
                Some(43_200),
                Some(86_400),
                None
            ]
        );
        assert_eq!(retry_delay_seconds(0), Some(30));
        assert_eq!(retry_delay_seconds(99), None);
    }

    #[test]
    fn the_body_has_ids_and_the_time_and_nothing_else() {
        let (event, pid) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        let at = chrono::DateTime::parse_from_rfc3339("2026-10-24T10:00:00Z")
            .unwrap()
            .to_utc();
        let value: serde_json::Value =
            serde_json::from_str(&body(event, "contact", "created", pid, at)).unwrap();
        assert_eq!(value["id"], event.to_string());
        assert_eq!(value["type"], "contact.created");
        assert_eq!(value["data"]["pid"], pid.to_string());
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert_eq!(keys.len(), 4, "{keys:?}");
        assert_eq!(value["data"].as_object().unwrap().len(), 3);
    }
}
