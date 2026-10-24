//! `SeaORM` Entity — `audit_logs`. One row per mutation or sensitive
//! read (who / what / when + a snapshot), across every record kind.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `audit_logs` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "AuditEntry")]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    pub entity: String,
    pub entity_pid: Uuid,
    pub action: String,
    pub actor: Option<String>,
    pub snapshot: Option<Json>,
    /// Hash of the previous chained row (CRM-T31); `None` before the chain.
    pub prev_hash: Option<String>,
    /// This row's chain hash; `None` for rows written before the chain.
    pub hash: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
