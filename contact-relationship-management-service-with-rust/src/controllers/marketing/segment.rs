//! Segments and their preview (CRM-R7).

use super::{
    AppContext, Audit, Deserialize, Error, MaybeAuthUser, PidRef, Problems, Response, Result,
    State, Uuid, contacts, debug_handler, format, segment_rules, segments, tokens,
};
use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::controllers::PidPath;
use crate::db;
use crate::json::Json;
use crate::models::rows;

/// `POST /api/segments` body.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SegmentPayload {
    name: String,
    /// The filter language of `rules::segment` (see the segment spec).
    #[serde(default)]
    #[schemars(with = "serde_json::Map<String, serde_json::Value>")]
    filter: segment_rules::Filter,
}

/// `POST /api/segments`.
#[debug_handler]
pub(super) async fn create_segment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SegmentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    for status in &payload.filter.statuses {
        problems.require_token("filter.statuses", tokens::CONTACT_STATUSES, status);
    }
    for channel in &payload.filter.channels {
        problems.require_token("filter.channels", tokens::CHANNELS, channel);
    }
    for tier in &payload.filter.account_tiers {
        problems.require_token("filter.account_tiers", tokens::ACCOUNT_TIERS, tier);
    }
    // Tags are tidied the way a record's are, so a segment and a record agree on a name.
    let mut filter = payload.filter.clone();
    match crate::rules::tags::set_of(&payload.filter.tags) {
        Ok(tags) => filter.tags = tags,
        Err(why) => problems.push_field("filter.tags", why),
    }
    problems.finish()?;
    let pid = Uuid::new_v4();
    let mut txn = db::pool(&ctx).begin().await?;
    sqlx::query!(
        "INSERT INTO segments (pid, name, filter) VALUES ($1, $2, $3)",
        pid,
        payload.name,
        serde_json::to_value(&filter)
            .map_err(|_| Error::string("the segment filter could not be stored"))?,
    )
    .execute(&mut *txn)
    .await?;
    Audit::record_on(&mut txn, "segment", pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// The contacts a segment matches right now, gated on consent for `purpose`.
pub(super) async fn evaluate_segment(
    pool: &PgPool,
    segment: &segments::Model,
    purpose: &str,
) -> Result<Vec<contacts::Model>> {
    let filter: segment_rules::Filter = crate::json::stored(segment.filter.clone(), "filter")?;
    let all = rows::contacts(pool).await?;
    let facts = facts_of_all(pool, &all, purpose).await?;
    Ok(all
        .into_iter()
        .zip(facts)
        .filter(|(_, facts)| segment_rules::matches(&filter, facts))
        .map(|(contact, _)| contact)
        .collect())
}

/// Each contact's segment facts, in the order given, from four queries whatever the number of
/// contacts (CRM-T243): their accounts' tiers, their tags, which of them are employees, and their
/// consent for `purpose` when it is not `marketing` (whose state is on the contact). One query a
/// contact for each of these made a preview of a few thousand contacts take most of a second and
/// hold a connection all the while.
pub(super) async fn facts_of_all(
    pool: &PgPool,
    contacts: &[contacts::Model],
    purpose: &str,
) -> Result<Vec<segment_rules::ContactFacts>> {
    let pids: Vec<Uuid> = contacts.iter().map(|c| c.pid).collect();
    let account_pids: Vec<Uuid> = contacts.iter().filter_map(|c| c.account_pid).collect();
    let tiers: HashMap<Uuid, String> = sqlx::query!(
        "SELECT pid, tier FROM accounts WHERE pid = ANY($1) AND deleted_at IS NULL",
        &account_pids,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| (row.pid, row.tier))
    .collect();
    let mut tags: HashMap<Uuid, Vec<String>> = HashMap::new();
    for row in sqlx::query!(
        "SELECT g.subject_pid, t.name FROM taggings g JOIN tags t ON t.pid = g.tag_pid \
         WHERE g.subject_pid = ANY($1) ORDER BY g.subject_pid, lower(t.name)",
        &pids,
    )
    .fetch_all(pool)
    .await?
    {
        tags.entry(row.subject_pid).or_default().push(row.name);
    }
    let employees: HashSet<Uuid> = sqlx::query_scalar!(
        "SELECT DISTINCT contact_pid FROM contact_employments WHERE contact_pid = ANY($1)",
        &pids,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    let consents: HashMap<Uuid, String> = if purpose == tokens::DEFAULT_PURPOSE {
        HashMap::new()
    } else {
        sqlx::query!(
            "SELECT contact_pid, state FROM contact_consents \
             WHERE contact_pid = ANY($1) AND purpose = $2 AND channel = 'email'",
            &pids,
            purpose,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| (row.contact_pid, row.state))
        .collect()
    };
    Ok(contacts
        .iter()
        .map(|contact| segment_rules::ContactFacts {
            consent: if purpose == tokens::DEFAULT_PURPOSE {
                contact.marketing_consent.clone()
            } else {
                // A purpose nobody has asked about is `never`, as `consents::state_of` says.
                consents
                    .get(&contact.pid)
                    .cloned()
                    .unwrap_or_else(|| "never".to_string())
            },
            status: contact.status.clone(),
            channel: contact.preferred_channel.as_str().to_string(),
            account_tier: contact.account_pid.and_then(|a| tiers.get(&a).cloned()),
            tags: tags.remove(&contact.pid).unwrap_or_default(),
            employee: employees.contains(&contact.pid),
        })
        .collect())
}

/// `GET /api/segments/{pid}/preview` query.
#[derive(Debug, Deserialize)]
pub(super) struct PreviewQuery {
    /// The consent purpose to gate on (`marketing` when absent).
    purpose: Option<String>,
}

/// `GET /api/segments/{pid}/preview`: how many contacts match, and a few names.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct Preview {
    count: usize,
    sample: Vec<String>,
}

/// `GET /api/segments/{pid}/preview` — count + sample before
/// scheduling (CRM-R7).
#[debug_handler]
pub(super) async fn preview_segment(
    State(ctx): State<AppContext>,
    PidPath(pid): PidPath,
    axa::extract::Query(query): axa::extract::Query<PreviewQuery>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt("purpose", tokens::PURPOSES, query.purpose.as_deref());
    problems.finish()?;
    let pool = db::pool(&ctx);
    let segment = rows::segment(pool, pid).await?.ok_or(Error::NotFound)?;
    let purpose = query.purpose.as_deref().unwrap_or(tokens::DEFAULT_PURPOSE);
    let matched = evaluate_segment(pool, &segment, purpose).await?;
    let sample: Vec<_> = matched
        .iter()
        .take(5)
        .map(|c| c.display_name.clone())
        .collect();
    format::json(Preview {
        count: matched.len(),
        sample,
    })
}

/// `GET /api/segments`.
#[debug_handler]
pub(super) async fn list_segments(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(rows::segments(db::pool(&ctx)).await?)
}
