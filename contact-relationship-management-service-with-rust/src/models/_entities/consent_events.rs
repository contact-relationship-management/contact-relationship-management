//! The row of `consent_events`. One append-only consent history row (CRM-R6) -- compliance evidence, never edited.

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ConsentEvent")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub contact_pid: Uuid,
    pub action: String,
    pub source: String,
    /// What the event was about (`marketing`, `newsletter`, …).
    pub purpose: String,
    /// The channel it was about (`email`, `sms`, `phone`, `post`; CRM-D21). Events before CRM-T166 are `email`.
    pub channel: String,
    pub occurred_at: DateTimeWithTimeZone,
}
