//! DPO insights: consent coverage and withdrawals, duplicate-contact hygiene, consent by account.

use super::{
    AppContext, ColumnTrait, DaysQuery, EntityTrait, HeaderMap, QueryFilter, Response, Result,
    State, Uuid, accounts, conditional, consent_events, contacts, days_or, debug_handler,
};

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
    let event_rows = consent_events::Entity::find().all(&ctx.db).await?;
    let withdrawals_in_window = event_rows
        .iter()
        .filter(|event| event.action == "withdrawn" && event.occurred_at.to_utc() >= since)
        .count();
    let mut per_source: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for event in &event_rows {
        *per_source.entry(event.source.as_str()).or_default() += 1;
    }
    // Duplicate hygiene: live contacts sharing a person_ref.
    let mut by_person: std::collections::BTreeMap<&str, Vec<&contacts::Model>> =
        std::collections::BTreeMap::new();
    for contact in &contact_rows {
        by_person
            .entry(contact.person_ref.as_str())
            .or_default()
            .push(contact);
    }
    let duplicates: Vec<serde_json::Value> = by_person
        .iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(person_ref, group)| {
            serde_json::json!({
                "person_ref": person_ref,
                "contacts": group
                    .iter()
                    .map(|c| serde_json::json!({ "pid": c.pid, "display_name": c.display_name }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "coverage counts each contact's current marketing_consent verbatim; \
                     duplicates are CRM-local rows sharing one person_ref — identity \
                     dedup stays upstream in the person service",
            "contacts": contact_rows.len(),
            "consent_coverage": coverage,
            "window_days": window_days,
            "withdrawals_in_window": withdrawals_in_window,
            "consent_events_by_source": per_source,
            "duplicate_person_refs": duplicates,
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
    let event_rows = consent_events::Entity::find().all(&ctx.db).await?;
    let account_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = contact_rows
        .iter()
        .map(|c| (c.pid, c.account_pid))
        .collect();
    let mut withdrawals: std::collections::BTreeMap<Uuid, usize> =
        std::collections::BTreeMap::new();
    for event in &event_rows {
        if event.action == "withdrawn"
            && event.occurred_at.to_utc() >= since
            && let Some(Some(account_pid)) = account_of.get(&event.contact_pid)
        {
            *withdrawals.entry(*account_pid).or_default() += 1;
        }
    }
    let rows: Vec<serde_json::Value> = account_rows
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
            serde_json::json!({
                "pid": account.pid,
                "display_name": account.display_name,
                "consent_coverage": coverage,
                "withdrawals_in_window": withdrawals.get(&account.pid).copied().unwrap_or(0),
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "window_days": window,
            "note": "coverage counts each contact's current marketing_consent verbatim; \
                     contacts without an account are in the estate-wide /insights/dpo view",
            "accounts": rows,
        }),
    ))
}
