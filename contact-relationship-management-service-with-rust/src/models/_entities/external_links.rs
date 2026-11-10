//! The row of `external_links`. A link from a contact, account, deal or lead to its record in another system (CRM-R27).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ExternalLink")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub subject_kind: String,
    pub subject_pid: Uuid,
    pub system: String,
    pub url: String,
    pub external_id: Option<String>,
    pub label: Option<String>,
    pub created_by: Option<String>,
}
