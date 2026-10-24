//! Outbound webhooks (CRM-R28): registration, signing, retries, `410`, rotation and erasure.
//!
//! The network is a recording [`Sender`]; the address guard is unit-tested in
//! `rules::webhooks` and the real sender's refusal below.

use std::sync::Mutex;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use contact_relationship_management::app::App;
use contact_relationship_management::rules::webhooks as rules;
use contact_relationship_management::webhooks::{HttpSender, Outcome, Sender, run};
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use super::seed_contact;

/// One call the relay made.
struct Call {
    url: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Call {
    fn header(&self, name: &str) -> &str {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map_or("", |(_, v)| v.as_str())
    }
}

/// Answers every call with `status` and keeps what it was sent.
struct Recorder {
    status: Mutex<u16>,
    calls: Mutex<Vec<Call>>,
}

impl Recorder {
    fn answering(status: u16) -> Self {
        Self {
            status: Mutex::new(status),
            calls: Mutex::new(Vec::new()),
        }
    }
    fn respond_with(&self, status: u16) {
        *self.status.lock().unwrap() = status;
    }
    fn calls(&self) -> std::sync::MutexGuard<'_, Vec<Call>> {
        self.calls.lock().unwrap()
    }
}

#[async_trait::async_trait]
impl Sender for Recorder {
    async fn post(&self, url: &str, headers: &[(&'static str, String)], body: String) -> Outcome {
        self.calls.lock().unwrap().push(Call {
            url: url.to_string(),
            headers: headers
                .iter()
                .map(|(n, v)| ((*n).to_string(), v.clone()))
                .collect(),
            body,
        });
        Outcome::Status(*self.status.lock().unwrap())
    }
}

/// Write an outbox row, as the `outbox` transport does with a mutation.
async fn emit(
    db: &DatabaseConnection,
    entity: &str,
    kind: &str,
    pid: Uuid,
    at: DateTime<Utc>,
) -> Uuid {
    let event_id = Uuid::new_v4();
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO event_outbox (event_id, entity, entity_pid, kind, occurred_at, schema_version, payload) \
         VALUES ($1, $2, $3, $4, $5, 1, $6)",
        [
            event_id.into(),
            entity.into(),
            pid.into(),
            kind.into(),
            at.into(),
            json!({ "name": "Private Person Name", "data": { "email": "private@example.test" } }).into(),
        ],
    ))
    .await
    .unwrap();
    event_id
}

macro_rules! subscribe {
    ($request:expr, $events:expr) => {
        async {
            let r = $request
                .post("/api/webhook-subscriptions")
                .json(&json!({ "url": "https://hooks.example.com/crm", "events": $events }))
                .await;
            r.assert_status_ok();
            r.json::<Value>()
        }
    };
}

macro_rules! log {
    ($request:expr, $pid:expr) => {
        async {
            let rows: Vec<Value> = $request
                .get(&format!("/api/webhook-subscriptions/{}/deliveries", $pid))
                .await
                .json();
            rows
        }
    };
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_subscription_is_registered_listed_without_its_secret_and_refused_when_unsafe() {
    request::<App, _, _>(|request, _ctx| async move {
        let made = subscribe!(request, ["contact.*", "deal.won"]).await;
        let secret = made["secret"].as_str().unwrap();
        assert!(secret.starts_with("whsec_"));
        let listed: Vec<Value> = request.get("/api/webhook-subscriptions").await.json();
        let ours = listed.iter().find(|s| s["pid"] == made["pid"]).unwrap();
        assert_eq!(ours["events"], json!(["contact.*", "deal.won"]));
        assert_eq!(ours["enabled"], true);
        assert!(
            !ours.to_string().contains(secret),
            "the secret is shown once"
        );

        for (url, events, pointer) in [
            ("http://hooks.example.com/", json!(["*"]), "/url"),
            ("https://127.0.0.1/", json!(["*"]), "/url"),
            ("https://169.254.169.254/latest", json!(["*"]), "/url"),
            ("https://localhost/", json!(["*"]), "/url"),
            ("https://user:pw@hooks.example.com/", json!(["*"]), "/url"),
            ("https://hooks.example.com/", json!([]), "/events"),
            (
                "https://hooks.example.com/",
                json!(["not a pattern"]),
                "/events",
            ),
        ] {
            let r = request
                .post("/api/webhook-subscriptions")
                .json(&json!({ "url": url, "events": events }))
                .await;
            r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
            let body: Value = r.json();
            assert_eq!(
                body["errors"][0]["pointer"], pointer,
                "{url} {events}: {body}"
            );
        }
        request
            .post("/api/webhook-subscriptions")
            .json(&json!({ "url": "https://hooks.example.com/", "events": ["*"], "extra": 1 }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_matching_event_is_sent_signed_once_with_ids_and_no_contact_values() {
    request::<App, _, _>(|request, ctx| async move {
        let made = subscribe!(request, ["contact.*"]).await;
        let (pid, secret) = (
            made["pid"].as_str().unwrap(),
            made["secret"].as_str().unwrap(),
        );
        let contact = Uuid::new_v4();
        let event = emit(&ctx.db, "contact", "created", contact, Utc::now()).await;
        emit(&ctx.db, "deal", "opened", Uuid::new_v4(), Utc::now()).await;

        let recorder = Recorder::answering(200);
        let now = Utc::now();
        let report = run(&ctx.db, &recorder, now).await.unwrap();
        assert_eq!(
            (report.deliveries_created, report.delivered),
            (1, 1),
            "{report:?}"
        );

        {
            let calls = recorder.calls();
            assert_eq!(calls.len(), 1, "the deal event is not wanted");
            let call = &calls[0];
            assert_eq!(call.url, "https://hooks.example.com/crm");
            assert_eq!(call.header("webhook-id"), event.to_string());
            let timestamp: i64 = call.header("webhook-timestamp").parse().unwrap();
            assert_eq!(timestamp, now.timestamp());
            let expected = rules::sign(secret, &event.to_string(), timestamp, &call.body).unwrap();
            assert_eq!(call.header("webhook-signature"), expected);
            let body: Value = serde_json::from_str(&call.body).unwrap();
            assert_eq!(body["type"], "contact.created");
            assert_eq!(body["data"]["pid"], contact.to_string());
            assert!(
                !call.body.contains("Private Person Name")
                    && !call.body.contains("private@example"),
                "no contact value in the body: {}",
                call.body
            );
        }

        // Nothing is sent twice, however often the relay runs.
        let again = run(&ctx.db, &recorder, now + Duration::minutes(10))
            .await
            .unwrap();
        assert_eq!((again.deliveries_created, again.attempted), (0, 0));
        let rows = log!(request, pid).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["status"], "delivered");
        assert_eq!(rows[0]["attempts"], 1);
        assert_eq!(rows[0]["last_status"], 200);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_subscription_does_not_hear_what_happened_before_it_existed() {
    request::<App, _, _>(|request, ctx| async move {
        emit(
            &ctx.db,
            "contact",
            "created",
            Uuid::new_v4(),
            Utc::now() - Duration::hours(1),
        )
        .await;
        let made = subscribe!(request, ["*"]).await;
        let recorder = Recorder::answering(200);
        let report = run(&ctx.db, &recorder, Utc::now()).await.unwrap();
        assert_eq!(report.deliveries_created, 0);
        assert!(
            log!(request, made["pid"].as_str().unwrap())
                .await
                .is_empty()
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_failing_receiver_is_retried_with_backoff_and_then_the_delivery_is_dead() {
    request::<App, _, _>(|request, ctx| async move {
        let made = subscribe!(request, ["*"]).await;
        let pid = made["pid"].as_str().unwrap();
        emit(&ctx.db, "lead", "lead_captured", Uuid::new_v4(), Utc::now()).await;
        let recorder = Recorder::answering(500);
        let start = Utc::now();

        let first = run(&ctx.db, &recorder, start).await.unwrap();
        assert_eq!((first.attempted, first.retrying), (1, 1));
        // Too soon: the first retry waits 30 seconds.
        let soon = run(&ctx.db, &recorder, start + Duration::seconds(10))
            .await
            .unwrap();
        assert_eq!(soon.attempted, 0);
        let rows = log!(request, pid).await;
        assert_eq!(rows[0]["status"], "pending");
        assert_eq!(rows[0]["last_status"], 500);
        assert_eq!(rows[0]["last_error"], "the receiver answered 500");

        // Seven more attempts, each after the previous wait, then dead.
        for n in 1..8 {
            let r = run(&ctx.db, &recorder, start + Duration::days(2 * n))
                .await
                .unwrap();
            assert_eq!(r.attempted, 1, "attempt {}", n + 1);
        }
        let rows = log!(request, pid).await;
        assert_eq!(rows[0]["status"], "dead");
        assert_eq!(rows[0]["attempts"], 8);
        let after = run(&ctx.db, &recorder, start + Duration::days(30))
            .await
            .unwrap();
        assert_eq!(after.attempted, 0, "a dead delivery is not tried again");
        assert_eq!(recorder.calls().len(), 8);

        // A receiver that recovers gets the next one.
        recorder.respond_with(200);
        emit(&ctx.db, "lead", "lead_captured", Uuid::new_v4(), Utc::now()).await;
        let ok = run(&ctx.db, &recorder, Utc::now() + Duration::days(31))
            .await
            .unwrap();
        assert_eq!(ok.delivered, 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_410_disables_the_subscription_cancels_what_is_queued_and_is_audited() {
    request::<App, _, _>(|request, ctx| async move {
        let made = subscribe!(request, ["*"]).await;
        let pid = made["pid"].as_str().unwrap();
        for _ in 0..3 {
            emit(&ctx.db, "contact", "created", Uuid::new_v4(), Utc::now()).await;
        }
        let recorder = Recorder::answering(410);
        let report = run(&ctx.db, &recorder, Utc::now()).await.unwrap();
        assert_eq!(
            (report.attempted, report.subscriptions_disabled),
            (1, 1),
            "{report:?}"
        );
        let listed: Vec<Value> = request.get("/api/webhook-subscriptions").await.json();
        let ours = listed.iter().find(|s| s["pid"] == pid).unwrap();
        assert_eq!(ours["enabled"], false);
        assert_eq!(ours["disabled_reason"], "the receiver answered 410 Gone");
        let rows = log!(request, pid).await;
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|r| r["status"] == "cancelled"), "{rows:?}");
        let later = run(&ctx.db, &recorder, Utc::now() + Duration::days(1))
            .await
            .unwrap();
        assert_eq!(later.attempted, 0);
        let audits: Value = request.get(&format!("/api/audits/{pid}")).await.json();
        assert!(
            audits
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["action"] == "webhook_subscription_disabled"),
            "{audits}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_rotated_secret_signs_beside_the_old_one_for_a_day() {
    request::<App, _, _>(|request, ctx| async move {
        let made = subscribe!(request, ["*"]).await;
        let (pid, old) = (
            made["pid"].as_str().unwrap(),
            made["secret"].as_str().unwrap().to_string(),
        );
        let rotated: Value = {
            let r = request
                .post(&format!("/api/webhook-subscriptions/{pid}/rotate-secret"))
                .await;
            r.assert_status_ok();
            r.json()
        };
        let new = rotated["secret"].as_str().unwrap().to_string();
        assert_ne!(new, old);

        let recorder = Recorder::answering(200);
        let event = emit(&ctx.db, "contact", "created", Uuid::new_v4(), Utc::now()).await;
        run(&ctx.db, &recorder, Utc::now()).await.unwrap();
        {
            let calls = recorder.calls();
            let header = calls[0].header("webhook-signature");
            let ts: i64 = calls[0].header("webhook-timestamp").parse().unwrap();
            let id = event.to_string();
            let signatures: Vec<&str> = header.split(' ').collect();
            assert_eq!(signatures.len(), 2, "{header}");
            assert!(
                signatures.contains(&rules::sign(&new, &id, ts, &calls[0].body).unwrap().as_str())
            );
            assert!(
                signatures.contains(&rules::sign(&old, &id, ts, &calls[0].body).unwrap().as_str())
            );
        }

        // Two days on, only the new secret signs.
        let event = emit(&ctx.db, "contact", "created", Uuid::new_v4(), Utc::now()).await;
        run(&ctx.db, &recorder, Utc::now() + Duration::days(2))
            .await
            .unwrap();
        {
            let calls = recorder.calls();
            let header = calls[1].header("webhook-signature");
            assert_eq!(header.split(' ').count(), 1, "{header}");
            let ts: i64 = calls[1].header("webhook-timestamp").parse().unwrap();
            assert_eq!(
                header,
                rules::sign(&new, &event.to_string(), ts, &calls[1].body).unwrap()
            );
        }

        request
            .post(&format!(
                "/api/webhook-subscriptions/{}/rotate-secret",
                Uuid::new_v4()
            ))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn erasing_a_contact_cancels_its_queued_deliveries() {
    request::<App, _, _>(|request, ctx| async move {
        let made = subscribe!(request, ["*"]).await;
        let pid = made["pid"].as_str().unwrap();
        let contact = seed_contact!(&request, "Erase Hook").await;
        let other = Uuid::new_v4();
        emit(
            &ctx.db,
            "contact",
            "created",
            contact.parse().unwrap(),
            Utc::now(),
        )
        .await;
        emit(&ctx.db, "contact", "created", other, Utc::now()).await;
        let recorder = Recorder::answering(500);
        let start = Utc::now();
        let first = run(&ctx.db, &recorder, start).await.unwrap();
        assert_eq!(first.retrying, 2);

        request
            .post(&format!("/api/contacts/{contact}/erase"))
            .await
            .assert_status_ok();

        let later = run(&ctx.db, &recorder, start + Duration::days(1))
            .await
            .unwrap();
        assert_eq!(
            later.attempted, 1,
            "only the other contact's delivery is still queued"
        );
        let rows = log!(request, pid).await;
        let erased = rows.iter().find(|r| r["entity_pid"] == contact).unwrap();
        assert_eq!(erased["status"], "cancelled");
        assert_eq!(erased["last_error"], "contact erased");
        let kept = rows
            .iter()
            .find(|r| r["entity_pid"] == other.to_string())
            .unwrap();
        assert_eq!(kept["status"], "pending");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_removed_subscription_sends_nothing_more_and_keeps_its_log() {
    request::<App, _, _>(|request, ctx| async move {
        let made = subscribe!(request, ["*"]).await;
        let pid = made["pid"].as_str().unwrap();
        emit(&ctx.db, "contact", "created", Uuid::new_v4(), Utc::now()).await;
        let recorder = Recorder::answering(500);
        run(&ctx.db, &recorder, Utc::now()).await.unwrap();

        let removed: Value = request
            .delete(&format!("/api/webhook-subscriptions/{pid}"))
            .await
            .json();
        assert_eq!(removed["deliveries_cancelled"], 1);
        emit(&ctx.db, "contact", "created", Uuid::new_v4(), Utc::now()).await;
        let report = run(&ctx.db, &recorder, Utc::now() + Duration::days(1))
            .await
            .unwrap();
        assert_eq!((report.deliveries_created, report.attempted), (0, 0));
        let rows = log!(request, pid).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["status"], "cancelled");
        request
            .delete(&format!("/api/webhook-subscriptions/{}", Uuid::new_v4()))
            .await
            .assert_status(StatusCode::NOT_FOUND);
        let audits: Value = request.get(&format!("/api/audits/{pid}")).await.json();
        let actions: Vec<&str> = audits
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| a["action"].as_str())
            .collect();
        assert!(
            actions.contains(&"webhook_subscription_created")
                && actions.contains(&"webhook_subscription_removed")
        );
        assert!(
            !audits.to_string().contains("hooks.example.com"),
            "the address is not audited"
        );
    })
    .await;
}

#[tokio::test]
async fn the_real_sender_refuses_an_address_inside_the_network_before_connecting() {
    for url in [
        "https://127.0.0.1/hook",
        "https://169.254.169.254/latest/meta-data",
        "https://[::1]/hook",
        "https://localhost/hook",
        "http://hooks.example.com/",
    ] {
        let outcome = HttpSender.post(url, &[], "{}".into()).await;
        assert!(matches!(outcome, Outcome::Failed(_)), "{url}: {outcome:?}");
    }
}

#[tokio::test]
async fn a_name_that_resolves_to_a_private_address_is_refused() {
    // `localhost` is refused by name; a name the resolver maps to loopback must be refused by
    // its answer. Use the system resolver's own mapping of a numeric host.
    let refused =
        contact_relationship_management::webhooks::public_addresses("127.0.0.1", 443).await;
    assert!(refused.is_err());
}
