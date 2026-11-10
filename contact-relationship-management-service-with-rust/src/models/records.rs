//! Shared record helpers: `ActiveModelBehavior` impls for every domain entity, and
//! [`parse_pid`]. The finders are in [`super::rows`] on `sqlx` (CRM-D24, CRM-T216).

use axa::prelude::*;
use uuid::Uuid;

use super::_entities::{
    accounts, activities, articles, campaigns, consent_events, contacts, deals, email_deliveries,
    forecast_snapshots, leads, nurture_enrollments, nurture_sequences, nurture_steps,
    pipeline_stages, pipelines, segments, sla_policies, tickets,
};

impl ActiveModelBehavior for contacts::ActiveModel {}
impl ActiveModelBehavior for accounts::ActiveModel {}
impl ActiveModelBehavior for activities::ActiveModel {}
impl ActiveModelBehavior for consent_events::ActiveModel {}
impl ActiveModelBehavior for leads::ActiveModel {}
impl ActiveModelBehavior for pipelines::ActiveModel {}
impl ActiveModelBehavior for pipeline_stages::ActiveModel {}
impl ActiveModelBehavior for deals::ActiveModel {}
impl ActiveModelBehavior for forecast_snapshots::ActiveModel {}
impl ActiveModelBehavior for segments::ActiveModel {}
impl ActiveModelBehavior for campaigns::ActiveModel {}
impl ActiveModelBehavior for email_deliveries::ActiveModel {}
impl ActiveModelBehavior for nurture_sequences::ActiveModel {}
impl ActiveModelBehavior for nurture_steps::ActiveModel {}
impl ActiveModelBehavior for nurture_enrollments::ActiveModel {}
impl ActiveModelBehavior for sla_policies::ActiveModel {}
impl ActiveModelBehavior for tickets::ActiveModel {}
impl ActiveModelBehavior for articles::ActiveModel {}

/// Parse a path `pid` string to a [`Uuid`], mapping failure to `404`.
///
/// # Errors
///
/// [`Error::NotFound`] when `pid` is not a UUID.
pub fn parse_pid(pid: &str) -> Result<Uuid> {
    Uuid::parse_str(pid).map_err(|_| Error::NotFound)
}
