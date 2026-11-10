//! The row of `working_group_members`. The group ↔ contact
//! roster join.

// A row type: documented by the migration.
#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub group_pid: Uuid,
    pub contact_pid: Uuid,
}
