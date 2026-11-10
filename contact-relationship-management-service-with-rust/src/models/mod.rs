//! Model layer: the row types (`_entities/`), plain structs since CRM-T231, plus helpers.
//!
//! CRM owns a **normalized relational schema** (CRM-D2) — pipelines,
//! consent history, SLA clocks and money sums are constraint-heavy.
//! [`rows`] has the finders, on `sqlx`; [`records`] parses a pid; [`audit_logs`]
//! and [`event_outbox`] are the audit / durable-event side tables.

pub mod _entities;
pub mod audit_logs;
pub mod deliveries;
pub mod employments;
pub mod encrypted;
pub mod event_outbox;
pub mod records;
pub mod rows;

/// A `timestamptz` as the models carry it: the instant with its offset, the type `SeaORM`'s alias
/// named, kept so the records serialize as they did (CRM-D24).
pub type DateTimeWithTimeZone = chrono::DateTime<chrono::FixedOffset>;
