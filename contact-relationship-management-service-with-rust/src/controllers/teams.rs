//! Teams and their quota roll-up (CRM-R67).
//!
//! `/api/quotas/teams` is kept by hand and audited; `GET /api/quotas/team-attainment` adds the
//! standings of each team's workers. Both sit under the admin-only `/api/quotas` prefix: what a
//! team is paid is no more everyone's to read than what a person is.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::quotas::standings;
use super::views::owner_of;
use super::{PidPath, PidRef};
use crate::auth::MaybeAuthUser;
use crate::json::Json;
use crate::models::audit_logs::Model as Audit;
use crate::rules::analytics::{self, Ratio};
use crate::rules::quota;
use crate::rules::user_ref;
use crate::validation::Problems;

/// Most workers on a team.
const MAX_MEMBERS: usize = 50;
/// Most teams listed.
const MAX_LISTED: i64 = 500;

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// `PUT /api/quotas/teams` body: sets the team of that name (any case), replacing its workers.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "TeamPayload")]
#[serde(deny_unknown_fields)]
pub(crate) struct TeamPayload {
    /// The team's name, up to 80 characters.
    name: String,
    /// The people on it (`person:<id>`, the sign-in subject; `worker:<id>` is read as
    /// `person:<id>`, CRM-D18), from 1 to 50, each named once.
    members: Vec<String>,
}

/// A team.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "Team")]
pub(crate) struct TeamView {
    #[schemars(with = "uuid::Uuid")]
    pid: String,
    name: String,
    members: Vec<String>,
    set_by: String,
}

fn team_view(row: &sea_orm::QueryResult) -> Result<TeamView> {
    let members: serde_json::Value = row.try_get("", "members")?;
    Ok(TeamView {
        pid: row.try_get::<Uuid>("", "pid")?.to_string(),
        name: row.try_get("", "name")?,
        members: serde_json::from_value(members).map_err(|e| Error::string(&e.to_string()))?,
        set_by: row.try_get("", "set_by")?,
    })
}

const TEAM_COLUMNS: &str = "pid, name, members, set_by";

/// `GET /api/quotas/teams` — the teams, by name.
#[debug_handler]
async fn list_teams(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = ctx
        .db
        .query_all_raw(stmt(
            &format!("SELECT {TEAM_COLUMNS} FROM teams ORDER BY lower(name) LIMIT $1"),
            [MAX_LISTED.into()],
        ))
        .await?;
    format::json(rows.iter().map(team_view).collect::<Result<Vec<_>>>()?)
}

/// Set the team of `name` (any case) to `members` inside `txn`, audited; `via` names what caused it
/// (the worker service import, CRM-R73).
pub(super) async fn put_team<C: ConnectionTrait>(
    txn: &C,
    who: &str,
    actor: Option<&str>,
    name: &str,
    members: &[String],
    via: Option<&str>,
) -> Result<TeamView> {
    let members_json = serde_json::to_value(members).map_err(|e| Error::string(&e.to_string()))?;
    let row = txn
        .query_one_raw(stmt(
            &format!(
                "INSERT INTO teams (pid, name, members, set_by) VALUES ($1, $2, $3::jsonb, $4) \
                 ON CONFLICT ((lower(name))) DO UPDATE SET name = EXCLUDED.name, \
                   members = EXCLUDED.members, set_by = EXCLUDED.set_by, updated_at = now() \
                 RETURNING {TEAM_COLUMNS}"
            ),
            [
                Uuid::new_v4().into(),
                name.into(),
                members_json.into(),
                who.into(),
            ],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    let made = team_view(&row)?;
    Audit::record(
        txn,
        "team",
        row.try_get("", "pid")?,
        "set",
        actor,
        Some(match via {
            Some(via) => {
                serde_json::json!({ "name": made.name, "members": made.members, "via": via })
            }
            None => serde_json::json!({ "name": made.name, "members": made.members }),
        }),
    )
    .await?;
    Ok(made)
}

/// `PUT /api/quotas/teams` — set a team; audited.
#[debug_handler]
async fn set_team(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<TeamPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    let name = quota::name(&payload.name)
        .map_err(|why| problems.push_field("name", why.replace("a plan", "a team")))
        .ok();
    if payload.members.is_empty() || payload.members.len() > MAX_MEMBERS {
        problems.push_field("members", format!("from 1 to {MAX_MEMBERS} workers"));
    }
    user_ref::normalise_all(&mut payload.members);
    for member in &payload.members {
        problems.require_user("members", member);
    }
    let mut seen = std::collections::BTreeSet::new();
    if !payload.members.iter().all(|m| seen.insert(m.as_str())) {
        problems.push_field("members", "names a worker twice");
    }
    problems.finish()?;
    let Some(name) = name else {
        return Err(super::unprocessable("the team cannot be set"));
    };
    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    let made = put_team(&txn, &who, caller.actor(), &name, &payload.members, None).await?;
    txn.commit().await?;
    format::json(made)
}

/// `DELETE /api/quotas/teams/{pid}` — remove a team (its workers' quotas are untouched); audited.
/// `404` when there is none.
#[debug_handler]
async fn remove_team(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let txn = ctx.db.begin().await?;
    let gone = txn
        .execute_raw(stmt("DELETE FROM teams WHERE pid = $1", [pid.into()]))
        .await?;
    if gone.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Audit::record(&txn, "team", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: pid.to_string(),
    })
}

/// What `GET /api/quotas/team-attainment` may be asked.
#[derive(Debug, Deserialize)]
struct TeamQuery {
    /// The day the quotas' periods must contain (today when absent).
    at: Option<NaiveDate>,
}

/// A team's quotas and revenue in one currency.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TeamCurrency")]
pub(crate) struct TeamCurrency {
    currency: String,
    /// The workers' quotas in this currency, added.
    quota_minor: i64,
    /// Their credited won revenue in this currency, added.
    revenue_minor: i64,
    won_deals: i64,
    /// `revenue ÷ quota`, with both numbers.
    attainment: Ratio,
    /// What is still to win; zero once met.
    remaining_minor: i64,
    /// How many workers have a quota in this currency.
    workers: usize,
}

/// One team's roll-up.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TeamStanding")]
pub(crate) struct TeamStanding {
    #[schemars(with = "uuid::Uuid")]
    team_pid: String,
    name: String,
    members: Vec<String>,
    /// Members with no quota whose period contains the day: they add nothing to the roll-up.
    without_quota: Vec<String>,
    /// One entry per currency a member has a quota in; currencies are never added together.
    currencies: Vec<TeamCurrency>,
    /// Members' won revenue in currencies they have no quota in, listed and never converted.
    other_currencies: BTreeMap<String, i64>,
}

/// The answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TeamAttainmentReport")]
pub(crate) struct TeamReport {
    as_of: chrono::DateTime<chrono::Utc>,
    at: NaiveDate,
    rows: Vec<TeamStanding>,
    note: &'static str,
}

/// `GET /api/quotas/team-attainment?at=` — each team's members' standings added up per currency.
/// An audited read.
#[debug_handler]
async fn team_attainment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<TeamQuery>,
) -> Result<Response> {
    let at = match query.at {
        Some(at) => at,
        None => super::organization::caller_today(&ctx.db, &headers).await?,
    };
    let teams = ctx
        .db
        .query_all_raw(stmt(
            &format!("SELECT {TEAM_COLUMNS} FROM teams ORDER BY lower(name) LIMIT $1"),
            [MAX_LISTED.into()],
        ))
        .await?;
    let standing = standings(&ctx.db, at, None).await?;
    let mut rows = Vec::with_capacity(teams.len());
    for team in teams.iter().map(team_view) {
        let team = team?;
        let mine: Vec<_> = standing
            .iter()
            .filter(|s| team.members.contains(&s.owner_ref))
            .collect();
        let mut per: BTreeMap<String, (i64, i64, i64, usize)> = BTreeMap::new();
        let mut other: BTreeMap<String, i64> = BTreeMap::new();
        for s in &mine {
            let slot = per.entry(s.currency.clone()).or_default();
            slot.0 = slot.0.saturating_add(s.quota_minor);
            slot.1 = slot.1.saturating_add(s.revenue_minor);
            slot.2 += s.won_deals;
            slot.3 += 1;
        }
        // A worker's revenue in a currency the team has a quota in is already counted there.
        for s in &mine {
            for (code, minor) in &s.other_currencies {
                if !per.contains_key(code) {
                    let slot = other.entry(code.clone()).or_default();
                    *slot = slot.saturating_add(*minor);
                }
            }
        }
        rows.push(TeamStanding {
            team_pid: team.pid,
            name: team.name,
            without_quota: team
                .members
                .iter()
                .filter(|m| !mine.iter().any(|s| &s.owner_ref == *m))
                .cloned()
                .collect(),
            members: team.members,
            currencies: per
                .into_iter()
                .map(
                    |(currency, (quota_minor, revenue_minor, won_deals, workers))| TeamCurrency {
                        currency,
                        quota_minor,
                        revenue_minor,
                        won_deals,
                        attainment: analytics::ratio(revenue_minor, quota_minor),
                        remaining_minor: quota::remaining(quota_minor, revenue_minor),
                        workers,
                    },
                )
                .collect(),
            other_currencies: other,
        });
    }
    Audit::record_read(
        &ctx.db,
        "team",
        Uuid::nil(),
        "attainment_read",
        caller.actor(),
        Some(serde_json::json!({ "at": at, "teams": rows.len() })),
    )
    .await?;
    format::json(TeamReport {
        as_of: chrono::Utc::now(),
        at,
        rows,
        note: "a team's revenue is its workers' credited won revenue (a split deal counts each \
               worker's share) against their quotas in the same currency; currencies are listed, \
               never added or converted",
    })
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/quotas/teams", get(list_teams))
        .add("/quotas/teams", put(set_team))
        .add("/quotas/teams/{pid}", delete(remove_team))
        .add("/quotas/team-attainment", get(team_attainment))
}
