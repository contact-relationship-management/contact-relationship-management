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
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::models::_entities::{
    accounts, activities, articles, audit_logs, campaigns, contacts, deals, forecast_snapshots,
    leads, memberships, nurture_sequences, nurture_steps, partnerships, pipeline_stages, segments,
    sla_policies, tickets,
};
use crate::models::DateTimeWithTimeZone;

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

/// The live ticket `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn ticket<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<tickets::Model>, sqlx::Error> {
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
        deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live ticket `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn ticket_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<tickets::Model>, sqlx::Error> {
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
        deleted_at IS NULL AND pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// Live tickets in one of `statuses`.
///
/// # Errors
///
/// When the query fails.
pub async fn tickets_with_status<'e>(
    db: impl PgExecutor<'e>,
    statuses: &[&str],
) -> Result<Vec<tickets::Model>, sqlx::Error> {
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
        deleted_at IS NULL AND status = ANY($1) ORDER BY id",
        statuses as &[&str],
    )
    .fetch_all(db)
    .await
}

/// One page of live tickets, of one status when `status` is given, in board order.
///
/// # Errors
///
/// When the query fails.
pub async fn tickets_page<'e>(
    db: impl PgExecutor<'e>,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<tickets::Model>, sqlx::Error> {
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
        deleted_at IS NULL AND ($1::text IS NULL OR status = $1) ORDER BY board_position, id LIMIT \
        $2 OFFSET $3",
        status,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}

/// The live tickets of one status column, in board order.
///
/// # Errors
///
/// When the query fails.
pub async fn ticket_column<'e>(
    db: impl PgExecutor<'e>,
    status: &str,
) -> Result<Vec<tickets::Model>, sqlx::Error> {
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
        deleted_at IS NULL AND status = $1 ORDER BY board_position, id",
        status,
    )
    .fetch_all(db)
    .await
}

/// The live SLA policy `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn sla_policy<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<sla_policies::Model>, sqlx::Error> {
    sqlx::query_as!(
        sla_policies::Model,
        "SELECT created_at, updated_at, id, pid, priority, first_response_minutes, \
        resolution_minutes, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", calendar_pid, \
        max_wait_minutes, wait_limit_action, escalate_to, auto_close_days FROM sla_policies WHERE \
        deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// Every live SLA policy.
///
/// # Errors
///
/// When the query fails.
pub async fn sla_policies<'e>(
    db: impl PgExecutor<'e>,
) -> Result<Vec<sla_policies::Model>, sqlx::Error> {
    sqlx::query_as!(
        sla_policies::Model,
        "SELECT created_at, updated_at, id, pid, priority, first_response_minutes, \
        resolution_minutes, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", calendar_pid, \
        max_wait_minutes, wait_limit_action, escalate_to, auto_close_days FROM sla_policies WHERE \
        deleted_at IS NULL ORDER BY id",
    )
    .fetch_all(db)
    .await
}

/// The live SLA policy of one priority, the oldest when there are several.
///
/// # Errors
///
/// When the query fails.
pub async fn sla_policy_of_priority<'e>(
    db: impl PgExecutor<'e>,
    priority: &str,
) -> Result<Option<sla_policies::Model>, sqlx::Error> {
    sqlx::query_as!(
        sla_policies::Model,
        "SELECT created_at, updated_at, id, pid, priority, first_response_minutes, \
        resolution_minutes, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", calendar_pid, \
        max_wait_minutes, wait_limit_action, escalate_to, auto_close_days FROM sla_policies WHERE \
        deleted_at IS NULL AND priority = $1 ORDER BY id LIMIT 1",
        priority,
    )
    .fetch_optional(db)
    .await
}

/// The live article `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn article<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<articles::Model>, sqlx::Error> {
    sqlx::query_as!(
        articles::Model,
        "SELECT created_at, updated_at, id, pid, title, body, keywords, status, version, deleted_at \
        AS \"deleted_at: DateTimeWithTimeZone\" FROM articles WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// One page of live articles whose title or keywords match `pattern` (a `LIKE` pattern, lower case), when given.
///
/// # Errors
///
/// When the query fails.
pub async fn articles_page<'e>(
    db: impl PgExecutor<'e>,
    pattern: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<articles::Model>, sqlx::Error> {
    sqlx::query_as!(
        articles::Model,
        "SELECT created_at, updated_at, id, pid, title, body, keywords, status, version, deleted_at \
        AS \"deleted_at: DateTimeWithTimeZone\" FROM articles WHERE deleted_at IS NULL AND ($1::text \
        IS NULL OR lower(title) LIKE $1 OR lower(coalesce(keywords, '')) LIKE $1) ORDER BY id LIMIT \
        $2 OFFSET $3",
        pattern,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}

/// Every live published article.
///
/// # Errors
///
/// When the query fails.
pub async fn published_articles_by_id<'e>(
    db: impl PgExecutor<'e>,
) -> Result<Vec<articles::Model>, sqlx::Error> {
    sqlx::query_as!(
        articles::Model,
        "SELECT created_at, updated_at, id, pid, title, body, keywords, status, version, deleted_at \
        AS \"deleted_at: DateTimeWithTimeZone\" FROM articles WHERE deleted_at IS NULL AND status = \
        'published' ORDER BY id",
    )
    .fetch_all(db)
    .await
}

/// The live activities of one subject, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn activities_of<'e>(
    db: impl PgExecutor<'e>,
    kind: &str,
    subject: Uuid,
) -> Result<Vec<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE deleted_at IS NULL AND subject_kind = $1 AND subject_pid = $2 ORDER BY id",
        kind,
        subject,
    )
    .fetch_all(db)
    .await
}

/// The live activity `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn activity<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live contact `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn contact<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live deal `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn deal<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live deal `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn deal_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The open live deals of one stage, in board order.
///
/// # Errors
///
/// When the query fails.
pub async fn deal_column<'e>(
    db: impl PgExecutor<'e>,
    stage: Uuid,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND closed_at IS NULL AND stage_pid = $1 ORDER BY kanban_position, id",
        stage,
    )
    .fetch_all(db)
    .await
}

/// The live lead `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn lead<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<leads::Model>, sqlx::Error> {
    sqlx::query_as!(
        leads::Model,
        "SELECT created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
        email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", board_position, owner_ref FROM leads WHERE deleted_at IS NULL AND pid \
        = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live lead `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn lead_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<leads::Model>, sqlx::Error> {
    sqlx::query_as!(
        leads::Model,
        "SELECT created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
        email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", board_position, owner_ref FROM leads WHERE deleted_at IS NULL AND pid \
        = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live leads of one status column, in board order, the higher score first.
///
/// # Errors
///
/// When the query fails.
pub async fn lead_column<'e>(
    db: impl PgExecutor<'e>,
    status: &str,
) -> Result<Vec<leads::Model>, sqlx::Error> {
    sqlx::query_as!(
        leads::Model,
        "SELECT created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
        email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", board_position, owner_ref FROM leads WHERE deleted_at IS NULL AND \
        status = $1 ORDER BY board_position, score DESC, id",
        status,
    )
    .fetch_all(db)
    .await
}

/// The live article `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn article_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<articles::Model>, sqlx::Error> {
    sqlx::query_as!(
        articles::Model,
        "SELECT created_at, updated_at, id, pid, title, body, keywords, status, version, deleted_at \
        AS \"deleted_at: DateTimeWithTimeZone\" FROM articles WHERE deleted_at IS NULL AND pid = $1 \
        FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live activity `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn activity_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE deleted_at IS NULL AND pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live contact for `person_ref`, the oldest when there are several.
///
/// # Errors
///
/// When the query fails.
pub async fn contact_by_person_ref<'e>(
    db: impl PgExecutor<'e>,
    person_ref: &str,
) -> Result<Option<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL AND person_ref = $1 \
        ORDER BY id LIMIT 1",
        person_ref,
    )
    .fetch_optional(db)
    .await
}

/// The live nurture sequence `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn sequence<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<nurture_sequences::Model>, sqlx::Error> {
    sqlx::query_as!(
        nurture_sequences::Model,
        "SELECT created_at, updated_at, id, pid, name, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", purpose, consent_jurisdiction FROM nurture_sequences WHERE deleted_at \
        IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The first live contacts, oldest first, at most `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn first_contacts<'e>(
    db: impl PgExecutor<'e>,
    limit: i64,
) -> Result<Vec<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL ORDER BY id LIMIT $1",
        limit,
    )
    .fetch_all(db)
    .await
}

/// The newest live leads, at most `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn newest_leads<'e>(
    db: impl PgExecutor<'e>,
    limit: i64,
) -> Result<Vec<leads::Model>, sqlx::Error> {
    sqlx::query_as!(
        leads::Model,
        "SELECT created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
        email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", board_position, owner_ref FROM leads WHERE deleted_at IS NULL ORDER \
        BY id DESC LIMIT $1",
        limit,
    )
    .fetch_all(db)
    .await
}

/// The live campaign `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn campaign<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<campaigns::Model>, sqlx::Error> {
    sqlx::query_as!(
        campaigns::Model,
        "SELECT created_at, updated_at, id, pid, kind, name, status, cost_minor, currency, \
        segment_pid, recipients, delivered, opened, clicked, unsubscribed, deleted_at AS \
        \"deleted_at: DateTimeWithTimeZone\", subject, body, purpose, template_pid, \
        consent_jurisdiction FROM campaigns WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live campaign `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn campaign_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<campaigns::Model>, sqlx::Error> {
    sqlx::query_as!(
        campaigns::Model,
        "SELECT created_at, updated_at, id, pid, kind, name, status, cost_minor, currency, \
        segment_pid, recipients, delivered, opened, clicked, unsubscribed, deleted_at AS \
        \"deleted_at: DateTimeWithTimeZone\", subject, body, purpose, template_pid, \
        consent_jurisdiction FROM campaigns WHERE deleted_at IS NULL AND pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// Every live campaign.
///
/// # Errors
///
/// When the query fails.
pub async fn campaigns<'e>(db: impl PgExecutor<'e>) -> Result<Vec<campaigns::Model>, sqlx::Error> {
    sqlx::query_as!(
        campaigns::Model,
        "SELECT created_at, updated_at, id, pid, kind, name, status, cost_minor, currency, \
        segment_pid, recipients, delivered, opened, clicked, unsubscribed, deleted_at AS \
        \"deleted_at: DateTimeWithTimeZone\", subject, body, purpose, template_pid, \
        consent_jurisdiction FROM campaigns WHERE deleted_at IS NULL ORDER BY id",
    )
    .fetch_all(db)
    .await
}

/// The live segment `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn segment<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<segments::Model>, sqlx::Error> {
    sqlx::query_as!(
        segments::Model,
        "SELECT created_at, updated_at, id, pid, name, filter, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM segments WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// Every live segment.
///
/// # Errors
///
/// When the query fails.
pub async fn segments<'e>(db: impl PgExecutor<'e>) -> Result<Vec<segments::Model>, sqlx::Error> {
    sqlx::query_as!(
        segments::Model,
        "SELECT created_at, updated_at, id, pid, name, filter, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM segments WHERE deleted_at IS NULL ORDER BY id",
    )
    .fetch_all(db)
    .await
}

/// The live won deals attributed to one campaign.
///
/// # Errors
///
/// When the query fails.
pub async fn won_deals_of_campaign<'e>(
    db: impl PgExecutor<'e>,
    campaign: Uuid,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND source_campaign_pid = $1 AND won ORDER BY id",
        campaign,
    )
    .fetch_all(db)
    .await
}

/// Every live nurture sequence.
///
/// # Errors
///
/// When the query fails.
pub async fn sequences<'e>(
    db: impl PgExecutor<'e>,
) -> Result<Vec<nurture_sequences::Model>, sqlx::Error> {
    sqlx::query_as!(
        nurture_sequences::Model,
        "SELECT created_at, updated_at, id, pid, name, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", purpose, consent_jurisdiction FROM nurture_sequences WHERE deleted_at \
        IS NULL ORDER BY id",
    )
    .fetch_all(db)
    .await
}

/// The live steps of one nurture sequence, in order.
///
/// # Errors
///
/// When the query fails.
pub async fn steps_of<'e>(
    db: impl PgExecutor<'e>,
    sequence: Uuid,
) -> Result<Vec<nurture_steps::Model>, sqlx::Error> {
    sqlx::query_as!(
        nurture_steps::Model,
        "SELECT created_at, updated_at, id, pid, sequence_pid, position, delay_hours, template_ref, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", subject, body, template_pid FROM \
        nurture_steps WHERE deleted_at IS NULL AND sequence_pid = $1 ORDER BY position, id",
        sequence,
    )
    .fetch_all(db)
    .await
}
