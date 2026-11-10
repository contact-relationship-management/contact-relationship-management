//! Whole-row reads on `sqlx` (CRM-D24, CRM-T207). Each finder names every
//! column, so `query_as!` checks the entity's `Model` against the schema at
//! build time; a nullable timestamp names its type, which `query_as!` cannot
//! reach from `Option<DateTime<Utc>>` by `Into`. A soft-deleted row is left
//! out, as `find_live` did, and the rows come in a stated order.
//!
//! A selected column the `Model` lacks, or one whose type differs, fails the
//! build. A column added to a table and not listed here does not: add it to
//! the `Model` and to every finder of that table.

use chrono::{DateTime, Utc};
use sea_orm::prelude::DateTimeWithTimeZone;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::_entities::{
    accounts, activities, articles, audit_logs, contacts, deals, forecast_snapshots, leads,
    memberships, partnerships, pipeline_stages, tickets,
};

/// Every live account.
///
/// # Errors
///
/// When the query fails.
pub async fn accounts(pool: &PgPool) -> Result<Vec<accounts::Model>, sqlx::Error> {
    sqlx::query_as!(
        accounts::Model,
        "SELECT created_at, updated_at, id, pid, organization_ref, owner_ref, display_name, tier, \
        industry, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stakeholder_role FROM accounts \
        WHERE deleted_at IS NULL ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Every live contact.
///
/// # Errors
///
/// When the query fails.
pub async fn contacts(pool: &PgPool) -> Result<Vec<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Every live membership.
///
/// # Errors
///
/// When the query fails.
pub async fn memberships(pool: &PgPool) -> Result<Vec<memberships::Model>, sqlx::Error> {
    sqlx::query_as!(
        memberships::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, joined_on, status, renewal_on, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM memberships WHERE deleted_at IS NULL \
        ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Every live partnership.
///
/// # Errors
///
/// When the query fails.
pub async fn partnerships(pool: &PgPool) -> Result<Vec<partnerships::Model>, sqlx::Error> {
    sqlx::query_as!(
        partnerships::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, kind, stage, summary, started_on, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM partnerships WHERE deleted_at IS NULL \
        ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Every live pipeline stage.
///
/// # Errors
///
/// When the query fails.
pub async fn pipeline_stages(pool: &PgPool) -> Result<Vec<pipeline_stages::Model>, sqlx::Error> {
    sqlx::query_as!(
        pipeline_stages::Model,
        "SELECT created_at, updated_at, id, pid, pipeline_pid, name, position, probability_percent, \
        is_won, is_lost, deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM pipeline_stages WHERE \
        deleted_at IS NULL ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Live deals still open (not closed).
///
/// # Errors
///
/// When the query fails.
pub async fn open_deals(pool: &PgPool) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND closed_at IS NULL ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Live deals closed between `from` and `to`, inclusive.
///
/// # Errors
///
/// When the query fails.
pub async fn deals_closed_between(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND closed_at >= $1 AND closed_at <= $2 ORDER BY id",
        from,
        to,
    )
    .fetch_all(pool)
    .await
}

/// Live deals in one pipeline.
///
/// # Errors
///
/// When the query fails.
pub async fn deals_in_pipeline(
    pool: &PgPool,
    pipeline: Uuid,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND pipeline_pid = $1 ORDER BY id",
        pipeline,
    )
    .fetch_all(pool)
    .await
}

/// Live deals won for one account.
///
/// # Errors
///
/// When the query fails.
pub async fn won_deals_of_account(
    pool: &PgPool,
    account: Uuid,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND account_pid = $1 AND won ORDER BY id",
        account,
    )
    .fetch_all(pool)
    .await
}

/// Live tickets that are `open` or `pending`.
///
/// # Errors
///
/// When the query fails.
pub async fn open_tickets(pool: &PgPool) -> Result<Vec<tickets::Model>, sqlx::Error> {
    sqlx::query_as!(
        tickets::Model,
        "SELECT created_at, updated_at, id, pid, contact_pid, account_pid, assignee_ref, title, \
        priority, channel, status, opened_at, first_response_due_at AS \"first_response_due_at: \
        DateTimeWithTimeZone\", resolution_due_at AS \"resolution_due_at: DateTimeWithTimeZone\", \
        first_responded_at AS \"first_responded_at: DateTimeWithTimeZone\", resolved_at AS \
        \"resolved_at: DateTimeWithTimeZone\", first_response_breached, resolution_breached, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", board_position, paused_at AS \"paused_at: \
        DateTimeWithTimeZone\", first_response_paused_seconds, resolution_paused_seconds, \
        wait_limit_exceeded_at AS \"wait_limit_exceeded_at: DateTimeWithTimeZone\" FROM tickets WHERE \
        deleted_at IS NULL AND status IN ('open', 'pending') ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Live leads that are `new` or `contacted`.
///
/// # Errors
///
/// When the query fails.
pub async fn unworked_leads(pool: &PgPool) -> Result<Vec<leads::Model>, sqlx::Error> {
    sqlx::query_as!(
        leads::Model,
        "SELECT created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
        email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", board_position, owner_ref FROM leads WHERE deleted_at IS NULL AND \
        status IN ('new', 'contacted') ORDER BY id"
    )
    .fetch_all(pool)
    .await
}

/// Live activities not done that carry a due date, of one kind when `kind` is given.
///
/// # Errors
///
/// When the query fails.
pub async fn open_dated_activities(
    pool: &PgPool,
    kind: Option<&str>,
) -> Result<Vec<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE deleted_at IS NULL AND NOT done AND due_on IS NOT NULL AND ($1::text IS NULL OR kind = \
        $1) ORDER BY id",
        kind,
    )
    .fetch_all(pool)
    .await
}

/// Every live forecast snapshot, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn forecast_snapshots(
    pool: &PgPool,
) -> Result<Vec<forecast_snapshots::Model>, sqlx::Error> {
    sqlx::query_as!(
        forecast_snapshots::Model,
        "SELECT created_at, updated_at, id, pid, taken_on, totals, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM forecast_snapshots WHERE deleted_at IS NULL ORDER BY taken_on, id"
    )
    .fetch_all(pool)
    .await
}

/// The audit rows of deals with one of `actions`.
///
/// # Errors
///
/// When the query fails.
pub async fn deal_audits(
    pool: &PgPool,
    actions: &[&str],
) -> Result<Vec<audit_logs::Model>, sqlx::Error> {
    sqlx::query_as!(
        audit_logs::Model,
        "SELECT created_at, updated_at, id, entity, entity_pid, action, actor, snapshot, prev_hash, \
        hash FROM audit_logs WHERE entity = 'deal' AND action = ANY($1) ORDER BY id",
        actions as &[&str],
    )
    .fetch_all(pool)
    .await
}

/// Published, live articles, by title, at most `limit`; with `pattern` (a `LIKE` pattern, already
/// lower-cased and escaped), only those whose title, keywords or body match it.
///
/// # Errors
///
/// When the query fails.
pub async fn published_articles(
    pool: &PgPool,
    pattern: Option<&str>,
    limit: i64,
) -> Result<Vec<articles::Model>, sqlx::Error> {
    sqlx::query_as!(
        articles::Model,
        "SELECT created_at, updated_at, id, pid, title, body, keywords, status, version, deleted_at \
        AS \"deleted_at: DateTimeWithTimeZone\" FROM articles WHERE deleted_at IS NULL AND status = \
        'published' AND ($1::text IS NULL OR lower(title) LIKE $1 OR lower(coalesce(keywords, '')) \
        LIKE $1 OR lower(body) LIKE $1) ORDER BY title, id LIMIT $2",
        pattern,
        limit,
    )
    .fetch_all(pool)
    .await
}

/// One published, live article.
///
/// # Errors
///
/// When the query fails.
pub async fn published_article(
    pool: &PgPool,
    pid: Uuid,
) -> Result<Option<articles::Model>, sqlx::Error> {
    sqlx::query_as!(
        articles::Model,
        "SELECT created_at, updated_at, id, pid, title, body, keywords, status, version, deleted_at \
        AS \"deleted_at: DateTimeWithTimeZone\" FROM articles WHERE deleted_at IS NULL AND status = \
        'published' AND pid = $1",
        pid,
    )
    .fetch_optional(pool)
    .await
}
