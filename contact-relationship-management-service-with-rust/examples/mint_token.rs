//! Mint a PASETO `v4.public` token and the matching key set, for trying the service with
//! authentication on (the image smoke test, `scripts/image-smoke.sh`, uses it).
//!
//!     cargo run --example mint_token -- KEYS_FILE SUB [name=value,name=value ...]
//!
//! Writes the published-key-set JSON to KEYS_FILE (give it to the service as
//! `CRM_PASETO_KEYS`) and prints one token on stdout. The Ed25519 seed is fixed and public:
//! these tokens prove nothing outside a test, so never trust this key anywhere real.
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const SEED: [u8; 32] = [7; 32];

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(keys_file), Some(sub)) = (args.next(), args.next()) else {
        eprintln!("usage: mint_token KEYS_FILE SUB [name=value,name=value ...]");
        std::process::exit(2);
    };
    let mut attrs = Map::new();
    for pair in args.next().unwrap_or_default().split(',').filter(|p| !p.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, "true"));
        attrs.insert(name.to_string(), json!([value]));
    }

    let signing = SigningKey::from_bytes(&SEED);
    let public = signing.verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({ "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }] });
    std::fs::write(&keys_file, keys.to_string()).expect("write the key set");

    let now = chrono::Utc::now().timestamp();
    let payload: Value = json!({
        "sub": sub, "email": "operator@example.com", "name": "Image smoke test",
        "iss": "authentication-service", "aud": "main-x-service",
        "iat": now, "exp": now + 3600, "sid": "image-smoke", "attrs": attrs,
    });
    let key = Key::<64>::from(signing.to_keypair_bytes());
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    let payload = payload.to_string();
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    println!("{}", builder.try_sign(&private).expect("sign"));
}
