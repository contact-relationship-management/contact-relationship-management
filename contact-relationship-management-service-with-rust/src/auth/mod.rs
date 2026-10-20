//! Bearer-token authentication for the contact-relationship-management API.
//!
//! [`AuthUser`] is an Axum extractor that pulls `Authorization: Bearer
//! <paseto>`, verifies the PASETO `v4.public` (Ed25519) signature, issuer,
//! audience and expiry against the
//! [authentication-service](../../../authentication/authentication-service-with-loco)
//! published key set, and yields the verified [`Claims`]. Verification is
//! stateless and offline — no database hit, no introspection call — so any
//! handler can require authentication by taking an `AuthUser` argument, and
//! a handler that wants the caller identity *when present* (e.g. to stamp
//! an audit `actor`) takes [`MaybeAuthUser`] instead.
//!
//! ## Key source
//!
//! The process-wide [`verifier`] is seeded once at boot ([`init`] is
//! called from `App::after_routes`, before the app serves traffic) and
//! built from the environment:
//!
//! - `CRM_PASETO_KEYS_URL` — optional URL of the auth service's
//!   published key set (`/.well-known/paseto-keys`). Set (non-blank) ⇒
//!   the key set is fetched over HTTP **once at boot** via
//!   [`Verifier::from_paseto_keys_url`]; on success the fetched key set
//!   wins over `CRM_PASETO_KEYS` (`tracing::info!`), on failure the
//!   service logs a `tracing::warn!` and falls back to the env path
//!   below — the service always boots. The key set is then **re-fetched
//!   periodically** ([`spawn_key_refresh`]) so a key rotation is picked
//!   up without a restart (interval `CRM_PASETO_KEYS_REFRESH_SECS`,
//!   default 1 h; `0` disables; keeps the current keys on a failed
//!   fetch).
//! - `CRM_PASETO_KEYS` — the Ed25519 key set (JSON, OKP/Ed25519
//!   JWK form) the auth service publishes at `/.well-known/paseto-keys`.
//!   Absent ⇒ an empty key set, so every token is rejected (the service
//!   still boots).
//! - `CRM_TOKEN_ISSUER` — expected `iss` (default
//!   `authentication-service`).
//! - `CRM_TOKEN_AUDIENCE` — expected `aud` (default
//!   `main-x-service`).
//!
//! ## Blanket enforcement
//!
//! When `CRM_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as an Axum
//! middleware layer in `src/app.rs` — requires a valid bearer token on
//! every route except the public health/ping, OpenAPI/Swagger, and
//! Prometheus metrics paths (see [`is_public_path`]). It is **off by
//! default**: unset/blank/junk
//! ⇒ today's behaviour, where the extractor is opt-in per handler and
//! the extractor path proves end-to-end verification.
//! Activation is an operations decision once the SSO token flow is live;
//! see `agents/share/authentication-sessions.md` and
//! `agents/share/jwt-enforcement.md` for the family-wide contract.
//!
//! ## Authorization (ABAC)
//!
//! Inside the same guard — so it applies only when `CRM_REQUIRE_AUTH`
//! is on — a verified token is further checked against an
//! **attribute-based access control** policy per
//! `agents/share/authorization-attributes.md`: the request's action is
//! derived from the HTTP method plus this crate's destructive named
//! POSTs ([`DESTRUCTIVE_POST_SUFFIXES`]), and the shared engine in the
//! `authentication-verifier` crate evaluates the policy over the
//! token's `attrs` claim. The policy is read once per process
//! ([`policy`], built by [`policy_from_env`]) from `CRM_ABAC_POLICY`
//! (inline JSON) or `CRM_ABAC_POLICY_FILE` (path); unset or unparsable
//! ⇒ the built-in default policy (`svc=true` ⇒ everything;
//! `access=admin` ⇒ destructive+write; `access=write` ⇒ write;
//! otherwise read-only) — the service always boots. **401** =
//! missing/bad credential; **403** = valid credential, policy denied
//! (body carries the deciding rule). Relationship data is personal + commercial data:
//! deployments can express e.g. department or purpose-of-use scoping
//! as configured policy rules over the same `attrs` claim, no code
//! change required.

use crate::models::_entities::{contacts, deals};
use authentication_verifier::{Action, Claims, Policy, ReloadablePolicy};
#[cfg(feature = "paseto")]
use authentication_verifier::{ReloadableVerifier, Verifier};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};
use std::collections::BTreeMap;
use std::sync::OnceLock;

mod extract;
mod policy;
mod records;
mod verifier;

pub use extract::*;
pub use policy::*;
pub use records::*;
pub use verifier::*;

/// The resource entity this crate guards, as seen by ABAC policies
/// (the `entity` pseudo-attribute in rule `when` clauses).
pub const ENTITY: &str = "crm";

#[cfg(all(test, feature = "paseto"))]
mod tests;
