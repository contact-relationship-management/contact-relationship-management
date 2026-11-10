//! The row of `sla_policies`. One priority SLA-target row (CRM-R11).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "SlaPolicy")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub priority: String,
    pub first_response_minutes: i32,
    pub resolution_minutes: i32,
    pub calendar_pid: Option<Uuid>,
    /// The most minutes a ticket of this priority may wait in `pending` before the sweep flags it (CRM-R64).
    pub max_wait_minutes: Option<i32>,
    /// What the sweep does past that limit: `flag`, `resolve` or `escalate` (CRM-R69).
    pub wait_limit_action: String,
    /// The worker an `escalate` hands the ticket to.
    pub escalate_to: Option<String>,
    /// Days a ticket may stay `resolved` before the sweep closes it (CRM-R82).
    pub auto_close_days: Option<i32>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
