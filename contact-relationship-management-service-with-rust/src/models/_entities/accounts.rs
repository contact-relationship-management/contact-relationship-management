//! The row of `accounts`. One relationship wrapper over an `organization:` record (CRM-R1).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Account")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub organization_ref: String,
    pub owner_ref: Option<String>,
    pub display_name: String,
    pub tier: String,
    pub industry: Option<String>,
    pub stakeholder_role: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
