//! The insight-view round-trip: stale-deal aging, follow-ups,
//! pipeline hygiene, the executive pack, forecast trends, the SLA
//! register, and the DPO view — over a seeded estate.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_user, seed_contact, seed_pipeline};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded estate, seven views
async fn insight_views_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline_pid, stages) = seed_pipeline!(&request).await;
        let contact_pid = seed_contact!(&request, "Prime Contact").await;

        // A duplicate CRM row for the same person URN (hygiene case).
        let shared_person = a_person();
        for name in ["Row One", "Row Two"] {
            request
                .post("/api/contacts")
                .json(&json!({ "person_ref": shared_person, "display_name": name }))
                .await
                .assert_status_ok();
        }

        // Deals: one moved (fresh stage clock), one never moved with
        // no amount and no expected close (hygiene findings), one won.
        let moved: Value = request
            .post("/api/deals")
            .json(&json!({ "name": "Moved deal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 400_000_i64, "currency": "GBP" }))
            .await
            .json();
        let moved_pid = moved["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/deals/{moved_pid}/stage"))
            .json(&json!({ "stage_pid": stages[1] }))
            .await
            .assert_status_ok();
        request
            .post("/api/deals")
            .json(
                &json!({ "name": "Neglected deal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 0_i64, "currency": "GBP" }),
            )
            .await
            .assert_status_ok();
        let won: Value = request
            .post("/api/deals")
            .json(&json!({ "name": "Won deal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 250_000_i64, "currency": "GBP" }))
            .await
            .json();
        let won_pid = won["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/deals/{won_pid}/stage"))
            .json(&json!({ "stage_pid": stages[2] }))
            .await
            .assert_status_ok();

        // Activities: an overdue follow-up on the moved deal.
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "deal", "subject_pid": moved_pid,
                            "kind": "call", "summary": "Chase the proposal",
                            "actor_ref": a_user(), "due_on": "2026-07-01" }))
            .await
            .assert_status_ok();

        // Support: a policy then a ticket whose deadlines are derived.
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "high", "first_response_minutes": 1,
                            "resolution_minutes": 2 }))
            .await
            .assert_status_ok();
        request
            .post("/api/tickets")
            .json(&json!({ "title": "Login broken", "priority": "high",
                            "contact_pid": contact_pid,
                            "assignee_ref": a_user() }))
            .await
            .assert_status_ok();

        // A forecast snapshot for the trend series.
        request
            .post("/api/forecast/snapshot")
            .await
            .assert_status_ok();

        // One withdrawal for the DPO window.
        request
            .post(&format!("/api/contacts/{contact_pid}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "email link" }))
            .await
            .assert_status_ok();

        // ── Stale deals: both open deals reported; the never-moved one
        // ages from creation (0 days today), so nothing is stale yet.
        let stale: Value = request.get("/api/insights/stale-deals?days=1").await.json();
        assert_eq!(stale["open_deals"], 2);
        assert_eq!(stale["stale_deals"], 0);
        assert!(
            stale["derivation"]
                .as_str()
                .unwrap()
                .contains("deal_stage_changed")
        );

        // ── Follow-ups: the overdue call shows with its age.
        let followups: Value = request.get("/api/insights/followups").await.json();
        assert_eq!(followups["overdue"].as_array().unwrap().len(), 1);
        assert!(followups["overdue"][0]["overdue_days"].as_i64().unwrap() > 0);
        assert_eq!(followups["overdue"][0]["summary"], "Chase the proposal");

        // ── Hygiene: the neglected deal fires amount + close + activity
        // rules; the moved deal only the recent-activity rule is spared
        // (it has an activity logged today via occurred_at default).
        let hygiene: Value = request
            .get("/api/insights/pipeline-hygiene?days=1")
            .await
            .json();
        let rules: Vec<&str> = hygiene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| f["rule"].as_str())
            .collect();
        assert!(rules.contains(&"open_deal_without_amount"));
        assert!(rules.contains(&"open_deal_without_expected_close"));
        assert!(rules.contains(&"open_deal_without_recent_activity"));

        // ── Executive pack: the won deal + its value; the withdrawal.
        let pack: Value = request.get("/api/insights/executive").await.json();
        assert_eq!(pack["deals_won"], 1);
        assert_eq!(pack["deals_lost"], 0);
        assert_eq!(pack["won_value_by_currency_minor"]["GBP"], 250_000);
        assert_eq!(pack["tickets_opened"], 1);
        assert_eq!(pack["consent_withdrawals"], 1);
        assert!(pack["activities_logged"].as_u64().unwrap() >= 1);

        // ── Forecast trends: the stored snapshot only.
        let trends: Value = request.get("/api/insights/forecast-trends").await.json();
        assert_eq!(trends["series"].as_array().unwrap().len(), 1);
        assert!(
            trends["note"]
                .as_str()
                .unwrap()
                .contains("no interpolated history")
        );

        // ── SLA register: the 1-minute first-response deadline may or
        // may not have passed yet; workload always reports the open
        // ticket under its assignee.
        let sla: Value = request.get("/api/insights/sla").await.json();
        let workload = sla["workload"].as_array().unwrap();
        assert_eq!(workload.len(), 1);
        assert_eq!(workload[0]["open"], 1);

        // ── DPO: coverage counts, the withdrawal, the duplicate pair.
        let dpo: Value = request.get("/api/insights/dpo").await.json();
        assert_eq!(dpo["contacts"], 3);
        assert_eq!(dpo["consent_coverage"]["withdrawn"], 1);
        assert_eq!(dpo["withdrawals_in_window"], 1);
        assert_eq!(dpo["consent_events_by_source"]["web form"], 1);
        let dupes = dpo["duplicate_person_refs"].as_array().unwrap();
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0]["contacts"].as_array().unwrap().len(), 2);

        // ── ETag replay on one view.
        let first = request.get("/api/insights/dpo").await;
        let etag = first
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let replay = request
            .get("/api/insights/dpo")
            .add_header("if-none-match", etag.clone())
            .await;
        assert_eq!(replay.status_code(), 304);
        // axa's stack (CRM-T222) also answers a list holding the tag, which the handler's own
        // comparison does not.
        let listed = request
            .get("/api/insights/dpo")
            .add_header("if-none-match", format!("\"other\", {etag}"))
            .await;
        assert_eq!(listed.status_code(), 304);
        assert_eq!(listed.headers()["etag"], etag.as_str());
    })
    .await;
}

/// The insight cache serves a repeat read from cache, and a write through
/// the API makes the next read fresh at once (generation bump).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn insight_cache_hits_then_write_invalidates() {
    request::<App, _, _>(|request, ctx| async move {
        let (pipeline_pid, _stages) = seed_pipeline!(&request).await;
        for name in ["Cached one", "Cached two"] {
            request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline_pid,
                                "amount_minor": 1_i64, "currency": "GBP" }))
                .await
                .assert_status_ok();
        }
        let count = |v: &Value| v["open_deals"].as_u64().unwrap_or(99);
        let first: Value = request.get("/api/insights/stale-deals?days=0").await.json();
        assert_eq!(count(&first), 2, "{first}");

        // Behind the API's back (no generation bump) the cached body stays.
        sqlx::raw_sql("UPDATE deals SET deleted_at = now()")
            .execute(contact_relationship_management::db::pool(&ctx))
            .await
            .unwrap();
        let second: Value = request.get("/api/insights/stale-deals?days=0").await.json();
        assert_eq!(count(&second), 2, "second read must come from cache");

        // A write through the API bumps the generation: fresh again.
        request
            .post("/api/deals")
            .json(&json!({ "name": "Fresh", "pipeline_pid": pipeline_pid,
                            "amount_minor": 1_i64, "currency": "GBP" }))
            .await
            .assert_status_ok();
        let third: Value = request.get("/api/insights/stale-deals?days=0").await.json();
        assert_eq!(count(&third), 1, "write must invalidate: {third}");
    })
    .await;
}

/// CRM-T207: a view of one record answers `404` for a record that is not there.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_view_of_a_missing_record_is_not_found() {
    request::<App, _, _>(|request, _ctx| async move {
        let missing = uuid::Uuid::new_v4();
        request
            .get(&format!("/api/insights/funnel?pipeline={missing}"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        request
            .get(&format!("/api/accounts/{missing}/clv"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

/// The SLA register (CRM-R14, CRM-R55): late tickets most-overdue first, each under its worst
/// deadline; at-risk counts for what falls due within four hours; a paused ticket's clock stopped
/// when it paused, so it is not at risk but stays late if it was already late.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_sla_register_lists_late_tickets_worst_first_and_counts_what_is_at_risk() {
    request::<App, _, _>(|request, ctx| async move {
        let pool = contact_relationship_management::db::pool(&ctx);
        let agent = a_user();
        let ticket = |title: &'static str, assignee: Option<String>| {
            let request = &request;
            async move {
                let made: Value = request
                    .post("/api/tickets")
                    .json(&json!({ "title": title, "assignee_ref": assignee }))
                    .await
                    .json();
                made["pid"].as_str().unwrap().to_string()
            }
        };
        let set = |pid: String, sql: &'static str| async move {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE tickets SET {sql} WHERE pid = '{pid}'::uuid"
            )))
            .execute(pool)
            .await
            .unwrap();
        };
        let both_late = ticket("Both late", Some(agent.clone())).await;
        set(
            both_late.clone(),
            "first_response_due_at = now() - interval '3 hours', \
             resolution_due_at = now() - interval '10 hours'",
        )
        .await;
        let at_risk = ticket("Due soon", Some(agent.clone())).await;
        set(
            at_risk.clone(),
            "first_response_due_at = now() + interval '2 hours', \
             resolution_due_at = now() + interval '2 days'",
        )
        .await;
        let paused_soon = ticket("Paused, due soon", Some(agent.clone())).await;
        set(
            paused_soon.clone(),
            "paused_at = now(), first_response_due_at = now() + interval '1 hour', \
             resolution_due_at = NULL",
        )
        .await;
        let paused_late = ticket("Paused after it was late", None).await;
        set(
            paused_late.clone(),
            "paused_at = now() - interval '5 hours', \
             first_response_due_at = now() - interval '7 hours', resolution_due_at = NULL",
        )
        .await;
        let no_deadline = ticket("No deadline", None).await;
        set(
            no_deadline,
            "first_response_due_at = NULL, resolution_due_at = NULL",
        )
        .await;

        let sla: Value = request.get("/api/insights/sla").await.json();
        let breaches = sla["breaches"].as_array().unwrap();
        let listed: Vec<(&str, &str, i64)> = breaches
            .iter()
            .map(|b| {
                (
                    b["pid"].as_str().unwrap(),
                    b["breached"].as_str().unwrap(),
                    b["overdue_hours"].as_i64().unwrap(),
                )
            })
            .collect();
        // Worst first; the paused one is late by the 2 hours before it paused, not 7.
        assert_eq!(
            listed,
            [
                (both_late.as_str(), "resolution", 10),
                (paused_late.as_str(), "first_response", 2)
            ],
            "{sla}"
        );
        let workload = sla["workload"].as_array().unwrap();
        let of = |who: &str| {
            workload
                .iter()
                .find(|w| w["assignee_ref"] == who)
                .cloned()
                .unwrap()
        };
        let theirs = of(&agent);
        assert_eq!(
            (
                theirs["open"].as_i64(),
                theirs["breached"].as_i64(),
                theirs["at_risk_4h"].as_i64()
            ),
            (Some(3), Some(1), Some(1)),
            "{theirs}"
        );
        let unassigned = of("unassigned");
        assert_eq!(
            (
                unassigned["open"].as_i64(),
                unassigned["breached"].as_i64(),
                unassigned["at_risk_4h"].as_i64()
            ),
            (Some(2), Some(1), Some(0)),
            "{unassigned}"
        );
    })
    .await;
}

/// The hygiene findings the first estate does not reach (a deal past its expected close, leads nobody
/// has worked) and the executive pack's lost reasons, counted per reason (CRM-T292, round 16).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn hygiene_flags_a_deal_past_its_close_and_leads_nobody_worked_and_lost_reasons_are_counted()
{
    request::<App, _, _>(|request, ctx| async move {
        let pool = contact_relationship_management::db::pool(&ctx);
        let (pipeline_pid, stages) = seed_pipeline!(&request).await;

        // An open deal with an amount and a close date that has gone by.
        let overdue: Value = request
            .post("/api/deals")
            .json(
                &json!({ "name": "Overdue close", "pipeline_pid": pipeline_pid,
                           "amount_minor": 90_000_i64, "currency": "GBP",
                           "expected_close_on": "2020-01-31" }),
            )
            .await
            .json();
        let overdue_pid = overdue["pid"].as_str().unwrap().to_string();

        // Three leads, all made forty days ago: one never touched, one touched a month and a half
        // ago, one touched today.
        let mut leads = Vec::new();
        for name in ["Never touched", "Touched long ago", "Touched today"] {
            let made = request
                .post("/api/leads")
                .json(&json!({ "display_name": name, "source": "web" }))
                .await;
            let text = made.text();
            let lead: Value = serde_json::from_str(&text).unwrap();
            leads.push(
                lead["pid"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{text}"))
                    .to_string(),
            );
        }
        sqlx::query("UPDATE leads SET created_at = now() - interval '40 days'")
            .execute(pool)
            .await
            .unwrap();
        for (pid, days_ago) in [(&leads[1], 45_i64), (&leads[2], 0)] {
            request
                .post("/api/activities")
                .json(&json!({ "subject_kind": "lead", "subject_pid": pid,
                               "kind": "call", "summary": "Rang them", "actor_ref": a_user() }))
                .await
                .assert_status_ok();
            sqlx::query(
                "UPDATE activities SET occurred_at = now() - make_interval(days => $2::int) \
                 WHERE subject_pid = $1::uuid",
            )
            .bind(pid)
            .bind(days_ago as i32)
            .execute(pool)
            .await
            .unwrap();
        }

        let hygiene: Value = request
            .get("/api/insights/pipeline-hygiene?days=30")
            .await
            .json();
        let findings = hygiene["findings"].as_array().unwrap();
        let by_rule =
            |rule: &str| -> Vec<&Value> { findings.iter().filter(|f| f["rule"] == rule).collect() };
        let past = by_rule("open_deal_past_expected_close");
        assert_eq!(past.len(), 1, "{hygiene}");
        assert_eq!(past[0]["deal"]["pid"], overdue_pid);
        assert_eq!(past[0]["expected_close_on"], "2020-01-31");
        assert!(by_rule("open_deal_without_expected_close").is_empty());
        let unworked: Vec<&str> = by_rule("unworked_lead")
            .iter()
            .map(|f| f["lead_pid"].as_str().unwrap())
            .collect();
        assert!(
            unworked.contains(&leads[0].as_str()),
            "never touched: {hygiene}"
        );
        assert!(
            unworked.contains(&leads[1].as_str()),
            "touched long ago: {hygiene}"
        );
        assert!(
            !unworked.contains(&leads[2].as_str()),
            "touched today: {hygiene}"
        );
        assert!(by_rule("unworked_lead")[0]["score"].is_number());

        // Lost deals are counted per reason, the reason kept as it was written.
        for (name, reason) in [("A", "price"), ("B", "price"), ("C", "timing")] {
            let deal: Value = request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline_pid,
                               "amount_minor": 1_000_i64, "currency": "GBP" }))
                .await
                .json();
            request
                .post(&format!(
                    "/api/deals/{}/stage",
                    deal["pid"].as_str().unwrap()
                ))
                .json(&json!({ "stage_pid": stages[3], "lost_reason": reason }))
                .await
                .assert_status_ok();
        }
        let pack: Value = request.get("/api/insights/executive").await.json();
        assert_eq!(pack["deals_lost"], 3, "{pack}");
        assert_eq!(pack["lost_reasons"]["price"], 2, "{pack}");
        assert_eq!(pack["lost_reasons"]["timing"], 1, "{pack}");
    })
    .await;
}
