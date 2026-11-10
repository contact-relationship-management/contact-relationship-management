//! The row of `audit_logs`. One row per mutation or sensitive
//! read (who / what / when + a snapshot), across every record kind.

// A row type: the field-level shape is documented by the
// migration and the `audit_logs` table, not by per-field rustdoc.
#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "AuditEntry")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
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
