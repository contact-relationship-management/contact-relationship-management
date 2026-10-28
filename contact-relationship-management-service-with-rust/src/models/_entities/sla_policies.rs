//! `SeaORM` Entity -- `sla_policies`. One priority SLA-target row (CRM-R11).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "SlaPolicy")]
#[sea_orm(table_name = "sla_policies")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub priority: String,
    pub first_response_minutes: i32,
    pub resolution_minutes: i32,
    pub calendar_pid: Option<Uuid>,
    /// The most minutes a ticket of this priority may wait in `pending` before the sweep flags it (CRM-R64).
    pub max_wait_minutes: Option<i32>,
    /// What the sweep does past that limit: `flag`, `resolve` or `escalate` (CRM-R69).
    pub wait_limit_action: String,
    /// The worker an `escalate` hands the ticket to.
    pub escalate_to: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
