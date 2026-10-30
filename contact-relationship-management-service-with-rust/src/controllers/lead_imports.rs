//! Lead import from a CSV (CRM-R34): preview, commit in batches, undo.
//!
//! The file arrives as text in a JSON body. A preview saves nothing and reports each row;
//! a commit saves the valid rows as `source=import` leads and keeps their pids on an import
//! record so the whole import can be taken back. Duplicate advice comes from the matcher and
//! never blocks a row. Imported leads are not contacts and carry no consent.

use std::collections::{HashMap, HashSet};

use contact_relationship_management_matcher::{Contact as MatchContact, Verdict};
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::paging::{Page, PageParams};
use super::relationships::{DUPLICATE_POOL, account_names, match_contact, matcher};
use super::sales::{NewLead, capture_lead};
use super::{PidPath, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::json::Json;
use crate::models::_entities::{contacts, leads};
use crate::models::audit_logs::Model as Audit;
use crate::models::live::FindLive;
use crate::rules::imports::{self as rules, Mapping};
use crate::validation::Problems;

/// Existing leads compared against, newest first.
const LEAD_POOL: u64 = 2000;
/// Candidates compared per row: those sharing a word of the name, at most this many.
const PER_ROW: usize = 200;
/// Duplicates reported per row.
const ADVICE: usize = 3;

/// `POST /api/lead-imports/preview` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImportPreviewPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewPayload {
    /// The file's text: a header row, then one lead per row.
    csv: String,
    /// Which column holds what; leave out to get the columns and a suggestion.
    #[serde(default)]
    mapping: Option<Mapping>,
}

/// `POST /api/lead-imports` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImportCommitPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct CommitPayload {
    csv: String,
    mapping: Mapping,
    /// Where the list came from (a trade show, a partner).
    source_note: String,
    /// The day the list was obtained.
    obtained_on: chrono::NaiveDate,
    /// Leave out rows that look like someone already known.
    #[serde(default)]
    skip_likely_duplicates: bool,
}

/// Something a row looks like.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImportDuplicate")]
pub(crate) struct Likeness {
    /// `contact`, `lead`, `account` or `row` (an earlier row of this file).
    pub(super) kind: &'static str,
    /// The record's pid, for a contact, a lead or an account.
    pub(super) pid: Option<Uuid>,
    /// The file row, for `row` (1-based, after the header).
    pub(super) row: Option<usize>,
    pub(super) name: String,
    pub(super) probability: f64,
    #[schemars(with = "String")]
    pub(super) verdict: Verdict,
}

/// One row of the report.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImportRow")]
pub(crate) struct RowReport {
    /// 1-based, after the header.
    pub(super) row: usize,
    /// `valid` or `invalid`.
    pub(super) status: &'static str,
    /// Why a row is invalid.
    pub(super) reason: Option<String>,
    pub(super) name: Option<String>,
    /// Advice only: who this row looks like.
    pub(super) likely_duplicates: Vec<Likeness>,
}

/// `POST /api/lead-imports/preview` response.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImportPreview")]
pub(crate) struct Preview {
    columns: Vec<String>,
    rows: usize,
    suggested_mapping: Mapping,
    /// Present when a mapping was given.
    report: Option<Vec<RowReport>>,
    valid: usize,
    invalid: usize,
    likely_duplicates: usize,
}

fn tokens(name: &str) -> Vec<String> {
    crate::rules::search::words(name)
        .into_iter()
        .filter(|w| w.chars().count() >= 3)
        .collect()
}

/// One thing a row can be compared with.
struct Known {
    kind: &'static str,
    pid: Option<Uuid>,
    row: Option<usize>,
    name: String,
    matched: MatchContact,
}

/// Everyone the rows are compared with (contacts the caller may read, then leads), and an index
/// of them by the words of their names.
async fn load_pool(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
) -> Result<(Vec<Known>, HashMap<String, Vec<usize>>)> {
    // Everyone the rows are compared with: contacts the caller may read, then leads.
    let mut contact_rows = contacts::Entity::find_live()
        .order_by_asc(contacts::Column::Id)
        .limit(u64::try_from(DUPLICATE_POOL).unwrap_or(u64::MAX))
        .all(&ctx.db)
        .await?;
    contact_rows.retain(|row| {
        auth::authorize_record(
            caller,
            authentication_verifier::Action::Read,
            &auth::contact_resource_attrs(row),
        )
        .is_ok()
    });
    let accounts = account_names(&ctx.db, contact_rows.iter()).await?;
    let mut pool: Vec<Known> = contact_rows
        .iter()
        .map(|c| Known {
            kind: "contact",
            pid: Some(c.pid),
            row: None,
            name: c.display_name.clone(),
            matched: match_contact(c, &accounts),
        })
        .collect();
    for lead in leads::Entity::find_live()
        .order_by_desc(leads::Column::Id)
        .limit(LEAD_POOL)
        .all(&ctx.db)
        .await?
    {
        pool.push(Known {
            kind: "lead",
            pid: Some(lead.pid),
            row: None,
            matched: MatchContact::new().display_name(&lead.display_name),
            name: lead.display_name,
        });
    }
    let mut by_word: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, known) in pool.iter().enumerate() {
        for word in tokens(&known.name) {
            by_word.entry(word).or_default().push(index);
        }
    }
    Ok((pool, by_word))
}

/// Judge every row and, for the valid ones, find who they look like.
async fn analyse(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    table: &rules::Table,
    mapping: &Mapping,
) -> Result<(Vec<RowReport>, Vec<Option<rules::Row>>)> {
    let columns = rules::resolve(mapping, &table.headers).map_err(|m| unprocessable(&m))?;
    let judged = table
        .rows
        .iter()
        .map(|cells| rules::judge(cells, &columns, table.headers.len()))
        .collect();
    advise(ctx, caller, judged).await
}

/// For each judged row that is valid, the contacts, leads and earlier rows it looks like (advice
/// only). The contact and account imports (CRM-R43) use it too.
pub(super) async fn advise(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    rows: Vec<std::result::Result<rules::Row, String>>,
) -> Result<(Vec<RowReport>, Vec<Option<rules::Row>>)> {
    let (mut pool, mut by_word) = load_pool(ctx, caller).await?;
    let mut reports = Vec::with_capacity(rows.len());
    let mut judged = Vec::with_capacity(rows.len());
    for (offset, judgement) in rows.into_iter().enumerate() {
        let number = offset + 1;
        let row = match judgement {
            Ok(row) => row,
            Err(reason) => {
                reports.push(RowReport {
                    row: number,
                    status: "invalid",
                    reason: Some(reason),
                    name: None,
                    likely_duplicates: Vec::new(),
                });
                judged.push(None);
                continue;
            }
        };
        let mut query = MatchContact::new().display_name(&row.name);
        if let Some(company) = &row.company {
            query = query.organization(company);
        }
        let mut seen = HashSet::new();
        let candidates: Vec<usize> = tokens(&row.name)
            .iter()
            .filter_map(|w| by_word.get(w))
            .flatten()
            .copied()
            .filter(|i| seen.insert(*i))
            .take(PER_ROW)
            .collect();
        let compared: Vec<MatchContact> = candidates
            .iter()
            .map(|i| pool[*i].matched.clone())
            .collect();
        let mut likely: Vec<Likeness> = matcher()
            .compare_one_to_many(&query, &compared)
            .into_iter()
            .zip(&candidates)
            .filter(|(result, _)| result.verdict != Verdict::NonMatch)
            .map(|(result, i)| Likeness {
                kind: pool[*i].kind,
                pid: pool[*i].pid,
                row: pool[*i].row,
                name: pool[*i].name.clone(),
                probability: result.probability,
                verdict: result.verdict,
            })
            .collect();
        likely.sort_by(|a, b| b.probability.total_cmp(&a.probability));
        likely.truncate(ADVICE);
        // Later rows are compared with this one.
        let index = pool.len();
        for word in tokens(&row.name) {
            by_word.entry(word).or_default().push(index);
        }
        pool.push(Known {
            kind: "row",
            pid: None,
            row: Some(number),
            name: row.name.clone(),
            matched: query,
        });
        reports.push(RowReport {
            row: number,
            status: "valid",
            reason: None,
            name: Some(row.name.clone()),
            likely_duplicates: likely,
        });
        judged.push(Some(row));
    }
    Ok((reports, judged))
}

fn read(csv: &str) -> Result<rules::Table> {
    rules::parse(csv).map_err(|m| unprocessable(&m))
}

/// `POST /api/lead-imports/preview` — columns and a suggested mapping; with a mapping, a report
/// per row. Saves nothing.
#[debug_handler]
async fn preview(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PreviewPayload>,
) -> Result<Response> {
    let table = read(&payload.csv)?;
    let suggested = rules::suggest(&table.headers);
    let report = match &payload.mapping {
        Some(mapping) => Some(analyse(&ctx, &caller, &table, mapping).await?.0),
        None => None,
    };
    let (valid, invalid, likely) = report.as_ref().map_or((0, 0, 0), |r| {
        (
            r.iter().filter(|x| x.status == "valid").count(),
            r.iter().filter(|x| x.status == "invalid").count(),
            r.iter().filter(|x| !x.likely_duplicates.is_empty()).count(),
        )
    });
    Audit::record_read(
        &ctx.db,
        "lead_import",
        Uuid::nil(),
        "lead_import_previewed",
        caller.actor(),
        Some(serde_json::json!({
            "rows": table.rows.len(), "valid": valid, "invalid": invalid,
        })),
    )
    .await?;
    format::json(Preview {
        columns: table.headers,
        rows: table.rows.len(),
        suggested_mapping: suggested,
        report,
        valid,
        invalid,
        likely_duplicates: likely,
    })
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// An import record.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImport")]
pub(crate) struct ImportRecord {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    created_by: Option<String>,
    source_note: String,
    obtained_on: chrono::NaiveDate,
    /// `running`, `completed` or `undone`.
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    row_count: i32,
    invalid_count: i32,
    skipped_count: i32,
    /// Leads this import made.
    created_count: usize,
    undone_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Leads removed by the undo.
    undone_count: Option<i32>,
}

const COLUMNS: &str = "pid, created_by, source_note, obtained_on, status, created_at, row_count, \
                       invalid_count, skipped_count, lead_pids, undone_at, undone_count";

fn record(row: &sea_orm::QueryResult) -> Result<ImportRecord> {
    let pids: serde_json::Value = row.try_get("", "lead_pids")?;
    Ok(ImportRecord {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        created_by: row.try_get("", "created_by")?,
        source_note: row.try_get("", "source_note")?,
        obtained_on: row.try_get("", "obtained_on")?,
        status: row.try_get("", "status")?,
        created_at: row.try_get("", "created_at")?,
        row_count: row.try_get("", "row_count")?,
        invalid_count: row.try_get("", "invalid_count")?,
        skipped_count: row.try_get("", "skipped_count")?,
        created_count: pids.as_array().map_or(0, Vec::len),
        undone_at: row.try_get("", "undone_at")?,
        undone_count: row.try_get("", "undone_count")?,
    })
}

async fn find_import(db: &sea_orm::DatabaseConnection, pid: Uuid) -> Result<ImportRecord> {
    let row = db
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM lead_imports WHERE pid = $1"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    record(&row)
}

/// `POST /api/lead-imports` — save the valid rows as leads, in batches.
#[debug_handler]
async fn commit(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CommitPayload>,
) -> Result<Response> {
    let today = super::organization::caller_today(&ctx.db, &headers).await?;
    let mut problems = Problems::new();
    problems.require_text("source_note", &payload.source_note);
    problems.cap_text("source_note", &payload.source_note);
    if payload.obtained_on > today {
        problems.push_field("obtained_on", "cannot be in the future");
    }
    problems.finish()?;
    let table = read(&payload.csv)?;
    let (reports, judged) = analyse(&ctx, &caller, &table, &payload.mapping).await?;
    let invalid = reports.iter().filter(|r| r.status == "invalid").count();
    let mut to_save = Vec::new();
    let mut skipped = 0_usize;
    for (report, row) in reports.iter().zip(judged) {
        let Some(row) = row else { continue };
        if payload.skip_likely_duplicates && !report.likely_duplicates.is_empty() {
            skipped += 1;
        } else {
            to_save.push(row);
        }
    }
    let import = Uuid::new_v4();
    let toi = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
    let txn = ctx.db.begin().await?;
    txn.execute_raw(stmt(
        "INSERT INTO lead_imports (pid, created_by, source_note, obtained_on, row_count, \
         invalid_count, skipped_count) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        [
            import.into(),
            caller.actor().map(ToString::to_string).into(),
            payload.source_note.trim().to_string().into(),
            payload.obtained_on.into(),
            toi(table.rows.len()).into(),
            toi(invalid).into(),
            toi(skipped).into(),
        ],
    ))
    .await?;
    txn.commit().await?;
    for batch in to_save.chunks(rules::BATCH) {
        let txn = ctx.db.begin().await?;
        let mut made = Vec::with_capacity(batch.len());
        for row in batch {
            let (lead, _) = capture_lead(
                &txn,
                &NewLead {
                    source: "import",
                    campaign_pid: None,
                    contact_pid: None,
                    display_name: &row.name,
                    email: None,
                    email_domain: row.domain.clone(),
                },
                caller.actor(),
            )
            .await?;
            made.push(lead.pid.to_string());
        }
        txn.execute_raw(stmt(
            "UPDATE lead_imports SET lead_pids = lead_pids || $2::jsonb WHERE pid = $1",
            [import.into(), serde_json::json!(made).into()],
        ))
        .await?;
        txn.commit().await?;
    }
    let txn = ctx.db.begin().await?;
    txn.execute_raw(stmt(
        "UPDATE lead_imports SET status = 'completed' WHERE pid = $1",
        [import.into()],
    ))
    .await?;
    Audit::record(
        &txn,
        "lead_import",
        import,
        "lead_import_committed",
        caller.actor(),
        Some(serde_json::json!({
            "rows": table.rows.len(), "created": to_save.len(), "invalid": invalid,
            "skipped": skipped, "source_note": payload.source_note.trim(),
            "obtained_on": payload.obtained_on,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(find_import(&ctx.db, import).await?)
}

/// `POST /api/lead-imports/{pid}/undo` response.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImportUndone")]
pub(crate) struct Undone {
    /// Leads removed.
    removed: usize,
    /// Leads kept because they have been worked since.
    kept: usize,
}

/// `POST /api/lead-imports/{pid}/undo` — remove the leads this import made that are still new
/// and have no activity.
#[debug_handler]
async fn undo(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let existing = find_import(&ctx.db, pid).await?;
    if existing.status == "undone" {
        return Err(unprocessable("this import has already been undone"));
    }
    let txn = ctx.db.begin().await?;
    // Lock the record so a second undo waits for the first.
    let locked = txn
        .query_one_raw(stmt(
            "SELECT lead_pids, status FROM lead_imports WHERE pid = $1 FOR UPDATE",
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    if locked.try_get::<String>("", "status")? == "undone" {
        return Err(unprocessable("this import has already been undone"));
    }
    let all: Vec<Uuid> = locked
        .try_get::<serde_json::Value>("", "lead_pids")?
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
                .collect()
        })
        .unwrap_or_default();
    let removed = txn
        .query_all_raw(stmt(
            "UPDATE leads SET deleted_at = now() WHERE pid = ANY($1) AND deleted_at IS NULL \
             AND status = 'new' AND NOT EXISTS (SELECT 1 FROM activities a \
                 WHERE a.subject_kind = 'lead' AND a.subject_pid = leads.pid \
                   AND a.deleted_at IS NULL) \
             RETURNING pid",
            [all.clone().into()],
        ))
        .await?;
    for row in &removed {
        let lead: Uuid = row.try_get("", "pid")?;
        Audit::record(&txn, "lead", lead, "import_undone", caller.actor(), None).await?;
    }
    let kept = all.len() - removed.len();
    txn.execute_raw(stmt(
        "UPDATE lead_imports SET status = 'undone', undone_at = now(), undone_count = $2 \
         WHERE pid = $1",
        [
            pid.into(),
            i32::try_from(removed.len()).unwrap_or(i32::MAX).into(),
        ],
    ))
    .await?;
    Audit::record(
        &txn,
        "lead_import",
        pid,
        "lead_import_undone",
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

/// `GET /api/lead-imports` — import records, newest first.
#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    Query(paging): Query<PageParams>,
    uri: axum::http::Uri,
) -> Result<Response> {
    let page = Page::new(&paging, 100)?;
    let (limit, offset) = page.window();
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!("SELECT {COLUMNS} FROM lead_imports ORDER BY id DESC LIMIT $1 OFFSET $2"),
            [
                i64::try_from(limit).unwrap_or(i64::MAX).into(),
                i64::try_from(offset).unwrap_or(i64::MAX).into(),
            ],
        ))
        .await?;
    let records = rows.iter().map(record).collect::<Result<Vec<_>>>()?;
    let (records, more) = page.split(records);
    page.respond(records, more, &uri)
}

/// `GET /api/lead-imports/{pid}`.
#[debug_handler]
async fn get_one(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    format::json(find_import(&ctx.db, pid).await?)
}

/// The lead import routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/lead-imports", get(list))
        .add("/lead-imports", post(commit))
        .add("/lead-imports/preview", post(preview))
        .add("/lead-imports/{pid}", get(get_one))
        .add("/lead-imports/{pid}/undo", post(undo))
}
