//! Links to records in other systems (CRM-R27): `HubSpot`, `Salesforce`, `Attio` and
//! the like.
//!
//! A contact, account, deal or lead can carry links to the same thing in other
//! systems, so a person can jump to it. A link is a web address and a label: the
//! service fetches nothing and synchronises nothing. Addresses are checked by
//! [`crate::rules::external_links`]. Contact links move with a merge and are
//! deleted with an erasure.

use crate::controllers::PidPath;
use crate::json::Json;
use axa::error::ErrorDetail;
use axa::prelude::*;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::record_rejection;
use crate::auth::{self, MaybeAuthUser};
use crate::db;
use crate::models::_entities::external_links;
use crate::models::audit_logs::Model as Audit;
use crate::models::rows;
use crate::rules::external_links as rules;
use crate::validation::Problems;

/// The most links one record can carry.
const MAX_PER_SUBJECT: u64 = 50;

/// `POST …/external-links` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "ExternalLinkPayload")]
pub(crate) struct LinkPayload {
    /// `hubspot`, `salesforce`, `attio`, `pipedrive`, `dynamics` or `other`.
    pub system: String,
    /// The record's web address in that system. `https` only.
    pub url: String,
    /// What the person clicking should see. Optional.
    pub label: Option<String>,
    /// The system's own id for the record. Optional.
    pub external_id: Option<String>,
}

/// Check the record exists and is live. For a contact, also apply the
/// per-record access policy.
async fn check_subject(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    kind: &str,
    pid: Uuid,
    action: authentication_verifier::Action,
) -> Result<()> {
    let pool = db::pool(ctx);
    // One checked statement per table, in place of a table name written into the SQL.
    let found = match kind {
        "contact" => {
            let contact = rows::contact(pool, pid).await?.ok_or(Error::NotFound)?;
            auth::authorize_record(caller, action, &auth::contact_resource_attrs(&contact))
                .map_err(record_rejection)?;
            return Ok(());
        }
        "account" => {
            sqlx::query_scalar!(
                "SELECT EXISTS (SELECT 1 FROM accounts WHERE pid = $1 AND deleted_at IS NULL) \
             AS \"found!\"",
                pid,
            )
            .fetch_one(pool)
            .await?
        }
        "deal" => {
            sqlx::query_scalar!(
                "SELECT EXISTS (SELECT 1 FROM deals WHERE pid = $1 AND deleted_at IS NULL) \
             AS \"found!\"",
                pid,
            )
            .fetch_one(pool)
            .await?
        }
        _ => {
            sqlx::query_scalar!(
                "SELECT EXISTS (SELECT 1 FROM leads WHERE pid = $1 AND deleted_at IS NULL) \
             AS \"found!\"",
                pid,
            )
            .fetch_one(pool)
            .await?
        }
    };
    if !found {
        return Err(Error::NotFound);
    }
    Ok(())
}

/// `GET /api/{contacts,accounts,deals,leads}/{pid}/external-links`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct ExternalLinks {
    links: Vec<external_links::Model>,
    /// The systems a link may name.
    #[schemars(with = "Vec<String>")]
    systems: [&'static str; 6],
}

async fn list_for(
    ctx: AppContext,
    caller: MaybeAuthUser,
    kind: &'static str,
    pid: Uuid,
) -> Result<Response> {
    check_subject(
        &ctx,
        &caller,
        kind,
        pid,
        authentication_verifier::Action::Read,
    )
    .await?;
    let links = sqlx::query_as!(
        external_links::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, system, url, external_id, \
        label, created_by FROM external_links WHERE subject_kind = $1 AND subject_pid = $2 ORDER BY \
        id",
        kind,
        pid,
    )
    .fetch_all(db::pool(&ctx))
    .await?;
    format::json(ExternalLinks {
        links,
        systems: rules::SYSTEM_TOKENS,
    })
}

async fn create_for(
    ctx: AppContext,
    caller: MaybeAuthUser,
    kind: &'static str,
    pid: Uuid,
    payload: LinkPayload,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("system", &rules::SYSTEM_TOKENS, &payload.system);
    problems.require_text("url", &payload.url);
    problems.cap_opt("label", payload.label.as_deref());
    problems.cap_opt("external_id", payload.external_id.as_deref());
    let address = match rules::check_url(&payload.system, &payload.url) {
        Ok(address) => Some(address),
        Err(why) => {
            // An unknown system is already reported above.
            if rules::SYSTEM_TOKENS.contains(&payload.system.as_str()) {
                problems.push(why);
            }
            None
        }
    };
    problems.finish()?;
    let address = address.ok_or(Error::BadRequest("url".into()))?;
    check_subject(
        &ctx,
        &caller,
        kind,
        pid,
        authentication_verifier::Action::Write,
    )
    .await?;

    let mut txn = db::pool(&ctx).begin().await?;
    let existing = sqlx::query_scalar!(
        "SELECT url FROM external_links WHERE subject_kind = $1 AND subject_pid = $2",
        kind,
        pid,
    )
    .fetch_all(&mut *txn)
    .await?;
    if existing.contains(&address) {
        return Err(Error::CustomError(
            StatusCode::CONFLICT,
            ErrorDetail::new("conflict", "this record already has that link"),
        ));
    }
    if existing.len() as u64 >= MAX_PER_SUBJECT {
        return Err(super::unprocessable("a record can carry at most 50 links"));
    }
    let link = sqlx::query_as!(
        external_links::Model,
        "INSERT INTO external_links (pid, subject_kind, subject_pid, system, url, external_id, label, \
        created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING created_at, updated_at, id, \
        pid, subject_kind, subject_pid, system, url, external_id, label, created_by",
        Uuid::new_v4(),
        kind,
        pid,
        payload.system,
        address,
        payload.external_id.filter(|s| !s.trim().is_empty()),
        payload.label.filter(|s| !s.trim().is_empty()),
        caller.actor(),
    )
    .fetch_one(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        kind,
        pid,
        "external_link_added",
        caller.actor(),
        Some(json!({ "system": payload.system })),
    )
    .await?;
    txn.commit().await?;
    format::json(link)
}

/// `DELETE /api/external-links/{pid}`.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let link = sqlx::query_as!(
        external_links::Model,
        "SELECT created_at, updated_at, id, pid, subject_kind, subject_pid, system, url, external_id, \
        label, created_by FROM external_links WHERE pid = $1 FOR UPDATE",
        pid,
    )
    .fetch_optional(&mut *txn)
    .await?
    .ok_or(Error::NotFound)?;
    let kind = match link.subject_kind.as_str() {
        "contact" => "contact",
        "account" => "account",
        "deal" => "deal",
        _ => "lead",
    };
    if kind == "contact" {
        let contact = rows::contact(&mut *txn, link.subject_pid)
            .await?
            .ok_or(Error::NotFound)?;
        auth::authorize_record(
            &caller,
            authentication_verifier::Action::Write,
            &auth::contact_resource_attrs(&contact),
        )
        .map_err(record_rejection)?;
    }
    sqlx::query!("DELETE FROM external_links WHERE id = $1", link.id)
        .execute(&mut *txn)
        .await?;
    Audit::record_on(
        &mut txn,
        kind,
        link.subject_pid,
        "external_link_removed",
        caller.actor(),
        Some(json!({ "system": link.system })),
    )
    .await?;
    txn.commit().await?;
    format::json(json!({ "deleted": pid }))
}

/// One list handler and one create handler per kind of record.
macro_rules! handlers {
    ($kind:literal, $list:ident, $create:ident) => {
        #[debug_handler]
        async fn $list(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
        ) -> Result<Response> {
            list_for(ctx, caller, $kind, pid).await
        }

        #[debug_handler]
        async fn $create(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
            Json(payload): Json<LinkPayload>,
        ) -> Result<Response> {
            create_for(ctx, caller, $kind, pid, payload).await
        }
    };
}

handlers!("contact", contact_links, add_contact_link);
handlers!("account", account_links, add_account_link);
handlers!("deal", deal_links, add_deal_link);
handlers!("lead", lead_links, add_lead_link);

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/external-links", get(contact_links))
        .add("/contacts/{pid}/external-links", post(add_contact_link))
        .add("/accounts/{pid}/external-links", get(account_links))
        .add("/accounts/{pid}/external-links", post(add_account_link))
        .add("/deals/{pid}/external-links", get(deal_links))
        .add("/deals/{pid}/external-links", post(add_deal_link))
        .add("/leads/{pid}/external-links", get(lead_links))
        .add("/leads/{pid}/external-links", post(add_lead_link))
        .add("/external-links/{pid}", delete(remove))
}
