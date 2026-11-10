//! Typed custom fields (CRM-R41): `/api/custom-fields` and the values on each record.
//!
//! An admin declares a field on contacts, accounts, leads or deals; anyone who may write the
//! record sets its values. Each field declares its sensitivity ([`rules::SENSITIVITIES`]), and that
//! decides masking, storage, subject access and erasure for its values.
//!
//! The routes run on `sqlx` (CRM-D24, CRM-T206), and so, since CRM-T216, do [`subject`] and
//! [`export_for`].

use std::collections::BTreeMap;

use axa::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::merge::conflict;
use super::{PidPath, PidRef, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::crypto;
use crate::db;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::models::rows;
use crate::rules::custom_fields as rules;
use crate::validation::Problems;

/// `POST /api/custom-fields` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "CustomFieldPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct DefPayload {
    /// `contact`, `account`, `lead` or `deal`.
    entity: String,
    /// The name the API uses; lower case, fixed once made.
    key: String,
    /// What people see.
    label: String,
    /// `text`, `number`, `date` or `choice`; fixed once made.
    kind: String,
    /// The allowed values of a `choice` field.
    #[serde(default)]
    choices: Vec<String>,
    /// `business`, `personal` or `sensitive`; fixed once made.
    sensitivity: String,
}

/// `PUT /api/custom-fields/{pid}` body: what can change after the field is made.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "CustomFieldChange")]
#[serde(deny_unknown_fields)]
pub(crate) struct DefChange {
    label: String,
    #[serde(default)]
    choices: Vec<String>,
    #[serde(default)]
    position: i32,
}

/// One declared field.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "CustomField")]
pub(crate) struct DefView {
    pid: Uuid,
    entity: String,
    key: String,
    label: String,
    kind: String,
    choices: Vec<String>,
    sensitivity: String,
    position: i32,
}

/// `PUT /api/{records}/{pid}/custom-fields` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "CustomFieldValuesPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct ValuesPayload {
    /// Field key to value; `null` clears it. A field not named is left as it is.
    #[schemars(with = "BTreeMap<String, Option<serde_json::Value>>")]
    values: BTreeMap<String, Option<serde_json::Value>>,
}

/// One field of a record, with its value.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "CustomFieldValue")]
pub(crate) struct ValueView {
    key: String,
    label: String,
    kind: String,
    sensitivity: String,
    choices: Vec<String>,
    /// The value, or `null` when none is set or the caller sees it masked.
    #[schemars(with = "Option<serde_json::Value>")]
    value: Option<serde_json::Value>,
    /// The caller may not see this value (a masked read of a personal field).
    masked: bool,
}

#[derive(Debug, Clone)]
struct Def {
    pid: Uuid,
    entity: String,
    key: String,
    label: String,
    kind: String,
    choices: Vec<String>,
    sensitivity: String,
    position: i32,
}

/// A `custom_field_defs` row as `sqlx` reads it.
struct DefRow {
    pid: Uuid,
    entity: String,
    key: String,
    label: String,
    kind: String,
    choices: serde_json::Value,
    sensitivity: String,
    position: i32,
}

impl From<DefRow> for Def {
    fn from(row: DefRow) -> Self {
        Self {
            pid: row.pid,
            entity: row.entity,
            key: row.key,
            label: row.label,
            kind: row.kind,
            choices: serde_json::from_value(row.choices).unwrap_or_default(),
            sensitivity: row.sensitivity,
            position: row.position,
        }
    }
}

impl Def {
    fn view(&self) -> DefView {
        DefView {
            pid: self.pid,
            entity: self.entity.clone(),
            key: self.key.clone(),
            label: self.label.clone(),
            kind: self.kind.clone(),
            choices: self.choices.clone(),
            sensitivity: self.sensitivity.clone(),
            position: self.position,
        }
    }

    /// Whether the value is kept sealed in the database.
    fn sealed(&self) -> bool {
        self.sensitivity == "sensitive"
    }

    /// Whether the value is about a person (masked, exported, erased).
    fn personal(&self) -> bool {
        self.sensitivity != "business"
    }
}

/// The declared fields of `entity`, or of every entity.
async fn defs(conn: &mut PgConnection, entity: Option<&str>) -> Result<Vec<Def>> {
    let rows = sqlx::query_as!(
        DefRow,
        "SELECT pid, entity, key, label, kind, choices, sensitivity, position \
         FROM custom_field_defs WHERE ($1::text IS NULL OR entity = $1) \
         ORDER BY entity, position, id",
        entity
    )
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(Def::from).collect())
}

async fn def_by_pid(conn: &mut PgConnection, pid: Uuid) -> Result<Def> {
    sqlx::query_as!(
        DefRow,
        "SELECT pid, entity, key, label, kind, choices, sensitivity, position \
         FROM custom_field_defs WHERE pid = $1",
        pid
    )
    .fetch_optional(conn)
    .await?
    .map(Def::from)
    .ok_or(Error::NotFound)
}

fn problems_of(found: Vec<(String, String)>) -> Result<()> {
    let mut problems = Problems::new();
    for (member, message) in found {
        problems.push_field(&member, message);
    }
    problems.finish()
}

/// `GET /api/custom-fields?entity=` — the declared fields.
#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    axa::extract::Query(query): axa::extract::Query<BTreeMap<String, String>>,
) -> Result<Response> {
    let mut conn = db::pool(&ctx).acquire().await?;
    let defs = defs(&mut conn, query.get("entity").map(String::as_str)).await?;
    format::json(defs.iter().map(Def::view).collect::<Vec<_>>())
}

/// `POST /api/custom-fields`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<DefPayload>,
) -> Result<Response> {
    problems_of(rules::check_def(
        &payload.entity,
        &payload.key,
        &payload.label,
        &payload.kind,
        &payload.choices,
        &payload.sensitivity,
    ))?;
    let mut txn = db::pool(&ctx).begin().await?;
    let held = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM custom_field_defs WHERE entity = $1",
        payload.entity
    )
    .fetch_one(&mut *txn)
    .await?;
    if held >= rules::MAX_PER_ENTITY {
        return Err(unprocessable(&format!(
            "a {} may have at most {} custom fields",
            payload.entity,
            rules::MAX_PER_ENTITY
        )));
    }
    let pid = Uuid::new_v4();
    let inserted = sqlx::query!(
        "INSERT INTO custom_field_defs (pid, entity, key, label, kind, choices, sensitivity, position) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
        pid,
        payload.entity,
        payload.key,
        payload.label.trim(),
        payload.kind,
        serde_json::json!(payload.choices),
        payload.sensitivity,
        i32::try_from(held).unwrap_or(i32::MAX),
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(unprocessable(&format!(
            "a {} already has a custom field with the key {}",
            payload.entity, payload.key
        )));
    }
    Audit::record_on(
        &mut txn,
        "custom_field",
        pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({
            "entity": payload.entity, "key": payload.key, "kind": payload.kind,
            "sensitivity": payload.sensitivity,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `PUT /api/custom-fields/{pid}` — change the label, the choices or the position.
#[debug_handler]
async fn change(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<DefChange>,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let def = def_by_pid(&mut txn, pid).await?;
    let mut found = rules::check_choices(&def.kind, &payload.choices);
    if payload.label.trim().is_empty() || payload.label.chars().count() > rules::MAX_LABEL {
        found.push((
            "label".to_string(),
            format!("must be 1 to {} characters", rules::MAX_LABEL),
        ));
    }
    problems_of(found)?;
    // A choice that values still use cannot be taken away. Sealed values are opened to compare.
    let rows = sqlx::query_scalar!(
        "SELECT value FROM custom_field_values WHERE field_pid = $1",
        pid
    )
    .fetch_all(&mut *txn)
    .await?;
    let mut held = Vec::new();
    for raw in rows {
        held.push(if def.sealed() {
            crypto::open(&raw).map_err(|e| Error::string(&e.to_string()))?
        } else {
            raw
        });
    }
    for gone in def.choices.iter().filter(|c| !payload.choices.contains(c)) {
        let uses = held.iter().filter(|v| *v == gone).count();
        if uses > 0 {
            return Err(conflict(&format!(
                "{uses} record(s) still use the choice {gone}"
            )));
        }
    }
    sqlx::query!(
        "UPDATE custom_field_defs SET label = $2, choices = $3, position = $4, updated_at = now() \
         WHERE pid = $1",
        pid,
        payload.label.trim(),
        serde_json::json!(payload.choices),
        payload.position,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(
        &mut txn,
        "custom_field",
        pid,
        "updated",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `DELETE /api/custom-fields/{pid}` — the field and every value it holds.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let mut txn = db::pool(&ctx).begin().await?;
    let def = def_by_pid(&mut txn, pid).await?;
    let values = sqlx::query!("DELETE FROM custom_field_values WHERE field_pid = $1", pid)
        .execute(&mut *txn)
        .await?
        .rows_affected();
    sqlx::query!("DELETE FROM custom_field_defs WHERE pid = $1", pid)
        .execute(&mut *txn)
        .await?;
    Audit::record_on(
        &mut txn,
        "custom_field",
        pid,
        "deleted",
        caller.actor(),
        Some(serde_json::json!({
            "entity": def.entity, "key": def.key, "values_deleted": values,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// Check the record exists and the caller may act on it; says whether the caller's read is masked.
pub(super) async fn subject(
    pool: &sqlx::PgPool,
    caller: &MaybeAuthUser,
    entity: &str,
    pid: Uuid,
    action: authentication_verifier::Action,
) -> Result<bool> {
    let attrs = match entity {
        "contact" => Some(auth::contact_resource_attrs(
            &rows::contact(pool, pid).await?.ok_or(Error::NotFound)?,
        )),
        "deal" => Some(auth::deal_resource_attrs(
            &rows::deal(pool, pid).await?.ok_or(Error::NotFound)?,
        )),
        "account" => {
            rows::account(pool, pid).await?.ok_or(Error::NotFound)?;
            None
        }
        _ => {
            rows::lead(pool, pid).await?.ok_or(Error::NotFound)?;
            None
        }
    };
    let Some(attrs) = attrs else {
        return Ok(false);
    };
    let obligations = auth::authorize_record(caller, action, &attrs).map_err(record_rejection)?;
    Ok(obligations.iter().any(|o| o == "mask"))
}

/// The values stored for the record, by field pid, opened.
async fn stored(
    conn: &mut PgConnection,
    subject_pid: Uuid,
    defs: &[Def],
) -> Result<BTreeMap<Uuid, String>> {
    let rows = sqlx::query!(
        "SELECT field_pid, value FROM custom_field_values WHERE subject_pid = $1",
        subject_pid
    )
    .fetch_all(conn)
    .await?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (field, value) = (row.field_pid, row.value);
        let Some(def) = defs.iter().find(|d| d.pid == field) else {
            continue;
        };
        let value = if def.sealed() {
            crypto::open(&value).map_err(|e| Error::string(&e.to_string()))?
        } else {
            value
        };
        out.insert(field, value);
    }
    Ok(out)
}

/// A record's fields with their values, masked where the caller's read is masked.
async fn values_of(
    conn: &mut PgConnection,
    entity: &str,
    subject_pid: Uuid,
    masked: bool,
) -> Result<Vec<ValueView>> {
    let defs = defs(&mut *conn, Some(entity)).await?;
    let held = stored(conn, subject_pid, &defs).await?;
    Ok(defs
        .iter()
        .map(|d| {
            let hide = masked && d.personal();
            ValueView {
                key: d.key.clone(),
                label: d.label.clone(),
                kind: d.kind.clone(),
                sensitivity: d.sensitivity.clone(),
                choices: d.choices.clone(),
                value: if hide {
                    None
                } else {
                    held.get(&d.pid).map(|v| rules::to_json(&d.kind, v))
                },
                masked: hide && held.contains_key(&d.pid),
            }
        })
        .collect())
}

async fn get_values(
    ctx: AppContext,
    caller: MaybeAuthUser,
    entity: &'static str,
    pid: Uuid,
) -> Result<Response> {
    let masked = subject(
        db::pool(&ctx),
        &caller,
        entity,
        pid,
        authentication_verifier::Action::Read,
    )
    .await?;
    let mut conn = db::pool(&ctx).acquire().await?;
    format::json(values_of(&mut conn, entity, pid, masked).await?)
}

async fn put_values(
    ctx: AppContext,
    caller: MaybeAuthUser,
    entity: &'static str,
    pid: Uuid,
    payload: ValuesPayload,
) -> Result<Response> {
    subject(
        db::pool(&ctx),
        &caller,
        entity,
        pid,
        authentication_verifier::Action::Write,
    )
    .await?;
    let mut txn = db::pool(&ctx).begin().await?;
    let defs = defs(&mut txn, Some(entity)).await?;
    if payload.values.is_empty() {
        return Err(unprocessable("nothing to change"));
    }
    let mut problems = Problems::new();
    let mut plan: Vec<(&Def, Option<String>)> = Vec::new();
    for (key, value) in &payload.values {
        let Some(def) = defs.iter().find(|d| &d.key == key) else {
            problems.push_field(key, format!("{key} is not a custom field of a {entity}"));
            continue;
        };
        match value {
            None | Some(serde_json::Value::Null) => plan.push((def, None)),
            Some(v) => match rules::check_value(&def.kind, &def.choices, v) {
                Ok(text) => plan.push((def, Some(text))),
                Err(why) => problems.push_field(key, why),
            },
        }
    }
    problems.finish()?;
    for (def, value) in &plan {
        match value {
            Some(text) => {
                let kept = if def.sealed() {
                    crypto::seal(text)
                } else {
                    text.clone()
                };
                sqlx::query!(
                    "INSERT INTO custom_field_values (field_pid, subject_pid, value) \
                     VALUES ($1, $2, $3) ON CONFLICT (field_pid, subject_pid) \
                     DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
                    def.pid,
                    pid,
                    kept,
                )
                .execute(&mut *txn)
                .await?;
            }
            None => {
                sqlx::query!(
                    "DELETE FROM custom_field_values WHERE field_pid = $1 AND subject_pid = $2",
                    def.pid,
                    pid,
                )
                .execute(&mut *txn)
                .await?;
            }
        }
    }
    // The audit row names the fields, never the values.
    Audit::record_on(
        &mut txn,
        entity,
        pid,
        "custom_fields_updated",
        caller.actor(),
        Some(serde_json::json!({ "fields": plan.iter().map(|(d, _)| d.key.clone()).collect::<Vec<_>>() })),
    )
    .await?;
    let values = values_of(&mut txn, entity, pid, false).await?;
    txn.commit().await?;
    format::json(values)
}

macro_rules! record_routes {
    ($get:ident, $put:ident, $entity:literal) => {
        #[debug_handler]
        async fn $get(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
        ) -> Result<Response> {
            get_values(ctx, caller, $entity, pid).await
        }

        #[debug_handler]
        async fn $put(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            PidPath(pid): PidPath,
            Json(payload): Json<ValuesPayload>,
        ) -> Result<Response> {
            put_values(ctx, caller, $entity, pid, payload).await
        }
    };
}

record_routes!(contact_values, set_contact_values, "contact");
record_routes!(account_values, set_account_values, "account");
record_routes!(lead_values, set_lead_values, "lead");
record_routes!(deal_values, set_deal_values, "deal");

/// What a subject-access export holds of a person's custom fields: every field set on the
/// contact or on a contact merged into it, unmasked.
///
/// # Errors
///
/// A database or key error.
pub(crate) async fn export_for(
    conn: &mut PgConnection,
    contact: Uuid,
) -> Result<Vec<serde_json::Value>> {
    let defs = defs(&mut *conn, Some("contact")).await?;
    let rows = sqlx::query!(
        "SELECT field_pid, subject_pid, value FROM custom_field_values WHERE subject_pid = $1 \
         OR subject_pid IN (SELECT pid FROM contacts WHERE merged_into_pid = $1 \
         /* whatever deleted_at holds: a merged duplicate is the subject's */) ORDER BY id",
        contact,
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Vec::new();
    for row in rows {
        let Some(def) = defs.iter().find(|d| d.pid == row.field_pid) else {
            continue;
        };
        let value = if def.sealed() {
            crypto::open(&row.value).map_err(|e| Error::string(&e.to_string()))?
        } else {
            row.value
        };
        out.push(serde_json::json!({
            "key": def.key, "label": def.label, "sensitivity": def.sensitivity,
            "from_contact": row.subject_pid,
            "value": rules::to_json(&def.kind, &value),
        }));
    }
    Ok(out)
}

/// The SQL that deletes the personal and sensitive values of a contact being erased: its own, the
/// contacts merged into it, and the leads the erasure also scrubs.
#[must_use]
pub(crate) fn erase_sql(contact: Uuid) -> String {
    format!(
        "DELETE FROM custom_field_values v USING custom_field_defs d \
         WHERE d.pid = v.field_pid AND d.sensitivity <> 'business' AND ( \
           (d.entity = 'contact' AND (v.subject_pid = '{contact}' OR v.subject_pid IN \
              (SELECT pid FROM contacts WHERE merged_into_pid = '{contact}'))) \
        OR (d.entity = 'lead' AND v.subject_pid IN \
              (SELECT pid FROM leads WHERE contact_pid = '{contact}')))"
    )
}

/// The custom field routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/custom-fields", get(list))
        .add("/custom-fields", post(create))
        .add("/custom-fields/{pid}", put(change))
        .add("/custom-fields/{pid}", delete(remove))
        .add("/contacts/{pid}/custom-fields", get(contact_values))
        .add("/contacts/{pid}/custom-fields", put(set_contact_values))
        .add("/accounts/{pid}/custom-fields", get(account_values))
        .add("/accounts/{pid}/custom-fields", put(set_account_values))
        .add("/leads/{pid}/custom-fields", get(lead_values))
        .add("/leads/{pid}/custom-fields", put(set_lead_values))
        .add("/deals/{pid}/custom-fields", get(deal_values))
        .add("/deals/{pid}/custom-fields", put(set_deal_values))
}
