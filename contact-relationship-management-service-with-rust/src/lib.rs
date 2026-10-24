//! `contact-relationship-management` — a loco.rs consumer
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
//! - [`app`] — loco `Hooks` wiring (routes, truncate, seed).
//! - [`controllers`] — Axum controllers, one per module.
//! - [`rules`] — the **pure core**: lifecycle machines, lead scoring
//!   with breakdown, KPI arithmetic, SLA derivation, segment
//!   evaluation with the structural consent gate.
//! - [`models`] — `SeaORM` entities + CRUD helpers.
//! - [`clients`] — upstream service lookups (stub-first).
//! - [`auth`] — offline PASETO verification + ABAC + masking.
//! - [`streaming`] — event envelope + in-memory / outbox transports.
//!
//! See `../spec/index.md` for the living specification.

#![warn(clippy::pedantic)]
// `unwrap` and `expect` only where failure is impossible, each allowed in
// place with its reason; tests may use both freely.
#![warn(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
// New in Rust 1.99. The tests write `assert!(x.is_empty())`, which reads plainly;
// the lint's replacement needs a typed empty array for each case.
#![cfg_attr(test, allow(clippy::assert_is_empty))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
// The hand-written OpenAPI document is one large nested `json!` literal.
#![recursion_limit = "512"]

pub mod app;
pub mod auth;
pub mod break_glass;
pub mod clients;
pub mod controllers;
pub mod crypto;
pub mod hardening;
pub mod idempotency;
pub mod idempotency_store;
pub mod json;
pub mod mail;
pub mod metrics;
pub(crate) mod middleware;
pub mod models;
pub mod openapi;
pub(crate) mod openapi_operations;
pub mod openapi_schemas;
pub mod problem;
pub mod provider;
pub mod provider_api;
pub mod provider_cloud;
pub mod rate_limit;
pub mod redis_store;
pub mod rules;
pub mod settings;
pub mod streaming;
pub mod tasks;
pub mod unsubscribe;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
pub mod webhook;
