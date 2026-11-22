//! `contact-relationship-management` — a service built on Axum and axa
//! application for **contact relationship management** across four
//! modules: sales automation (contacts / accounts / leads with
//! deterministic scoring / deal pipelines / forecasting), marketing
//! automation (consent-first campaigns / segments / nurture),
//! customer service & support (tickets / SLA / knowledge base), and
//! analytics (derived dashboards).
//!
//! CRM **owns relationship state** (its own tables) and **references
//! identities**: contacts are person-service records, accounts
//! organization-service, reps/agents worker-service — as `EntityRef`
//! URNs, never duplicated demographics. Identity dedup stays with the
//! upstream matchers.
//!
//! ## Modules
//!
//! - [`app`] — the routes, the command line, and start-up.
//! - [`controllers`] — Axum controllers, one per module.
//! - [`rules`] — the **pure core**: lifecycle machines, lead scoring
//!   with breakdown, KPI arithmetic, SLA derivation, segment
//!   evaluation with the structural consent gate.
//! - [`models`] — the row types and their `sqlx` finders.
//! - [`clients`] — upstream service lookups (stub-first).
//! - [`auth`] — offline PASETO verification + ABAC + masking.
//! - [`streaming`] — event envelope + in-memory / outbox transports.
//!
//! See `../spec/index.md` for the living specification.

#![warn(clippy::pedantic)]
// Nothing a request can reach may panic (CRM-T300): no `unwrap`, `expect`, `panic!`,
// `unreachable!`, `todo!` or indexing that can run past the end. Failure is a value, or the
// operation fails closed. Tests may use them freely.
#![warn(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
// New in Rust 1.99. The tests write `assert!(x.is_empty())`, which reads plainly;
// the lint's replacement needs a typed empty array for each case.
#![cfg_attr(test, allow(clippy::assert_is_empty))]
// The spec writes a declared floor as `checked_*` plus `unwrap_or(FLOOR)`, citing its declaration, and
// not as `saturating_*` (RCA-INT-4, RCA-NONE-2); clippy would have the latter.
#![allow(clippy::manual_saturating_arithmetic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
// The hand-written OpenAPI document is one large nested `json!` literal.
#![recursion_limit = "512"]

// The domain rules and their dates and times are their own crate (CRM-T260), so an edit to a route
// or a query does not rebuild them, and their tests build without the service. They are re-exported
// under their old paths, `crate::rules` and `crate::time`.
pub use contact_relationship_management_domain::{rules, time};

pub mod app;
pub mod arith;
pub mod attachments;
pub mod auth;
pub mod break_glass;
pub mod clients;
pub mod clock;
pub mod consent_profiles;
pub mod consents;
pub mod controllers;
pub mod crypto;
pub mod db;
pub mod deployment;
pub mod hardening;
pub mod idempotency;
pub mod idempotency_store;
pub mod json;
pub mod link_tokens;
pub mod mail;
pub mod mcp;
pub mod metrics;
pub(crate) mod middleware;
pub mod models;
pub mod notifications;
pub mod openapi;
pub(crate) mod openapi_operations;
pub mod openapi_schemas;
pub mod problem;
pub mod provider;
pub mod provider_api;
pub mod provider_cloud;
pub mod rate_limit;
pub mod redis_store;
pub mod residency;
pub mod schema;
pub mod search;
pub mod search_docs;
pub mod settings;
pub mod streaming;
pub mod tasks;
pub mod unsubscribe;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
pub mod webhook;
pub mod webhooks;
pub mod workflows;
