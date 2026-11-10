//! Tags (CRM-R47): `/api/tags`, a record's tags, and the filter on the lists.
//!
//! A tag is a short label. A contact, account, lead or deal carries a set of them; a segment can
//! select by them (any of), and a list can be narrowed to one. A tag on a person is personal data:
//! it is hidden from a masked read, listed in a subject-access export, and deleted when the contact
//! is erased.
//!
//! Everything runs on `sqlx` (CRM-D24, CRM-T205); the readers other modules call since CRM-T216.

use axa::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::custom_fields::subject;
use super::{PidPath, PidRef, record_rejection, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::db;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::tags as rules;

/// `POST /api/tags` and `PUT /api/tags/{pid}` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "TagPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct TagPayload {
    /// Unique, ignoring case.
    name: String,
}

/// `PUT /api/{records}/{pid}/tags` body: the whole set of the record.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "RecordTagsPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordTagsPayload {
    /// The names the record should carry, at most 20. A name that is not yet a tag becomes one.
    tags: Vec<String>,
}

/// One tag, with how many records carry it.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Tag")]
pub(crate) struct TagView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    uses: i64,
}

/// `?tag=` on a list: only the records that carry this tag.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct TagFilter {
    #[serde(default)]
    pub tag: Option<String>,
}

fn taken() -> Error {
    unprocessable("a tag with that name already exists")
}

/// `GET /api/tags`.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = sqlx::query!(
        r#"SELECT t.pid, t.name,
             (SELECT count(*) FROM taggings g WHERE g.tag_pid = t.pid) AS "uses!"
           FROM tags t ORDER BY lower(t.name), t.id"#,
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    format::json(
        rows.into_iter()
            .map(|row| TagView {
                pid: row.pid.to_string(),
                name: row.name,
                uses: row.uses,
            })
            .collect::<Vec<_>>(),
    )
}

/// `POST /api/tags`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<TagPayload>,
) -> Result<Response> {
    let name = rules::normalise(&payload.name).map_err(|why| unprocessable(&why))?;
    let mut txn = db::pool(&ctx).begin().await?;
    let held = count_tags(&mut txn).await?;
    if held >= rules::MAX_TAGS {
        return Err(unprocessable(&format!(
            "there are already {} tags",
            rules::MAX_TAGS
        )));
    }
    let pid = Uuid::new_v4();
    let inserted = sqlx::query!(
        "INSERT INTO tags (pid, name) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        pid,
        name,
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record_on(&mut txn, "tag", pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

async fn count_tags(conn: &mut PgConnection) -> Result<i64> {
    Ok(sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM tags"#)
        .fetch_one(conn)
        .await?)
}

/// `PUT /api/tags/{pid}` — rename; every record keeps it.
#[debug_handler]
async fn rename(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<TagPayload>,
) -> Result<Response> {
    let name = rules::normalise(&payload.name).map_err(|why| unprocessable(&why))?;
    let mut txn = db::pool(&ctx).begin().await?;
    let changed = sqlx::query!("UPDATE tags SET name = $2 WHERE pid = $1", pid, name)
        .execute(&mut *txn)
        .await;
    if changed.is_err() {
        return Err(taken());
    }
    if changed.is_ok_and(|r| r.rows_affected() == 0) {
        return Err(Error::NotFound);
    }
    Audit::record_on(&mut txn, "tag", pid, "renamed", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `DELETE /api/tags/{pid}` — the tag comes off every record that carries it.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let removed = sqlx::query!("DELETE FROM taggings WHERE tag_pid = $1", pid)
        .execute(&mut *txn)
        .await?
        .rows_affected();
    let gone = sqlx::query!("DELETE FROM tags WHERE pid = $1", pid)
        .execute(&mut *txn)
        .await?
        .rows_affected();
    if gone == 0 {
        return Err(Error::NotFound);
    }
    Audit::record_on(
        &mut txn,
        "tag",
        pid,
        "deleted",
        caller.actor(),
        Some(serde_json::json!({ "taggings_removed": removed })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

pub(crate) async fn names_of<'e>(
    db: impl sqlx::PgExecutor<'e>,
    subject_pid: Uuid,
) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar!(
        "SELECT t.name FROM taggings g JOIN tags t ON t.pid = g.tag_pid \
         WHERE g.subject_pid = $1 ORDER BY lower(t.name)",
        subject_pid,
    )
    .fetch_all(db)
    .await?)
}

pub(crate) async fn filter_pids<'e>(
    db: impl sqlx::PgExecutor<'e>,
    kind: &str,
    filter: &TagFilter,
) -> Result<Option<Vec<Uuid>>> {
    let Some(raw) = filter.tag.as_deref() else {
        return Ok(None);
    };
    let name = rules::normalise(raw).map_err(|why| unprocessable(&why))?;
    Ok(Some(
        sqlx::query_scalar!(
            "SELECT g.subject_pid FROM taggings g JOIN tags t ON t.pid = g.tag_pid \
             WHERE g.subject_kind = $1 AND lower(t.name) = $2",
            kind,
            rules::key(&name),
        )
        .fetch_all(db)
        .await?,
    ))
}

/// A masked read may not see tags: they say things about a person.
fn refuse_masked(masked: bool) -> Result<()> {
    if masked {
        return Err(record_rejection((
            axum::http::StatusCode::FORBIDDEN,
            "tags need an unmasked read of the record".to_string(),
        )));
    }
    Ok(())
}

async fn get_tags(
    ctx: AppContext,
    caller: MaybeAuthUser,
    entity: &'static str,
    pid: Uuid,
) -> Result<Response> {
    let masked = subject(
        crate::db::pool(&ctx),
        &caller,
        entity,
        pid,
        authentication_verifier::Action::Read,
    )
    .await?;
    refuse_masked(masked)?;
    format::json(names_of(crate::db::pool(&ctx), pid).await?)
}

async fn put_tags(
    ctx: AppContext,
    caller: MaybeAuthUser,
    entity: &'static str,
    pid: Uuid,
    payload: RecordTagsPayload,
) -> Result<Response> {
    let masked = subject(
        crate::db::pool(&ctx),
        &caller,
        entity,
        pid,
        authentication_verifier::Action::Write,
    )
    .await?;
    refuse_masked(masked)?;
    let wanted = rules::set_of(&payload.tags).map_err(|why| unprocessable(&why))?;
    let mut txn = db::pool(&ctx).begin().await?;
    // Every name becomes a tag if it is not one yet.
    let mut keep = Vec::new();
    for name in &wanted {
        let found = sqlx::query_scalar!(
            "SELECT pid FROM tags WHERE lower(name) = $1",
            rules::key(name),
        )
        .fetch_optional(&mut *txn)
        .await?;
        let tag: Uuid = if let Some(pid) = found {
            pid
        } else {
            if count_tags(&mut txn).await? >= rules::MAX_TAGS {
                return Err(unprocessable(&format!(
                    "there are already {} tags",
                    rules::MAX_TAGS
                )));
            }
            let made = Uuid::new_v4();
            sqlx::query!("INSERT INTO tags (pid, name) VALUES ($1, $2)", made, name)
                .execute(&mut *txn)
                .await?;
            Audit::record_on(&mut txn, "tag", made, "created", caller.actor(), None).await?;
            made
        };
        keep.push(tag);
    }
    sqlx::query!(
        "DELETE FROM taggings WHERE subject_pid = $1 AND NOT (tag_pid = ANY($2))",
        pid,
        &keep,
    )
    .execute(&mut *txn)
    .await?;
    for tag in &keep {
        sqlx::query!(
            "INSERT INTO taggings (tag_pid, subject_kind, subject_pid) VALUES ($1, $2, $3) \
             ON CONFLICT DO NOTHING",
            tag,
            entity,
            pid,
        )
        .execute(&mut *txn)
        .await?;
    }
    // The audit row counts the tags and names none: a tag can be about a person.
    Audit::record_on(
        &mut txn,
        entity,
        pid,
        "tags_set",
        caller.actor(),
        Some(serde_json::json!({ "count": keep.len() })),
    )
    .await?;
    txn.commit().await?;
    format::json(names_of(crate::db::pool(&ctx), pid).await?)
}

macro_rules! record_routes {
    ($get:ident, $put:ident, $entity:literal) => {
        #[debug_handler]
        async fn $get(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
        ) -> Result<Response> {
            get_tags(ctx, caller, $entity, pid).await
        }

        #[debug_handler]
        async fn $put(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
            Json(payload): Json<RecordTagsPayload>,
        ) -> Result<Response> {
            put_tags(ctx, caller, $entity, pid, payload).await
        }
    };
}

record_routes!(contact_tags, set_contact_tags, "contact");
record_routes!(account_tags, set_account_tags, "account");
record_routes!(lead_tags, set_lead_tags, "lead");
record_routes!(deal_tags, set_deal_tags, "deal");

pub(crate) async fn export_for<'e>(
    db: impl sqlx::PgExecutor<'e>,
    contact: Uuid,
) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar!(
        "SELECT DISTINCT t.name FROM taggings g JOIN tags t ON t.pid = g.tag_pid \
         WHERE ((g.subject_kind = 'contact' AND (g.subject_pid = $1 OR g.subject_pid IN \
           (SELECT pid FROM contacts WHERE merged_into_pid = $1))) \
     OR (g.subject_kind = 'lead' AND g.subject_pid IN \
           (SELECT pid FROM leads WHERE contact_pid = $1))) \
         /* whatever deleted_at holds: a merged duplicate and a lead are the subject's */ ORDER BY t.name",
        contact,
    )
    .fetch_all(db)
    .await?)
}

pub(crate) async fn erase_rows<'e>(db: impl sqlx::PgExecutor<'e>, contact: Uuid) -> Result<u64> {
    Ok(sqlx::query!(
        "DELETE FROM taggings g WHERE ((g.subject_kind = 'contact' AND (g.subject_pid = $1 OR g.subject_pid IN \
           (SELECT pid FROM contacts WHERE merged_into_pid = $1))) \
     OR (g.subject_kind = 'lead' AND g.subject_pid IN \
           (SELECT pid FROM leads WHERE contact_pid = $1))) \
         /* whatever deleted_at holds: a merged duplicate and a lead are the subject's */",
        contact,
    )
    .execute(db)
    .await?
    .rows_affected())
}

/// The tag routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/tags", get(list))
        .add("/tags", post(create))
        .add("/tags/{pid}", put(rename))
        .add("/tags/{pid}", delete(remove))
        .add("/contacts/{pid}/tags", get(contact_tags))
        .add("/contacts/{pid}/tags", put(set_contact_tags))
        .add("/accounts/{pid}/tags", get(account_tags))
        .add("/accounts/{pid}/tags", put(set_account_tags))
        .add("/leads/{pid}/tags", get(lead_tags))
        .add("/leads/{pid}/tags", put(set_lead_tags))
        .add("/deals/{pid}/tags", get(deal_tags))
        .add("/deals/{pid}/tags", put(set_deal_tags))
}
