//! Integration-test crate root: pulls in the request-level suites
//! under [`requests`]. These boot the app against the `test`
//! environment (needs PostgreSQL via `config/test.yaml` /
//! `DATABASE_URL`) and are `#[ignore]`d by default — run with
//! `cargo test -- --ignored`. The auth-activation matrix is
//! `requests/enforcement.rs`; its tests change the settings for their own length
//! (`support/scoped_auth.rs`) and so share this process.

#[path = "support/scoped_auth.rs"]
mod scoped_auth;

mod requests;
