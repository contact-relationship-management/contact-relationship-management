//! The row of `deals`. One revenue opportunity moving through stages; terminal stages close it (CRM-R4).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use chrono::NaiveDate as Date;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Deal")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub account_pid: Option<Uuid>,
    pub primary_contact_pid: Option<Uuid>,
    pub owner_ref: Option<String>,
    pub pipeline_pid: Uuid,
    pub stage_pid: Uuid,
    pub name: String,
    /// Absent (`null`) when masked: a masked amount is never a fake zero.
    #[schemars(with = "Option<i64>")]
    pub amount_minor: i64,
    pub currency: String,
    pub expected_close_on: Option<Date>,
    pub kanban_position: i32,
    pub source_campaign_pid: Option<Uuid>,
    pub closed_at: Option<DateTimeWithTimeZone>,
    pub won: bool,
    pub lost_reason: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    /// The last stage move or reopen; the deal's creation until the first (CRM-D20).
    pub stage_moved_at: DateTimeWithTimeZone,
}
