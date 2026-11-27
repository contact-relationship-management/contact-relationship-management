//! Who a ticket was handed to, and why (CRM-T317): an append-only history, one row a move, in
//! migration `20261127000000_ticket_movements`. A row is never changed, except that erasure scrubs
//! its free-text reason; it goes only with its ticket.

use axa::prelude::*;
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

/// How a ticket was moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    /// A person, through the API, with their own reason.
    Person,
    /// A sweep, on its policy's wait limit (CRM-R64).
    Escalation,
}

impl Via {
    /// The stored `via`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Escalation => "escalation",
        }
    }
}

/// One move, as the ticket's page shows it.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TicketMovement")]
pub struct Movement {
    /// The assignee before, a `person:` URN, or none.
    pub from_ref: Option<String>,
    /// The assignee after, or none.
    pub to_ref: Option<String>,
    /// `person` or `escalation`.
    pub via: String,
    /// Why, as the person wrote it, or the sweep's own reason.
    pub reason: String,
    /// Who moved it.
    pub actor_ref: Option<String>,
    /// When.
    #[schemars(with = "crate::time::Timestamp")]
    pub created_at: crate::time::Timestamp,
}

/// A move, as [`record`] stores it.
#[derive(Debug, Clone, Copy)]
pub struct Move<'a> {
    /// The ticket moved.
    pub ticket_pid: Uuid,
    /// The assignee before.
    pub from_ref: Option<&'a str>,
    /// The assignee after.
    pub to_ref: Option<&'a str>,
    /// How.
    pub via: Via,
    /// Why.
    pub reason: &'a str,
    /// Who.
    pub actor_ref: Option<&'a str>,
}

/// Record a move.
///
/// # Errors
///
/// When the statement fails.
pub async fn record<'e>(db: impl PgExecutor<'e>, movement: Move<'_>) -> Result<()> {
    sqlx::query!(
        "INSERT INTO ticket_movements (ticket_pid, from_ref, to_ref, via, reason, actor_ref) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        movement.ticket_pid,
        movement.from_ref,
        movement.to_ref,
        movement.via.as_str(),
        movement.reason,
        movement.actor_ref,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The ticket's moves, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn of_ticket<'e>(db: impl PgExecutor<'e>, ticket_pid: Uuid) -> Result<Vec<Movement>> {
    Ok(sqlx::query_as!(
        Movement,
        "SELECT from_ref, to_ref, via, reason, actor_ref, created_at FROM ticket_movements \
         WHERE ticket_pid = $1 ORDER BY id",
        ticket_pid,
    )
    .fetch_all(db)
    .await?)
}
