//! The integration tests, in one binary (CRM-T263): each suite below was its own binary, and every
//! binary links the whole service, so one binary builds and links once. The request-level suites
//! (`requests`), the router order, the request limit and the outbox boot the app against the `test`
//! environment (PostgreSQL via `DB_URL`) and are `#[ignore]`d by default: run them with
//! `cargo test --test main -- --ignored`. A test that changes the settings or the environment for
//! its own length does it through `support/scoped_auth.rs` or `support/scoped_env.rs`, which put
//! them back, and is `#[serial]`, so they share this process with the others.

#[path = "support/scoped_auth.rs"]
mod scoped_auth;
#[path = "support/scoped_env.rs"]
mod scoped_env;

mod command_line;
#[cfg(feature = "keycloak")]
mod keycloak_guard;
mod outbox;
mod rate_limit;
mod requests;
mod router_order;
mod smtp_live;
