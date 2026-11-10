//! The row of `forecast_snapshots`. One frozen forecast roll-up (CRM-R5) -- an output, never an input.

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use chrono::NaiveDate as Date;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Snapshot")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub taken_on: Date,
    /// Per-currency totals in minor units; `null` when masked.
    #[schemars(with = "Option<std::collections::BTreeMap<String, i64>>")]
    pub totals: Json,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
