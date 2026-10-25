//! Email templates with merge tokens (CRM-R36): `/api/email-templates`.
//!
//! A template is written once and filled in per recipient when a campaign or a nurture step makes
//! its message. The parsing, checking and rendering are in [`crate::rules::templates`]; this module
//! stores templates and gathers a contact's values.

use std::collections::BTreeMap;

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::merge::conflict;
use super::{PidPath, PidRef, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::clients;
use crate::json::Json;
use crate::models::_entities::{accounts, contacts};
use crate::models::audit_logs::Model as Audit;
use crate::rules::templates as rules;
use crate::validation::Problems;

/// A template as stored.
#[derive(Debug, Clone)]
pub(crate) struct Template {
    subject: String,
    body: String,
    fallbacks: BTreeMap<String, String>,
}

/// `POST /api/email-templates` and `PUT /api/email-templates/{pid}` body (a `PUT` replaces it).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "EmailTemplatePayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplatePayload {
    /// Unique, ignoring case.
    name: String,
    /// One line; may hold merge tokens except `{{unsubscribe_url}}`.
    subject: String,
    /// Plain text; may hold merge tokens and must hold `{{unsubscribe_url}}`.
    body: String,
    /// The template's own text for a token with no value, by token name.
    #[serde(default)]
    fallbacks: BTreeMap<String, String>,
}

/// `POST /api/email-templates/preview` body: a draft, not yet saved.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "EmailTemplatePreviewPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewPayload {
    subject: String,
    body: String,
    #[serde(default)]
    fallbacks: BTreeMap<String, String>,
    /// Tokens to treat as having no value, to see their fallbacks.
    #[serde(default)]
    blank: Vec<String>,
}

/// One template.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "EmailTemplate")]
pub(crate) struct TemplateView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    subject: String,
    body: String,
    fallbacks: BTreeMap<String, String>,
    /// How many live campaigns and nurture steps refer to it.
    in_use: i64,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// A template rendered for the synthetic contact.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "EmailTemplatePreview")]
pub(crate) struct Preview {
    subject: String,
    /// The plain text that would be mailed.
    text: String,
    /// The body as HTML, every value escaped.
    html: String,
    /// The tokens that fell back to their fallback text.
    fell_back: Vec<String>,
    /// What would stop this draft being saved; empty when it can be.
    problems: Vec<String>,
}

/// One merge token.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "MergeToken")]
pub(crate) struct TokenInfo {
    name: String,
    description: String,
    /// The text used when the contact has no value; absent for `unsubscribe_url`.
    fallback: Option<String>,
    /// Whether every template body must contain it.
    required: bool,
}

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

const COLUMNS: &str = "t.pid, t.name, t.subject, t.body, t.fallbacks, t.updated_at, \
    (SELECT count(*) FROM campaigns c WHERE c.template_pid = t.pid AND c.deleted_at IS NULL) \
    + (SELECT count(*) FROM nurture_steps s WHERE s.template_pid = t.pid \
       AND s.deleted_at IS NULL) AS in_use";

fn view(row: &sea_orm::QueryResult) -> Result<TemplateView> {
    let fallbacks: serde_json::Value = row.try_get("", "fallbacks")?;
    Ok(TemplateView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        name: row.try_get("", "name")?,
        subject: row.try_get("", "subject")?,
        body: row.try_get("", "body")?,
        fallbacks: serde_json::from_value(fallbacks).unwrap_or_default(),
        in_use: row.try_get("", "in_use")?,
        updated_at: row.try_get("", "updated_at")?,
    })
}

fn validate(payload: &TemplatePayload) -> Result<()> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if payload.name.trim().chars().count() > rules::MAX_NAME {
        problems.push_field(
            "name",
            format!("is longer than {} characters", rules::MAX_NAME),
        );
    }
    for (field, message) in rules::check(&payload.subject, &payload.body, &payload.fallbacks) {
        problems.push_field(&field, message);
    }
    problems.finish()
}

fn taken() -> Error {
    unprocessable("a template with that name already exists")
}

/// `GET /api/email-templates/tokens` — the merge tokens and their default fallbacks.
#[debug_handler]
async fn tokens() -> Result<Response> {
    let mut out: Vec<TokenInfo> = rules::TOKENS
        .iter()
        .map(|t| TokenInfo {
            name: t.name.to_string(),
            description: t.description.to_string(),
            fallback: Some(t.fallback.to_string()),
            required: false,
        })
        .collect();
    out.push(TokenInfo {
        name: rules::UNSUBSCRIBE.to_string(),
        description: "The contact's signed unsubscribe link; every body must contain it"
            .to_string(),
        fallback: None,
        required: true,
    });
    format::json(out)
}

/// `GET /api/email-templates`.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!("SELECT {COLUMNS} FROM email_templates t ORDER BY lower(t.name), t.id"),
            [],
        ))
        .await?;
    format::json(rows.iter().map(view).collect::<Result<Vec<_>>>()?)
}

/// `GET /api/email-templates/{pid}`.
#[debug_handler]
async fn show(State(ctx): State<AppContext>, PidPath(pid): PidPath) -> Result<Response> {
    let row = ctx
        .db
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM email_templates t WHERE t.pid = $1"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    format::json(view(&row)?)
}

fn bind(payload: &TemplatePayload) -> Vec<Value> {
    vec![
        payload.name.trim().to_string().into(),
        payload.subject.clone().into(),
        payload.body.clone().into(),
        serde_json::json!(payload.fallbacks).into(),
    ]
}

/// `POST /api/email-templates`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<TemplatePayload>,
) -> Result<Response> {
    validate(&payload)?;
    let pid = Uuid::new_v4();
    let mut values = vec![pid.into()];
    values.extend(bind(&payload));
    let txn = ctx.db.begin().await?;
    let inserted = txn
        .execute_raw(stmt(
            "INSERT INTO email_templates (pid, name, subject, body, fallbacks) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            values,
        ))
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(taken());
    }
    Audit::record(&txn, "email_template", pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `PUT /api/email-templates/{pid}` — replace a template.
#[debug_handler]
async fn replace(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
    Json(payload): Json<TemplatePayload>,
) -> Result<Response> {
    validate(&payload)?;
    let mut values = vec![pid.into()];
    values.extend(bind(&payload));
    let txn = ctx.db.begin().await?;
    let changed = txn
        .execute_raw(stmt(
            "UPDATE email_templates SET name = $2, subject = $3, body = $4, fallbacks = $5, \
             updated_at = now() WHERE pid = $1",
            values,
        ))
        .await;
    // The only constraint a replace can break is the unique name.
    if changed.is_err() {
        return Err(taken());
    }
    if changed.is_ok_and(|r| r.rows_affected() == 0) {
        return Err(Error::NotFound);
    }
    Audit::record(&txn, "email_template", pid, "updated", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `DELETE /api/email-templates/{pid}` — refused while a campaign or step uses it.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let row = txn
        .query_one_raw(stmt(
            &format!("SELECT {COLUMNS} FROM email_templates t WHERE t.pid = $1 FOR UPDATE OF t"),
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    if view(&row)?.in_use > 0 {
        return Err(conflict(
            "a campaign or nurture step still uses this template",
        ));
    }
    txn.execute_raw(stmt(
        "DELETE FROM email_templates WHERE pid = $1",
        [pid.into()],
    ))
    .await?;
    Audit::record(&txn, "email_template", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// `POST /api/email-templates/preview` — a draft rendered for a synthetic contact.
#[debug_handler]
async fn preview(Json(payload): Json<PreviewPayload>) -> Result<Response> {
    let found = rules::check(&payload.subject, &payload.body, &payload.fallbacks);
    let mut problems = Problems::new();
    for token in &payload.blank {
        if !rules::TOKENS.iter().any(|t| t.name == token) {
            problems.push_field("blank", format!("{token} is not a token that can be blank"));
        }
    }
    if rules::parse(&payload.subject).is_err() || rules::parse(&payload.body).is_err() {
        for (field, message) in &found {
            problems.push_field(field, message.clone());
        }
    }
    problems.finish()?;
    let mut values = rules::synthetic_values();
    for token in &payload.blank {
        values.remove(token);
    }
    let rendered = rules::render(
        &payload.subject,
        &payload.body,
        &rules::Fill {
            values: &values,
            fallbacks: &payload.fallbacks,
            unsubscribe_url: PREVIEW_UNSUBSCRIBE,
        },
    );
    format::json(Preview {
        subject: rendered.subject,
        text: rendered.text,
        html: rendered.html,
        fell_back: rendered.fell_back,
        problems: found
            .into_iter()
            .map(|(field, message)| format!("{field} {message}"))
            .collect(),
    })
}

/// The link shown in a preview. It is not a real token and goes nowhere.
const PREVIEW_UNSUBSCRIBE: &str = "https://example.invalid/unsubscribe/preview";

/// A saved template, or `None` when it is gone.
pub(crate) async fn load(db: &impl ConnectionTrait, pid: Uuid) -> Result<Option<Template>> {
    let Some(row) = db
        .query_one_raw(stmt(
            "SELECT subject, body, fallbacks FROM email_templates WHERE pid = $1",
            [pid.into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    let fallbacks: serde_json::Value = row.try_get("", "fallbacks")?;
    Ok(Some(Template {
        subject: row.try_get("", "subject")?,
        body: row.try_get("", "body")?,
        fallbacks: serde_json::from_value(fallbacks).unwrap_or_default(),
    }))
}

/// The values a contact supplies to the tokens. A token the contact has no value for is left out.
pub(crate) async fn contact_values(
    db: &impl ConnectionTrait,
    contact: &contacts::Model,
) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    if let Some(first) = rules::first_word(&contact.display_name) {
        values.insert("first_name".to_string(), first);
    }
    values.insert("name".to_string(), contact.display_name.clone());
    if let Some(title) = &contact.job_title {
        values.insert("job_title".to_string(), title.clone());
    }
    let account = match contact.account_pid {
        Some(pid) => {
            accounts::Entity::find()
                .filter(accounts::Column::Pid.eq(pid))
                .filter(accounts::Column::DeletedAt.is_null())
                .one(db)
                .await?
        }
        None => None,
    };
    if let Some(account) = &account {
        values.insert("account_name".to_string(), account.display_name.clone());
    }
    let owner = contact
        .owner_ref
        .as_deref()
        .or(account.as_ref().and_then(|a| a.owner_ref.as_deref()));
    if let Some(owner) = owner
        && let Ok(entity_ref) = <entity_ref::EntityRef as std::str::FromStr>::from_str(owner)
        && let Some(name) = clients::display_name(&entity_ref).await
    {
        values.insert("rep_name".to_string(), name);
    }
    Ok(values)
}

/// The subject and plain text of template `pid` for `contact`, or `None` when the template is
/// gone. `unsubscribe_url` is the contact's signed link; real sending always has one, so an absent
/// link only happens when nothing is mailed.
pub(crate) async fn content_for(
    db: &impl ConnectionTrait,
    pid: Uuid,
    contact: &contacts::Model,
    unsubscribe_url: Option<&str>,
) -> Result<Option<(String, String)>> {
    let Some(template) = load(db, pid).await? else {
        return Ok(None);
    };
    let values = contact_values(db, contact).await?;
    let rendered = rules::render(
        &template.subject,
        &template.body,
        &rules::Fill {
            values: &values,
            fallbacks: &template.fallbacks,
            unsubscribe_url: unsubscribe_url.unwrap_or_default(),
        },
    );
    Ok(Some((rendered.subject, rendered.text)))
}

/// Whether template `pid` exists.
pub(crate) async fn exists(db: &impl ConnectionTrait, pid: Uuid) -> Result<bool> {
    Ok(load(db, pid).await?.is_some())
}

/// The email template routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/email-templates", get(list))
        .add("/email-templates", post(create))
        .add("/email-templates/tokens", get(tokens))
        .add("/email-templates/preview", post(preview))
        .add("/email-templates/{pid}", get(show))
        .add("/email-templates/{pid}", put(replace))
        .add("/email-templates/{pid}", delete(remove))
}
