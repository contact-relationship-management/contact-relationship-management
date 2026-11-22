//! The row of `memberships`. One confederation-membership
//! record per account (joined / status / renewal).

// A row type: documented by the migration.
#![expect(missing_docs, reason = "a row type, documented by the migration")]

use crate::models::DateTimeWithTimeZone;
use crate::time::Date;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub account_pid: Uuid,
    pub joined_on: Date,
    pub status: String,
    pub renewal_on: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
