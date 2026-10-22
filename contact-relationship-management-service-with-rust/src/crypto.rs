//! Field-level envelope encryption (CRM-T41): AES-256-GCM over selected
//! personal-data columns, with a keyring that supports rotation.
//!
//! Stored form: `enc:v1:<key id>:<base64url(nonce || ciphertext+tag)>`. A
//! value without the `enc:v1:` prefix is legacy plaintext and reads back
//! unchanged, so turning encryption on never strands existing rows; the
//! `rotate-field-keys` task re-seals everything under the active key.
//!
//! Keys come from `CRM_FIELD_KEYS` (`id:base64,id:base64`, 32-byte keys; the
//! **first** is the active one) or the file named by `CRM_FIELD_KEYS_FILE`.
//! With no keys configured values are stored as plaintext (development), and
//! `CRM_REQUIRE_FIELD_ENCRYPTION=1` makes the service refuse to start in that
//! state. **Honest limits:** keys are read from the environment or a mounted
//! file, not fetched from a KMS (the [`Keyring`] is the seam for one); the
//! additional authenticated data names the format and key id but not the
//! column, so a ciphertext copied between encrypted columns would decrypt;
//! encrypted columns cannot be searched or filtered in SQL.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};

use aes_gcm::aead::{Aead, Generate, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};

/// Prefix of every sealed value.
pub const PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;

/// Why a value could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// The sealed value is malformed.
    Malformed,
    /// No key with this id is configured.
    UnknownKey(String),
    /// Authentication failed: wrong key or tampered data.
    Authentication,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => write!(f, "malformed encrypted field"),
            Self::UnknownKey(id) => write!(f, "no field key with id `{id}` is configured"),
            Self::Authentication => write!(f, "encrypted field failed authentication"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// A set of named keys, one of them active for new writes.
#[derive(Clone)]
pub struct Keyring {
    active: String,
    keys: BTreeMap<String, Key<Aes256Gcm>>,
}

impl std::fmt::Debug for Keyring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material.
        f.debug_struct("Keyring")
            .field("active", &self.active)
            .field("key_ids", &self.keys.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Keyring {
    /// Parse `id:base64,id:base64`; the first entry is the active key.
    ///
    /// # Errors
    ///
    /// A message when the spec is empty, an entry is malformed, an id is
    /// duplicated or contains `:`, or a key is not 32 bytes of base64.
    pub fn from_spec(spec: &str) -> Result<Self, String> {
        let mut keys = BTreeMap::new();
        let mut active = None;
        for entry in spec.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let (id, encoded) = entry
                .split_once(':')
                .ok_or_else(|| "each field key must look like `id:base64`".to_string())?;
            if id.is_empty() || id.contains(':') {
                return Err("a field key id must be non-empty and contain no `:`".to_string());
            }
            let bytes = STANDARD
                .decode(encoded.trim())
                .map_err(|_| format!("field key `{id}` is not valid base64"))?;
            if bytes.len() != 32 {
                return Err(format!("field key `{id}` must be 32 bytes (AES-256)"));
            }
            if keys
                .insert(
                    id.to_string(),
                    Key::<Aes256Gcm>::try_from(bytes.as_slice())
                        .map_err(|_| format!("field key `{id}` must be 32 bytes (AES-256)"))?,
                )
                .is_some()
            {
                return Err(format!("field key id `{id}` is duplicated"));
            }
            active.get_or_insert_with(|| id.to_string());
        }
        Ok(Self {
            active: active.ok_or_else(|| "no field keys configured".to_string())?,
            keys,
        })
    }

    /// The id new values are sealed under.
    #[must_use]
    pub fn active_id(&self) -> &str {
        &self.active
    }

    fn aad(kid: &str) -> Vec<u8> {
        format!("crm-field-v1|{kid}").into_bytes()
    }

    /// Seal `plain` under the active key.
    ///
    /// # Panics
    ///
    /// Never in practice: AES-GCM encryption of in-memory data does not fail.
    #[must_use]
    pub fn seal(&self, plain: &str) -> String {
        let cipher = Aes256Gcm::new(&self.keys[&self.active]);
        let nonce = Nonce::generate();
        #[allow(
            clippy::expect_used,
            reason = "AES-GCM encryption of in-memory data cannot fail"
        )]
        let sealed = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plain.as_bytes(),
                    aad: &Self::aad(&self.active),
                },
            )
            .expect("AES-GCM encryption of in-memory data does not fail");
        let mut blob = nonce.to_vec();
        blob.extend_from_slice(&sealed);
        format!("{PREFIX}{}:{}", self.active, URL_SAFE_NO_PAD.encode(blob))
    }

    /// Open a stored value; legacy plaintext (no prefix) is returned as is.
    ///
    /// # Errors
    ///
    /// [`CryptoError`] when the value is malformed, names an unknown key, or
    /// fails authentication.
    pub fn open(&self, stored: &str) -> Result<String, CryptoError> {
        let Some(rest) = stored.strip_prefix(PREFIX) else {
            return Ok(stored.to_string());
        };
        let (kid, blob) = rest.split_once(':').ok_or(CryptoError::Malformed)?;
        let key = self
            .keys
            .get(kid)
            .ok_or_else(|| CryptoError::UnknownKey(kid.to_string()))?;
        let blob = URL_SAFE_NO_PAD
            .decode(blob)
            .map_err(|_| CryptoError::Malformed)?;
        if blob.len() <= NONCE_LEN {
            return Err(CryptoError::Malformed);
        }
        let (nonce, ciphertext) = blob.split_at(NONCE_LEN);
        let plain = Aes256Gcm::new(key)
            .decrypt(
                &Nonce::try_from(nonce).map_err(|_| CryptoError::Malformed)?,
                Payload {
                    msg: ciphertext,
                    aad: &Self::aad(kid),
                },
            )
            .map_err(|_| CryptoError::Authentication)?;
        String::from_utf8(plain).map_err(|_| CryptoError::Malformed)
    }

    /// Whether `stored` should be re-sealed: plaintext, or under an old key.
    #[must_use]
    pub fn needs_rotation(&self, stored: &str) -> bool {
        match stored.strip_prefix(PREFIX) {
            None => true,
            Some(rest) => rest
                .split_once(':')
                .is_none_or(|(kid, _)| kid != self.active),
        }
    }
}

fn slot() -> &'static RwLock<Option<Arc<Keyring>>> {
    static SLOT: OnceLock<RwLock<Option<Arc<Keyring>>>> = OnceLock::new();
    SLOT.get_or_init(|| RwLock::new(load_from_env()))
}

fn load_from_env() -> Option<Arc<Keyring>> {
    let settings = crate::settings::current();
    let spec = settings.field_keys.clone().or_else(|| {
        let path = settings.field_keys_file.as_deref()?;
        std::fs::read_to_string(path.trim()).ok()
    })?;
    match Keyring::from_spec(&spec) {
        Ok(ring) => Some(Arc::new(ring)),
        Err(error) => {
            tracing::error!(%error, "invalid field-encryption keys; fields will NOT be encrypted");
            None
        }
    }
}

/// The process keyring, if any is configured.
#[must_use]
pub fn current() -> Option<Arc<Keyring>> {
    slot()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Replace the process keyring (tests, and deployments that fetch keys at boot).
pub fn install(ring: Option<Keyring>) {
    *slot()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = ring.map(Arc::new);
}

/// Seal with the process keyring; plaintext when none is configured.
#[must_use]
pub fn seal(plain: &str) -> String {
    current().map_or_else(|| plain.to_string(), |ring| ring.seal(plain))
}

/// Open with the process keyring. A sealed value with no keyring configured
/// is an error, never silently returned as ciphertext.
///
/// # Errors
///
/// [`CryptoError`] as for [`Keyring::open`]; sealed data with no keyring is
/// [`CryptoError::UnknownKey`].
pub fn open(stored: &str) -> Result<String, CryptoError> {
    match current() {
        Some(ring) => ring.open(stored),
        None if stored.starts_with(PREFIX) => {
            Err(CryptoError::UnknownKey("(none configured)".into()))
        }
        None => Ok(stored.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(ids: &[(&str, u8)]) -> String {
        ids.iter()
            .map(|(id, byte)| format!("{id}:{}", STANDARD.encode([*byte; 32])))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn ring(ids: &[(&str, u8)]) -> Keyring {
        Keyring::from_spec(&spec(ids)).unwrap()
    }

    #[test]
    fn round_trips_and_is_not_plaintext() {
        let ring = ring(&[("k1", 1)]);
        let sealed = ring.seal("dana@example.org");
        assert!(sealed.starts_with("enc:v1:k1:"));
        assert!(!sealed.contains("dana"));
        assert_eq!(ring.open(&sealed).unwrap(), "dana@example.org");
    }

    #[test]
    fn every_seal_uses_a_fresh_nonce() {
        let ring = ring(&[("k1", 1)]);
        assert_ne!(ring.seal("same"), ring.seal("same"));
    }

    #[test]
    fn unicode_and_empty_round_trip() {
        let ring = ring(&[("k1", 1)]);
        for text in ["", "héllo — 你好", "line\nbreak"] {
            assert_eq!(ring.open(&ring.seal(text)).unwrap(), text);
        }
    }

    #[test]
    fn tampering_is_detected() {
        let ring = ring(&[("k1", 1)]);
        let sealed = ring.seal("secret");
        let mut bytes = sealed.into_bytes();
        let last = bytes.len() - 2;
        bytes[last] = if bytes[last] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert_eq!(ring.open(&tampered), Err(CryptoError::Authentication));
    }

    #[test]
    fn a_wrong_key_fails_authentication() {
        let sealed = ring(&[("k1", 1)]).seal("secret");
        assert_eq!(
            ring(&[("k1", 2)]).open(&sealed),
            Err(CryptoError::Authentication)
        );
    }

    #[test]
    fn the_key_id_is_authenticated() {
        // Same key bytes under two ids: moving a value to another id must fail.
        let ring = ring(&[("a", 7), ("b", 7)]);
        let sealed = ring.seal("secret"); // under `a`
        let relabeled = sealed.replacen(":a:", ":b:", 1);
        assert_eq!(ring.open(&relabeled), Err(CryptoError::Authentication));
    }

    #[test]
    fn plaintext_reads_through_unchanged() {
        assert_eq!(ring(&[("k1", 1)]).open("email").unwrap(), "email");
    }

    #[test]
    fn rotation_reads_old_keys_and_seals_with_the_new_one() {
        let old = ring(&[("k1", 1)]);
        let sealed_old = old.seal("secret");
        let rotated = ring(&[("k2", 2), ("k1", 1)]);
        assert_eq!(rotated.active_id(), "k2");
        assert_eq!(rotated.open(&sealed_old).unwrap(), "secret");
        assert!(rotated.needs_rotation(&sealed_old));
        assert!(rotated.needs_rotation("plain"));
        assert!(!rotated.needs_rotation(&rotated.seal("secret")));
        // Once k1 is retired, its data is unreadable: fail loudly.
        let retired = ring(&[("k2", 2)]);
        assert_eq!(
            retired.open(&sealed_old),
            Err(CryptoError::UnknownKey("k1".into()))
        );
    }

    #[test]
    fn bad_specs_are_rejected() {
        assert!(Keyring::from_spec("").is_err());
        assert!(Keyring::from_spec("nocolon").is_err());
        assert!(Keyring::from_spec("k:not-base64!!").is_err());
        assert!(Keyring::from_spec(&format!("k:{}", STANDARD.encode([0_u8; 16]))).is_err());
        let one = STANDARD.encode([1_u8; 32]);
        assert!(Keyring::from_spec(&format!("k:{one},k:{one}")).is_err());
    }

    #[test]
    fn debug_never_prints_key_material() {
        let shown = format!("{:?}", ring(&[("k1", 9)]));
        assert!(shown.contains("k1"));
        assert!(!shown.contains(&STANDARD.encode([9_u8; 32])));
    }

    #[test]
    fn malformed_sealed_values_error() {
        let ring = ring(&[("k1", 1)]);
        assert_eq!(
            ring.open("enc:v1:nokeyseparator"),
            Err(CryptoError::Malformed)
        );
        assert_eq!(ring.open("enc:v1:k1:!!!"), Err(CryptoError::Malformed));
        assert_eq!(ring.open("enc:v1:k1:AAAA"), Err(CryptoError::Malformed));
    }
}
