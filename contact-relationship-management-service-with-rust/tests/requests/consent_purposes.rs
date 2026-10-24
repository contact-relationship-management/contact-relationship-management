//! Consent per purpose and double opt-in (CRM-R29): the send gate for each state, the
//! confirmation and preference links, unsubscribe by purpose, and merge, unmerge and erasure.

use axum::http::StatusCode;
use contact_relationship_management::app::App;
use contact_relationship_management::mail::{self, Recorder, Sender};
use contact_relationship_management::unsubscribe::{self, Unsubscriber};
use loco_rs::testing::prelude::*;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;

const SECRET: &str = "0123456789abcdef0123456789abcdef";
const BASE: &str = "https://crm.example.org";

/// Puts the simulated sender back when a test ends, even by panic.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        mail::set(Sender::Simulated);
        unsubscribe::install(None);
    }
}

fn recording() -> (Recorder, Restore) {
    let recorder = Recorder::default();
    mail::set(Sender::Recording(recorder.clone()));
    unsubscribe::install(Some(Unsubscriber::new(SECRET, BASE, true).unwrap()));
    (recorder, Restore)
}

fn path_of(url: &str) -> String {
    url.strip_prefix(BASE)
        .expect("under the public url")
        .to_string()
}

macro_rules! give_address {
    ($request:expr, $pid:expr, $address:expr) => {{
        let detail: Value = $request
            .get(&format!("/api/contacts/{}", $pid))
            .await
            .json();
        let person_ref = detail["contact"]["person_ref"]
            .as_str()
            .unwrap()
            .to_string();
        contact_relationship_management::clients::prime_email(&person_ref, $address);
    }};
}

/// A scheduled campaign over everyone, sent under `purpose`. Expands to its pid.
macro_rules! campaign {
    ($request:expr, $purpose:expr) => {{
        let segment: Value = $request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let campaign: Value = $request
            .post("/api/campaigns")
            .json(&json!({
                "name": "News", "segment_pid": segment["pid"], "subject": "News",
                "body": "Hello", "purpose": $purpose,
            }))
            .await
            .json();
        let pid = campaign["pid"].as_str().unwrap().to_string();
        $request
            .post(&format!("/api/campaigns/{pid}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status_ok();
        pid
    }};
}

macro_rules! set_consent {
    ($request:expr, $pid:expr, $purpose:expr, $action:expr) => {
        $request
            .post(&format!("/api/contacts/{}/consent", $pid))
            .json(&json!({ "action": $action, "source": "test", "purpose": $purpose }))
            .await
            .assert_status_ok()
    };
}

macro_rules! state {
    ($request:expr, $pid:expr, $purpose:expr) => {{
        let body: Value = $request
            .get(&format!("/api/contacts/{}/consents", $pid))
            .await
            .json();
        body["purposes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["purpose"] == $purpose)
            .unwrap()["state"]
            .as_str()
            .unwrap()
            .to_string()
    }};
}

macro_rules! request_consent {
    ($request:expr, $pid:expr, $purpose:expr) => {{
        let r = $request
            .post(&format!("/api/contacts/{}/consent/request", $pid))
            .json(&json!({ "purpose": $purpose, "source": "web form" }))
            .await;
        r.assert_status_ok();
        r.json::<Value>()
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn each_purpose_is_its_own_and_only_granted_allows_a_send() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let names = ["granted", "pending", "withdrawn", "never", "both"];
        let mut pids = Vec::new();
        for name in names {
            let pid = seed_contact!(&request, format!("Purpose {name}")).await;
            give_address!(&request, pid, &format!("{name}@example.test"));
            pids.push(pid);
        }
        // Everyone starts with marketing granted (seed_contact!) and nothing else.
        for pid in &pids {
            assert_eq!(state!(request, pid, "newsletter"), "never");
            assert_eq!(state!(request, pid, "marketing"), "granted");
        }
        set_consent!(request, pids[0], "newsletter", "granted");
        let _ = request_consent!(request, pids[1], "newsletter");
        set_consent!(request, pids[2], "newsletter", "granted");
        set_consent!(request, pids[2], "newsletter", "withdrawn");
        set_consent!(request, pids[4], "newsletter", "granted");
        // Withdrawing one purpose changes no other.
        set_consent!(request, pids[0], "marketing", "withdrawn");
        assert_eq!(state!(request, pids[0], "newsletter"), "granted");
        assert_eq!(state!(request, pids[1], "newsletter"), "pending");
        assert_eq!(state!(request, pids[2], "newsletter"), "withdrawn");

        // A newsletter campaign reaches those who granted the newsletter, and nobody else.
        let news = campaign!(request, "newsletter");
        request
            .post(&format!("/api/campaigns/{news}/run"))
            .await
            .assert_status_ok();
        let mut to: Vec<String> = recorder.sent().iter().map(|m| m.to.clone()).collect();
        to.sort();
        assert_eq!(to, ["both@example.test", "granted@example.test"]);

        // A marketing campaign reaches those who granted marketing: the first one withdrew it.
        let general = campaign!(request, "marketing");
        request
            .post(&format!("/api/campaigns/{general}/run"))
            .await
            .assert_status_ok();
        let mut to: Vec<String> = recorder
            .sent()
            .iter()
            .skip(2)
            .map(|m| m.to.clone())
            .collect();
        to.sort();
        assert_eq!(
            to,
            [
                "both@example.test",
                "never@example.test",
                "pending@example.test",
                "withdrawn@example.test"
            ]
        );

        // The segment preview gates on the purpose asked for.
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "S", "filter": {} }))
            .await
            .json();
        let spid = segment["pid"].as_str().unwrap();
        let marketing: Value = request
            .get(&format!("/api/segments/{spid}/preview"))
            .await
            .json();
        let newsletter: Value = request
            .get(&format!("/api/segments/{spid}/preview?purpose=newsletter"))
            .await
            .json();
        assert!(marketing["count"].as_u64().unwrap() >= 4);
        assert_eq!(newsletter["count"], 2);
        request
            .get(&format!("/api/segments/{spid}/preview?purpose=bogus"))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        // History says which purpose each event was about.
        let history: Value = request
            .get(&format!("/api/contacts/{}/consent", pids[0]))
            .await
            .json();
        let purposes: Vec<&str> = history
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["purpose"].as_str())
            .collect();
        assert_eq!(purposes, ["marketing", "newsletter", "marketing"]);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_double_opt_in_is_pending_until_the_person_confirms_and_never_overrides_a_withdrawal() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_recorder, _restore) = recording();
        let pid = seed_contact!(&request, "Double Opt In").await;
        let requested = request_consent!(request, pid, "events");
        assert_eq!(requested["state"], "pending");
        let path = path_of(requested["confirm_url"].as_str().unwrap());
        assert!(path.starts_with("/confirm/"));
        assert_eq!(state!(request, pid, "events"), "pending");

        // Opening the link (a mail scanner would) changes nothing.
        let page = request.get(&path).await;
        page.assert_status_ok();
        assert!(page.text().contains("<form method=\"post\""));
        assert_eq!(state!(request, pid, "events"), "pending");

        // The button confirms, with no credentials; the event records source and purpose.
        request.post(&path).await.assert_status_ok();
        assert_eq!(state!(request, pid, "events"), "granted");
        let history: Value = request
            .get(&format!("/api/contacts/{pid}/consent"))
            .await
            .json();
        let last = history.as_array().unwrap().last().unwrap().clone();
        assert_eq!(
            (
                last["purpose"].as_str(),
                last["action"].as_str(),
                last["source"].as_str()
            ),
            (Some("events"), Some("granted"), Some("double_opt_in")),
            "{last}"
        );
        // A repeat is harmless and records nothing more.
        request.post(&path).await.assert_status_ok();
        let again: Value = request
            .get(&format!("/api/contacts/{pid}/consent"))
            .await
            .json();
        assert_eq!(
            again.as_array().unwrap().len(),
            history.as_array().unwrap().len()
        );

        // Already granted: another request is refused.
        request
            .post(&format!("/api/contacts/{pid}/consent/request"))
            .json(&json!({ "purpose": "events", "source": "x" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        // Withdrawn after asking: the link grants nothing.
        let second = request_consent!(request, pid, "newsletter");
        let stale = path_of(second["confirm_url"].as_str().unwrap());
        set_consent!(request, pid, "newsletter", "withdrawn");
        request
            .post(&stale)
            .await
            .assert_status(StatusCode::NOT_FOUND);
        assert_eq!(state!(request, pid, "newsletter"), "withdrawn");

        // Forged, wrong-purpose and unknown links are all the same not-found.
        request
            .get("/confirm/garbage")
            .await
            .assert_status(StatusCode::NOT_FOUND);
        request
            .post("/confirm/garbage")
            .await
            .assert_status(StatusCode::NOT_FOUND);
        let mut tampered = path.clone();
        tampered.push('x');
        request
            .post(&tampered)
            .await
            .assert_status(StatusCode::NOT_FOUND);
        // A preference token is not a confirmation token.
        let link: Value = request
            .post(&format!("/api/contacts/{pid}/preferences-link"))
            .await
            .json();
        let prefs = path_of(link["url"].as_str().unwrap());
        let as_confirm = prefs.replace("/preferences/", "/confirm/");
        request
            .post(&as_confirm)
            .await
            .assert_status(StatusCode::NOT_FOUND);

        // Bad requests and an unconfigured signer.
        request
            .post(&format!("/api/contacts/{pid}/consent/request"))
            .json(&json!({ "purpose": "bogus", "source": "x" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        request
            .post(&format!("/api/contacts/{pid}/consent"))
            .json(&json!({ "action": "granted", "source": "x", "purpose": "bogus" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        request
            .post(&format!("/api/contacts/{pid}/consent"))
            .json(&json!({ "action": "pending", "source": "x" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        unsubscribe::install(None);
        request
            .post(&format!("/api/contacts/{pid}/consent/request"))
            .json(&json!({ "purpose": "newsletter", "source": "x" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_preference_page_changes_one_purpose_at_a_time() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_recorder, _restore) = recording();
        let pid = seed_contact!(&request, "Preferences").await;
        let link: Value = request
            .post(&format!("/api/contacts/{pid}/preferences-link"))
            .await
            .json();
        let path = path_of(link["url"].as_str().unwrap());
        assert!(path.starts_with("/preferences/"));

        let page = request.get(&path).await;
        page.assert_status_ok();
        let html = page.text();
        for purpose in ["marketing", "newsletter", "events", "product_updates"] {
            assert!(
                html.contains(&format!("value=\"{purpose}\"")),
                "{purpose} in {html}"
            );
        }
        assert_eq!(page.header("cache-control"), "no-store");

        request
            .post(&path)
            .form(&[("purpose", "newsletter"), ("action", "granted")])
            .await
            .assert_status_ok();
        assert_eq!(state!(request, pid, "newsletter"), "granted");
        assert_eq!(state!(request, pid, "marketing"), "granted");
        request
            .post(&path)
            .form(&[("purpose", "marketing"), ("action", "withdrawn")])
            .await
            .assert_status_ok();
        assert_eq!(state!(request, pid, "marketing"), "withdrawn");
        assert_eq!(state!(request, pid, "newsletter"), "granted");
        let history: Value = request
            .get(&format!("/api/contacts/{pid}/consent"))
            .await
            .json();
        assert_eq!(
            history.as_array().unwrap().last().unwrap()["source"],
            "preference_page"
        );

        request
            .post(&path)
            .form(&[("purpose", "bogus"), ("action", "granted")])
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        request
            .post(&path)
            .form(&[("purpose", "events"), ("action", "pending")])
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        request
            .get("/preferences/garbage")
            .await
            .assert_status(StatusCode::NOT_FOUND);
        request
            .post("/preferences/garbage")
            .form(&[("purpose", "events"), ("action", "granted")])
            .await
            .assert_status(StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unsubscribe_link_withdraws_only_the_purpose_of_its_message() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let pid = seed_contact!(&request, "Unsub Purpose").await;
        give_address!(&request, pid, "unsub@example.test");
        set_consent!(request, pid, "newsletter", "granted");
        let news = campaign!(request, "newsletter");
        request
            .post(&format!("/api/campaigns/{news}/run"))
            .await
            .assert_status_ok();
        let link = recorder.sent()[0].unsubscribe_url.clone().unwrap();

        request.post(&path_of(&link)).await.assert_status_ok();
        assert_eq!(state!(request, pid, "newsletter"), "withdrawn");
        assert_eq!(
            state!(request, pid, "marketing"),
            "granted",
            "the others are untouched"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_merge_and_an_unmerge_apply_the_stricter_rule_to_every_purpose() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_recorder, _restore) = recording();
        let survivor = seed_contact!(&request, "Survivor").await;
        let merged = seed_contact!(&request, "Merged Away").await;
        // newsletter: granted + pending -> pending. events: granted + withdrawn -> withdrawn.
        // product_updates: never + granted -> never.
        set_consent!(request, survivor, "newsletter", "granted");
        let _ = request_consent!(request, merged, "newsletter");
        set_consent!(request, survivor, "events", "granted");
        set_consent!(request, merged, "events", "granted");
        set_consent!(request, merged, "events", "withdrawn");
        set_consent!(request, merged, "product_updates", "granted");

        let dry: Value = {
            let r = request
                .post(&format!("/api/contacts/{merged}/merge"))
                .json(&json!({ "into_pid": survivor, "reason": "same person", "dry_run": true }))
                .await;
            r.assert_status_ok();
            r.json()
        };
        let result_of = |body: &Value, purpose: &str| {
            body["consent"]["purposes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["purpose"] == purpose)
                .map(|p| p["result"].as_str().unwrap().to_string())
        };
        assert_eq!(
            result_of(&dry, "newsletter").as_deref(),
            Some("pending"),
            "{dry}"
        );
        assert_eq!(
            state!(request, survivor, "newsletter"),
            "granted",
            "a dry run changes nothing"
        );

        request
            .post(&format!("/api/contacts/{merged}/merge"))
            .json(&json!({ "into_pid": survivor, "reason": "same person", "dry_run": false }))
            .await
            .assert_status_ok();
        assert_eq!(state!(request, survivor, "newsletter"), "pending");
        assert_eq!(state!(request, survivor, "events"), "withdrawn");
        assert_eq!(state!(request, survivor, "product_updates"), "never");
        let history: Value = request
            .get(&format!("/api/contacts/{survivor}/consent"))
            .await
            .json();
        assert!(history.as_array().unwrap().iter().any(|e| {
            e["purpose"] == "events"
                && e["action"] == "withdrawn"
                && e["source"].as_str().unwrap().starts_with("merge:")
        }));

        // Unmerge: the returning contact gets the stricter of its own and the survivor's.
        request
            .post(&format!("/api/contacts/{merged}/unmerge"))
            .json(&json!({ "reason": "wrong person", "dry_run": false }))
            .await
            .assert_status_ok();
        assert_eq!(state!(request, merged, "newsletter"), "pending");
        assert_eq!(state!(request, merged, "events"), "withdrawn");
        // It had granted product_updates itself; the survivor holds never, which is stricter.
        assert_eq!(state!(request, merged, "product_updates"), "never");
        // And the survivor keeps what it has.
        assert_eq!(state!(request, survivor, "events"), "withdrawn");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn erasure_withdraws_every_purpose_and_the_export_lists_them() {
    request::<App, _, _>(|request, ctx| async move {
        let pid = seed_contact!(&request, "Erase Purposes").await;
        set_consent!(request, pid, "newsletter", "granted");
        set_consent!(request, pid, "events", "granted");
        let export: Value = request
            .get(&format!("/api/contacts/{pid}/subject-access"))
            .await
            .json();
        let held: Vec<(&str, &str)> = export["consent_purposes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p["purpose"].as_str().unwrap(), p["state"].as_str().unwrap()))
            .collect();
        assert_eq!(
            held,
            [
                ("marketing", "granted"),
                ("newsletter", "granted"),
                ("events", "granted"),
                ("product_updates", "never")
            ]
        );

        let erased = request.post(&format!("/api/contacts/{pid}/erase")).await;
        erased.assert_status_ok();
        let body = erased.json::<Value>();
        assert_eq!(body["erased"], pid.as_str());
        // The contact is gone from the API now, so read the table.
        let rows = ctx
            .db
            .query_all_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT purpose, state FROM contact_consents WHERE contact_pid = $1 ORDER BY purpose",
                [pid.parse::<uuid::Uuid>().unwrap().into()],
            ))
            .await
            .unwrap();
        let held: Vec<(String, String)> = rows
            .iter()
            .map(|r| (r.try_get("", "purpose").unwrap(), r.try_get("", "state").unwrap()))
            .collect();
        assert_eq!(
            held,
            [("events".to_string(), "withdrawn".to_string()), ("newsletter".to_string(), "withdrawn".to_string())]
        );
    })
    .await;
}
