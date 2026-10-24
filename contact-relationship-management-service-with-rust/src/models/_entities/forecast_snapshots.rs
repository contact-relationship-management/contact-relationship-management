//! `SeaORM` Entity -- `forecast_snapshots`. One frozen forecast roll-up (CRM-R5) -- an output, never an input.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "Snapshot")]
#[sea_orm(table_name = "forecast_snapshots")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub taken_on: Date,
    /// Per-currency totals in minor units; `null` when masked.
    #[schemars(with = "Option<std::collections::BTreeMap<String, i64>>")]
    pub totals: Json,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
