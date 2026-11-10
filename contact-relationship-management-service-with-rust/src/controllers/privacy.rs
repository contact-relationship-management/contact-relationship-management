//! Subject rights & retention (CRM-R20 / CRM-D14): the subject-access
//! export, erasure-as-anonymisation, and the retention report + sweep.
//! Erasure and the sweep are destructive-classified POSTs (`/erase`,
//! `/sweep` — `access=admin` under enforcement, via
//! [`crate::auth::DESTRUCTIVE_POST_SUFFIXES`]). CRM covers its own
//! store only: what the upstream person/organization/worker services
//! hold is the deployment's coordination duty, stated in the payloads.
//!
//! On `sqlx` since CRM-T213 (CRM-D24). The retention report and sweep run one statement per table
//! of [`rules::SOFT_DELETED_TABLES`] and per log rule, built from those lists at run time so the
//! census and the sweep share one list: every name comes from them, every number is a clamped
//! integer, and they are reviewed by hand. Every other statement is checked and binds its values.

use crate::controllers::PidPath;
use crate::db::{self, Bridge};
use axa::prelude::*;
use serde::Serialize;
use sqlx::{AssertSqlSafe, PgConnection};
use uuid::Uuid;

use super::{record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{
    activities, consent_events, contacts, deals, email_deliveries, external_links, leads,
    nurture_enrollments, tickets,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::rows;
use crate::rules::privacy as rules;

/// The tombstone URN an erased contact's `person_ref` becomes: a
/// syntactically valid `EntityRef` that resolves to no one.
pub(crate) const TOMBSTONE_PERSON: &str = "person:00000000-0000-0000-0000-000000000000";

/// The scrub placeholder for erased free text and names.
const ERASED: &str = "[erased]";

/// Ticket statuses that count as a **live** engagement, blocking
/// erasure (`rules::lifecycle::TICKET`: only these two are non-terminal).
const OPEN_TICKET_STATUSES: [&str; 2] = ["open", "pending"];

/// `GET /api/contacts/{pid}/subject-access`: everything CRM holds keyed to the contact.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct SubjectAccessExport {
    as_of: chrono::DateTime<chrono::Utc>,
    contact: contacts::Model,
    consent_history: Vec<consent_events::Model>,
    /// Where the contact stands on each purpose now.
    consent_purposes: Vec<crate::consents::PurposeState>,
    activities: Vec<activities::Model>,
    leads: Vec<leads::Model>,
    deals_as_primary_contact: Vec<deals::Model>,
    tickets: Vec<tickets::Model>,
    nurture_enrollments: Vec<nurture_enrollments::Model>,
    email_deliveries: Vec<email_deliveries::Model>,
    external_links: Vec<external_links::Model>,
    /// The custom fields set on the contact and on contacts merged into it (CRM-R41).
    #[schemars(with = "Vec<serde_json::Value>")]
    custom_fields: Vec<serde_json::Value>,
    /// The files attached to the contact, to contacts merged into it and to its leads: their names,
    /// types, sizes and digests (CRM-R42). The files themselves are fetched one by one.
    attachments: Vec<super::attachments::AttachmentView>,
    /// The names of the tags on the contact, on contacts merged into it and on its leads (CRM-R47).
    tags: Vec<String>,
    /// The employments the worker service told the CRM of (CRM-R92): the organization and the date.
    employments: Vec<crate::models::employments::Employment>,
    /// The last hire or leave the worker service told the CRM of, by organization, kept to put
    /// its events in order (CRM-R93).
    employment_last_events: Vec<crate::models::employments::LastEvent>,
    /// What the export does not cover, and where to ask for it.
    exclusions: [&'static str; 3],
}

/// `GET /api/contacts/{pid}/subject-access` — everything CRM holds
/// keyed to this contact, in one audited JSON document. Exclusions
/// are named, not hidden: campaign-level counters are aggregate (no
/// per-recipient send log exists to attribute to one contact), and
/// account/organization data is a separate subject.
#[debug_handler]
async fn subject_access(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let pool = db::pool(&ctx);
    let contact = rows::contact(pool, pid).await?.ok_or(Error::NotFound)?;
    let obligations = auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::contact_resource_attrs(&contact),
    )
    .map_err(record_rejection)?;
    // A masked read of a *full export* would be a contradiction — the
    // export exists to disclose everything. Refuse rather than leak:
    // subject access is for the subject (`$sub`) and unmasked-read
    // personas (sales manager/admin/svc), never the masked fallback.
    if obligations.iter().any(|o| o == "mask") {
        return Err(record_rejection((
            axum::http::StatusCode::FORBIDDEN,
            "subject access requires an unmasked read (the export discloses \
             deal amounts and consent history; masked callers cannot receive it)"
                .to_string(),
        )));
    }
    let cpid = contact.pid;
    // Every row keyed to the contact, deleted or not: a soft-deleted row is still the subject's.
    let mut conn = pool.acquire().await?;
    // The consent, field, file, tag and employment helpers are on `ConnectionTrait`; `Bridge` runs
    // them on this connection.
    let export = SubjectAccessExport {
        as_of: crate::clock::now(),
        contact: contact.clone(),
        consent_history: rows::all_consent_events_of(&mut *conn, cpid).await?,
        consent_purposes: crate::consents::all(&Bridge::new(&mut conn), &contact).await?,
        activities: rows::all_activities_about(&mut *conn, cpid).await?,
        leads: rows::all_leads_of_contact(&mut *conn, cpid).await?,
        deals_as_primary_contact: rows::all_deals_of_contact(&mut *conn, cpid).await?,
        tickets: rows::all_tickets_of_contact(&mut *conn, cpid).await?,
        nurture_enrollments: rows::all_enrollments_of_contact(&mut *conn, cpid).await?,
        email_deliveries: rows::all_deliveries_of_contact(&mut *conn, cpid).await?,
        external_links: rows::all_links_of_subject(&mut *conn, cpid).await?,
        custom_fields: super::custom_fields::export_for(&Bridge::new(&mut conn), cpid).await?,
        attachments: super::attachments::export_for(&Bridge::new(&mut conn), cpid).await?,
        tags: super::tags::export_for(&Bridge::new(&mut conn), cpid).await?,
        employments: crate::models::employments::of_contact(&Bridge::new(&mut conn), cpid).await?,
        employment_last_events: crate::models::employments::last_events_of(
            &Bridge::new(&mut conn),
            &contact.person_ref,
        )
        .await?,
        exclusions: [
            "campaign delivery/open/click counters: aggregate counters. Which messages were \
             sent to this contact is in email_deliveries (state, time and message id, no \
             address or body); open and click events are not recorded per contact",
            "the contact's account (organization-level relationship data): a separate \
             subject; request its own export if the account itself is a data subject",
            "upstream person/organization/worker records: held by the identity services, \
             not CRM — coordinate subject access there too",
        ],
    };
    drop(conn);
    Audit::record_read_on(
        pool,
        "contact",
        cpid,
        "subject_access_exported",
        caller.actor(),
        None,
    )
    .await?;
    format::json(export)
}

/// Whether `contact_pid` currently has an open deal (naming it primary
/// contact, not soft-deleted, not yet closed).
async fn has_open_deal(conn: &mut PgConnection, contact_pid: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM deals WHERE primary_contact_pid = $1 \
         AND deleted_at IS NULL AND closed_at IS NULL) AS \"found!\"",
        contact_pid,
    )
    .fetch_one(conn)
    .await?)
}

/// Whether `contact_pid` currently has an open (`open`/`pending`)
/// support ticket.
async fn has_open_ticket(conn: &mut PgConnection, contact_pid: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM tickets WHERE contact_pid = $1 AND deleted_at IS NULL \
         AND status = ANY($2)) AS \"found!\"",
        contact_pid,
        &OPEN_TICKET_STATUSES as &[&str],
    )
    .fetch_one(conn)
    .await?)
}

/// Whether `contact_pid` currently has an active nurture enrolment.
async fn has_active_nurture(conn: &mut PgConnection, contact_pid: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM nurture_enrollments WHERE contact_pid = $1 \
         AND deleted_at IS NULL AND status = 'active') AS \"found!\"",
        contact_pid,
    )
    .fetch_one(conn)
    .await?)
}

/// Delete what is attached to a contact being erased: its files (the rows here; the caller removes
/// the files once the transaction has committed) and its tags' taggings.
async fn erase_attached(txn: &mut PgConnection, contact: Uuid) -> Result<(Vec<Uuid>, u64)> {
    // The file and tag helpers are on `ConnectionTrait`; `Bridge` runs them in this transaction.
    let files = super::attachments::erase_rows(&Bridge::new(txn), contact).await?;
    let tags = super::tags::erase_rows(&Bridge::new(txn), contact).await?;
    Ok((files, tags))
}

/// Scrub what is keyed to the erased contact `cpid` and to the duplicates merged into it, one
/// statement each, in the order of the audit row's counts. Returns the rows each one changed.
#[allow(clippy::too_many_lines)] // one statement for each place the contact's data lives
async fn scrub_linked(txn: &mut PgConnection, cpid: Uuid) -> Result<Vec<u64>> {
    let mut affected = Vec::new();
    // Free text linked to this contact, and rows that are about it only. A plaintext tombstone on
    // purpose: `[erased]` is not personal data.
    affected.push(
        sqlx::query!(
            "UPDATE activities SET summary = $2 WHERE subject_kind = 'contact' \
             AND subject_pid = $1 AND deleted_at IS NULL",
            cpid,
            ERASED,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    affected.push(
        sqlx::query!(
            "UPDATE leads SET display_name = $2, email = NULL \
             WHERE contact_pid = $1 AND deleted_at IS NULL",
            cpid,
            ERASED,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    affected.push(
        sqlx::query!(
            "DELETE FROM working_group_members WHERE contact_pid = $1",
            cpid
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // Notifications about this contact and the duplicates merged into it (CRM-R32). The merged
    // duplicates are deleted contacts, so they are found whatever their `deleted_at`.
    affected.push(
        sqlx::query!(
            "DELETE FROM notifications WHERE subject_kind = 'contact' AND (subject_pid = $1 \
             OR subject_pid IN (SELECT pid FROM contacts WHERE merged_into_pid = $1 \
             /* deleted_at set or not: a merged duplicate is deleted */))",
            cpid,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // Web form submissions of this contact and the duplicates merged into it (CRM-R31).
    affected.push(
        sqlx::query!(
            "DELETE FROM form_submissions WHERE contact_pid = $1 OR contact_pid IN \
             (SELECT pid FROM contacts WHERE merged_into_pid = $1 \
             /* deleted_at set or not: a merged duplicate is deleted */)",
            cpid,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    affected.push(
        sqlx::query!(
            "DELETE FROM external_links WHERE subject_kind = 'contact' AND subject_pid = $1",
            cpid,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // Queued webhook deliveries about this person or the duplicates merged into it (CRM-R28).
    affected.push(
        sqlx::query!(
            "UPDATE webhook_deliveries SET status = 'cancelled', last_error = 'contact erased', \
             updated_at = now() WHERE status = 'pending' AND (entity_pid = $1 OR entity_pid IN \
             (SELECT pid FROM contacts WHERE merged_into_pid = $1 \
             /* deleted_at set or not: a merged duplicate is deleted */))",
            cpid,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // Contacts merged into this one are the same person: scrub their identity too, and the
    // person reference the merge ledger kept (CRM-R26). They are deleted already.
    affected.push(
        sqlx::query!(
            "UPDATE contacts SET display_name = $2, person_ref = $3, job_title = NULL, \
             given_name = NULL, family_name = NULL, marketing_consent = 'withdrawn' \
             WHERE merged_into_pid = $1 /* deleted_at set or not: a merged duplicate is deleted */",
            cpid,
            ERASED,
            TOMBSTONE_PERSON,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    affected.push(
        sqlx::query!(
            "UPDATE contact_merges SET merged_person_ref = $2 WHERE survivor_pid = $1",
            cpid,
            TOMBSTONE_PERSON,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // Every purpose is withdrawn, for this contact and the duplicates merged into it (CRM-R29).
    affected.push(
        sqlx::query!(
            "UPDATE contact_consents SET state = 'withdrawn', changed_at = now() \
             WHERE contact_pid = $1 OR contact_pid IN \
             (SELECT pid FROM contacts WHERE merged_into_pid = $1 \
             /* deleted_at set or not: a merged duplicate is deleted */)",
            cpid,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // Mailed links that still work for this person or the duplicates merged into it (CRM-R79).
    affected.push(
        sqlx::query!(
            "DELETE FROM link_tokens WHERE contact_pid = $1 OR contact_pid IN \
             (SELECT pid FROM contacts WHERE merged_into_pid = $1 \
             /* deleted_at set or not: a merged duplicate is deleted */)",
            cpid,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // What the worker service said of this person's employment, which names them (CRM-R92).
    affected.push(
        sqlx::query!(
            "DELETE FROM contact_employments WHERE contact_pid = $1 OR contact_pid IN \
             (SELECT pid FROM contacts WHERE merged_into_pid = $1 \
             /* deleted_at set or not: a merged duplicate is deleted */)",
            cpid,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // The custom fields' own statement, built and reviewed in `custom_fields` (CRM-R41).
    affected.push(
        sqlx::raw_sql(AssertSqlSafe(super::custom_fields::erase_sql(cpid)))
            .execute(&mut *txn)
            .await?
            .rows_affected(),
    );
    Ok(affected)
}

/// `POST /api/contacts/{pid}/erase` — anonymise (CRM-D14): scrub the
/// contact's identity fields to a tombstone and soft-delete the row,
/// scrub the personal free text linked to it (activity summaries
/// logged directly about the contact, the lead record's name/email),
/// and remove working-group roster entries naming it. Deals, tickets
/// and consent history remain — keyed to a pid that no longer
/// identifies anyone — under the same "financial/operational rows
/// remain" posture the family uses elsewhere (WPM keeps payroll rows;
/// CRM has no monetary field on Contact itself to null, so nothing
/// analogous to WPM's `salary_minor` needs clearing here — the
/// monetary data lives on Deal rows, which stay for revenue-reporting
/// continuity). Refused while a live engagement exists (CRM-R20).
/// Destructive-classified; audited with counts.
#[debug_handler]
async fn erase(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let contact = rows::contact_for_update(&mut *txn, pid)
        .await?
        .ok_or(Error::NotFound)?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Destructive,
        &auth::contact_resource_attrs(&contact),
    )
    .map_err(record_rejection)?;
    let cpid = contact.pid;
    let (open_deal, open_ticket, active_nurture) = (
        has_open_deal(&mut txn, cpid).await?,
        has_open_ticket(&mut txn, cpid).await?,
        has_active_nurture(&mut txn, cpid).await?,
    );
    if !rules::erasable(open_deal, open_ticket, active_nurture) {
        return Err(unprocessable(
            "erasure requires no open deal, no open ticket, and no active nurture \
             enrolment (a live engagement is the lawful basis for holding the data)",
        ));
    }
    // The person references of this contact and the duplicates merged into it, read before they
    // are scrubbed: the order of their employment events is kept by reference (CRM-R93).
    let mut person_refs = vec![contact.person_ref.clone()];
    person_refs.extend(
        sqlx::query_scalar!(
            "SELECT person_ref FROM contacts WHERE merged_into_pid = $1 \
             /* deleted_at set or not: a merged duplicate is deleted */",
            cpid,
        )
        .fetch_all(&mut *txn)
        .await?,
    );
    let employment_events_deleted =
        crate::models::employments::forget_last_events(&Bridge::new(&mut txn), &person_refs)
            .await?;
    // Identity fields scrubbed in place; the row soft-deleted.
    let now: crate::models::DateTimeWithTimeZone = crate::clock::now().into();
    sqlx::query!(
        "UPDATE contacts SET display_name = $2, person_ref = $3, job_title = NULL, \
         given_name = NULL, family_name = NULL, marketing_consent = 'withdrawn', deleted_at = $4 \
         WHERE id = $1 AND deleted_at IS NULL",
        contact.id,
        ERASED,
        TOMBSTONE_PERSON,
        now,
    )
    .execute(&mut *txn)
    .await?;
    let affected = scrub_linked(&mut txn, cpid).await?;
    // The rows go in the transaction; the files go once it has committed.
    let (files, tags_removed) = erase_attached(&mut txn, cpid).await?;
    Audit::record_on(
        &mut txn,
        "contact",
        cpid,
        "erased",
        caller.actor(),
        Some(serde_json::json!({
            "activity_summaries_scrubbed": affected[0],
            "leads_scrubbed": affected[1],
            "working_group_memberships_deleted": affected[2],
            "notifications_deleted": affected[3],
            "form_submissions_deleted": affected[4],
            "external_links_deleted": affected[5],
            "webhook_deliveries_cancelled": affected[6],
            "merged_duplicates_scrubbed": affected[7],
            "purposes_withdrawn": affected[9],
            "link_tokens_deleted": affected[10],
            "employments_deleted": affected[11],
            "employment_last_events_deleted": employment_events_deleted,
            "custom_field_values_deleted": affected[affected.len() - 1],
            "attachments_deleted": files.len(),
            "taggings_deleted": tags_removed,
        })),
    )
    .await?;
    txn.commit().await?;
    super::attachments::remove_files(&files).await;
    format::json(Erased {
        erased: cpid,
        note: "anonymised, not deleted: deals/tickets/consent history remain \
               under CRM's own retention posture, keyed to a pid that no \
               longer identifies anyone; coordinate erasure with the \
               upstream identity services",
    })
}

/// `POST /api/contacts/{pid}/erase`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct Erased {
    erased: Uuid,
    note: &'static str,
}

/// `GET /api/retention`: what the sweep would delete, and what needs an operator.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct RetentionReport {
    as_of: chrono::DateTime<chrono::Utc>,
    horizon_days: i64,
    /// Rows past the horizon, by table (tables with none are left out).
    soft_deleted_past_horizon: std::collections::BTreeMap<String, i64>,
    withdrawn_consent_past_horizon: usize,
    /// Days the event outbox and webhook deliveries are kept (`CRM_EVENT_RETENTION_DAYS`, CRM-R81).
    event_horizon_days: i64,
    /// Days email deliveries and events are kept (`CRM_MAIL_RETENTION_DAYS`, CRM-R81).
    mail_horizon_days: i64,
    /// Delivery and event rows the sweep would delete for their age, by table (none left out).
    logs_past_horizon: std::collections::BTreeMap<String, i64>,
    /// Each horizon's setting, whether the deployer set it, its default and floor, and whose the
    /// default is (CRM-T165): none is any jurisdiction's value.
    horizon_basis: Vec<HorizonBasis>,
    derivation: &'static str,
}

/// One retention horizon and where its value came from (CRM-T165, CRM-D19).
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct HorizonBasis {
    /// The environment setting, such as `CRM_RETENTION_DAYS`.
    setting: &'static str,
    /// The horizon in force, in days.
    days: i64,
    /// Whether the deployer set it (`false`: the default, or a value that could not be read).
    set: bool,
    default_days: i64,
    floor_days: i64,
    /// Whose the default is.
    default_from: rules::DefaultFrom,
}

/// The basis of each of the three horizons in force.
fn horizon_basis() -> Vec<HorizonBasis> {
    let settings = crate::settings::current();
    rules::RETENTION_DEFAULTS
        .iter()
        .map(|entry| {
            let (raw, days) = match entry.setting {
                "CRM_EVENT_RETENTION_DAYS" => (
                    settings.event_retention_days.as_deref(),
                    log_horizon(rules::Horizon::Event),
                ),
                "CRM_MAIL_RETENTION_DAYS" => (
                    settings.mail_retention_days.as_deref(),
                    log_horizon(rules::Horizon::Mail),
                ),
                _ => (settings.retention_days.as_deref(), horizon_days()),
            };
            HorizonBasis {
                setting: entry.setting,
                days,
                set: raw.is_some_and(|value| value.trim().parse::<i64>().is_ok()),
                default_days: entry.default_days,
                floor_days: entry.floor_days,
                default_from: entry.default_from,
            }
        })
        .collect()
}

/// `POST /api/retention/sweep`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct RetentionSweep {
    horizon_days: i64,
    /// Rows deleted, by table (tables with none are left out).
    deleted: std::collections::BTreeMap<String, u64>,
    /// Delivery and event rows deleted for their age, by table (CRM-R81).
    logs_deleted: std::collections::BTreeMap<String, u64>,
    event_horizon_days: i64,
    mail_horizon_days: i64,
    /// Everything deleted, soft-deleted rows and logs together.
    rows_deleted: u64,
}

/// The retention horizon in days (`CRM_RETENTION_DAYS`, default 365,
/// floor 30 — CRM-D14).
fn horizon_days() -> i64 {
    rules::retention_days(crate::settings::current().retention_days.as_deref())
}

/// The horizon in days for a log table's rule (CRM-R81).
fn log_horizon(horizon: rules::Horizon) -> i64 {
    let settings = crate::settings::current();
    match horizon {
        rules::Horizon::Event => {
            rules::event_retention_days(settings.event_retention_days.as_deref())
        }
        rules::Horizon::Mail => rules::mail_retention_days(settings.mail_retention_days.as_deref()),
    }
}

/// The `WHERE` of a log rule at its horizon. The horizon is a clamped integer, never text.
fn log_condition(rule: &rules::LogRule) -> String {
    rule.condition
        .replace("{days}", &log_horizon(rule.horizon).to_string())
}

/// `GET /api/retention` — the report: per table, soft-deleted rows
/// older than the horizon, plus a count of contacts whose marketing
/// consent has stood `withdrawn` since before the horizon. That count
/// is **informational only** — unlike WPM's expired-consent
/// candidates (which the sweep scrubs directly, because a candidate
/// has no employment-style lawful-basis gate), a CRM contact carries
/// the same `/erase` gate (CRM-R20) regardless of consent state, so
/// the sweep never bulk-anonymises contacts; an operator reviews this
/// count and erases individually. Read-only; the sweep is the
/// separate destructive POST.
#[debug_handler]
async fn retention_report(State(ctx): State<AppContext>) -> Result<Response> {
    let days = horizon_days();
    let pool = db::pool(&ctx);
    let mut tables = std::collections::BTreeMap::new();
    for table in rules::SOFT_DELETED_TABLES {
        // Built from the list at run time; reviewed by hand (see the module documentation).
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM {table} WHERE deleted_at < now() - interval '{days} days'"
        )))
        .fetch_one(pool)
        .await?;
        if count > 0 {
            tables.insert((*table).to_string(), count);
        }
    }
    let withdrawn = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"n!\" FROM contacts WHERE deleted_at IS NULL \
         AND marketing_consent = 'withdrawn' AND consent_changed_at < $1",
        crate::clock::now() - chrono::Duration::days(days),
    )
    .fetch_one(pool)
    .await?;
    let withdrawn_consent_past_horizon = usize::try_from(withdrawn).unwrap_or(0);
    let mut logs = std::collections::BTreeMap::new();
    for rule in rules::log_rules(crate::models::deliveries::MAX_ATTEMPTS) {
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM {} WHERE {}",
            rule.table,
            log_condition(&rule)
        )))
        .fetch_one(pool)
        .await?;
        if count > 0 {
            logs.insert(rule.table.to_string(), count);
        }
    }
    format::json(RetentionReport {
        as_of: crate::clock::now(),
        horizon_days: days,
        soft_deleted_past_horizon: tables,
        withdrawn_consent_past_horizon,
        event_horizon_days: log_horizon(rules::Horizon::Event),
        mail_horizon_days: log_horizon(rules::Horizon::Mail),
        logs_past_horizon: logs,
        horizon_basis: horizon_basis(),
        derivation: "soft-deleted rows older than the horizon are hard-deleted by the \
                     sweep; contacts with marketing_consent = withdrawn since before the \
                     horizon are reported (not auto-scrubbed — each still needs the \
                     /erase gate, CRM-R20); the horizon floors at 30 days (CRM-D14); delivery \
                     and event rows that are finished with are deleted by their own, separate \
                     horizons (CRM-R81); no default is any jurisdiction's retention period \
                     (horizon_basis says whose each is, CRM-T165)",
    })
}

/// `POST /api/retention/sweep` — hard-delete soft-deleted rows past
/// the horizon across every soft-deleting table. Destructive-classified;
/// audited with counts.
#[debug_handler]
async fn retention_sweep(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let days = horizon_days();
    let mut txn = db::pool(&ctx).begin().await?;
    let mut deleted = std::collections::BTreeMap::new();
    let mut total: u64 = 0;
    for table in rules::SOFT_DELETED_TABLES {
        // Built from the list at run time; reviewed by hand (see the module documentation).
        let result = sqlx::raw_sql(AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE deleted_at < now() - interval '{days} days'"
        )))
        .execute(&mut *txn)
        .await?;
        if result.rows_affected() > 0 {
            deleted.insert((*table).to_string(), result.rows_affected());
            total += result.rows_affected();
        }
    }
    // Delivery and event logs that are finished with, on their own horizons (CRM-R81).
    let mut logs_deleted = std::collections::BTreeMap::new();
    for rule in rules::log_rules(crate::models::deliveries::MAX_ATTEMPTS) {
        let result = sqlx::raw_sql(AssertSqlSafe(format!(
            "DELETE FROM {} WHERE {}",
            rule.table,
            log_condition(&rule)
        )))
        .execute(&mut *txn)
        .await?;
        if result.rows_affected() > 0 {
            logs_deleted.insert(rule.table.to_string(), result.rows_affected());
            total += result.rows_affected();
        }
    }
    let (event_horizon_days, mail_horizon_days) = (
        log_horizon(rules::Horizon::Event),
        log_horizon(rules::Horizon::Mail),
    );
    Audit::record_on(
        &mut txn,
        "retention",
        Uuid::nil(),
        "retention_swept",
        caller.actor(),
        Some(serde_json::json!({
            "horizon_days": days,
            "rows_deleted": total,
            "logs_deleted": logs_deleted,
            "event_horizon_days": event_horizon_days,
            "mail_horizon_days": mail_horizon_days,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(RetentionSweep {
        horizon_days: days,
        deleted,
        logs_deleted,
        event_horizon_days,
        mail_horizon_days,
        rows_deleted: total,
    })
}

/// The privacy routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/subject-access", get(subject_access))
        .add("/contacts/{pid}/erase", post(erase))
        .add("/retention", get(retention_report))
        .add("/retention/sweep", post(retention_sweep))
}
