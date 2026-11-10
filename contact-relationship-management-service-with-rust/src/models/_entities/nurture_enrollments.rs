//! The row of `nurture_enrollments`. One contact through a sequence (CRM-R9); the scheduler is idempotent per (enrolment, step).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "NurtureEnrollment")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub sequence_pid: Uuid,
    pub contact_pid: Uuid,
    pub current_step: i32,
    pub next_due_at: Option<DateTimeWithTimeZone>,
    pub status: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
