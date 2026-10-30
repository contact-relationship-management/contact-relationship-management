//! Contact and account import from a CSV (CRM-R43): preview, commit in batches, undo.
//!
//! Like the lead import (CRM-R34) a preview saves nothing and a commit keeps the pid of every
//! record it makes so the import can be taken back. A contact or an account is a reference to the
//! person or organization service, so each row is looked up there first (the address of a contact
//! goes to the person service and is kept nowhere here). A new contact has no consent, and an
//! import starts no workflows.

use std::collections::HashMap;

use contact_relationship_management_matcher::Verdict;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::lead_imports::{Likeness, RowReport, advise};
use super::paging::{Page, PageParams};
use super::{PidPath, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::clients;
use crate::json::Json;
use crate::models::_entities::{accounts, contacts};
use crate::models::audit_logs::Model as Audit;
use crate::models::live::FindLive;
use crate::rules::forms::domain_of;
use crate::rules::imports::{self as csv_rules, Row};
use crate::rules::record_imports::{
    self as rules, AccountMapping, AccountRow, ContactMapping, ContactRow,
};
use crate::streaming;
use crate::validation::Problems;

/// Existing accounts compared against.
const ACCOUNT_POOL: u64 = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Contact,
    Account,
}

impl Kind {
    const fn word(self) -> &'static str {
        match self {
            Self::Contact => "contact",
            Self::Account => "account",
        }
    }
}

/// `POST /api/contact-imports/preview` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ContactImportPreviewPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct ContactPreviewPayload {
    /// The file's text: a header row, then one contact per row.
    csv: String,
    /// Which column holds what; leave out to get the columns and a suggestion.
    #[serde(default)]
    mapping: Option<ContactMapping>,
}

/// `POST /api/account-imports/preview` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "AccountImportPreviewPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct AccountPreviewPayload {
    csv: String,
    #[serde(default)]
    mapping: Option<AccountMapping>,
}

/// `POST /api/contact-imports` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ContactImportCommitPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct ContactCommitPayload {
    csv: String,
    mapping: ContactMapping,
    /// Where the list came from.
    source_note: String,
    /// The day the list was obtained.
    obtained_on: chrono::NaiveDate,
    /// Leave out rows that look like someone already known.
    #[serde(default)]
    skip_likely_duplicates: bool,
}

/// `POST /api/account-imports` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "AccountImportCommitPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct AccountCommitPayload {
    csv: String,
    mapping: AccountMapping,
    source_note: String,
    obtained_on: chrono::NaiveDate,
    #[serde(default)]
    skip_likely_duplicates: bool,
}

/// The preview of a contact import.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ContactImportPreview")]
pub(crate) struct ContactPreview {
    columns: Vec<String>,
    rows: usize,
    suggested_mapping: ContactMapping,
    /// Present when a mapping was given.
    report: Option<Vec<RowReport>>,
    valid: usize,
    invalid: usize,
    likely_duplicates: usize,
}

/// The preview of an account import.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "AccountImportPreview")]
pub(crate) struct AccountPreview {
    columns: Vec<String>,
    rows: usize,
    suggested_mapping: AccountMapping,
    report: Option<Vec<RowReport>>,
    valid: usize,
    invalid: usize,
    likely_duplicates: usize,
}

/// One import.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "RecordImport")]
pub(crate) struct ImportRecord {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    /// `contact` or `account`.
    kind: String,
    created_by: Option<String>,
    source_note: String,
    obtained_on: chrono::NaiveDate,
    /// `running`, `completed` or `undone`.
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    row_count: i32,
    invalid_count: i32,
    /// Left out as likely duplicates, or because the person or organization was already here.
    skipped_count: i32,
    /// Rows the person or organization service could not answer for.
    failed_count: i32,
    /// Records this import made.
    created_count: usize,
    undone_at: Option<chrono::DateTime<chrono::Utc>>,
    undone_count: Option<i32>,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

const COLUMNS: &str = "pid, kind, created_by, source_note, obtained_on, status, created_at, \
                       row_count, invalid_count, skipped_count, failed_count, created_pids, \
                       undone_at, undone_count";

fn record(row: &sea_orm::QueryResult) -> Result<ImportRecord> {
    let pids: serde_json::Value = row.try_get("", "created_pids")?;
    Ok(ImportRecord {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        kind: row.try_get("", "kind")?,
        created_by: row.try_get("", "created_by")?,
        source_note: row.try_get("", "source_note")?,
        obtained_on: row.try_get("", "obtained_on")?,
        status: row.try_get("", "status")?,
        created_at: row.try_get("", "created_at")?,
        row_count: row.try_get("", "row_count")?,
        invalid_count: row.try_get("", "invalid_count")?,
        skipped_count: row.try_get("", "skipped_count")?,
        failed_count: row.try_get("", "failed_count")?,
        created_count: pids.as_array().map_or(0, Vec::len),
        undone_at: row.try_get("", "undone_at")?,
        undone_count: row.try_get("", "undone_count")?,
    })
}

async fn find_import(
    db: &sea_orm::DatabaseConnection,
    kind: Kind,
    pid: Uuid,
) -> Result<ImportRecord> {
    let row = db
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM record_imports WHERE pid = $1 AND kind = $2"),
            [pid.into(), kind.word().into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    record(&row)
}

fn read(csv: &str) -> Result<csv_rules::Table> {
    csv_rules::parse(csv).map_err(|m| unprocessable(&m))
}

fn invalid(row: usize, reason: String) -> RowReport {
    RowReport {
        row,
        status: "invalid",
        reason: Some(reason),
        name: None,
        likely_duplicates: Vec::new(),
    }
}

/// Judge the rows of a contact file, and find who each valid one looks like.
async fn analyse_contacts(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    table: &csv_rules::Table,
    mapping: &ContactMapping,
) -> Result<(Vec<RowReport>, Vec<Option<ContactRow>>)> {
    let columns = rules::resolve_contact(mapping, &table.headers).map_err(|m| unprocessable(&m))?;
    let judged: Vec<std::result::Result<ContactRow, String>> = table
        .rows
        .iter()
        .map(|cells| rules::judge_contact(cells, &columns, table.headers.len()))
        .collect();
    // The matcher sees the name and the domain of the address, as for leads; the address itself
    // goes no further than the person service.
    let for_advice = judged
        .iter()
        .map(|j| {
            j.as_ref()
                .map(|r| Row {
                    name: r.name.clone(),
                    domain: domain_of(&r.email),
                    company: None,
                })
                .map_err(Clone::clone)
        })
        .collect();
    let (reports, _) = advise(ctx, caller, for_advice).await?;
    Ok((
        reports,
        judged.into_iter().map(std::result::Result::ok).collect(),
    ))
}

/// Judge the rows of an account file: duplicates are an existing account, or an earlier row, with
/// the same name.
async fn analyse_accounts(
    ctx: &AppContext,
    table: &csv_rules::Table,
    mapping: &AccountMapping,
) -> Result<(Vec<RowReport>, Vec<Option<AccountRow>>)> {
    let columns = rules::resolve_account(mapping, &table.headers).map_err(|m| unprocessable(&m))?;
    let mut known: HashMap<String, (Uuid, String)> = HashMap::new();
    for account in accounts::Entity::find_live()
        .limit(ACCOUNT_POOL)
        .all(&ctx.db)
        .await?
    {
        known.insert(
            rules::account_key(&account.display_name),
            (account.pid, account.display_name),
        );
    }
    let mut earlier: HashMap<String, usize> = HashMap::new();
    let mut reports = Vec::with_capacity(table.rows.len());
    let mut rows = Vec::with_capacity(table.rows.len());
    for (offset, cells) in table.rows.iter().enumerate() {
        let number = offset + 1;
        match rules::judge_account(cells, &columns, table.headers.len()) {
            Err(reason) => {
                reports.push(invalid(number, reason));
                rows.push(None);
            }
            Ok(row) => {
                let key = rules::account_key(&row.name);
                let mut likely = Vec::new();
                if let Some((pid, name)) = known.get(&key) {
                    likely.push(Likeness {
                        kind: "account",
                        pid: Some(*pid),
                        row: None,
                        name: name.clone(),
                        probability: 1.0,
                        verdict: Verdict::Match,
                    });
                }
                if let Some(first) = earlier.get(&key) {
                    likely.push(Likeness {
                        kind: "row",
                        pid: None,
                        row: Some(*first),
                        name: row.name.clone(),
                        probability: 1.0,
                        verdict: Verdict::Match,
                    });
                }
                earlier.entry(key).or_insert(number);
                reports.push(RowReport {
                    row: number,
                    status: "valid",
                    reason: None,
                    name: Some(row.name.clone()),
                    likely_duplicates: likely,
                });
                rows.push(Some(row));
            }
        }
    }
    Ok((reports, rows))
}

fn counts(report: Option<&Vec<RowReport>>) -> (usize, usize, usize) {
    report.map_or((0, 0, 0), |r| {
        (
            r.iter().filter(|x| x.status == "valid").count(),
            r.iter().filter(|x| x.status == "invalid").count(),
            r.iter().filter(|x| !x.likely_duplicates.is_empty()).count(),
        )
    })
}

async fn audit_preview(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    kind: Kind,
    rows: usize,
    counts: (usize, usize, usize),
) -> Result<()> {
    Audit::record_read(
        &ctx.db,
        "record_import",
        Uuid::nil(),
        &format!("{}_import_previewed", kind.word()),
        caller.actor(),
        Some(serde_json::json!({ "rows": rows, "valid": counts.0, "invalid": counts.1 })),
    )
    .await?;
    Ok(())
}

/// `POST /api/contact-imports/preview`.
#[debug_handler]
async fn preview_contacts(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ContactPreviewPayload>,
) -> Result<Response> {
    let table = read(&payload.csv)?;
    let report = match &payload.mapping {
        Some(mapping) => Some(analyse_contacts(&ctx, &caller, &table, mapping).await?.0),
        None => None,
    };
    let (valid, invalid, likely) = counts(report.as_ref());
    audit_preview(
        &ctx,
        &caller,
        Kind::Contact,
        table.rows.len(),
        (valid, invalid, likely),
    )
    .await?;
    format::json(ContactPreview {
        suggested_mapping: rules::suggest_contact(&table.headers),
        rows: table.rows.len(),
        columns: table.headers,
        report,
        valid,
        invalid,
        likely_duplicates: likely,
    })
}

/// `POST /api/account-imports/preview`.
#[debug_handler]
async fn preview_accounts(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<AccountPreviewPayload>,
) -> Result<Response> {
    let table = read(&payload.csv)?;
    let report = match &payload.mapping {
        Some(mapping) => Some(analyse_accounts(&ctx, &table, mapping).await?.0),
        None => None,
    };
    let (valid, invalid, likely) = counts(report.as_ref());
    audit_preview(
        &ctx,
        &caller,
        Kind::Account,
        table.rows.len(),
        (valid, invalid, likely),
    )
    .await?;
    format::json(AccountPreview {
        suggested_mapping: rules::suggest_account(&table.headers),
        rows: table.rows.len(),
        columns: table.headers,
        report,
        valid,
        invalid,
        likely_duplicates: likely,
    })
}

/// What a commit needs checked before it starts.
fn check_commit(
    source_note: &str,
    obtained_on: chrono::NaiveDate,
    today: chrono::NaiveDate,
) -> Result<()> {
    let mut problems = Problems::new();
    problems.require_text("source_note", source_note);
    problems.cap_text("source_note", source_note);
    if obtained_on > today {
        problems.push_field("obtained_on", "cannot be in the future");
    }
    problems.finish()
}

/// Write the import record first, so that an import that stops half way can be undone.
async fn open_import(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    kind: Kind,
    source_note: &str,
    obtained_on: chrono::NaiveDate,
    counts: (usize, usize, usize),
) -> Result<Uuid> {
    let import = Uuid::new_v4();
    let toi = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
    ctx.db
        .execute_raw(stmt(
            "INSERT INTO record_imports (pid, kind, created_by, source_note, obtained_on, \
             row_count, invalid_count, skipped_count) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            [
                import.into(),
                kind.word().into(),
                caller.actor().map(ToString::to_string).into(),
                source_note.trim().to_string().into(),
                obtained_on.into(),
                toi(counts.0).into(),
                toi(counts.1).into(),
                toi(counts.2).into(),
            ],
        ))
        .await?;
    Ok(import)
}

async fn close_import(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    kind: Kind,
    import: Uuid,
    totals: (usize, usize, usize, usize),
    note: (&str, chrono::NaiveDate),
) -> Result<()> {
    let (rows, created, skipped_more, failed) = totals;
    let toi = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
    let txn = ctx.db.begin().await?;
    txn.execute_raw(stmt(
        "UPDATE record_imports SET status = 'completed', failed_count = $2, \
         skipped_count = skipped_count + $3 WHERE pid = $1",
        [import.into(), toi(failed).into(), toi(skipped_more).into()],
    ))
    .await?;
    Audit::record(
        &txn,
        "record_import",
        import,
        &format!("{}_import_committed", kind.word()),
        caller.actor(),
        Some(serde_json::json!({
            "rows": rows, "created": created, "skipped_existing": skipped_more,
            "failed": failed, "source_note": note.0.trim(), "obtained_on": note.1,
        })),
    )
    .await?;
    txn.commit().await?;
    Ok(())
}

/// Save one imported contact: no consent, no workflow; audited and announced like any new contact.
async fn insert_contact(
    txn: &sea_orm::DatabaseTransaction,
    caller: &MaybeAuthUser,
    import: Uuid,
    row: &ContactRow,
    urn: String,
) -> Result<String> {
    let saved = contacts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(urn),
        account_pid: ActiveValue::set(None),
        owner_ref: ActiveValue::set(None),
        display_name: ActiveValue::set(row.name.clone()),
        status: ActiveValue::set("active".to_string()),
        job_title: ActiveValue::set(row.job_title.clone()),
        given_name: ActiveValue::set(row.given_name.clone()),
        family_name: ActiveValue::set(row.family_name.clone()),
        preferred_channel: ActiveValue::set("email".to_string().into()),
        marketing_consent: ActiveValue::set("never".to_string()),
        consent_changed_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    created_event(
        txn,
        caller,
        "contact",
        saved.pid,
        &saved.display_name,
        import,
    )
    .await?;
    Ok(saved.pid.to_string())
}

/// Save one imported account.
async fn insert_account(
    txn: &sea_orm::DatabaseTransaction,
    caller: &MaybeAuthUser,
    import: Uuid,
    row: &AccountRow,
    urn: String,
) -> Result<String> {
    let saved = accounts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        organization_ref: ActiveValue::set(urn),
        owner_ref: ActiveValue::set(None),
        display_name: ActiveValue::set(row.name.clone()),
        tier: ActiveValue::set(row.tier.clone()),
        industry: ActiveValue::set(row.industry.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    created_event(
        txn,
        caller,
        "account",
        saved.pid,
        &saved.display_name,
        import,
    )
    .await?;
    Ok(saved.pid.to_string())
}

/// The audit row and the event for a record an import made.
async fn created_event(
    txn: &sea_orm::DatabaseTransaction,
    caller: &MaybeAuthUser,
    entity: &str,
    pid: Uuid,
    name: &str,
    import: Uuid,
) -> Result<()> {
    Audit::record(
        txn,
        entity,
        pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "import": import })),
    )
    .await?;
    streaming::emit_on(
        txn,
        entity,
        "created",
        &pid.to_string(),
        name,
        caller.actor(),
        None,
    )
    .await?;
    Ok(())
}

/// The valid rows to save, and how many were left out as likely duplicates.
fn select_rows<T>(
    reports: &[RowReport],
    judged: Vec<Option<T>>,
    skip_likely_duplicates: bool,
) -> (Vec<T>, usize) {
    let mut keep = Vec::new();
    let mut skipped = 0;
    for (report, row) in reports.iter().zip(judged) {
        let Some(row) = row else { continue };
        if skip_likely_duplicates && !report.likely_duplicates.is_empty() {
            skipped += 1;
        } else {
            keep.push(row);
        }
    }
    (keep, skipped)
}

/// `POST /api/contact-imports`.
#[debug_handler]
async fn commit_contacts(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: axum::http::HeaderMap,
    Json(payload): Json<ContactCommitPayload>,
) -> Result<Response> {
    let today = super::organization::caller_today(&ctx.db, &headers).await?;
    check_commit(&payload.source_note, payload.obtained_on, today)?;
    let table = read(&payload.csv)?;
    let (reports, judged) = analyse_contacts(&ctx, &caller, &table, &payload.mapping).await?;
    let invalid = reports.iter().filter(|r| r.status == "invalid").count();
    let (to_save, skipped) = select_rows(&reports, judged, payload.skip_likely_duplicates);
    let import = open_import(
        &ctx,
        &caller,
        Kind::Contact,
        &payload.source_note,
        payload.obtained_on,
        (table.rows.len(), invalid, skipped),
    )
    .await?;
    let (mut created, mut existing, mut failed) = (0_usize, 0_usize, 0_usize);
    for batch in to_save.chunks(rules::BATCH) {
        // Ask the person service first, outside the transaction.
        let mut resolved = Vec::with_capacity(batch.len());
        for row in batch {
            match clients::resolve_person(&row.name, &row.email).await {
                Ok(urn) => resolved.push((row, urn)),
                Err(why) => {
                    tracing::warn!(%why, "contact import: person lookup failed");
                    failed += 1;
                }
            }
        }
        let txn = ctx.db.begin().await?;
        let mut made = Vec::new();
        for (row, urn) in resolved {
            let held = contacts::Entity::find_live()
                .filter(contacts::Column::PersonRef.eq(urn.clone()))
                .one(&txn)
                .await?;
            if held.is_some() {
                existing += 1;
                continue;
            }
            made.push(insert_contact(&txn, &caller, import, row, urn).await?);
        }
        created += made.len();
        txn.execute_raw(stmt(
            "UPDATE record_imports SET created_pids = created_pids || $2::jsonb WHERE pid = $1",
            [import.into(), serde_json::json!(made).into()],
        ))
        .await?;
        txn.commit().await?;
    }
    close_import(
        &ctx,
        &caller,
        Kind::Contact,
        import,
        (table.rows.len(), created, existing, failed),
        (&payload.source_note, payload.obtained_on),
    )
    .await?;
    format::json(find_import(&ctx.db, Kind::Contact, import).await?)
}

/// `POST /api/account-imports`.
#[debug_handler]
async fn commit_accounts(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: axum::http::HeaderMap,
    Json(payload): Json<AccountCommitPayload>,
) -> Result<Response> {
    let today = super::organization::caller_today(&ctx.db, &headers).await?;
    check_commit(&payload.source_note, payload.obtained_on, today)?;
    let table = read(&payload.csv)?;
    let (reports, judged) = analyse_accounts(&ctx, &table, &payload.mapping).await?;
    let invalid = reports.iter().filter(|r| r.status == "invalid").count();
    let (to_save, skipped) = select_rows(&reports, judged, payload.skip_likely_duplicates);
    let import = open_import(
        &ctx,
        &caller,
        Kind::Account,
        &payload.source_note,
        payload.obtained_on,
        (table.rows.len(), invalid, skipped),
    )
    .await?;
    let (mut created, mut existing, mut failed) = (0_usize, 0_usize, 0_usize);
    for batch in to_save.chunks(rules::BATCH) {
        let mut resolved = Vec::with_capacity(batch.len());
        for row in batch {
            let urn = match &row.organization_ref {
                Some(urn) => Ok(urn.clone()),
                None => clients::resolve_organization(&row.name).await,
            };
            match urn {
                Ok(urn) => resolved.push((row, urn)),
                Err(why) => {
                    tracing::warn!(%why, "account import: organization lookup failed");
                    failed += 1;
                }
            }
        }
        let txn = ctx.db.begin().await?;
        let mut made = Vec::new();
        for (row, urn) in resolved {
            let held = accounts::Entity::find_live()
                .filter(accounts::Column::OrganizationRef.eq(urn.clone()))
                .one(&txn)
                .await?;
            if held.is_some() {
                existing += 1;
                continue;
            }
            made.push(insert_account(&txn, &caller, import, row, urn).await?);
        }
        created += made.len();
        txn.execute_raw(stmt(
            "UPDATE record_imports SET created_pids = created_pids || $2::jsonb WHERE pid = $1",
            [import.into(), serde_json::json!(made).into()],
        ))
        .await?;
        txn.commit().await?;
    }
    close_import(
        &ctx,
        &caller,
        Kind::Account,
        import,
        (table.rows.len(), created, existing, failed),
        (&payload.source_note, payload.obtained_on),
    )
    .await?;
    format::json(find_import(&ctx.db, Kind::Account, import).await?)
}

/// `POST .../undo` response.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "RecordImportUndone")]
pub(crate) struct Undone {
    /// Records removed.
    removed: usize,
    /// Records kept because they have been used since.
    kept: usize,
}

/// A contact that has been used: it has history, a consent decision, or something that refers to it.
const CONTACT_IN_USE: &str = "marketing_consent <> 'never' \
    OR merged_into_pid IS NOT NULL \
    OR EXISTS (SELECT 1 FROM contacts m WHERE m.merged_into_pid = contacts.pid) \
    OR EXISTS (SELECT 1 FROM activities a WHERE a.subject_kind = 'contact' AND a.subject_pid = contacts.pid AND a.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM deals d WHERE d.primary_contact_pid = contacts.pid AND d.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM tickets t WHERE t.contact_pid = contacts.pid AND t.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM leads l WHERE l.contact_pid = contacts.pid AND l.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM nurture_enrollments n WHERE n.contact_pid = contacts.pid) \
    OR EXISTS (SELECT 1 FROM consent_events c WHERE c.contact_pid = contacts.pid) \
    OR EXISTS (SELECT 1 FROM attachments x WHERE x.subject_pid = contacts.pid) \
    OR EXISTS (SELECT 1 FROM custom_field_values v WHERE v.subject_pid = contacts.pid)";

/// An account that has been used.
const ACCOUNT_IN_USE: &str = "EXISTS (SELECT 1 FROM contacts c WHERE c.account_pid = accounts.pid AND c.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM deals d WHERE d.account_pid = accounts.pid AND d.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM tickets t WHERE t.account_pid = accounts.pid AND t.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM partnerships p WHERE p.account_pid = accounts.pid) \
    OR EXISTS (SELECT 1 FROM memberships m WHERE m.account_pid = accounts.pid) \
    OR EXISTS (SELECT 1 FROM activities a WHERE a.subject_kind = 'account' AND a.subject_pid = accounts.pid AND a.deleted_at IS NULL) \
    OR EXISTS (SELECT 1 FROM attachments x WHERE x.subject_pid = accounts.pid) \
    OR EXISTS (SELECT 1 FROM custom_field_values v WHERE v.subject_pid = accounts.pid)";

async fn undo_import(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    kind: Kind,
    pid: Uuid,
) -> Result<Response> {
    if find_import(&ctx.db, kind, pid).await?.status == "undone" {
        return Err(unprocessable("this import has already been undone"));
    }
    let txn = ctx.db.begin().await?;
    // Lock the record so a second undo waits for the first.
    let locked = txn
        .query_one_raw(stmt(
            "SELECT created_pids, status FROM record_imports WHERE pid = $1 FOR UPDATE",
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    if locked.try_get::<String>("", "status")? == "undone" {
        return Err(unprocessable("this import has already been undone"));
    }
    let all: Vec<Uuid> = locked
        .try_get::<serde_json::Value>("", "created_pids")?
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
                .collect()
        })
        .unwrap_or_default();
    let (table, in_use) = match kind {
        Kind::Contact => ("contacts", CONTACT_IN_USE),
        Kind::Account => ("accounts", ACCOUNT_IN_USE),
    };
    let removed = txn
        .query_all_raw(stmt(
            &format!(
                "UPDATE {table} SET deleted_at = now() WHERE pid = ANY($1) AND deleted_at IS NULL \
                 AND NOT ({in_use}) RETURNING pid"
            ),
            [all.clone().into()],
        ))
        .await?;
    for row in &removed {
        let record: Uuid = row.try_get("", "pid")?;
        Audit::record(
            &txn,
            kind.word(),
            record,
            "import_undone",
            caller.actor(),
            None,
        )
        .await?;
    }
    let kept = all.len() - removed.len();
    txn.execute_raw(stmt(
        "UPDATE record_imports SET status = 'undone', undone_at = now(), undone_count = $2 \
         WHERE pid = $1",
        [
            pid.into(),
            i32::try_from(removed.len()).unwrap_or(i32::MAX).into(),
        ],
    ))
    .await?;
    Audit::record(
        &txn,
        "record_import",
        pid,
        &format!("{}_import_undone", kind.word()),
        caller.actor(),
        Some(serde_json::json!({ "removed": removed.len(), "kept": kept })),
    )
    .await?;
    txn.commit().await?;
    format::json(Undone {
        removed: removed.len(),
        kept,
    })
}

async fn list_imports(
    ctx: &AppContext,
    kind: Kind,
    paging: &PageParams,
    uri: &axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(paging, 100)?;
    let (limit, offset) = page.window();
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM record_imports WHERE kind = $1 ORDER BY id DESC \
                 LIMIT $2 OFFSET $3"
            ),
            [
                kind.word().into(),
                i64::try_from(limit).unwrap_or(i64::MAX).into(),
                i64::try_from(offset).unwrap_or(i64::MAX).into(),
            ],
        ))
        .await?;
    let records = rows.iter().map(record).collect::<Result<Vec<_>>>()?;
    let (records, more) = page.split(records);
    page.respond(records, more, uri)
}

macro_rules! kind_routes {
    ($list:ident, $one:ident, $undo:ident, $kind:expr) => {
        #[debug_handler]
        async fn $list(
            State(ctx): State<AppContext>,
            Query(paging): Query<PageParams>,
            uri: axum::http::Uri,
        ) -> Result<Response> {
            list_imports(&ctx, $kind, &paging, &uri).await
        }

        #[debug_handler]
        async fn $one(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
            format::json(find_import(&ctx.db, $kind, pid).await?)
        }

        #[debug_handler]
        async fn $undo(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
        ) -> Result<Response> {
            undo_import(&ctx, &caller, $kind, pid).await
        }
    };
}

kind_routes!(list_contacts, one_contact, undo_contacts, Kind::Contact);
kind_routes!(list_accounts, one_account, undo_accounts, Kind::Account);

/// The contact and account import routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contact-imports", get(list_contacts))
        .add("/contact-imports", post(commit_contacts))
        .add("/contact-imports/preview", post(preview_contacts))
        .add("/contact-imports/{pid}", get(one_contact))
        .add("/contact-imports/{pid}/undo", post(undo_contacts))
        .add("/account-imports", get(list_accounts))
        .add("/account-imports", post(commit_accounts))
        .add("/account-imports/preview", post(preview_accounts))
        .add("/account-imports/{pid}", get(one_account))
        .add("/account-imports/{pid}/undo", post(undo_accounts))
}
