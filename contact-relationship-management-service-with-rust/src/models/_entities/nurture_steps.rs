//! The row of `nurture_steps`. One ordered, delayed step in a sequence (CRM-R9).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "NurtureStep")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub sequence_pid: Uuid,
    pub position: i32,
    pub delay_hours: i32,
    pub template_ref: String,
    pub subject: Option<String>,
    pub body: Option<String>,
    /// The email template the message is made from (CRM-R36); then `subject` and `body` are absent.
    pub template_pid: Option<Uuid>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
