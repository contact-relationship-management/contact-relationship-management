//! DPO insights: consent coverage and withdrawals, duplicate-contact hygiene, consent by account.

use super::{
    AppContext, DaysQuery, HeaderMap, Response, Result, State, accounts, conditional, contacts,
    days_or, debug_handler,
};

// Each view carries `as_of: Option<..>`, left `None` here: [`conditional`] stamps it after the
// `ETag` is taken, and the schema says it is always there.

/// A contact that shares a `person_ref` with another.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "DuplicatePersonContact")]
pub(crate) struct DuplicateContact {
    pid: uuid::Uuid,
    display_name: String,
}

/// Live contacts sharing one `person_ref`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "DuplicatePersonRef")]
pub(crate) struct DuplicatePersonRef {
    person_ref: String,
    contacts: Vec<DuplicateContact>,
}

/// `GET /api/insights/dpo`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct DpoView {
    note: &'static str,
    contacts: usize,
    consent_coverage: std::collections::BTreeMap<String, usize>,
    window_days: i64,
    withdrawals_in_window: usize,
    consent_events_by_source: std::collections::BTreeMap<String, usize>,
    duplicate_person_refs: Vec<DuplicatePersonRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// One account's consent picture.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[schemars(rename = "ConsentAccount")]
pub(crate) struct ConsentAccount {
    pid: uuid::Uuid,
    display_name: String,
    consent_coverage: std::collections::BTreeMap<String, usize>,
    withdrawals_in_window: usize,
}

/// `GET /api/insights/consent-by-account`.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct ConsentByAccount {
    window_days: i64,
    note: &'static str,
    accounts: Vec<ConsentAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "chrono::DateTime<chrono::Utc>")]
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// `GET /api/insights/dpo?days=` — the data-protection view: consent
/// coverage (current per-contact state, counted verbatim),
/// withdrawals in the window, per-source consent-event counts, and
/// duplicate-contact hygiene (live contacts sharing one `person_ref`
/// — CRM-local hygiene; identity dedup stays upstream).
#[debug_handler]
pub(super) async fn dpo(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window_days = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let since = now - chrono::Days::new(u64::try_from(window_days).unwrap_or(30));
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let mut coverage: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for contact in &contact_rows {
        *coverage
            .entry(contact.marketing_consent.as_str())
            .or_default() += 1;
    }
    let withdrawals_in_window = super::rollup::withdrawals_since(&ctx.db, since).await?;
    let per_source = super::rollup::consent_events_by_source(&ctx.db).await?;
    // Duplicate hygiene: live contacts sharing a person_ref.
    let mut by_person: std::collections::BTreeMap<&str, Vec<&contacts::Model>> =
        std::collections::BTreeMap::new();
    for contact in &contact_rows {
        by_person
            .entry(contact.person_ref.as_str())
            .or_default()
            .push(contact);
    }
    let duplicates: Vec<DuplicatePersonRef> = by_person
        .iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(person_ref, group)| DuplicatePersonRef {
            person_ref: (*person_ref).to_string(),
            contacts: group
                .iter()
                .map(|c| DuplicateContact {
                    pid: c.pid,
                    display_name: c.display_name.clone(),
                })
                .collect(),
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(DpoView {
            note: "coverage counts each contact's current marketing_consent verbatim; \
                   duplicates are CRM-local rows sharing one person_ref — identity \
                   dedup stays upstream in the person service",
            contacts: contact_rows.len(),
            consent_coverage: coverage
                .into_iter()
                .map(|(k, n)| (k.to_string(), n))
                .collect(),
            window_days,
            withdrawals_in_window,
            consent_events_by_source: per_source,
            duplicate_person_refs: duplicates,
            as_of: None,
        }),
    ))
}

/// `GET /api/insights/consent-by-account?days=` — the DPO consent
/// rollup per member account: each account's contacts' current
/// consent states (verbatim) and withdrawals in the window.
#[debug_handler]
pub(super) async fn consent_by_account(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let since = now - chrono::Days::new(u64::try_from(window).unwrap_or(30));
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let withdrawals = super::rollup::withdrawals_by_account(&ctx.db, since).await?;
    let rows: Vec<ConsentAccount> = account_rows
        .iter()
        .map(|account| {
            let mut coverage: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for contact in contact_rows
                .iter()
                .filter(|c| c.account_pid == Some(account.pid))
            {
                *coverage
                    .entry(contact.marketing_consent.as_str())
                    .or_default() += 1;
            }
            ConsentAccount {
                pid: account.pid,
                display_name: account.display_name.clone(),
                consent_coverage: coverage
                    .into_iter()
                    .map(|(k, n)| (k.to_string(), n))
                    .collect(),
                withdrawals_in_window: withdrawals.get(&account.pid).copied().unwrap_or(0),
            }
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!(ConsentByAccount {
            window_days: window,
            note: "coverage counts each contact's current marketing_consent verbatim; \
                   contacts without an account are in the estate-wide /insights/dpo view",
            accounts: rows,
            as_of: None,
        }),
    ))
}
