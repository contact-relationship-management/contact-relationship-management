//! The row of `activities`. One interaction, attached to any relationship object (CRM-R2).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use chrono::NaiveDate as Date;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Activity")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub subject_kind: String,
    pub subject_pid: Uuid,
    pub kind: String,
    pub occurred_at: DateTimeWithTimeZone,
    pub actor_ref: Option<String>,
    pub summary: crate::models::encrypted::EncryptedText,
    pub due_on: Option<Date>,
    pub assignee_ref: Option<String>,
    pub remind_at: Option<DateTimeWithTimeZone>,
    pub done: bool,
    pub sentiment: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
