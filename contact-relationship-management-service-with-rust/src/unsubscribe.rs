//! Signed unsubscribe links for campaign mail (CRM-T39, RFC 8058).
//!
//! Every message carries a link that withdraws the recipient's marketing
//! consent. The link holds no secret of the recipient's: it is the contact
//! and campaign identifiers plus an HMAC-SHA256 tag, so it cannot be forged
//! or enumerated. It carries no expiry, because a recipient must be able to
//! unsubscribe from an old message. The withdrawal itself is the ordinary
//! consent change (append-only history, enrolments exited, audited).

use std::sync::{Arc, RwLock};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

type Tag = Hmac<Sha256>;

/// Shortest accepted `CRM_UNSUBSCRIBE_SECRET`.
const MIN_SECRET: usize = 32;

/// Makes and checks unsubscribe links.
#[derive(Clone)]
pub struct Unsubscriber {
    secret: Vec<u8>,
    base: String,
}

impl std::fmt::Debug for Unsubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Unsubscriber")
            .field("base", &self.base)
            .field("secret", &"<redacted>")
            .finish()
    }
}

impl Unsubscriber {
    /// Build from `CRM_UNSUBSCRIBE_SECRET` and `CRM_PUBLIC_URL`.
    ///
    /// # Errors
    ///
    /// A message when either is missing or unusable. In production the public
    /// URL must be `https`.
    pub fn new(secret: &str, public_url: &str, production: bool) -> Result<Self, String> {
        if secret.len() < MIN_SECRET {
            return Err(format!(
                "CRM_UNSUBSCRIBE_SECRET must be at least {MIN_SECRET} characters"
            ));
        }
        let base = public_url.trim().trim_end_matches('/').to_string();
        let secure = base.starts_with("https://");
        let local = base.starts_with("http://");
        if !(secure || (local && !production)) {
            return Err(
                "CRM_PUBLIC_URL must be an https URL (http is allowed outside \
                        production)"
                    .to_string(),
            );
        }
        if base.contains(char::is_whitespace) {
            return Err("CRM_PUBLIC_URL must not contain whitespace".to_string());
        }
        Ok(Self {
            secret: secret.as_bytes().to_vec(),
            base,
        })
    }

    fn mac(&self, payload: &[u8]) -> Tag {
        // HMAC accepts any key length, so this cannot fail.
        let mut mac = Tag::new_from_slice(&self.secret).expect("hmac takes any key length");
        mac.update(payload);
        mac
    }

    /// The token for one recipient of one campaign.
    #[must_use]
    pub fn token(&self, contact: Uuid, campaign: Uuid) -> String {
        let mut payload = Vec::with_capacity(32);
        payload.extend_from_slice(contact.as_bytes());
        payload.extend_from_slice(campaign.as_bytes());
        let tag = self.mac(&payload).finalize().into_bytes();
        payload.extend_from_slice(&tag);
        URL_SAFE_NO_PAD.encode(payload)
    }

    /// The contact and campaign a genuine token names, or `None` when it is
    /// malformed or its tag does not verify (compared in constant time).
    #[must_use]
    pub fn verify(&self, token: &str) -> Option<(Uuid, Uuid)> {
        let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
        if bytes.len() != 32 + 32 {
            return None;
        }
        let (payload, tag) = bytes.split_at(32);
        self.mac(payload).verify_slice(tag).ok()?;
        let contact = Uuid::from_slice(&payload[..16]).ok()?;
        let campaign = Uuid::from_slice(&payload[16..]).ok()?;
        Some((contact, campaign))
    }

    /// The full link put in a message.
    #[must_use]
    pub fn url(&self, contact: Uuid, campaign: Uuid) -> String {
        format!(
            "{}/unsubscribe/{}",
            self.base,
            self.token(contact, campaign)
        )
    }
}

fn slot() -> &'static RwLock<Option<Arc<Unsubscriber>>> {
    static SLOT: RwLock<Option<Arc<Unsubscriber>>> = RwLock::new(None);
    &SLOT
}

/// The configured unsubscriber, if any.
#[must_use]
pub fn current() -> Option<Arc<Unsubscriber>> {
    slot()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Install (or clear) the process-wide unsubscriber.
pub fn install(value: Option<Unsubscriber>) {
    *slot()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = value.map(Arc::new);
}

/// Configure from the environment at start-up. Real mail needs links: when
/// `smtp_mode` is on and either variable is missing the service refuses to
/// start.
///
/// # Errors
///
/// A message when real mail is on without a usable configuration, or when a
/// partial configuration is present.
pub fn init(smtp_mode: bool, production: bool) -> Result<(), String> {
    let secret = std::env::var("CRM_UNSUBSCRIBE_SECRET").ok();
    let url = std::env::var("CRM_PUBLIC_URL").ok();
    match (secret, url) {
        (Some(secret), Some(url)) => install(Some(Unsubscriber::new(&secret, &url, production)?)),
        (None, None) if !smtp_mode => install(None),
        _ => {
            return Err(
                "CRM_UNSUBSCRIBE_SECRET and CRM_PUBLIC_URL are both required, and \
                        required for CRM_MAIL_MODE=smtp: every message must carry an \
                        unsubscribe link"
                    .to_string(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    fn make() -> Unsubscriber {
        Unsubscriber::new(SECRET, "https://crm.example.org/", true).unwrap()
    }

    #[test]
    fn a_token_round_trips() {
        let (c, k) = (Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(make().verify(&make().token(c, k)), Some((c, k)));
    }

    #[test]
    fn a_tampered_token_is_rejected() {
        let u = make();
        let token = u.token(Uuid::new_v4(), Uuid::new_v4());
        let mut bytes = URL_SAFE_NO_PAD.decode(&token).unwrap();
        bytes[0] ^= 1; // name a different contact, keep the old tag
        assert_eq!(u.verify(&URL_SAFE_NO_PAD.encode(bytes)), None);
        assert_eq!(u.verify("garbage"), None);
        assert_eq!(u.verify(""), None);
    }

    #[test]
    fn a_token_from_another_secret_is_rejected() {
        let other = Unsubscriber::new(&SECRET.replace('0', "9"), "https://x.test", true).unwrap();
        let token = other.token(Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(make().verify(&token), None);
    }

    #[test]
    fn the_link_is_under_the_public_url() {
        let url = make().url(Uuid::new_v4(), Uuid::new_v4());
        assert!(url.starts_with("https://crm.example.org/unsubscribe/"));
    }

    #[test]
    fn weak_or_insecure_settings_are_refused() {
        assert!(Unsubscriber::new("short", "https://x.test", true).is_err());
        assert!(Unsubscriber::new(SECRET, "http://x.test", true).is_err());
        assert!(Unsubscriber::new(SECRET, "http://localhost:5150", false).is_ok());
        assert!(Unsubscriber::new(SECRET, "ftp://x.test", false).is_err());
    }

    #[test]
    fn debug_hides_the_secret() {
        assert!(!format!("{:?}", make()).contains(SECRET));
    }
}
