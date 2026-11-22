//! The domain of Contact Relationship Management (CRM-T260): the business rules and the dates and
//! times they reason about, with no web routes and no database storage.
//!
//! [`rules`] is the pure core (CRM-D3 to D5): lifecycles, scoring, consent, segments, money,
//! calendars, templates, webhooks, imports and the rest. It is DB-free and clock-free; the service
//! (`contact-relationship-management`) reads the database and the clock, calls these rules, and
//! stores and sends what they return. [`time`] holds the instants and dates the rules take; with the
//! `postgres` feature they also read and write Postgres's `timestamptz` and `date`.

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
    clippy::arithmetic_side_effects,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason
)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::disallowed_methods,
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason
    )
)]
// New in Rust 1.99. The tests write `assert!(x.is_empty())`, which reads plainly;
// the lint's replacement needs a typed empty array for each case.
#![cfg_attr(test, allow(clippy::assert_is_empty))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod rules;
pub mod time;
