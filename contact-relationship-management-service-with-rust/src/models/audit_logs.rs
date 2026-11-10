//! `audit_logs` model — record and query the audit trail.
//!
//! Every mutation **and every sensitive read** (consent-history and
//! unmasked-amount reads) writes one row. The `snapshot` JSON carries the action
//! detail (old/new state, override reasons, the `owner` where
//! relevant — which is what the owner-scoped query filters on).

use axa::prelude::*;
use sqlx::{PgConnection, PgPool};

use crate::rules::audit_chain;
use uuid::Uuid;

pub use super::_entities::audit_logs::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::audit_logs::ActiveModel {}

impl Model {
    /// Record one audit entry in the caller's transaction. `actor` is the caller's `sub` when a
    /// verified token was presented, else `None`. It takes a connection, never the pool, so an
    /// audit row cannot commit apart from its mutation (family invariant 8). Chain writers are
    /// serialized by an advisory lock held until the transaction ends.
    ///
    /// # Errors
    ///
    /// When a statement fails.
    pub async fn record_on(
        conn: &mut PgConnection,
        entity: &str,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> Result<Self, sqlx::Error> {
        // `SELECT` straight from the function would be a `void` column, which the macro refuses.
        sqlx::query!("SELECT 1 AS locked FROM pg_advisory_xact_lock(7203197)")
            .fetch_one(&mut *conn)
            .await?;
        let prev_hash = sqlx::query_scalar!("SELECT hash FROM audit_logs ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut *conn)
            .await?
            .flatten()
            .unwrap_or_default();
        let entry = sqlx::query_as!(
            Self,
            "INSERT INTO audit_logs (entity, entity_pid, action, actor, snapshot, prev_hash) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING created_at, updated_at, id, entity, entity_pid, action, actor, snapshot, \
             prev_hash, hash",
            entity,
            entity_pid,
            action,
            actor,
            snapshot,
            prev_hash,
        )
        .fetch_one(&mut *conn)
        .await?;
        // As in `record`: hash the row as stored, because jsonb normalises the snapshot.
        let chain_hash = chain_hash_of(&entry);
        sqlx::query!(
            "UPDATE audit_logs SET hash = $1 WHERE id = $2",
            chain_hash,
            entry.id,
        )
        .execute(&mut *conn)
        .await?;
        Ok(Self {
            hash: Some(chain_hash),
            ..entry
        })
    }

    /// Record a **read** of personal data (CRM-T40) in its own short transaction, so the chain
    /// lock is taken and released around just the audit row. A single-record read passes that
    /// record's pid; a list read passes [`Uuid::nil`] and a summary snapshot (count, never the
    /// rows).
    ///
    /// # Errors
    ///
    /// When the transaction or insert fails; callers should fail the read.
    pub async fn record_read_on(
        pool: &sqlx::PgPool,
        entity: &str,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> Result<Self, sqlx::Error> {
        let mut txn = pool.begin().await?;
        let row = Self::record_on(&mut txn, entity, entity_pid, action, actor, snapshot).await?;
        txn.commit().await?;
        Ok(row)
    }

    /// Most-recent audit entries, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(pool: &PgPool, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT created_at, updated_at, id, entity, entity_pid, action, actor, snapshot, prev_hash, hash FROM audit_logs ORDER BY id DESC LIMIT $1",
            limit,
        )
        .fetch_all(pool)
        .await
    }

    /// Audit entries for one record, most-recent first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(pool: &PgPool, entity_pid: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT created_at, updated_at, id, entity, entity_pid, action, actor, snapshot, prev_hash, hash FROM audit_logs WHERE entity_pid = $1 ORDER BY id DESC",
            entity_pid,
        )
        .fetch_all(pool)
        .await
    }

    /// The owner-scoped query: entries since `since` whose snapshot names this `owner`,
    /// most-recent first, capped at `limit`. The filter is applied over the snapshot JSON in Rust
    /// (the candidate set is already time-bounded).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_owner_since(
        pool: &PgPool,
        owner: &str,
        since: chrono::DateTime<chrono::FixedOffset>,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let rows = sqlx::query_as!(
            Self,
            "SELECT created_at, updated_at, id, entity, entity_pid, action, actor, snapshot, prev_hash, hash FROM audit_logs WHERE created_at >= $1 ORDER BY id DESC LIMIT $2",
            since,
            limit.saturating_mul(10), // headroom before the in-app filter
        )
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter(|row| {
                row.snapshot
                    .as_ref()
                    .and_then(|s| s.get("owner"))
                    .and_then(serde_json::Value::as_str)
                    == Some(owner)
            })
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
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
pub async fn verify_chain(pool: &PgPool) -> Result<audit_chain::Verdict, sqlx::Error> {
    let rows = sqlx::query_as!(Model, "SELECT created_at, updated_at, id, entity, entity_pid, action, actor, snapshot, prev_hash, hash FROM audit_logs ORDER BY id")
        .fetch_all(pool)
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
