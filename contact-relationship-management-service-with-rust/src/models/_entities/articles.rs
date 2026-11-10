//! The row of `articles`. One knowledge-base article; published edits bump `version` (CRM-R12).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Article")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub title: String,
    pub body: String,
    pub keywords: Option<String>,
    pub status: String,
    pub version: i32,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
