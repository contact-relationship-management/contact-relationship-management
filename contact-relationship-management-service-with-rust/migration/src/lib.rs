//! `sea-orm-migration` schema migrations for
//! `contact-relationship-management-service`.
//!
//! The [`Migrator`] runs the ordered list below at boot: the
//! relationship layer (contacts / accounts / activities / consent),
//! sales (leads / pipelines / deals), marketing (segments / campaigns
//! / nurture), support (SLA / tickets / articles), then the
//! `audit_logs` and `event_outbox` side tables.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_relationships;
mod m20220101_000002_sales;
mod m20220101_000003_marketing;
mod m20220101_000004_support;
mod m20220101_000005_audit_logs;
mod m20220101_000006_event_outbox;
mod m20260720_000007_engagement;
mod m20261010_000008_audit_chain;
mod m20261010_000009_break_glass;
mod m20261011_000010_idempotency_keys;
mod m20261011_000011_email_deliveries;
mod m20261011_000012_email_events;
mod m20261012_000013_nurture_step_content;

/// The crate's migrator.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered migration set.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_relationships::Migration),
            Box::new(m20220101_000002_sales::Migration),
            Box::new(m20220101_000003_marketing::Migration),
            Box::new(m20220101_000004_support::Migration),
            Box::new(m20220101_000005_audit_logs::Migration),
            Box::new(m20220101_000006_event_outbox::Migration),
            Box::new(m20260720_000007_engagement::Migration),
            Box::new(m20261010_000008_audit_chain::Migration),
            Box::new(m20261010_000009_break_glass::Migration),
            Box::new(m20261011_000010_idempotency_keys::Migration),
            Box::new(m20261011_000011_email_deliveries::Migration),
            Box::new(m20261011_000012_email_events::Migration),
            Box::new(m20261012_000013_nurture_step_content::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
