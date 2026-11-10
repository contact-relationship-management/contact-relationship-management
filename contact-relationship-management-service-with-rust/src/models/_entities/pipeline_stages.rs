//! The row of `pipeline_stages`. One ordered stage with its win probability and terminal flags (CRM-R4).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "StageRecord")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub pipeline_pid: Uuid,
    pub name: String,
    pub position: i32,
    pub probability_percent: i32,
    pub is_won: bool,
    pub is_lost: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
