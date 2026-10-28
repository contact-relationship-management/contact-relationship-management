//! Quotas, commission plans and attainment (CRM-R59): what is set by hand, what is refused, and
//! what a worker's won deals reach.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_worker, seed_pipeline};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_plan_is_set_replaced_checked_and_cannot_be_removed_while_a_quota_uses_it() {
    request::<App, _, _>(|request, _ctx| async move {
        let made: Value = request
            .put("/api/commission-plans")
            .json(&json!({ "name": "Standard", "tiers": [
                { "from_percent": 0, "rate_bps": 500 },
                { "from_percent": 100, "rate_bps": 800 } ] }))
            .await
            .json();
        let pid = made["pid"].as_str().unwrap().to_string();
        // The same name in another case is the same plan, with new tiers.
        let again: Value = request
            .put("/api/commission-plans")
            .json(&json!({ "name": "  STANDARD ", "tiers": [{ "from_percent": 0, "rate_bps": 600 }] }))
            .await
            .json();
        assert_eq!(again["pid"], pid.as_str(), "{again}");
        assert_eq!(again["tiers"].as_array().unwrap().len(), 1);
        let all: Value = request.get("/api/commission-plans").await.json();
        assert_eq!(all.as_array().unwrap().len(), 1);

        for (tiers, name) in [
            (json!([]), "Standard"),
            (json!([{ "from_percent": 10, "rate_bps": 500 }]), "Standard"),
            (
                json!([{ "from_percent": 0, "rate_bps": 500 }, { "from_percent": 0, "rate_bps": 600 }]),
                "Standard",
            ),
            (json!([{ "from_percent": 0, "rate_bps": 10_001 }]), "Standard"),
            (json!([{ "from_percent": 0, "rate_bps": 500 }]), "   "),
        ] {
            let r = request
                .put("/api/commission-plans")
                .json(&json!({ "name": name, "tiers": tiers }))
                .await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        request
            .put("/api/commission-plans")
            .json(&json!({ "name": "X", "tiers": [{ "from_percent": 0, "rate_bps": 1 }], "extra": 1 }))
            .await
            .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

        // A quota paid under it keeps it from being removed.
        let worker = a_worker();
        let quota: Value = request
            .put("/api/quotas")
            .json(&json!({ "owner_ref": worker, "period_from": "2026-01-01",
                           "period_to": "2026-03-31", "currency": "GBP",
                           "amount_minor": 1_000_000, "plan_pid": pid }))
            .await
            .json();
        request
            .delete(&format!("/api/commission-plans/{pid}"))
            .await
            .assert_status(axum::http::StatusCode::CONFLICT);
        let listed: Value = request.get("/api/commission-plans").await.json();
        assert_eq!(listed[0]["quotas"], 1, "{listed}");
        request
            .delete(&format!("/api/quotas/{}", quota["pid"].as_str().unwrap()))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/commission-plans/{pid}"))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/commission-plans/{pid}"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_quota_that_cannot_be_set_is_refused_naming_the_member() {
    request::<App, _, _>(|request, _ctx| async move {
        let worker = a_worker();
        let body = |extra: Value| {
            let mut base = json!({ "owner_ref": worker, "period_from": "2026-01-01",
                                   "period_to": "2026-03-31", "currency": "GBP",
                                   "amount_minor": 100_000 });
            for (k, v) in extra.as_object().unwrap() {
                base[k] = v.clone();
            }
            base
        };
        request
            .put("/api/quotas")
            .json(&body(json!({})))
            .await
            .assert_status_ok();
        // Setting exactly the same period again replaces it.
        let replaced: Value = request
            .put("/api/quotas")
            .json(&body(json!({ "amount_minor": 250_000 })))
            .await
            .json();
        assert_eq!(replaced["amount_minor"], 250_000);
        assert_eq!(
            request
                .get("/api/quotas")
                .await
                .json::<Value>()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        // A different currency for the same days is a different quota.
        request
            .put("/api/quotas")
            .json(&body(json!({ "currency": "USD" })))
            .await
            .assert_status_ok();

        for (extra, pointer) in [
            (json!({ "amount_minor": 0 }), "/amount_minor"),
            (json!({ "amount_minor": -5 }), "/amount_minor"),
            (json!({ "currency": "gbp" }), "/currency"),
            (json!({ "owner_ref": "person:not-a-worker" }), "/owner_ref"),
            (json!({ "period_to": "2025-12-31" }), "/period_to"),
            (json!({ "period_to": "2027-06-30" }), "/period_to"),
            (
                json!({ "plan_pid": "00000000-0000-0000-0000-000000000001" }),
                "/plan_pid",
            ),
            // Overlaps the first quota's days, in the same currency.
            (
                json!({ "period_from": "2026-03-01", "period_to": "2026-06-30" }),
                "/period_from",
            ),
        ] {
            let r = request.put("/api/quotas").json(&body(extra.clone())).await;
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let refused: Value = r.json();
            assert_eq!(
                refused["errors"][0]["pointer"], pointer,
                "{extra}: {refused}"
            );
        }
        // The next quarter, touching but not overlapping, is fine.
        request
            .put("/api/quotas")
            .json(&body(
                json!({ "period_from": "2026-04-01", "period_to": "2026-06-30" }),
            ))
            .await
            .assert_status_ok();
        request
            .delete("/api/quotas/00000000-0000-0000-0000-000000000001")
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn attainment_is_the_workers_won_revenue_in_the_period_and_the_commission_its_plan_pays() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let (ada, grace) = (a_worker(), a_worker());
        let win = |owner: String, name: &'static str, amount: i64, currency: &'static str| {
            let (request, pipeline, stage) = (&request, pipeline.clone(), stages[2].clone());
            async move {
                let deal: Value = request
                    .post("/api/deals")
                    .json(
                        &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                                   "currency": currency, "owner_ref": owner }),
                    )
                    .await
                    .json();
                request
                    .post(&format!(
                        "/api/deals/{}/stage",
                        deal["pid"].as_str().unwrap()
                    ))
                    .json(&json!({ "stage_pid": stage }))
                    .await
                    .assert_status_ok();
            }
        };
        win(ada.clone(), "A1", 60_000, "GBP").await;
        win(ada.clone(), "A2", 70_000, "GBP").await;
        win(ada.clone(), "A3", 5_000, "USD").await;
        win(grace.clone(), "G1", 40_000, "GBP").await;
        // An open deal does not count.
        request
            .post("/api/deals")
            .json(
                &json!({ "name": "Open", "pipeline_pid": pipeline, "amount_minor": 999_999,
                           "currency": "GBP", "owner_ref": ada }),
            )
            .await
            .assert_status_ok();

        let plan: Value = request
            .put("/api/commission-plans")
            .json(&json!({ "name": "Standard", "tiers": [
                { "from_percent": 0, "rate_bps": 500 },
                { "from_percent": 100, "rate_bps": 800 } ] }))
            .await
            .json();
        let today = chrono::Utc::now().date_naive();
        let (from, to) = (
            today - chrono::Duration::days(10),
            today + chrono::Duration::days(80),
        );
        for (owner, amount, plan_pid) in [
            (&ada, 100_000, Some(plan["pid"].clone())),
            (&grace, 100_000, None),
        ] {
            request
                .put("/api/quotas")
                .json(
                    &json!({ "owner_ref": owner, "period_from": from, "period_to": to,
                               "currency": "GBP", "amount_minor": amount, "plan_pid": plan_pid }),
                )
                .await
                .assert_status_ok();
        }
        // A quota whose period is over does not appear for today.
        request
            .put("/api/quotas")
            .json(
                &json!({ "owner_ref": ada, "period_from": "2020-01-01", "period_to": "2020-03-31",
                           "currency": "GBP", "amount_minor": 1 }),
            )
            .await
            .assert_status_ok();

        let report: Value = request.get("/api/quotas/attainment").await.json();
        let rows = report["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2, "{report}");
        let row_of = |owner: &str| {
            rows.iter()
                .find(|r| r["owner_ref"] == owner)
                .unwrap()
                .clone()
        };

        let a = row_of(&ada);
        assert_eq!(a["revenue_minor"], 130_000, "{a}");
        assert_eq!(a["won_deals"], 2);
        assert_eq!(
            a["other_currencies"],
            json!({ "USD": 5_000 }),
            "never converted: {a}"
        );
        assert_eq!(a["attainment"]["numerator"], 130_000);
        assert_eq!(a["attainment"]["denominator"], 100_000);
        assert_eq!(a["remaining_minor"], 0);
        assert_eq!(a["plan"], "Standard");
        // 100,000 at 5% and 30,000 at 8%.
        assert_eq!(a["commission_minor"], 5_000 + 2_400, "{a}");
        assert_eq!(a["bands"].as_array().unwrap().len(), 2);
        assert_eq!(a["bands"][1]["revenue_minor"], 30_000);

        let g = row_of(&grace);
        assert_eq!(g["revenue_minor"], 40_000);
        assert_eq!(g["remaining_minor"], 60_000);
        assert!(
            g["commission_minor"].is_null() && g["plan"].is_null(),
            "{g}"
        );
        assert!(g["bands"].as_array().unwrap().is_empty());

        // One worker, and a day in the past.
        let only: Value = request
            .get(&format!("/api/quotas/attainment?owner_ref={grace}"))
            .await
            .json();
        assert_eq!(only["rows"].as_array().unwrap().len(), 1);
        let old: Value = request
            .get("/api/quotas/attainment?at=2020-02-01")
            .await
            .json();
        assert_eq!(old["rows"].as_array().unwrap().len(), 1, "{old}");
        assert_eq!(old["rows"][0]["revenue_minor"], 0);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_split_deal_is_credited_in_shares_to_the_minor_unit_and_a_removed_split_gives_it_back() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let (ada, grace) = (a_worker(), a_worker());
        let win = |owner: String, name: &'static str, amount: i64| {
            let (request, pipeline, stage) = (&request, pipeline.clone(), stages[2].clone());
            async move {
                let deal: Value = request
                    .post("/api/deals")
                    .json(
                        &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                                   "currency": "GBP", "owner_ref": owner }),
                    )
                    .await
                    .json();
                let pid = deal["pid"].as_str().unwrap().to_string();
                request
                    .post(&format!("/api/deals/{pid}/stage"))
                    .json(&json!({ "stage_pid": stage }))
                    .await
                    .assert_status_ok();
                pid
            }
        };
        let halved = win(ada.clone(), "Halved", 100_001).await;
        let handed = win(ada.clone(), "Handed over", 10_000).await;
        win(grace.clone(), "Own", 7_000).await;

        let today = chrono::Utc::now().date_naive();
        let (from, to) = (
            today - chrono::Duration::days(10),
            today + chrono::Duration::days(80),
        );
        for owner in [&ada, &grace] {
            request
                .put("/api/quotas")
                .json(
                    &json!({ "owner_ref": owner, "period_from": from, "period_to": to,
                               "currency": "GBP", "amount_minor": 100_000 }),
                )
                .await
                .assert_status_ok();
        }
        let standing = || {
            let request = &request;
            let (ada, grace) = (ada.clone(), grace.clone());
            async move {
                let report: Value = request.get("/api/quotas/attainment").await.json();
                let rows = report["rows"].as_array().unwrap().clone();
                let of = |owner: &str| {
                    let r = rows.iter().find(|r| r["owner_ref"] == owner).unwrap();
                    (r["revenue_minor"].as_i64().unwrap(), r["won_deals"].as_i64().unwrap())
                };
                (of(&ada), of(&grace))
            }
        };
        assert_eq!(standing().await, ((110_001, 2), (7_000, 1)));

        // Halved: the odd minor unit goes to the earlier share. Handed over: wholly to Grace.
        let url = |deal: &str| format!("/api/quotas/splits/{deal}");
        let set = request
            .put(&url(&halved))
            .json(&json!({ "shares": [
                { "owner_ref": ada, "share_bps": 5_000 },
                { "owner_ref": grace, "share_bps": 5_000 } ] }))
            .await;
        set.assert_status_ok();
        request
            .put(&url(&handed))
            .json(&json!({ "shares": [{ "owner_ref": grace, "share_bps": 10_000 }] }))
            .await
            .assert_status_ok();
        assert_eq!(standing().await, ((50_001, 1), (50_000 + 10_000 + 7_000, 3)));
        let read: Value = request.get(&url(&halved)).await.json();
        assert_eq!(read["shares"].as_array().unwrap().len(), 2, "{read}");

        // Refused, naming /shares; an unknown deal is 404; nothing changed.
        for shares in [
            json!([]),
            json!([{ "owner_ref": ada, "share_bps": 9_000 }]),
            json!([{ "owner_ref": ada, "share_bps": 5_000 }, { "owner_ref": ada, "share_bps": 5_000 }]),
            json!([{ "owner_ref": "contact:x", "share_bps": 10_000 }]),
            json!([{ "owner_ref": ada, "share_bps": 0 }, { "owner_ref": grace, "share_bps": 10_000 }]),
        ] {
            request
                .put(&url(&halved))
                .json(&json!({ "shares": shares }))
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        request
            .put(&url(&uuid::Uuid::new_v4().to_string()))
            .json(&json!({ "shares": [{ "owner_ref": ada, "share_bps": 10_000 }] }))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        assert_eq!(standing().await, ((50_001, 1), (67_000, 3)));

        // Removing the splits credits the owner again; a second removal is 404.
        for deal in [&halved, &handed] {
            request.delete(&url(deal)).await.assert_status_ok();
        }
        request
            .delete(&url(&halved))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        assert_eq!(standing().await, ((110_001, 2), (7_000, 1)));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_team_adds_its_workers_standings_per_currency_and_is_replaced_by_name_and_removed() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline, stages) = seed_pipeline!(&request).await;
        let (ada, grace, idle) = (a_worker(), a_worker(), a_worker());
        for (owner, name, amount) in [(&ada, "Ada's", 30_000), (&grace, "Grace's", 20_000)] {
            let deal: Value = request
                .post("/api/deals")
                .json(
                    &json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": amount,
                               "currency": "GBP", "owner_ref": owner }),
                )
                .await
                .json();
            request
                .post(&format!(
                    "/api/deals/{}/stage",
                    deal["pid"].as_str().unwrap()
                ))
                .json(&json!({ "stage_pid": stages[2] }))
                .await
                .assert_status_ok();
        }
        let today = chrono::Utc::now().date_naive();
        let (from, to) = (
            today - chrono::Duration::days(10),
            today + chrono::Duration::days(80),
        );
        for owner in [&ada, &grace] {
            request
                .put("/api/quotas")
                .json(
                    &json!({ "owner_ref": owner, "period_from": from, "period_to": to,
                               "currency": "GBP", "amount_minor": 100_000 }),
                )
                .await
                .assert_status_ok();
        }

        let team: Value = request
            .put("/api/quotas/teams")
            .json(&json!({ "name": "North", "members": [ada, grace, idle] }))
            .await
            .json();
        let pid = team["pid"].as_str().unwrap().to_string();

        let report: Value = request.get("/api/quotas/team-attainment").await.json();
        let row = &report["rows"].as_array().unwrap()[0];
        assert_eq!(row["name"], "North", "{report}");
        let gbp = &row["currencies"].as_array().unwrap()[0];
        assert_eq!(gbp["quota_minor"], 200_000, "{row}");
        assert_eq!(gbp["revenue_minor"], 50_000, "{row}");
        assert_eq!(gbp["remaining_minor"], 150_000, "{row}");
        assert_eq!(gbp["workers"], 2, "{row}");
        assert_eq!(row["without_quota"], json!([idle]), "{row}");

        // The same name in another case replaces the team rather than adding one.
        let again: Value = request
            .put("/api/quotas/teams")
            .json(&json!({ "name": "NORTH", "members": [ada] }))
            .await
            .json();
        assert_eq!(again["pid"], pid.as_str());
        let listed: Value = request.get("/api/quotas/teams").await.json();
        assert_eq!(listed.as_array().unwrap().len(), 1, "{listed}");

        // Refused: no workers, a repeat, a non-worker, no name.
        for body in [
            json!({ "name": "South", "members": [] }),
            json!({ "name": "South", "members": [ada, ada] }),
            json!({ "name": "South", "members": ["contact:x"] }),
            json!({ "name": "", "members": [ada] }),
        ] {
            request
                .put("/api/quotas/teams")
                .json(&body)
                .await
                .assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }

        request
            .delete(&format!("/api/quotas/teams/{pid}"))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/quotas/teams/{pid}"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}
