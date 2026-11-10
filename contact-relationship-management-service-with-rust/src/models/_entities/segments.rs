//! The row of `segments`. One declarative audience filter (CRM-R7); consent is `ANDed` structurally.

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Segment")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub name: String,
    #[schemars(with = "serde_json::Map<String, serde_json::Value>")]
    pub filter: Json,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
