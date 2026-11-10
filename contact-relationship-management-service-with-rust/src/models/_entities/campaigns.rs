//! The row of `campaigns`. One campaign with simulated engagement counters (CRM-R8).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Campaign")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub kind: String,
    /// The consent purpose the campaign is sent under.
    pub purpose: String,
    pub name: String,
    pub status: String,
    /// Absent (`null`) when masked.
    #[schemars(with = "Option<i64>")]
    pub cost_minor: i64,
    pub currency: String,
    pub segment_pid: Option<Uuid>,
    pub recipients: i32,
    pub delivered: i32,
    pub opened: i32,
    pub clicked: i32,
    pub unsubscribed: i32,
    /// Message subject; the campaign name when absent.
    pub subject: Option<String>,
    /// Plain-text message body.
    pub body: Option<String>,
    /// The email template the message is made from (CRM-R36); then `subject` and `body` are absent.
    pub template_pid: Option<Uuid>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    /// The jurisdiction code whose consent profile applies to a recipient without one of their own (CRM-D21).
    pub consent_jurisdiction: Option<String>,
}
