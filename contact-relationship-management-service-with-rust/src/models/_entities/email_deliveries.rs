//! The row of `email_deliveries`. One message to one contact for one campaign (CRM-T39); the idempotency key of a send.

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "EmailDelivery")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub campaign_pid: Uuid,
    pub contact_pid: Uuid,
    pub state: String,
    pub attempts: i32,
    pub message_id: Option<String>,
    pub provider_id: Option<String>,
    pub error: Option<String>,
    pub sent_at: Option<DateTimeWithTimeZone>,
}
