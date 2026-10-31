//! Relationship insights: the stakeholder grid, the partnership register and membership renewals.

use super::{
    AppContext, DaysQuery, HeaderMap, Response, Result, State, Uuid, accounts, conditional,
    contacts, days_or, debug_handler, engagement_rules, memberships, partnerships,
};

// Each view carries `as_of: Option<..>`, left `None` here: [`conditional`] stamps it after the
// `ETag` is taken, and the schema says it is always there.

/// A contact with a declared stakeholder role.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "StakeholderContact")]
pub(crate) struct StakeholderContact {
    pid: Uuid,
    display_name: String,
    marketing_consent: String,
    influence: Option<i32>,
    interest: Option<i32>,
    days_since_touch: i64,
}

/// An account with a declared role.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "StakeholderAccountRole")]
pub(crate) struct AccountRole {
    pid: Uuid,
    display_name: String,
    role: String,
}

/// `GET /api/insights/stakeholders`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct StakeholderRegister {
    note: &'static str,
    by_role: std::collections::BTreeMap<&'static str, Vec<StakeholderContact>>,
    grid: std::collections::BTreeMap<String, usize>,
    stakeholders_without_grid_scores: usize,
    undeclared_contacts: usize,
    account_roles: Vec<AccountRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// One partnership in the register.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "PartnershipRow")]
pub(crate) struct PartnershipRow {
    pid: Uuid,
    account_pid: Uuid,
    account: Option<String>,
    kind: String,
    stage: String,
    summary: String,
    started_on: Option<chrono::NaiveDate>,
}

/// `GET /api/insights/partnerships`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct PartnershipsRegister {
    by_kind: std::collections::BTreeMap<String, usize>,
    by_stage: std::collections::BTreeMap<String, usize>,
    register: Vec<PartnershipRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// One membership row.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct MembershipRow {
    pid: Uuid,
    account_pid: Uuid,
    account: Option<String>,
    status: String,
    joined_on: chrono::NaiveDate,
    renewal_on: Option<chrono::NaiveDate>,
}

/// `GET /api/insights/memberships`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct MembershipsView {
    window_days: i64,
    memberships: usize,
    renewals_due: Vec<MembershipRow>,
    lapsed: Vec<MembershipRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

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
    let last_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        super::rollup::touches(&ctx.db)
            .await?
            .into_iter()
            .filter(|t| t.subject_kind == "contact")
            .map(|t| (t.subject_pid, t.last_at))
            .collect();
    let mut by_role: std::collections::BTreeMap<&str, Vec<StakeholderContact>> =
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
                    bucket.push(StakeholderContact {
                        pid: contact.pid,
                        display_name: contact.display_name.clone(),
                        marketing_consent: contact.marketing_consent.clone(),
                        influence: contact.influence,
                        interest: contact.interest,
                        days_since_touch: days,
                    });
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
    let account_roles: Vec<AccountRole> = account_rows
        .iter()
        .filter_map(|account| {
            account.stakeholder_role.as_deref().map(|role| AccountRole {
                pid: account.pid,
                display_name: account.display_name.clone(),
                role: role.to_string(),
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(StakeholderRegister {
            note: "roles and grid scores are declared, never inferred; the grid \
                   covers stakeholders with both scores (key p<influence>i<interest>)",
            by_role,
            grid,
            stakeholders_without_grid_scores: ungridded,
            undeclared_contacts,
            account_roles,
            as_of: None,
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
    let register: Vec<PartnershipRow> = rows
        .iter()
        .map(|row| PartnershipRow {
            pid: row.pid,
            account_pid: row.account_pid,
            account: names.get(&row.account_pid).map(|name| (*name).to_string()),
            kind: row.kind.clone(),
            stage: row.stage.clone(),
            summary: row.summary.clone(),
            started_on: row.started_on,
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(PartnershipsRegister {
            by_kind: by_kind
                .into_iter()
                .map(|(k, n)| (k.to_string(), n))
                .collect(),
            by_stage: by_stage
                .into_iter()
                .map(|(k, n)| (k.to_string(), n))
                .collect(),
            register,
            as_of: None,
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
    // The caller's day (CRM-T179), as attainment's.
    let today = crate::controllers::organization::caller_today(&ctx.db, &headers).await?;
    let horizon = today + chrono::Days::new(u64::try_from(window).unwrap_or(90));
    let rows: Vec<memberships::Model> = live!(memberships, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let names: std::collections::BTreeMap<Uuid, &str> = account_rows
        .iter()
        .map(|a| (a.pid, a.display_name.as_str()))
        .collect();
    let view = |m: &memberships::Model| MembershipRow {
        pid: m.pid,
        account_pid: m.account_pid,
        account: names.get(&m.account_pid).map(|name| (*name).to_string()),
        status: m.status.clone(),
        joined_on: m.joined_on,
        renewal_on: m.renewal_on,
    };
    let mut renewals_due: Vec<&memberships::Model> = rows
        .iter()
        .filter(|m| m.status == "active" && m.renewal_on.is_some_and(|d| d <= horizon))
        .collect();
    renewals_due.sort_by_key(|m| m.renewal_on);
    let lapsed: Vec<MembershipRow> = rows
        .iter()
        .filter(|m| m.status == "lapsed")
        .map(view)
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(MembershipsView {
            window_days: window,
            memberships: rows.len(),
            renewals_due: renewals_due.iter().map(|m| view(m)).collect(),
            lapsed,
            as_of: None,
        }),
    ))
}
