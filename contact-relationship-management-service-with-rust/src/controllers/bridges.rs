//! Bridges to the family's other services (CRM-R72, CRM-R73): a won deal opens a delivery idea in
//! the project portfolio service (PPM), and the worker service (WPM) gives the teams.
//!
//! Both sit under the admin-only `/api/bridges` prefix, since they send deal data out and read
//! people in. Both are **stub-first** like the other upstream clients: unless
//! `CRM_UPSTREAM_MODE=http`, nothing leaves the process and the answer says what would have.
//!
//! The third bridge runs the other way (CRM-R92): WPM posts its hire and leave events to
//! `/api/bridges/wpm/events`, which is public to the session check and authenticated by the
//! event's signature instead. The fourth asks WPM which contacts are its workers today, to find
//! those the events never told of (CRM-R94).

use std::collections::BTreeMap;

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
    /// The teams set, one per department, by name; each member is a worker's `person:` reference.
    teams: Vec<TeamView>,
    /// Departments left alone, with the reason.
    skipped: Vec<Skipped>,
    /// Active workers with a person reference and no department, who join no team.
    without_department: usize,
    /// Active workers whose `person_ref` is missing or not a `person:` URN, who join no team.
    without_person_ref: usize,
}

/// `POST /api/bridges/wpm/teams-import` — read the active workers of the worker service and set a
/// team per department, each team's members being that department's workers by `person:` reference
/// (CRM-R73, CRM-D17). A worker without a valid `person_ref` joins no team and is counted. All the
/// teams are set together or none is; each is audited with `via: wpm`. Teams the import does not
/// name are left as they are, and nothing is written to the worker service. `502` when it cannot be
/// read.
#[debug_handler]
async fn import_teams(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    if clients::mode() != Mode::Http {
        return format::json(ImportView {
            mode: "stub",
            teams: Vec::new(),
            skipped: Vec::new(),
            without_department: 0,
            without_person_ref: 0,
        });
    }
    let workers = clients::active_workers()
        .await
        .map_err(|reason| unavailable(&reason))?;
    let grouped = crate::rules::teams_import::group(
        workers
            .iter()
            .map(|w| (w.department.as_str(), w.person_ref.as_deref())),
    );
    let mut skipped = Vec::new();
    let who = owner_of(&caller)?;
    let txn = ctx.db.begin().await?;
    let mut teams = Vec::new();
    // Departments that differ only in case are one team, as team names are.
    let mut seen = BTreeMap::new();
    for (department, members) in grouped.by_department {
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
        without_department: grouped.without_department,
        without_person_ref: grouped.without_person_ref,
    })
}

/// `POST /api/bridges/wpm/events` answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "WpmEventResult")]
pub(crate) struct WpmEventView {
    /// `paused` (a hire marked contacts as employees), `already_paused`, `resumed` (a leave lifted
    /// the pause), `not_paused`, `no_contact` (no live contact carries the person), `duplicate`
    /// (this event id was received before; nothing changed), `stale` (a hire or leave older than
    /// the last one applied for the same person and organization; nothing changed, CRM-R93), or
    /// `ignored`.
    result: &'static str,
    /// Why an event was ignored: `no_data` (a WPM from before WPM-R207), `not_used` or
    /// `unknown_kind`; `null` otherwise.
    reason: Option<&'static str>,
    /// How many contacts changed.
    contacts: usize,
}

/// `POST /api/bridges/wpm/events` body, as WPM sends it (WPM-R151, WPM-R207).
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "WpmEvent")]
#[allow(
    dead_code,
    reason = "documents the body; it is read by rules::wpm_events"
)]
pub(crate) struct WpmEventPayload {
    /// The event id; the same on every retry.
    id: String,
    /// `worker_hired`, `worker_terminated` and `worker_retired` are acted on; any other is
    /// acknowledged and ignored.
    kind: String,
    /// RFC 3339. Orders a hire or leave against the last one applied (CRM-R93); required for them.
    occurred_at: String,
    /// `worker:<pid>`; not read.
    subject: String,
    /// For the three kinds above: `person_ref`, `organization_ref`, `effective_on` (and `from` and
    /// `to` for the endings, not read). Absent from a WPM from before WPM-R207.
    #[schemars(with = "Option<serde_json::Map<String, serde_json::Value>>")]
    data: Option<serde_json::Value>,
}

/// `POST /api/bridges/wpm/events` — a hire or leave event from the worker service (CRM-R92).
/// `404` while `CRM_WPM_WEBHOOK_SECRET` is unset; `401` for a missing, wrong or stale
/// `x-wpm-signature`; `400` for a body that is not an event; `413` past 16 KiB; `422` for a hire or
/// leave whose `data` or `occurred_at` is malformed. A hire or leave older than the last one applied
/// for its person and organization is `stale` and changes nothing (CRM-R93). Every other event answers `200` with what it did, so WPM does
/// not retry one the CRM was right to ignore; a repeated `id` changes nothing.
#[debug_handler]
async fn wpm_event(
    State(ctx): State<AppContext>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response> {
    use crate::models::employments;
    use crate::rules::wpm_events::{self, Decision};
    use axum::http::StatusCode;
    let Some(secret) = crate::settings::current().wpm_webhook_secret.clone() else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let signature = headers
        .get("x-wpm-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !wpm_events::verify(
        secret.as_bytes(),
        signature,
        &body,
        chrono::Utc::now().timestamp(),
    ) {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }
    let Ok(event) = wpm_events::parse(&body) else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };
    let decision = wpm_events::decide(&event).map_err(|why| super::unprocessable(&why))?;
    let txn = ctx.db.begin().await?;
    // The id is claimed first, in the same transaction as the change: a retry that arrives while
    // this one runs waits on the row, then finds it.
    if !employments::claim_event(&txn, &event.id, &event.kind).await? {
        txn.commit().await?;
        return format::json(WpmEventView {
            result: "duplicate",
            reason: None,
            contacts: 0,
        });
    }
    let (result, reason, changed) = match &decision {
        Decision::Ignored(why) => ("ignored", Some(*why), 0),
        Decision::Hired(h) | Decision::Left(h) => {
            let hired = matches!(decision, Decision::Hired(_));
            let (result, changed) = apply_handoff(&txn, hired, h, &event.id).await?;
            (result, None, changed)
        }
    };
    employments::settle_event(&txn, &event.id, result).await?;
    txn.commit().await?;
    format::json(WpmEventView {
        result,
        reason,
        contacts: changed,
    })
}

/// Apply a hire (`hired`) or leave to the live contacts carrying its person (CRM-R92), unless an
/// event at least as new has been applied for that person and organization (CRM-R93: `stale`).
/// The result and how many contacts changed.
async fn apply_handoff<C: ConnectionTrait>(
    txn: &C,
    hired: bool,
    h: &crate::rules::wpm_events::Handoff,
    event_id: &str,
) -> Result<(&'static str, usize)> {
    use crate::models::employments;
    let pids = employments::contacts_of(txn, &h.person_ref).await?;
    if pids.is_empty() {
        // Nothing is kept about a person who is not a contact, not even the order.
        return Ok(("no_contact", 0));
    }
    employments::lock_order(txn, &h.person_ref, &h.organization_ref).await?;
    let last = employments::last_applied(txn, &h.person_ref, &h.organization_ref).await?;
    if crate::rules::wpm_events::is_stale(last, h.stamp()) {
        return Ok(("stale", 0));
    }
    employments::note_applied(
        txn,
        &h.person_ref,
        &h.organization_ref,
        if hired { "hired" } else { "left" },
        h.stamp(),
        event_id,
    )
    .await?;
    let mut changed = 0;
    for pid in &pids {
        let done = if hired {
            employments::record(
                txn,
                *pid,
                &h.person_ref,
                &h.organization_ref,
                h.effective_on,
                event_id,
            )
            .await?
        } else {
            employments::end(txn, *pid, &h.organization_ref).await?
        };
        if done {
            changed += 1;
            Audit::record(
                txn,
                "contact",
                *pid,
                if hired {
                    "employment_recorded"
                } else {
                    "employment_ended"
                },
                None,
                Some(serde_json::json!({
                    "via": "wpm",
                    "organization_ref": h.organization_ref,
                    "effective_on": h.effective_on,
                    "occurred_at": h.occurred_at,
                    "event_id": event_id,
                })),
            )
            .await?;
        }
    }
    Ok((
        match (hired, changed > 0) {
            (true, true) => "paused",
            (true, false) => "already_paused",
            (false, true) => "resumed",
            (false, false) => "not_paused",
        },
        changed,
    ))
}

/// `POST /api/bridges/wpm/employment-check` answer.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[schemars(rename = "EmploymentCheck")]
pub(crate) struct EmploymentCheckView {
    /// `stub` (nothing was sent) or `http`.
    mode: &'static str,
    /// The organization asked about (`CRM_WPM_ORGANIZATION_REF`); `null` when it is unset.
    organization_ref: Option<String>,
    /// The distinct person references of live contacts asked about (in `stub` mode, that would be).
    asked: usize,
    /// How many calls of at most 500 references that took (or would).
    batches: usize,
    /// How many of them the worker service says are workers today.
    workers: usize,
    /// Contacts newly marked employees, which pauses their marketing.
    recorded: usize,
    /// Contacts who were already marked employees of the organization.
    already_paused: usize,
    /// Workers left alone because the last event applied for them was a leave.
    left_by_event: usize,
}

/// `POST /api/bridges/wpm/employment-check` — ask the worker service which live contacts are its
/// workers today, and mark them employees of the organization as a hire event would (CRM-R94,
/// WPM-R208). Sends the contacts' `person:` references, never a name, at most 500 a call, for
/// `CRM_WPM_ORGANIZATION_REF`, under `CRM_WORKER_SERVICE_TOKEN` (CRM-D16). A "no" changes nothing;
/// in particular it never ends an employment. All the answers are read before anything is written;
/// one bad answer or failed call is `502` and records nothing. Admin only; stub-first.
#[debug_handler]
async fn employment_check(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    use crate::models::employments;
    use crate::rules::employment_check::{self as rules, Action, LastEvent};
    let organization_ref = crate::settings::current().wpm_organization_ref.clone();
    let refs = employments::live_person_refs(&ctx.db).await?;
    let batches = rules::batches(refs, super::privacy::TOMBSTONE_PERSON);
    let asked = batches.iter().map(Vec::len).sum();
    let mut view = EmploymentCheckView {
        mode: "stub",
        organization_ref: organization_ref.clone(),
        asked,
        batches: batches.len(),
        workers: 0,
        recorded: 0,
        already_paused: 0,
        left_by_event: 0,
    };
    if clients::mode() != Mode::Http {
        return format::json(view);
    }
    view.mode = "http";
    let organization_ref = organization_ref.ok_or_else(|| {
        unavailable("CRM_WPM_ORGANIZATION_REF is not set to an organization: reference")
    })?;
    let mut workers = std::collections::BTreeSet::new();
    for batch in &batches {
        let answer = clients::overlap_check(&organization_ref, batch)
            .await
            .map_err(|reason| unavailable(&reason))?;
        workers.extend(
            rules::workers_in(batch, &organization_ref, &answer)
                .map_err(|reason| unavailable(&reason))?,
        );
    }
    view.workers = workers.len();
    let check_id = Uuid::new_v4();
    let source = format!("overlap:{check_id}");
    // The day CRM records the employment from: CRM's own, in the organization's zone (CRM-T179).
    // WPM's answer carries no date (WPM-R208), so this is not a value of the bridge contract.
    let today = super::organization::organization_today(&ctx.db).await?;
    let txn = ctx.db.begin().await?;
    for person_ref in &workers {
        employments::lock_order(&txn, person_ref, &organization_ref).await?;
        let last = match employments::last_kind(&txn, person_ref, &organization_ref)
            .await?
            .as_deref()
        {
            Some("hired") => LastEvent::Hired,
            Some(_) => LastEvent::Left,
            None => LastEvent::None,
        };
        if rules::decide(true, last) != Action::Record {
            view.left_by_event += 1;
            continue;
        }
        for pid in employments::contacts_of(&txn, person_ref).await? {
            if employments::record(&txn, pid, person_ref, &organization_ref, today, &source).await?
            {
                view.recorded += 1;
                Audit::record(
                    &txn,
                    "contact",
                    pid,
                    "employment_recorded",
                    caller.actor(),
                    Some(serde_json::json!({
                        "via": "wpm",
                        "source": "employment_check",
                        "organization_ref": organization_ref,
                        "since": today,
                        "check_id": check_id,
                    })),
                )
                .await?;
            } else {
                view.already_paused += 1;
            }
        }
    }
    // The check itself, by count: no reference is in it.
    Audit::record(
        &txn,
        "bridge",
        check_id,
        "employment_checked",
        caller.actor(),
        Some(serde_json::json!({
            "via": "wpm",
            "organization_ref": organization_ref,
            "asked": view.asked,
            "batches": view.batches,
            "workers": view.workers,
            "recorded": view.recorded,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(view)
}

/// The routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/bridges/ppm/deals/{pid}", post(open_delivery))
        .add("/bridges/wpm/teams-import", post(import_teams))
        .add("/bridges/wpm/employment-check", post(employment_check))
        .add(
            "/bridges/wpm/events",
            post(wpm_event).layer(axum::extract::DefaultBodyLimit::max(
                crate::rules::wpm_events::MAX_BODY,
            )),
        )
}
