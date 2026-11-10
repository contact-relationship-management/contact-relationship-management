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
    accounts, activities, articles, audit_logs, campaigns, consent_events, contacts, deals,
    email_deliveries, external_links, forecast_snapshots, leads, memberships, nurture_enrollments,
    nurture_sequences, nurture_steps, partnerships, pipeline_stages, pipelines, segments,
    sla_policies, tickets, working_groups,
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

/// The live contact `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn contact_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL AND pid = $1 FOR \
        UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// One page of live contacts, of `pids` only when given, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn contacts_page<'e>(
    db: impl PgExecutor<'e>,
    pids: Option<&[Uuid]>,
    limit: i64,
    offset: i64,
) -> Result<Vec<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL AND ($1::uuid[] IS \
        NULL OR pid = ANY($1)) ORDER BY id LIMIT $2 OFFSET $3",
        pids,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}

/// Live contacts other than `pid`, oldest first, at most `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn other_contacts<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
    limit: i64,
) -> Result<Vec<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL AND pid <> $1 ORDER \
        BY id LIMIT $2",
        pid,
        limit,
    )
    .fetch_all(db)
    .await
}

/// The live contacts of one account.
///
/// # Errors
///
/// When the query fails.
pub async fn contacts_of_account<'e>(
    db: impl PgExecutor<'e>,
    account: Uuid,
) -> Result<Vec<contacts::Model>, sqlx::Error> {
    sqlx::query_as!(
        contacts::Model,
        "SELECT created_at, updated_at, id, pid, person_ref, account_pid, owner_ref, display_name, \
        status, job_title, preferred_channel, marketing_consent, consent_changed_at AS \
        \"consent_changed_at: DateTimeWithTimeZone\", deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", stakeholder_role, influence, interest, merged_into_pid, family_name, \
        given_name, consent_jurisdiction FROM contacts WHERE deleted_at IS NULL AND account_pid = $1 \
        ORDER BY id",
        account,
    )
    .fetch_all(db)
    .await
}

/// The live account `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn account<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<accounts::Model>, sqlx::Error> {
    sqlx::query_as!(
        accounts::Model,
        "SELECT created_at, updated_at, id, pid, organization_ref, owner_ref, display_name, tier, \
        industry, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stakeholder_role FROM accounts \
        WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live account `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn account_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<accounts::Model>, sqlx::Error> {
    sqlx::query_as!(
        accounts::Model,
        "SELECT created_at, updated_at, id, pid, organization_ref, owner_ref, display_name, tier, \
        industry, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stakeholder_role FROM accounts \
        WHERE deleted_at IS NULL AND pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// One page of live accounts, of `pids` only when given, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn accounts_page<'e>(
    db: impl PgExecutor<'e>,
    pids: Option<&[Uuid]>,
    limit: i64,
    offset: i64,
) -> Result<Vec<accounts::Model>, sqlx::Error> {
    sqlx::query_as!(
        accounts::Model,
        "SELECT created_at, updated_at, id, pid, organization_ref, owner_ref, display_name, tier, \
        industry, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stakeholder_role FROM accounts \
        WHERE deleted_at IS NULL AND ($1::uuid[] IS NULL OR pid = ANY($1)) ORDER BY id LIMIT $2 \
        OFFSET $3",
        pids,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}

/// The live deals of one account.
///
/// # Errors
///
/// When the query fails.
pub async fn deals_of_account<'e>(
    db: impl PgExecutor<'e>,
    account: Uuid,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND account_pid = $1 ORDER BY id",
        account,
    )
    .fetch_all(db)
    .await
}

/// A subject's live activities, newest first, at most `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn recent_activities_of<'e>(
    db: impl PgExecutor<'e>,
    kind: &str,
    subject: Uuid,
    limit: i64,
) -> Result<Vec<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE deleted_at IS NULL AND subject_kind = $1 AND subject_pid = $2 ORDER BY occurred_at \
        DESC, id DESC LIMIT $3",
        kind,
        subject,
        limit,
    )
    .fetch_all(db)
    .await
}

/// A contact's live deals, newest first, at most `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn recent_deals_of_contact<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
    limit: i64,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND primary_contact_pid = $1 ORDER BY id DESC LIMIT $2",
        contact,
        limit,
    )
    .fetch_all(db)
    .await
}

/// A contact's live tickets, newest first, at most `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn recent_tickets_of_contact<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
    limit: i64,
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
        deleted_at IS NULL AND contact_pid = $1 ORDER BY id DESC LIMIT $2",
        contact,
        limit,
    )
    .fetch_all(db)
    .await
}

/// One page of live activities, of one subject kind and one subject when given, newest first.
///
/// # Errors
///
/// When the query fails.
pub async fn activities_page<'e>(
    db: impl PgExecutor<'e>,
    kind: Option<&str>,
    subject: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE deleted_at IS NULL AND ($1::text IS NULL OR subject_kind = $1) AND ($2::uuid IS NULL \
        OR subject_pid = $2) ORDER BY occurred_at DESC, id DESC LIMIT $3 OFFSET $4",
        kind,
        subject,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}

/// The live partnerships of one account, newest first.
///
/// # Errors
///
/// When the query fails.
pub async fn partnerships_of_account<'e>(
    db: impl PgExecutor<'e>,
    account: Uuid,
) -> Result<Vec<partnerships::Model>, sqlx::Error> {
    sqlx::query_as!(
        partnerships::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, kind, stage, summary, started_on, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM partnerships WHERE deleted_at IS NULL \
        AND account_pid = $1 ORDER BY id DESC",
        account,
    )
    .fetch_all(db)
    .await
}

/// The live partnership `pid`, locked for this transaction.
///
/// # Errors
///
/// When the query fails.
pub async fn partnership_for_update<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<partnerships::Model>, sqlx::Error> {
    sqlx::query_as!(
        partnerships::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, kind, stage, summary, started_on, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM partnerships WHERE deleted_at IS NULL \
        AND pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// Every live working group.
///
/// # Errors
///
/// When the query fails.
pub async fn working_groups<'e>(
    db: impl PgExecutor<'e>,
) -> Result<Vec<working_groups::Model>, sqlx::Error> {
    sqlx::query_as!(
        working_groups::Model,
        "SELECT created_at, updated_at, id, pid, name, purpose, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM working_groups WHERE deleted_at IS NULL ORDER BY id",
    )
    .fetch_all(db)
    .await
}

/// The live working group `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn working_group<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<working_groups::Model>, sqlx::Error> {
    sqlx::query_as!(
        working_groups::Model,
        "SELECT created_at, updated_at, id, pid, name, purpose, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM working_groups WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live activities of some contacts, newest first, at most `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn recent_activities_of_contacts<'e>(
    db: impl PgExecutor<'e>,
    contacts: &[Uuid],
    limit: i64,
) -> Result<Vec<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE deleted_at IS NULL AND subject_kind = 'contact' AND subject_pid = ANY($1) ORDER BY \
        occurred_at DESC, id DESC LIMIT $2",
        contacts as &[Uuid],
        limit,
    )
    .fetch_all(db)
    .await
}

/// Every consent event of one contact.
///
/// # Errors
///
/// When the query fails.
pub async fn all_consent_events_of<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
) -> Result<Vec<consent_events::Model>, sqlx::Error> {
    sqlx::query_as!(
        consent_events::Model,
        "SELECT created_at, updated_at, id, pid, contact_pid, action, source, occurred_at, purpose, \
        channel FROM consent_events WHERE contact_pid = $1 ORDER BY id",
        contact,
    )
    .fetch_all(db)
    .await
}

/// Every activity about one subject pid, deleted or not, for the subject's own export.
///
/// # Errors
///
/// When the query fails.
pub async fn all_activities_about<'e>(
    db: impl PgExecutor<'e>,
    subject: Uuid,
) -> Result<Vec<activities::Model>, sqlx::Error> {
    sqlx::query_as!(
        activities::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, kind, occurred_at, \
        actor_ref, summary, due_on, done, deleted_at AS \"deleted_at: DateTimeWithTimeZone\", \
        sentiment, assignee_ref, remind_at AS \"remind_at: DateTimeWithTimeZone\" FROM activities \
        WHERE subject_pid = $1 ORDER BY id",
        subject,
    )
    .fetch_all(db)
    .await
}

/// Every lead of one contact, deleted or not.
///
/// # Errors
///
/// When the query fails.
pub async fn all_leads_of_contact<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
) -> Result<Vec<leads::Model>, sqlx::Error> {
    sqlx::query_as!(
        leads::Model,
        "SELECT created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
        email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", board_position, owner_ref FROM leads WHERE contact_pid = $1 ORDER BY \
        id",
        contact,
    )
    .fetch_all(db)
    .await
}

/// Every deal naming one contact as primary, deleted or not.
///
/// # Errors
///
/// When the query fails.
pub async fn all_deals_of_contact<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE \
        primary_contact_pid = $1 ORDER BY id",
        contact,
    )
    .fetch_all(db)
    .await
}

/// Every ticket of one contact, deleted or not.
///
/// # Errors
///
/// When the query fails.
pub async fn all_tickets_of_contact<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
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
        contact_pid = $1 ORDER BY id",
        contact,
    )
    .fetch_all(db)
    .await
}

/// Every nurture enrolment of one contact, deleted or not.
///
/// # Errors
///
/// When the query fails.
pub async fn all_enrollments_of_contact<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
) -> Result<Vec<nurture_enrollments::Model>, sqlx::Error> {
    sqlx::query_as!(
        nurture_enrollments::Model,
        "SELECT created_at, updated_at, id, pid, sequence_pid, contact_pid, current_step, next_due_at \
        AS \"next_due_at: DateTimeWithTimeZone\", status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM nurture_enrollments WHERE contact_pid = $1 ORDER BY id",
        contact,
    )
    .fetch_all(db)
    .await
}

/// Every email delivery to one contact, as its `Model` has it. The table's `basis` column (the
/// consent basis a message was sent on, since migration 59) is not in the `Model`, so it is not
/// read here: the subject-access export has never carried it (CRM-T213).
///
/// # Errors
///
/// When the query fails.
pub async fn all_deliveries_of_contact<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
) -> Result<Vec<email_deliveries::Model>, sqlx::Error> {
    sqlx::query_as!(
        email_deliveries::Model,
        "SELECT created_at, updated_at, id, pid, campaign_pid, contact_pid, state, attempts, \
        message_id, provider_id, error, sent_at AS \"sent_at: DateTimeWithTimeZone\" \
        FROM email_deliveries WHERE contact_pid = $1 ORDER BY id",
        contact,
    )
    .fetch_all(db)
    .await
}

/// Every external link of one subject pid.
///
/// # Errors
///
/// When the query fails.
pub async fn all_links_of_subject<'e>(
    db: impl PgExecutor<'e>,
    subject: Uuid,
) -> Result<Vec<external_links::Model>, sqlx::Error> {
    sqlx::query_as!(
        external_links::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, system, url, external_id, \
        label, created_by FROM external_links WHERE subject_pid = $1 ORDER BY id",
        subject,
    )
    .fetch_all(db)
    .await
}

/// The live pipeline `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn pipeline<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<pipelines::Model>, sqlx::Error> {
    sqlx::query_as!(
        pipelines::Model,
        "SELECT created_at, updated_at, id, pid, name, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM pipelines WHERE deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// Every live pipeline.
///
/// # Errors
///
/// When the query fails.
pub async fn pipelines<'e>(db: impl PgExecutor<'e>) -> Result<Vec<pipelines::Model>, sqlx::Error> {
    sqlx::query_as!(
        pipelines::Model,
        "SELECT created_at, updated_at, id, pid, name, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\" FROM pipelines WHERE deleted_at IS NULL ORDER BY id",
    )
    .fetch_all(db)
    .await
}

/// The live stage `pid`.
///
/// # Errors
///
/// When the query fails.
pub async fn stage<'e>(
    db: impl PgExecutor<'e>,
    pid: Uuid,
) -> Result<Option<pipeline_stages::Model>, sqlx::Error> {
    sqlx::query_as!(
        pipeline_stages::Model,
        "SELECT created_at, updated_at, id, pid, pipeline_pid, name, position, probability_percent, \
        is_won, is_lost, deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM pipeline_stages WHERE \
        deleted_at IS NULL AND pid = $1",
        pid,
    )
    .fetch_optional(db)
    .await
}

/// The live stages of one pipeline, in order.
///
/// # Errors
///
/// When the query fails.
pub async fn stages_of<'e>(
    db: impl PgExecutor<'e>,
    pipeline: Uuid,
) -> Result<Vec<pipeline_stages::Model>, sqlx::Error> {
    sqlx::query_as!(
        pipeline_stages::Model,
        "SELECT created_at, updated_at, id, pid, pipeline_pid, name, position, probability_percent, \
        is_won, is_lost, deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM pipeline_stages WHERE \
        deleted_at IS NULL AND pipeline_pid = $1 ORDER BY position, id",
        pipeline,
    )
    .fetch_all(db)
    .await
}

/// The first live stage of one pipeline.
///
/// # Errors
///
/// When the query fails.
pub async fn first_stage_of<'e>(
    db: impl PgExecutor<'e>,
    pipeline: Uuid,
) -> Result<Option<pipeline_stages::Model>, sqlx::Error> {
    sqlx::query_as!(
        pipeline_stages::Model,
        "SELECT created_at, updated_at, id, pid, pipeline_pid, name, position, probability_percent, \
        is_won, is_lost, deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM pipeline_stages WHERE \
        deleted_at IS NULL AND pipeline_pid = $1 ORDER BY position, id LIMIT 1",
        pipeline,
    )
    .fetch_optional(db)
    .await
}

/// The last live stage of one pipeline that is neither won nor lost.
///
/// # Errors
///
/// When the query fails.
pub async fn last_open_stage_of<'e>(
    db: impl PgExecutor<'e>,
    pipeline: Uuid,
) -> Result<Option<pipeline_stages::Model>, sqlx::Error> {
    sqlx::query_as!(
        pipeline_stages::Model,
        "SELECT created_at, updated_at, id, pid, pipeline_pid, name, position, probability_percent, \
        is_won, is_lost, deleted_at AS \"deleted_at: DateTimeWithTimeZone\" FROM pipeline_stages WHERE \
        deleted_at IS NULL AND pipeline_pid = $1 AND NOT is_won AND NOT is_lost ORDER BY position \
        DESC, id DESC LIMIT 1",
        pipeline,
    )
    .fetch_optional(db)
    .await
}

/// One page of live leads, of `pids` only when given, in board order, the higher score first.
///
/// # Errors
///
/// When the query fails.
pub async fn leads_page<'e>(
    db: impl PgExecutor<'e>,
    pids: Option<&[Uuid]>,
    limit: i64,
    offset: i64,
) -> Result<Vec<leads::Model>, sqlx::Error> {
    sqlx::query_as!(
        leads::Model,
        "SELECT created_at, updated_at, id, pid, source, campaign_pid, contact_pid, display_name, \
        email, email_domain, score, campaign_click, unsubscribed, status, deleted_at AS \"deleted_at: \
        DateTimeWithTimeZone\", board_position, owner_ref FROM leads WHERE deleted_at IS NULL AND \
        ($1::uuid[] IS NULL OR pid = ANY($1)) ORDER BY board_position, score DESC, id LIMIT $2 \
        OFFSET $3",
        pids,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}

/// One page of live deals, of one pipeline and of `pids` only when given, in board order.
///
/// # Errors
///
/// When the query fails.
pub async fn deals_page<'e>(
    db: impl PgExecutor<'e>,
    pipeline: Option<Uuid>,
    pids: Option<&[Uuid]>,
    limit: i64,
    offset: i64,
) -> Result<Vec<deals::Model>, sqlx::Error> {
    sqlx::query_as!(
        deals::Model,
        "SELECT created_at, updated_at, id, pid, account_pid, primary_contact_pid, owner_ref, \
        pipeline_pid, stage_pid, name, amount_minor, currency, expected_close_on, kanban_position, \
        source_campaign_pid, closed_at AS \"closed_at: DateTimeWithTimeZone\", won, lost_reason, \
        deleted_at AS \"deleted_at: DateTimeWithTimeZone\", stage_moved_at FROM deals WHERE deleted_at \
        IS NULL AND ($1::uuid IS NULL OR pipeline_pid = $1) AND ($2::uuid[] IS NULL OR pid = \
        ANY($2)) ORDER BY kanban_position, id LIMIT $3 OFFSET $4",
        pipeline,
        pids,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}
