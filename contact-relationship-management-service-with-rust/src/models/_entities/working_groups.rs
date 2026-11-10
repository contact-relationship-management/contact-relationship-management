//! The row of `working_groups`. Cross-member collaboration
//! groups (roster in `working_group_members`).

// A row type: documented by the migration.
#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WorkingGroup")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub name: String,
    pub purpose: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
