//! File attachments (CRM-R42): files kept with a contact, account, lead or deal.
//!
//! A file is accepted only from the allow-list in [`crate::rules::attachments`], is size-limited,
//! is scanned when a scan command is configured, and is stored outside the database under a random
//! id. A download is always an attachment, served with the type the allow-list gives, never the
//! one the uploader claimed. Without `CRM_ATTACHMENT_DIR` attachments are off, and say so.

use std::path::PathBuf;

use axum::body::Bytes;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, StatusCode, header};
use axum::response::IntoResponse;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::custom_fields::subject;
use super::{PidPath, record_rejection, unprocessable};
use crate::attachments::{self as store, StoreError};
use crate::auth::MaybeAuthUser;
use crate::models::audit_logs::Model as Audit;
use crate::rules::attachments as rules;
use crate::settings;

/// The hard ceiling on a request body here; the configured limit is checked inside.
const BODY_CEILING: usize = 101 * 1024 * 1024;

/// `POST /api/{records}/{pid}/attachments?name=`.
#[derive(Debug, Deserialize)]
pub(crate) struct UploadQuery {
    /// The file's name, with its extension.
    name: String,
}

/// One attached file's metadata.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Attachment")]
pub(crate) struct AttachmentView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    content_type: String,
    size_bytes: i64,
    /// SHA-256 of the content, hex.
    sha256: String,
    uploaded_by: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

const COLUMNS: &str = "pid, subject_kind, subject_pid, name, content_type, size_bytes, sha256, uploaded_by, created_at";

fn view(row: &sea_orm::QueryResult) -> Result<AttachmentView> {
    Ok(AttachmentView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        name: row.try_get("", "name")?,
        content_type: row.try_get("", "content_type")?,
        size_bytes: row.try_get("", "size_bytes")?,
        sha256: row.try_get("", "sha256")?,
        uploaded_by: row.try_get("", "uploaded_by")?,
        created_at: row.try_get("", "created_at")?,
    })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

fn unavailable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::SERVICE_UNAVAILABLE,
        ErrorDetail::new("unavailable", message),
    )
}

/// The directory attachments live in, or the reason there is none.
fn directory() -> Result<PathBuf> {
    settings::current()
        .attachment_dir
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| unavailable("attachments are not configured (CRM_ATTACHMENT_DIR)"))
}

/// A masked read may not see or fetch files: they are the record's content, not its summary.
fn refuse_masked(masked: bool) -> Result<()> {
    if masked {
        return Err(record_rejection((
            StatusCode::FORBIDDEN,
            "attachments need an unmasked read of the record".to_string(),
        )));
    }
    Ok(())
}

async fn list_for(
    ctx: AppContext,
    caller: MaybeAuthUser,
    entity: &'static str,
    pid: Uuid,
) -> Result<Response> {
    let masked = subject(
        &ctx.db,
        &caller,
        entity,
        pid,
        authentication_verifier::Action::Read,
    )
    .await?;
    refuse_masked(masked)?;
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM attachments WHERE subject_kind = $1 AND subject_pid = $2 \
                 ORDER BY id"
            ),
            [entity.into(), pid.into()],
        ))
        .await?;
    format::json(rows.iter().map(view).collect::<Result<Vec<_>>>()?)
}

/// What to answer when a file was not stored; an infected one is also audited.
async fn refused(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    entity: &str,
    pid: Uuid,
    key: Uuid,
    error: StoreError,
) -> Error {
    match error {
        StoreError::Infected => {
            let note = Some(serde_json::json!({ "subject_kind": entity, "subject_pid": pid }));
            match Audit::record(
                &ctx.db,
                "attachment",
                key,
                "refused_infected",
                caller.actor(),
                note,
            )
            .await
            {
                Ok(_) => unprocessable("the file was refused by the malware scan"),
                Err(error) => error.into(),
            }
        }
        StoreError::ScanFailed(why) => {
            tracing::warn!(%why, "attachment scan failed");
            unavailable("the file could not be scanned, so it was not stored")
        }
        StoreError::Io(error) => {
            tracing::error!(%error, "attachment write failed");
            unavailable("the file could not be stored")
        }
    }
}

async fn upload_to(
    ctx: AppContext,
    caller: MaybeAuthUser,
    entity: &'static str,
    pid: Uuid,
    name: &str,
    body: Bytes,
) -> Result<Response> {
    let dir = directory()?;
    subject(
        &ctx.db,
        &caller,
        entity,
        pid,
        authentication_verifier::Action::Write,
    )
    .await?;
    let cfg = settings::current();
    let (clean, kind) =
        rules::check(name, &body, cfg.attachment_max_bytes).map_err(|why| unprocessable(&why))?;
    let held: i64 = ctx
        .db
        .query_one_raw(stmt(
            "SELECT count(*) AS n FROM attachments WHERE subject_kind = $1 AND subject_pid = $2",
            [entity.into(), pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?
        .try_get("", "n")?;
    if held >= rules::MAX_PER_RECORD {
        return Err(unprocessable(&format!(
            "a record may have at most {} files",
            rules::MAX_PER_RECORD
        )));
    }
    let key = Uuid::new_v4();
    if let Err(error) = store::put(&dir, key, &body, cfg.attachment_scan_command.as_deref()).await {
        return Err(refused(&ctx, &caller, entity, pid, key, error).await);
    }
    let digest = hex(&body);
    let saved = async {
        let txn = ctx.db.begin().await?;
        let row = txn
            .query_one_raw(stmt(
                &format!(
                    "INSERT INTO attachments (pid, subject_kind, subject_pid, name, content_type, \
                     size_bytes, sha256, uploaded_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                     RETURNING {COLUMNS}"
                ),
                [
                    key.into(),
                    entity.into(),
                    pid.into(),
                    clean.clone().into(),
                    kind.content_type.to_string().into(),
                    i64::try_from(body.len()).unwrap_or(i64::MAX).into(),
                    digest.clone().into(),
                    caller.actor().map(ToString::to_string).into(),
                ],
            ))
            .await?
            .ok_or(Error::NotFound)?;
        // The audit row has the digest and size, not the name: a name can be personal.
        Audit::record(
            &txn,
            "attachment",
            key,
            "uploaded",
            caller.actor(),
            Some(serde_json::json!({
                "subject_kind": entity, "subject_pid": pid,
                "content_type": kind.content_type, "size_bytes": body.len(), "sha256": digest,
            })),
        )
        .await?;
        txn.commit().await?;
        view(&row)
    }
    .await;
    match saved {
        Ok(made) => format::json(made),
        Err(error) => {
            let _ = store::remove(&dir, key).await;
            Err(error)
        }
    }
}

/// Look up an attachment and check the caller may act on its record.
async fn find_for(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    pid: Uuid,
    action: authentication_verifier::Action,
) -> Result<sea_orm::QueryResult> {
    let row = ctx
        .db
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM attachments WHERE pid = $1"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let kind: String = row.try_get("", "subject_kind")?;
    let entity = match kind.as_str() {
        "contact" => "contact",
        "account" => "account",
        "deal" => "deal",
        _ => "lead",
    };
    let masked = subject(
        &ctx.db,
        caller,
        entity,
        row.try_get("", "subject_pid")?,
        action,
    )
    .await?;
    refuse_masked(masked)?;
    Ok(row)
}

/// `GET /api/attachments/{pid}/content` — the file, as a download.
#[debug_handler]
async fn content(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let dir = directory()?;
    let row = find_for(&ctx, &caller, pid, authentication_verifier::Action::Read).await?;
    let bytes = store::get(&dir, pid).await.map_err(|error| {
        tracing::error!(%error, %pid, "attachment file unreadable");
        unavailable("the file is not available")
    })?;
    let expected: String = row.try_get("", "sha256")?;
    if hex(&bytes) != expected {
        tracing::error!(%pid, "attachment file does not match its digest");
        return Err(unavailable("the stored file failed its integrity check"));
    }
    // A download of personal content is audited like any other disclosure.
    Audit::record_read(
        &ctx.db,
        "attachment",
        pid,
        "downloaded",
        caller.actor(),
        None,
    )
    .await?;
    let name: String = row.try_get("", "name")?;
    let content_type: String = row.try_get("", "content_type")?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, rules::disposition(&name)),
            (
                HeaderName::from_static("x-content-type-options"),
                "nosniff".to_string(),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// `DELETE /api/attachments/{pid}`.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let dir = directory()?;
    find_for(&ctx, &caller, pid, authentication_verifier::Action::Write).await?;
    let txn = ctx.db.begin().await?;
    txn.execute_raw(stmt("DELETE FROM attachments WHERE pid = $1", [pid.into()]))
        .await?;
    Audit::record(&txn, "attachment", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    if let Err(error) = store::remove(&dir, pid).await {
        tracing::error!(%error, %pid, "attachment file not removed");
    }
    format::json(super::PidRef {
        pid: pid.to_string(),
    })
}

macro_rules! record_routes {
    ($list:ident, $upload:ident, $entity:literal) => {
        #[debug_handler]
        async fn $list(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
        ) -> Result<Response> {
            list_for(ctx, caller, $entity, pid).await
        }

        #[debug_handler]
        async fn $upload(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
            axum::extract::Query(query): axum::extract::Query<UploadQuery>,
            body: Bytes,
        ) -> Result<Response> {
            upload_to(ctx, caller, $entity, pid, &query.name, body).await
        }
    };
}

record_routes!(contact_files, upload_contact_file, "contact");
record_routes!(account_files, upload_account_file, "account");
record_routes!(lead_files, upload_lead_file, "lead");
record_routes!(deal_files, upload_deal_file, "deal");

/// A contact's attached files, as metadata, for the subject-access export: the contact's own, those
/// of contacts merged into it, and those of the leads an erasure would scrub.
///
/// # Errors
///
/// A database error.
pub(crate) async fn export_for<C: ConnectionTrait>(
    db: &C,
    contact: Uuid,
) -> Result<Vec<AttachmentView>> {
    let rows = db
        .query_all_raw(stmt(
            &format!(
                "SELECT {COLUMNS} FROM attachments WHERE {} ORDER BY id",
                owned_by(contact)
            ),
            [],
        ))
        .await?;
    rows.iter().map(view).collect()
}

/// The SQL condition for "belongs to this contact, a contact merged into it, or a lead of it".
fn owned_by(contact: Uuid) -> String {
    format!(
        "((subject_kind = 'contact' AND (subject_pid = '{contact}' OR subject_pid IN \
           (SELECT pid FROM contacts WHERE merged_into_pid = '{contact}'))) \
         OR (subject_kind = 'lead' AND subject_pid IN \
           (SELECT pid FROM leads WHERE contact_pid = '{contact}')))"
    )
}

/// Delete a contact's attachment rows (see [`export_for`] for whose) and say which files to remove
/// once the transaction has committed.
///
/// # Errors
///
/// A database error.
pub(crate) async fn erase_rows<C: ConnectionTrait>(db: &C, contact: Uuid) -> Result<Vec<Uuid>> {
    let rows = db
        .query_all_raw(stmt(
            &format!(
                "DELETE FROM attachments WHERE {} RETURNING pid",
                owned_by(contact)
            ),
            [],
        ))
        .await?;
    rows.iter()
        .map(|r| r.try_get("", "pid").map_err(Into::into))
        .collect()
}

/// Remove files from disk after the rows are gone; returns how many could not be removed.
pub(crate) async fn remove_files(keys: &[Uuid]) -> usize {
    let Some(dir) = settings::current().attachment_dir.clone() else {
        return keys.len();
    };
    let dir = PathBuf::from(dir);
    let mut left = 0;
    for key in keys {
        if let Err(error) = store::remove(&dir, *key).await {
            tracing::error!(%error, %key, "attachment file not removed");
            left += 1;
        }
    }
    left
}

/// The attachment routes.
pub fn routes() -> Routes {
    let limit = || DefaultBodyLimit::max(BODY_CEILING);
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/attachments", get(contact_files))
        .add(
            "/contacts/{pid}/attachments",
            post(upload_contact_file).layer(limit()),
        )
        .add("/accounts/{pid}/attachments", get(account_files))
        .add(
            "/accounts/{pid}/attachments",
            post(upload_account_file).layer(limit()),
        )
        .add("/leads/{pid}/attachments", get(lead_files))
        .add(
            "/leads/{pid}/attachments",
            post(upload_lead_file).layer(limit()),
        )
        .add("/deals/{pid}/attachments", get(deal_files))
        .add(
            "/deals/{pid}/attachments",
            post(upload_deal_file).layer(limit()),
        )
        .add("/attachments/{pid}/content", get(content))
        .add("/attachments/{pid}", delete(remove))
}
