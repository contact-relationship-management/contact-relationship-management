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
use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveValue, ConnectionTrait, DatabaseBackend, QueryOrder, Statement, TransactionTrait, Value,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::record_rejection;
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::external_links;
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
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

/// The table behind each subject kind.
fn table_of(kind: &str) -> &'static str {
    match kind {
        "contact" => "contacts",
        "account" => "accounts",
        "deal" => "deals",
        _ => "leads",
    }
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
    if kind == "contact" {
        let contact = records::find_contact(&ctx.db, pid).await?;
        auth::authorize_record(caller, action, &auth::contact_resource_attrs(&contact))
            .map_err(record_rejection)?;
        return Ok(());
    }
    let found = ctx
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!(
                "SELECT 1 FROM {} WHERE pid = $1 AND deleted_at IS NULL",
                table_of(kind)
            ),
            [Value::from(pid)],
        ))
        .await?;
    if found.is_none() {
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
    let links = external_links::Entity::find()
        .filter(external_links::Column::SubjectKind.eq(kind))
        .filter(external_links::Column::SubjectPid.eq(pid))
        .order_by_asc(external_links::Column::Id)
        .all(&ctx.db)
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

    let txn = ctx.db.begin().await?;
    let existing = external_links::Entity::find()
        .filter(external_links::Column::SubjectKind.eq(kind))
        .filter(external_links::Column::SubjectPid.eq(pid))
        .all(&txn)
        .await?;
    if existing.iter().any(|l| l.url == address) {
        return Err(Error::CustomError(
            StatusCode::CONFLICT,
            ErrorDetail::new("conflict", "this record already has that link"),
        ));
    }
    if existing.len() as u64 >= MAX_PER_SUBJECT {
        return Err(super::unprocessable("a record can carry at most 50 links"));
    }
    let link = external_links::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set(kind.to_string()),
        subject_pid: ActiveValue::set(pid),
        system: ActiveValue::set(payload.system.clone()),
        url: ActiveValue::set(address),
        external_id: ActiveValue::set(payload.external_id.filter(|s| !s.trim().is_empty())),
        label: ActiveValue::set(payload.label.filter(|s| !s.trim().is_empty())),
        created_by: ActiveValue::set(caller.actor().map(str::to_string)),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
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
    let txn = ctx.db.begin().await?;
    let link = external_links::Entity::find()
        .filter(external_links::Column::Pid.eq(pid))
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    let kind = match link.subject_kind.as_str() {
        "contact" => "contact",
        "account" => "account",
        "deal" => "deal",
        _ => "lead",
    };
    if kind == "contact" {
        let contact = records::find_contact(&txn, link.subject_pid).await?;
        auth::authorize_record(
            &caller,
            authentication_verifier::Action::Write,
            &auth::contact_resource_attrs(&contact),
        )
        .map_err(record_rejection)?;
    }
    external_links::Entity::delete_by_id(link.id)
        .exec(&txn)
        .await?;
    Audit::record(
        &txn,
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
