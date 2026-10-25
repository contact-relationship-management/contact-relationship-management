//! `sea-orm-migration` schema migrations for
//! `contact-relationship-management`.
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
mod m20261013_000014_delivery_provider_id;
mod m20261017_000015_board_position;
mod m20261018_000016_touch_updated_at;
mod m20261019_000017_contact_merges;
mod m20261020_000018_contact_unmerge;
mod m20261020_000019_external_links;
mod m20261024_000020_webhooks;
mod m20261024_000021_consent_purposes;
mod m20261024_000022_web_forms;
mod m20261024_000023_notifications;
mod m20261024_000024_search;
mod m20261025_000025_lead_imports;
mod m20261025_000026_views;
mod m20261025_000027_email_templates;
mod m20261025_000028_contact_family_name;
mod m20261025_000029_contact_given_name;
mod m20261025_000030_workflows;
mod m20261025_000031_custom_fields;
mod m20261025_000032_attachments;
mod m20261025_000033_record_imports;
pub mod search_docs;

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
            Box::new(m20261013_000014_delivery_provider_id::Migration),
            Box::new(m20261017_000015_board_position::Migration),
            Box::new(m20261018_000016_touch_updated_at::Migration),
            Box::new(m20261019_000017_contact_merges::Migration),
            Box::new(m20261020_000018_contact_unmerge::Migration),
            Box::new(m20261020_000019_external_links::Migration),
            Box::new(m20261024_000020_webhooks::Migration),
            Box::new(m20261024_000021_consent_purposes::Migration),
            Box::new(m20261024_000022_web_forms::Migration),
            Box::new(m20261024_000023_notifications::Migration),
            Box::new(m20261024_000024_search::Migration),
            Box::new(m20261025_000025_lead_imports::Migration),
            Box::new(m20261025_000026_views::Migration),
            Box::new(m20261025_000027_email_templates::Migration),
            Box::new(m20261025_000028_contact_family_name::Migration),
            Box::new(m20261025_000029_contact_given_name::Migration),
            Box::new(m20261025_000030_workflows::Migration),
            Box::new(m20261025_000031_custom_fields::Migration),
            Box::new(m20261025_000032_attachments::Migration),
            Box::new(m20261025_000033_record_imports::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
