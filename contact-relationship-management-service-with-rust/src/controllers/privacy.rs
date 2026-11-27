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

use crate::arith::OrOverflow;
use crate::arith::Tally;
use crate::controllers::PidPath;
use crate::db;
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
    as_of: crate::time::Timestamp,
    contact: contacts::Model,
    consent_history: Vec<consent_events::Model>,
    /// Where the contact stands on each purpose now.
    consent_purposes: Vec<crate::consents::PurposeState>,
    activities: Vec<activities::Model>,
    leads: Vec<leads::Model>,
    deals_as_primary_contact: Vec<deals::Model>,
    tickets: Vec<tickets::Model>,
    /// Each handover of the contact's tickets, with its reason (CRM-T317).
    ticket_movements: Vec<crate::models::ticket_movements::ContactMovement>,
    /// Each resolution a reopen undid on the contact's tickets (CRM-T317).
    ticket_reopenings: Vec<crate::models::ticket_movements::ContactReopening>,
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
    let export = SubjectAccessExport {
        as_of: crate::clock::now(),
        contact: contact.clone(),
        consent_history: rows::all_consent_events_of(&mut *conn, cpid).await?,
        consent_purposes: crate::consents::all(&mut *conn, &contact).await?,
        activities: rows::all_activities_about(&mut *conn, cpid).await?,
        leads: rows::all_leads_of_contact(&mut *conn, cpid).await?,
        deals_as_primary_contact: rows::all_deals_of_contact(&mut *conn, cpid).await?,
        tickets: rows::all_tickets_of_contact(&mut *conn, cpid).await?,
        ticket_movements: crate::models::ticket_movements::of_contact(&mut *conn, cpid).await?,
        ticket_reopenings: crate::models::ticket_movements::reopenings_of_contact(&mut *conn, cpid)
            .await?,
        nurture_enrollments: rows::all_enrollments_of_contact(&mut *conn, cpid).await?,
        email_deliveries: rows::all_deliveries_of_contact(&mut *conn, cpid).await?,
        external_links: rows::all_links_of_subject(&mut *conn, cpid).await?,
        custom_fields: super::custom_fields::export_for(&mut conn, cpid).await?,
        attachments: super::attachments::export_for(&mut *conn, cpid).await?,
        tags: super::tags::export_for(&mut *conn, cpid).await?,
        employments: crate::models::employments::of_contact(&mut *conn, cpid).await?,
        employment_last_events: crate::models::employments::last_events_of(
            &mut *conn,
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
#[expect(
    clippy::as_conversions,
    reason = "`as &[T]` is sqlx's argument type override, not a conversion"
)]
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
    let files = super::attachments::erase_rows(&mut *txn, contact).await?;
    let tags = super::tags::erase_rows(&mut *txn, contact).await?;
    Ok((files, tags))
}

/// The places [`scrub_linked`] scrubs, one statement each.
const SCRUBBED_PLACES: usize = 15;

/// Scrub what is keyed to the erased contact `cpid` and to the duplicates merged into it, one
/// statement each, in the order of the audit row's counts. Returns the rows each one changed.
#[expect(
    clippy::too_many_lines,
    reason = "one statement for each place the contact's data lives"
)]
async fn scrub_linked(txn: &mut PgConnection, cpid: Uuid) -> Result<[u64; SCRUBBED_PLACES]> {
    let mut affected = Vec::new();
    // Free text linked to this contact, and rows that are about it only. A plaintext tombstone on
    // purpose: `[erased]` is not personal data.
    affected.push(
        sqlx::query!(
            "UPDATE activities SET summary = $2 WHERE deleted_at IS NULL AND (\
             (subject_kind = 'contact' AND subject_pid = $1) OR (subject_kind = 'ticket' AND subject_pid IN \
             (SELECT pid FROM tickets WHERE contact_pid = $1)))",
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
    // The contact's tickets keep their status, dates and numbers but not the free-text title, which
    // can name the person or their circumstances (CRM-T325).
    affected.push(
        sqlx::query!(
            "UPDATE tickets SET title = $2 WHERE contact_pid = $1 AND title <> $2 \
             /* deleted_at set or not: a deleted ticket's title still names the person */",
            cpid,
            ERASED,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // The reasons in the movement history of the contact's tickets are free text (CRM-T317); the
    // append-only trigger lets a reason become the tombstone and nothing else.
    affected.push(
        sqlx::query!(
            "UPDATE ticket_movements SET reason = $2 WHERE reason <> $2 AND ticket_pid IN \
             (SELECT pid FROM tickets WHERE contact_pid = $1 \
             /* deleted_at set or not: a deleted ticket's history still names the person */)",
            cpid,
            ERASED,
        )
        .execute(&mut *txn)
        .await?
        .rows_affected(),
    );
    // One count for each statement above; a different number is a mistake in this function, and no
    // count is invented for a place that is missing.
    <[u64; SCRUBBED_PLACES]>::try_from(affected)
        .map_err(|_| Error::string("the erasure counted an unexpected number of places"))
}

/// `POST /api/contacts/{pid}/erase` — anonymise (CRM-D14): scrub the
/// contact's identity fields to a tombstone and soft-delete the row,
/// scrub the personal free text linked to it (activity summaries
/// logged directly about the contact, the lead record's name/email),
/// and remove working-group roster entries naming it. Ticket titles are
/// scrubbed (CRM-T325). Deals, tickets
/// and consent history remain — keyed to a pid that no longer
/// identifies anyone — under the same "financial/operational rows
/// remain" posture the family uses elsewhere (WPM keeps payroll rows;
/// CRM has no monetary field on Contact itself to null, so nothing
/// analogous to WPM's `salary_minor` needs clearing here — the
/// monetary data lives on Deal rows, which stay for revenue-reporting
/// continuity). Refused while a live engagement exists (CRM-R20).
/// Destructive-classified; audited with counts.
#[debug_handler]
#[expect(
    clippy::too_many_lines,
    reason = "the audit row names the count of every place the erasure touched"
)]
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
        crate::models::employments::forget_last_events(&mut *txn, &person_refs).await?;
    // Identity fields scrubbed in place; the row soft-deleted.
    let now: crate::models::DateTimeWithTimeZone = crate::clock::now();
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
    let [
        activities,
        leads,
        memberships,
        notifications,
        submissions,
        links,
        webhooks,
        merged,
        _,
        purposes,
        tokens,
        employments,
        custom_field_values,
        ticket_titles,
        movement_reasons,
    ] = affected;
    // The rows go in the transaction; the files go once it has committed.
    let (files, tags_removed) = erase_attached(&mut txn, cpid).await?;
    Audit::record_on(
        &mut txn,
        "contact",
        cpid,
        "erased",
        caller.actor(),
        Some(serde_json::json!({
            "activity_summaries_scrubbed": activities,
            "leads_scrubbed": leads,
            "working_group_memberships_deleted": memberships,
            "notifications_deleted": notifications,
            "form_submissions_deleted": submissions,
            "external_links_deleted": links,
            "webhook_deliveries_cancelled": webhooks,
            "merged_duplicates_scrubbed": merged,
            "purposes_withdrawn": purposes,
            "link_tokens_deleted": tokens,
            "employments_deleted": employments,
            "employment_last_events_deleted": employment_events_deleted,
            "custom_field_values_deleted": custom_field_values,
            "ticket_titles_scrubbed": ticket_titles,
            "ticket_movement_reasons_scrubbed": movement_reasons,
            "attachments_deleted": files.len(),
            "taggings_deleted": tags_removed,
        })),
    )
    .await?;
    txn.commit().await?;
    // The rows are committed; a file that could not be removed is reported, never hidden.
    let files_not_removed = super::attachments::remove_files(&files).await;
    format::json(Erased {
        erased: cpid,
        files_not_removed: u64::try_from(files_not_removed).unwrap_or(u64::MAX),
        note: "anonymised, not deleted: deals/tickets/consent history remain \
               under CRM's own retention posture, keyed to a pid that no \
               longer identifies anyone; coordinate erasure with the \
               upstream identity services",
    })
}

/// `POST /api/contacts/{pid}/erase`.
#[expect(
    clippy::struct_field_names,
    reason = "`erased` is the published member name; renaming it would break every client"
)]
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct Erased {
    erased: Uuid,
    /// Attachment files still on disk after their rows were deleted; an operator must remove them.
    files_not_removed: u64,
    note: &'static str,
}

/// `GET /api/retention`: what the sweep would delete, and what needs an operator.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct RetentionReport {
    as_of: crate::time::Timestamp,
    horizon_days: i64,
    /// Rows past the horizon, by table (tables with none are left out).
    soft_deleted_past_horizon: std::collections::BTreeMap<String, i64>,
    withdrawn_consent_past_horizon: usize,
    /// Days the event outbox and webhook deliveries are kept (`CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS`, CRM-R81).
    event_horizon_days: i64,
    /// Days email deliveries and events are kept (`CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS`, CRM-R81).
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
    /// The environment setting, such as `CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS`.
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
fn horizon_basis() -> Result<Vec<HorizonBasis>> {
    let settings = crate::settings::current();
    let basis = rules::RETENTION_DEFAULTS
        .iter()
        .map(|entry| {
            let (raw, days) = match entry.setting {
                "CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS" => (
                    settings.event_retention_days.as_deref(),
                    log_horizon(rules::Horizon::Event)?,
                ),
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS" => (
                    settings.mail_retention_days.as_deref(),
                    log_horizon(rules::Horizon::Mail)?,
                ),
                _ => (settings.retention_days.as_deref(), horizon_days()?),
            };
            Ok(HorizonBasis {
                setting: entry.setting,
                days,
                set: raw.is_some_and(|value| value.trim().parse::<i64>().is_ok()),
                default_days: entry.default_days,
                floor_days: entry.floor_days,
                default_from: entry.default_from,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(basis)
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

/// A retention setting that is no number of days at or above its floor. Start-up refuses it
/// (`Settings::unusable`), so this is reached only when the settings were changed afterwards; it
/// names the setting and never its value.
fn invalid_horizon(setting: &str) -> Error {
    Error::string(&format!(
        "{setting} must be a whole number of days at or above its floor"
    ))
}

/// The retention horizon in days (`CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS`, default 365,
/// floor 30 — CRM-D14).
fn horizon_days() -> Result<i64> {
    rules::retention_days(crate::settings::current().retention_days.as_deref())
        .ok_or_else(|| invalid_horizon("CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS"))
}

/// The horizon in days for a log table's rule (CRM-R81).
fn log_horizon(horizon: rules::Horizon) -> Result<i64> {
    let settings = crate::settings::current();
    match horizon {
        rules::Horizon::Event => {
            rules::event_retention_days(settings.event_retention_days.as_deref()).ok_or_else(|| {
                invalid_horizon("CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS")
            })
        }
        rules::Horizon::Mail => rules::mail_retention_days(settings.mail_retention_days.as_deref())
            .ok_or_else(|| invalid_horizon("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS")),
    }
}

/// The `WHERE` of a log rule at its horizon. The horizon is an integer, never text.
fn log_condition(rule: &rules::LogRule) -> Result<String> {
    Ok(rule
        .condition
        .replace("{days}", &log_horizon(rule.horizon)?.to_string()))
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
    let days = horizon_days()?;
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
    let horizon = crate::clock::now()
        .checked_sub_days(days)
        .or_overflow("consent_horizon")?;
    let withdrawn = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"n!\" FROM contacts WHERE deleted_at IS NULL \
         AND marketing_consent = 'withdrawn' AND consent_changed_at < $1",
        horizon,
    )
    .fetch_one(pool)
    .await?;
    let withdrawn_consent_past_horizon = usize::try_from(withdrawn)
        .ok()
        .or_overflow("withdrawn_consent")?;
    let mut logs = std::collections::BTreeMap::new();
    for rule in rules::log_rules(crate::models::deliveries::MAX_ATTEMPTS) {
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM {} WHERE {}",
            rule.table,
            log_condition(&rule)?
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
        event_horizon_days: log_horizon(rules::Horizon::Event)?,
        mail_horizon_days: log_horizon(rules::Horizon::Mail)?,
        logs_past_horizon: logs,
        horizon_basis: horizon_basis()?,
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
    let days = horizon_days()?;
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
            total = total.tally(result.rows_affected());
        }
    }
    // Delivery and event logs that are finished with, on their own horizons (CRM-R81).
    let mut logs_deleted = std::collections::BTreeMap::new();
    for rule in rules::log_rules(crate::models::deliveries::MAX_ATTEMPTS) {
        let result = sqlx::raw_sql(AssertSqlSafe(format!(
            "DELETE FROM {} WHERE {}",
            rule.table,
            log_condition(&rule)?
        )))
        .execute(&mut *txn)
        .await?;
        if result.rows_affected() > 0 {
            logs_deleted.insert(rule.table.to_string(), result.rows_affected());
            total = total.tally(result.rows_affected());
        }
    }
    let (event_horizon_days, mail_horizon_days) = (
        log_horizon(rules::Horizon::Event)?,
        log_horizon(rules::Horizon::Mail)?,
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
