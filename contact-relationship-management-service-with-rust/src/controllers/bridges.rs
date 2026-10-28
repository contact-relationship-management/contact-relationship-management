//! Bridges to the family's other services (CRM-R72, CRM-R73): a won deal opens a delivery idea in
//! the project portfolio service (PPM), and the worker service (WPM) gives the teams.
//!
//! Both sit under the admin-only `/api/bridges` prefix, since they send deal data out and read
//! people in. Both are **stub-first** like the other upstream clients: unless
//! `CRM_UPSTREAM_MODE=http`, nothing leaves the process and the answer says what would have.

use std::collections::BTreeMap;
use std::str::FromStr;

use entity_ref::EntityRef;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait, Value};
use serde::Serialize;
use uuid::Uuid;

use super::PidPath;
use super::teams::{TeamView, put_team};
use super::views::owner_of;
use crate::auth::MaybeAuthUser;
use crate::clients::{self, Mode};
use crate::models::audit_logs::Model as Audit;
use crate::rules::quota;

/// Most workers on a team (the same as setting one by hand).
const MAX_MEMBERS: usize = 50;
/// Longest idea title the service takes is generous; ours is held well inside it.
const MAX_TITLE: usize = 200;

fn stmt(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

fn unavailable(reason: &str) -> Error {
    tracing::warn!(%reason, "bridge: upstream failed");
    Error::CustomError(
        axum::http::StatusCode::BAD_GATEWAY,
        loco_rs::controller::ErrorDetail::new("bad_gateway", reason),
    )
}

/// What is sent to PPM for a deal.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "DeliveryIdea")]
pub(crate) struct IdeaView {
    title: String,
    pitch: String,
    tags: Vec<String>,
}

/// `POST /api/bridges/ppm/deals/{pid}` answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "DealDelivery")]
pub(crate) struct DeliveryView {
    #[schemars(with = "uuid::Uuid")]
    deal_pid: String,
    /// `stub` (nothing was sent) or `http`.
    mode: &'static str,
    /// Whether the idea was sent in this call.
    sent: bool,
    /// The idea in PPM; `null` in `stub` mode.
    ppm_idea_pid: Option<String>,
    /// Whether the deal had been opened before, so nothing was sent again.
    already_opened: bool,
    idea: IdeaView,
}

/// `POST /api/bridges/ppm/deals/{pid}` — open a delivery idea in PPM for a won deal; audited. Once
/// per deal: asking again answers the idea already opened and sends nothing. `404` for an unknown
/// deal, `422` when it is not won, `502` when PPM cannot be reached or answers badly.
#[debug_handler]
async fn open_delivery(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    PidPath(pid): PidPath,
) -> Result<Response> {
    let deal = ctx
        .db
        .query_one_raw(stmt(
            "SELECT name, won, amount_minor, trim(currency) AS currency FROM deals \
             WHERE pid = $1 AND deleted_at IS NULL",
            [pid.into()],
        ))
        .await?
        .ok_or(Error::NotFound)?;
    if !deal.try_get::<bool>("", "won")? {
        return Err(super::unprocessable("only a won deal opens a delivery"));
    }
    let name: String = deal.try_get("", "name")?;
    let idea = IdeaView {
        title: format!("Deliver: {name}").chars().take(MAX_TITLE).collect(),
        pitch: format!(
            "Won deal {pid} in the CRM: {} minor units of {}.",
            deal.try_get::<i64>("", "amount_minor")?,
            deal.try_get::<String>("", "currency")?
        ),
        tags: vec!["crm".to_string(), "won-deal".to_string()],
    };
    let view = |mode, sent, idea_pid: Option<String>, already_opened| DeliveryView {
        deal_pid: pid.to_string(),
        mode,
        sent,
        ppm_idea_pid: idea_pid,
        already_opened,
        idea: IdeaView {
            title: idea.title.clone(),
            pitch: idea.pitch.clone(),
            tags: idea.tags.clone(),
        },
    };
    if let Some(row) = ctx
        .db
        .query_one_raw(stmt(
            "SELECT ppm_idea_pid FROM deal_deliveries WHERE deal_pid = $1",
            [pid.into()],
        ))
        .await?
    {
        let id: Uuid = row.try_get("", "ppm_idea_pid")?;
        return format::json(view("http", false, Some(id.to_string()), true));
    }
    if clients::mode() != Mode::Http {
        return format::json(view("stub", false, None, false));
    }
    let who = owner_of(&caller)?;
    let tags: Vec<&str> = idea.tags.iter().map(String::as_str).collect();
    let opened = clients::open_delivery_idea(&idea.title, &idea.pitch, &tags)
        .await
        .map_err(|reason| unavailable(&reason))?;
    let idea_pid = Uuid::parse_str(&opened).map_err(|e| Error::string(&e.to_string()))?;
    let txn = ctx.db.begin().await?;
    txn.execute_raw(stmt(
        "INSERT INTO deal_deliveries (deal_pid, ppm_idea_pid, opened_by) VALUES ($1, $2, $3) \
         ON CONFLICT (deal_pid) DO NOTHING",
        [pid.into(), idea_pid.into(), who.into()],
    ))
    .await?;
    Audit::record(
        &txn,
        "deal_delivery",
        pid,
        "opened",
        caller.actor(),
        Some(serde_json::json!({ "ppm_idea_pid": idea_pid })),
    )
    .await?;
    txn.commit().await?;
    format::json(view("http", true, Some(idea_pid.to_string()), false))
}

/// A department the import did not turn into a team, and why.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TeamImportSkipped")]
pub(crate) struct Skipped {
    department: String,
    reason: String,
}

/// `POST /api/bridges/wpm/teams-import` answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "TeamImport")]
pub(crate) struct ImportView {
    /// `stub` (nothing was read) or `http`.
    mode: &'static str,
    /// The teams set, one per department, by name.
    teams: Vec<TeamView>,
    /// Departments left alone, with the reason.
    skipped: Vec<Skipped>,
    /// Active workers with no department, who join no team.
    without_department: usize,
}

/// `POST /api/bridges/wpm/teams-import` — read the active workers of the worker service and set a
/// team per department, each team's workers being that department's (CRM-R73). All the teams are
/// set together or none is; each is audited with `via: wpm`. Teams the import does not name are
/// left as they are, and nothing is written to the worker service. `502` when it cannot be read.
#[debug_handler]
async fn import_teams(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    if clients::mode() != Mode::Http {
        return format::json(ImportView {
            mode: "stub",
            teams: Vec::new(),
            skipped: Vec::new(),
            without_department: 0,
        });
    }
    let workers = clients::active_workers()
        .await
        .map_err(|reason| unavailable(&reason))?;
    let mut by_department: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut without_department = 0;
    let mut skipped = Vec::new();
    for worker in workers {
        let urn = format!("worker:{}", worker.pid);
        if EntityRef::from_str(&urn).is_err() {
            skipped.push(Skipped {
                department: worker.department.clone(),
                reason: "a worker with an id that is not a reference".to_string(),
            });
        } else if worker.department.is_empty() {
            without_department += 1;
        } else {
            by_department
                .entry(worker.department)
                .or_default()
                .push(urn);
        }
    }
    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    let mut teams = Vec::new();
    // Departments that differ only in case are one team, as team names are.
    let mut seen = BTreeMap::new();
    for (department, mut members) in by_department {
        let name = match quota::name(&department) {
            Ok(name) => name.replace("a plan", "a team"),
            Err(why) => {
                skipped.push(Skipped {
                    department,
                    reason: why.replace("a plan", "a team"),
                });
                continue;
            }
        };
        if let Some(first) = seen.insert(name.to_lowercase(), name.clone()) {
            skipped.push(Skipped {
                department: name,
                reason: format!("the same name as the department {first} in another case"),
            });
            continue;
        }
        members.sort();
        members.dedup();
        if members.len() > MAX_MEMBERS {
            skipped.push(Skipped {
                department: name,
                reason: format!("more than {MAX_MEMBERS} workers"),
            });
            continue;
        }
        teams.push(put_team(&txn, &who, caller.actor(), &name, &members, Some("wpm")).await?);
    }
    txn.commit().await?;
    format::json(ImportView {
        mode: "http",
        teams,
        skipped,
        without_department,
    })
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/bridges/ppm/deals/{pid}", post(open_delivery))
        .add("/bridges/wpm/teams-import", post(import_teams))
}
