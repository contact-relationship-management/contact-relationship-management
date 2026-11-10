//! The row of `partnerships`. Innovation-partnership records on
//! an account (kind + a scouting→pilot→scaled→retired lifecycle).

// A row type: documented by the migration.
#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use chrono::NaiveDate as Date;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub account_pid: Uuid,
    pub kind: String,
    pub stage: String,
    pub summary: String,
    pub started_on: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
