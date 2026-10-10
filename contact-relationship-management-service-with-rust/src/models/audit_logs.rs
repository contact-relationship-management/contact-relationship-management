//! `audit_logs` model — record and query the audit trail.
//!
//! Every mutation **and every sensitive read** (consent-history and
//! unmasked-amount reads) writes one row. The `snapshot` JSON carries the action
//! detail (old/new state, override reasons, the `owner` where
//! relevant — which is what the owner-scoped query filters on).

use loco_rs::prelude::*;
use sea_orm::{
    ConnectionTrait, DatabaseBackend, QueryOrder, QuerySelect, Statement, TransactionTrait,
};

use crate::rules::audit_chain;
use uuid::Uuid;

pub use super::_entities::audit_logs::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::audit_logs::ActiveModel {}

impl Model {
    /// Record one audit entry. `actor` is the caller's `sub` (user
    /// `pid`) when a verified token was presented, else `None`.
    ///
    /// Generic over [`ConnectionTrait`] so it runs on the handler's
    /// `&DatabaseTransaction` — the audit row commits **with** the
    /// mutation it records (family invariant 8), never separately.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record<C: ConnectionTrait>(
        db: &C,
        entity: &str,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        // Serialize chain writers: the previous hash must be read and the new
        // row appended without another writer slipping in between. The lock
        // is released when the surrounding transaction ends.
        let backend = db.get_database_backend();
        if backend == DatabaseBackend::Postgres {
            db.execute_raw(Statement::from_string(
                backend,
                "SELECT pg_advisory_xact_lock(7203197)".to_string(),
            ))
            .await?;
        }
        let prev_hash = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::Id)
            .one(db)
            .await?
            .and_then(|row| row.hash)
            .unwrap_or_default();
        let entry = audit_logs::ActiveModel {
            entity: ActiveValue::set(entity.to_string()),
            entity_pid: ActiveValue::set(entity_pid),
            action: ActiveValue::set(action.to_string()),
            actor: ActiveValue::set(actor.map(ToString::to_string)),
            snapshot: ActiveValue::set(snapshot),
            prev_hash: ActiveValue::set(Some(prev_hash)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        // Hash the row as the database stored it (jsonb normalises the
        // snapshot), so the verifier recomputes the identical bytes.
        let chain_hash = chain_hash_of(&entry);
        db.execute_raw(Statement::from_sql_and_values(
            backend,
            "UPDATE audit_logs SET hash = $1 WHERE id = $2",
            [chain_hash.clone().into(), entry.id.into()],
        ))
        .await?;
        Ok(Self {
            hash: Some(chain_hash),
            ..entry
        })
    }

    /// Record a **read** of personal data (CRM-T40) in its own short
    /// transaction, so the chain lock is taken and released around just the
    /// audit row. A single-record read passes that record's pid; a list read
    /// passes [`Uuid::nil`] and a summary snapshot (count, never the rows).
    ///
    /// # Errors
    ///
    /// When the transaction or insert fails; callers should fail the read,
    /// because an unaudited disclosure is worse than an error.
    pub async fn record_read(
        db: &DatabaseConnection,
        entity: &str,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;
        let row = Self::record(&txn, entity, entity_pid, action, actor, snapshot).await?;
        txn.commit().await?;
        Ok(row)
    }

    /// Most-recent audit entries, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Audit entries for one record, most-recent first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(db: &DatabaseConnection, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .filter(audit_logs::Column::EntityPid.eq(entity_pid))
            .order_by_desc(audit_logs::Column::Id)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// The owner-scoped query: entries since `since` whose
    /// snapshot names this `owner`, most-recent first, capped at
    /// `limit`. The filter is applied over the snapshot JSON in Rust
    /// (cross-backend; the candidate set is already time-bounded).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_owner_since(
        db: &DatabaseConnection,
        owner: &str,
        since: chrono::DateTime<chrono::FixedOffset>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .filter(audit_logs::Column::CreatedAt.gte(since))
            .order_by_desc(audit_logs::Column::Id)
            .limit(limit.saturating_mul(10)) // headroom before the in-app filter
            .all(db)
            .await?;
        let rows = rows
            .into_iter()
            .filter(|row| {
                row.snapshot
                    .as_ref()
                    .and_then(|s| s.get("owner"))
                    .and_then(serde_json::Value::as_str)
                    == Some(owner)
            })
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect();
        Ok(rows)
    }
}

/// The chain hash of a stored row.
#[must_use]
pub fn chain_hash_of(row: &Model) -> String {
    let snapshot = row
        .snapshot
        .as_ref()
        .map(|value| serde_json::to_string(value).unwrap_or_default());
    audit_chain::hash(&audit_chain::Link {
        id: row.id,
        prev_hash: row.prev_hash.as_deref().unwrap_or(""),
        entity: &row.entity,
        entity_pid: &row.entity_pid.to_string(),
        action: &row.action,
        actor: row.actor.as_deref(),
        snapshot: snapshot.as_deref(),
        created_micros: row.created_at.timestamp_micros(),
    })
}

/// Verify the whole stored chain, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn verify_chain(db: &DatabaseConnection) -> ModelResult<audit_chain::Verdict> {
    let rows = audit_logs::Entity::find()
        .order_by_asc(audit_logs::Column::Id)
        .all(db)
        .await?;
    let prepared: Vec<(String, String, Option<String>)> = rows
        .iter()
        .map(|row| {
            (
                row.entity_pid.to_string(),
                row.prev_hash.clone().unwrap_or_default(),
                row.snapshot
                    .as_ref()
                    .map(|value| serde_json::to_string(value).unwrap_or_default()),
            )
        })
        .collect();
    Ok(audit_chain::verify(rows.iter().zip(&prepared).map(
        |(row, (pid, prev, snapshot))| audit_chain::Stored {
            link: audit_chain::Link {
                id: row.id,
                prev_hash: prev,
                entity: &row.entity,
                entity_pid: pid,
                action: &row.action,
                actor: row.actor.as_deref(),
                snapshot: snapshot.as_deref(),
                created_micros: row.created_at.timestamp_micros(),
            },
            hash: row.hash.as_deref(),
        },
    )))
}
