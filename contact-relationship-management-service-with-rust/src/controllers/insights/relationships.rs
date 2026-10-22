//! Relationship insights: the stakeholder grid, the partnership register and membership renewals.

use super::{
    AppContext, DaysQuery, HeaderMap, Response, Result, State, Uuid, accounts, activities,
    conditional, contacts, days_or, debug_handler, engagement_rules, memberships, partnerships,
};

/// `GET /api/insights/stakeholders` — the declared-stakeholder
/// register: per-role lists with cadence and consent per row, and the
/// power–interest grid over contacts with **both** scores declared
/// (undeclared counted, never guessed into a cell).
#[debug_handler]
pub(super) async fn stakeholders(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let mut last_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    for activity in activity_rows.iter().filter(|a| a.subject_kind == "contact") {
        let at = activity.occurred_at.to_utc();
        let entry = last_touch.entry(activity.subject_pid).or_insert(at);
        if at > *entry {
            *entry = at;
        }
    }
    let mut by_role: std::collections::BTreeMap<&str, Vec<serde_json::Value>> =
        engagement_rules::STAKEHOLDER_ROLES
            .iter()
            .map(|r| (*r, Vec::new()))
            .collect();
    let mut grid: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut undeclared_contacts = 0usize;
    let mut ungridded = 0usize;
    for contact in &contact_rows {
        match contact.stakeholder_role.as_deref() {
            Some(role) => {
                let days = last_touch.get(&contact.pid).map_or_else(
                    || (now - contact.created_at.to_utc()).num_days(),
                    |at| (now - *at).num_days(),
                );
                if let Some(bucket) = by_role.get_mut(role) {
                    bucket.push(serde_json::json!({
                        "pid": contact.pid,
                        "display_name": contact.display_name,
                        "marketing_consent": contact.marketing_consent,
                        "influence": contact.influence,
                        "interest": contact.interest,
                        "days_since_touch": days,
                    }));
                }
                match (contact.influence, contact.interest) {
                    (Some(influence), Some(interest)) => {
                        *grid.entry(format!("p{influence}i{interest}")).or_default() += 1;
                    }
                    _ => ungridded += 1,
                }
            }
            None => undeclared_contacts += 1,
        }
    }
    let account_roles: Vec<serde_json::Value> = account_rows
        .iter()
        .filter_map(|account| {
            account.stakeholder_role.as_deref().map(|role| {
                serde_json::json!({
                    "pid": account.pid, "display_name": account.display_name, "role": role,
                })
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "roles and grid scores are declared, never inferred; the grid \
                     covers stakeholders with both scores (key p<influence>i<interest>)",
            "by_role": by_role,
            "grid": grid,
            "stakeholders_without_grid_scores": ungridded,
            "undeclared_contacts": undeclared_contacts,
            "account_roles": account_roles,
        }),
    ))
}

/// `GET /api/insights/partnerships` — the innovation-partnership
/// register: per-kind and per-stage counts plus the live records with
/// their accounts.
#[debug_handler]
pub(super) async fn partnerships_register(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let rows: Vec<partnerships::Model> = live!(partnerships, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let names: std::collections::BTreeMap<Uuid, &str> = account_rows
        .iter()
        .map(|a| (a.pid, a.display_name.as_str()))
        .collect();
    let mut by_kind: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut by_stage: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for row in &rows {
        *by_kind.entry(row.kind.as_str()).or_default() += 1;
        *by_stage.entry(row.stage.as_str()).or_default() += 1;
    }
    let register: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "pid": row.pid,
                "account_pid": row.account_pid,
                "account": names.get(&row.account_pid),
                "kind": row.kind,
                "stage": row.stage,
                "summary": row.summary,
                "started_on": row.started_on,
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "by_kind": by_kind,
            "by_stage": by_stage,
            "register": register,
        }),
    ))
}

/// `GET /api/insights/memberships?days=` — membership renewals due
/// within the window plus the lapsed list.
#[debug_handler]
pub(super) async fn memberships_view(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window = days_or(query.days, 90);
    let today = chrono::Utc::now().date_naive();
    let horizon = today + chrono::Days::new(u64::try_from(window).unwrap_or(90));
    let rows: Vec<memberships::Model> = live!(memberships, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let names: std::collections::BTreeMap<Uuid, &str> = account_rows
        .iter()
        .map(|a| (a.pid, a.display_name.as_str()))
        .collect();
    let view = |m: &memberships::Model| {
        serde_json::json!({
            "pid": m.pid,
            "account_pid": m.account_pid,
            "account": names.get(&m.account_pid),
            "status": m.status,
            "joined_on": m.joined_on,
            "renewal_on": m.renewal_on,
        })
    };
    let mut renewals_due: Vec<&memberships::Model> = rows
        .iter()
        .filter(|m| m.status == "active" && m.renewal_on.is_some_and(|d| d <= horizon))
        .collect();
    renewals_due.sort_by_key(|m| m.renewal_on);
    let lapsed: Vec<serde_json::Value> = rows
        .iter()
        .filter(|m| m.status == "lapsed")
        .map(view)
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "window_days": window,
            "memberships": rows.len(),
            "renewals_due": renewals_due.iter().map(|m| view(m)).collect::<Vec<_>>(),
            "lapsed": lapsed,
        }),
    ))
}
