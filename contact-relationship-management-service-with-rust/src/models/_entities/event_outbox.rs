//! The row of `event_outbox`. The transactional-outbox hand-off
//! buffer for the durable event bus (Phase 2; one row per published
//! event, written in the entity mutation's transaction and drained by a
//! relay worker).

// A row type: the field-level shape is documented by the
// migration and the `event_outbox` table.
#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use uuid::Uuid;

/// One persisted outbox row: a canonical envelope awaiting relay to the
/// durable bus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp (set when marked published).
    pub updated_at: DateTimeWithTimeZone,
    /// Auto-increment pk; also the global relay order.
    pub id: i32,
    /// Envelope id — the consumer dedup key.
    pub event_id: Uuid,
    /// The entity name (e.g. `stay`).
    pub entity: String,
    /// The record pid — the bus partition key.
    pub entity_pid: Uuid,
    /// The change kind: `created` / `updated` / `deleted` / `merged`.
    pub kind: String,
    /// When the change occurred (stamped at enqueue).
    pub occurred_at: DateTimeWithTimeZone,
    /// The acting user's pid, or `None` when unauthenticated.
    pub actor: Option<String>,
    /// The envelope schema version.
    pub schema_version: i32,
    /// The full canonical envelope as JSONB.
    pub payload: Json,
    /// `None` until the relay ships the row to the bus.
    pub published_at: Option<DateTimeWithTimeZone>,
}
