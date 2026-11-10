//! `event_outbox` model — the transactional-outbox write surface for
//! the durable event bus (Phase 2; `agents/share/event-bus.md` §3).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_in`] performs the
//! in-transaction `INSERT` on the handler's own connection, giving the
//! entity write and the event one commit boundary. [`Model::unpublished`] /
//! [`Model::mark_published`] are the Phase-3 relay's poll + ack
//! (roadmap; the relay itself is not built yet).

use axa::prelude::*;
use sqlx::{PgConnection, PgPool};

use crate::models::DateTimeWithTimeZone;
use uuid::Uuid;

pub use super::_entities::event_outbox::{self, ActiveModel, Entity, Model};
use crate::streaming::{Envelope, EventView};

impl ActiveModelBehavior for super::_entities::event_outbox::ActiveModel {}

/// The column values for one outbox row, derived from a canonical
/// [`Envelope`] plus the enqueue timestamp. A plain struct (separate
/// from the `ActiveModel`) so the derivation is unit-testable DB-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxInsert {
    /// Envelope dedup id.
    pub event_id: Uuid,
    /// Entity name (`employee` / `requisition` / …).
    pub entity: String,
    /// Record pid (bus partition key).
    pub entity_pid: Uuid,
    /// Change kind token (`created`, `employee_hired`, …).
    pub kind: String,
    /// When the change occurred.
    pub occurred_at: DateTimeWithTimeZone,
    /// Actor pid, if any.
    pub actor: Option<String>,
    /// Envelope schema version.
    pub schema_version: i32,
    /// The full envelope as JSON.
    pub payload: serde_json::Value,
}

impl OutboxInsert {
    /// Derive the outbox row from an envelope, stamping `occurred_at`.
    /// Pure and DB-free: no clock, no connection.
    ///
    /// # Errors
    ///
    /// When `env.pid` is not a UUID, or the envelope fails to serialize.
    pub fn from_envelope(env: &Envelope, occurred_at: DateTimeWithTimeZone) -> ModelResult<Self> {
        let entity_pid = Uuid::parse_str(&env.pid).map_err(|e| ModelError::Any(Box::new(e)))?;
        let payload = serde_json::to_value(env).map_err(|e| ModelError::Any(Box::new(e)))?;
        Ok(Self {
            event_id: env.event_id,
            entity: env.entity.clone(),
            entity_pid,
            kind: env.kind.clone(),
            occurred_at,
            actor: env.actor.clone(),
            schema_version: i32::try_from(env.schema_version).unwrap_or(1),
            payload,
        })
    }

    /// Insert the row on the mutation's connection, so the event shares its commit boundary.
    ///
    /// # Errors
    ///
    /// When the insert fails (e.g. a duplicate `event_id`).
    pub async fn insert_in(&self, conn: &mut PgConnection) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO event_outbox \
             (event_id, entity, entity_pid, kind, occurred_at, actor, schema_version, payload) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            self.event_id,
            self.entity,
            self.entity_pid,
            self.kind,
            self.occurred_at,
            self.actor,
            self.schema_version,
            self.payload,
        )
        .execute(conn)
        .await?;
        Ok(())
    }
}

impl Model {
    /// The most recent outbox rows, newest first, projected to the
    /// frozen operator [`EventView`] shape. Drives `/events/recent`
    /// when the transport is `outbox`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(pool: &PgPool, limit: i64) -> Result<Vec<EventView>, sqlx::Error> {
        Ok(sqlx::query_scalar!(
            "SELECT payload FROM event_outbox ORDER BY id DESC LIMIT $1",
            limit,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|payload| serde_json::from_value::<Envelope>(payload).ok())
        .map(|env| EventView::from(&env))
        .collect())
    }

    /// The Phase-3 relay poll: oldest unpublished rows in id order,
    /// claimed with `FOR UPDATE SKIP LOCKED` (SEC-B6). Call inside the
    /// relay's transaction. The table's `webhook_fanned_out_at` is not in the `Model`, so it is not
    /// read here.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn unpublished(
        conn: &mut PgConnection,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT created_at, updated_at, id, event_id, entity, entity_pid, kind, occurred_at, actor, schema_version, payload, published_at AS \"published_at: DateTimeWithTimeZone\" FROM event_outbox WHERE published_at IS NULL ORDER BY id LIMIT $1 \
             FOR UPDATE SKIP LOCKED",
            limit,
        )
        .fetch_all(conn)
        .await
    }

    /// The Phase-3 relay ack: stamp `published_at = now()` on `ids`.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn mark_published(conn: &mut PgConnection, ids: &[i32]) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query!(
            "UPDATE event_outbox SET published_at = now() WHERE id = ANY($1)",
            ids,
        )
        .execute(conn)
        .await?
        .rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::OutboxInsert;
    use crate::streaming::Envelope;
    use chrono::{FixedOffset, TimeZone};
    use uuid::Uuid;

    fn an_envelope(kind: &str, pid: &str) -> Envelope {
        Envelope {
            event_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            schema_version: crate::streaming::SCHEMA_VERSION,
            entity: "employee".to_string(),
            kind: kind.to_string(),
            pid: pid.to_string(),
            seq: 7,
            actor: Some("user-1".to_string()),
            name: "E-1001".to_string(),
            data: None,
        }
    }

    fn an_instant() -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 17, 9, 0, 0)
            .unwrap()
    }

    /// Every field maps and the full envelope survives as the payload.
    #[test]
    fn from_envelope_maps_every_column_and_keeps_the_full_payload() {
        let pid = "0c4f1e2a-0000-4000-8000-000000000000";
        let env = an_envelope("employee_hired", pid);
        let row = OutboxInsert::from_envelope(&env, an_instant()).unwrap();
        assert_eq!(row.event_id, env.event_id);
        assert_eq!(row.entity, "employee");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
        assert_eq!(row.kind, "employee_hired");
        assert_eq!(row.actor.as_deref(), Some("user-1"));
        assert_eq!(row.payload["pid"], pid);
        assert_eq!(row.payload["kind"], "employee_hired");
    }

    /// A non-UUID pid is rejected, never panics.
    #[test]
    fn from_envelope_rejects_a_non_uuid_pid() {
        assert!(
            OutboxInsert::from_envelope(&an_envelope("created", "not-a-uuid"), an_instant())
                .is_err()
        );
    }
}
