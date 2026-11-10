//! The row of `nurture_sequences`. One drip sequence (CRM-R9).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "NurtureSequence")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub name: String,
    /// The consent purpose the sequence is sent under.
    pub purpose: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    /// The jurisdiction code whose consent profile applies to a recipient without one of their own (CRM-D21).
    pub consent_jurisdiction: Option<String>,
}
