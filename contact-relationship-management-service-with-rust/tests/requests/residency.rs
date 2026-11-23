//! Outside calls limited to a declared list (CRM-R106, CRM-T283): a webhook to an undeclared host
//! is refused at registration and, for one made before the list, at each send, with an audit row;
//! the residency read says what is declared and configured.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use contact_relationship_management::db;
use contact_relationship_management::webhooks::{Outcome, Sender, run};
use serde_json::{Value, json};
use serial_test::serial;
use std::sync::Mutex;
use uuid::Uuid;

use crate::scoped_env::ScopedEnv;

const HOSTS: &str = "CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS";

/// Counts the calls the relay makes and answers each `200`.
#[derive(Default)]
struct Counter(Mutex<Vec<String>>);

#[async_trait::async_trait]
impl Sender for Counter {
    async fn post(&self, url: &str, _: &[(&'static str, String)], _: String) -> Outcome {
        self.0.lock().unwrap().push(url.to_string());
        Outcome::Status(200)
    }
}

async fn subscribe(request: &TestServer, url: &str) -> axum_test::TestResponse {
    request
        .post("/api/webhook-subscriptions")
        .json(&json!({ "url": url, "events": ["*"] }))
        .await
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_webhook_to_an_undeclared_host_is_refused_when_registered_and_when_sent() {
    request::<App, _, _>(|request, ctx| async move {
        // Made before there is a list: allowed.
        let early = subscribe(&request, "https://hooks.elsewhere.example/crm").await;
        early.assert_status_ok();
        let early_pid = early.json::<Value>()["pid"].as_str().unwrap().to_string();

        let mut env = ScopedEnv::default();
        env.set(HOSTS, "*.declared.example");

        // Registration: an undeclared host is refused with the reason; a declared one is taken.
        let refused = subscribe(&request, "https://hooks.elsewhere.example/other").await;
        assert_eq!(refused.status_code(), 422);
        assert!(
            refused
                .text()
                .contains("CONTACT_RELATIONSHIP_MANAGEMENT_OUTBOUND_HOSTS"),
            "{}",
            refused.text()
        );
        subscribe(&request, "https://hooks.declared.example/crm")
            .await
            .assert_status_ok();

        // Send: the early subscription's delivery is dead and audited; the declared one is sent.
        sqlx::query(
            "INSERT INTO event_outbox (event_id, entity, entity_pid, kind, occurred_at, \
             schema_version, payload) VALUES ($1, 'contact', $2, 'created', $3, 1, '{}')",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        // The service stamps a subscription with the host clock, and a subscription only gets events
        // from its own creation on, so the event is stamped with the host clock too. The database's
        // `now()` can lag the host (a Podman VM does), and the event would then predate the subscription.
        .bind(contact_relationship_management::clock::now())
        .execute(db::pool(&ctx))
        .await
        .unwrap();
        let counter = Counter::default();
        let report = run(
            db::pool(&ctx),
            &counter,
            contact_relationship_management::clock::now(),
        )
        .await
        .unwrap();
        let calls = counter.0.lock().unwrap().clone();
        assert_eq!(calls, ["https://hooks.declared.example/crm"], "{report:?}");
        assert_eq!((report.delivered, report.dead), (1, 1), "{report:?}");

        let rows: Vec<Value> = request
            .get(&format!(
                "/api/webhook-subscriptions/{early_pid}/deliveries"
            ))
            .await
            .json();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["status"], "dead");
        assert!(
            rows[0]["last_error"]
                .as_str()
                .unwrap_or("")
                .contains("hooks.elsewhere.example"),
            "{rows:?}"
        );
        let audits: Value = request
            .get(&format!("/api/audits/{early_pid}"))
            .await
            .json();
        assert!(
            audits
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["action"] == "webhook_refused_undeclared_host"),
            "{audits}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_residency_read_names_the_region_the_list_and_each_configured_service() {
    request::<App, _, _>(|request, _ctx| async move {
        let unlimited: Value = request.get("/api/operations/residency").await.json();
        assert_eq!(unlimited["limited"], false, "{unlimited}");

        let mut env = ScopedEnv::default();
        env.set("CONTACT_RELATIONSHIP_MANAGEMENT_DATA_REGION", "uk-south");
        env.set(HOSTS, "person.declared.example");
        env.set(
            "CONTACT_RELATIONSHIP_MANAGEMENT_PERSON_SERVICE_URL",
            "https://person.declared.example/api",
        );
        env.set(
            "CONTACT_RELATIONSHIP_MANAGEMENT_PPM_SERVICE_URL",
            "https://ppm.elsewhere.example",
        );
        let read: Value = request.get("/api/operations/residency").await.json();
        assert_eq!(read["region"], "uk-south");
        assert_eq!(read["limited"], true);
        assert_eq!(read["declared_hosts"], json!(["person.declared.example"]));
        let configured = read["configured"].as_array().unwrap();
        let allowed = |setting: &str| {
            configured
                .iter()
                .find(|c| c["setting"] == setting)
                .map(|c| c["allowed"].clone())
        };
        assert_eq!(
            allowed("CONTACT_RELATIONSHIP_MANAGEMENT_PERSON_SERVICE_URL"),
            Some(json!(true))
        );
        assert_eq!(
            allowed("CONTACT_RELATIONSHIP_MANAGEMENT_PPM_SERVICE_URL"),
            Some(json!(false))
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn a_start_is_refused_when_a_configured_service_is_not_declared() {
    use contact_relationship_management::mail::Sender as Mail;
    use contact_relationship_management::residency::check_start;
    let mut env = ScopedEnv::default();
    env.set(
        "CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_URL",
        "https://wpm.elsewhere.example",
    );
    // No list: allowed, with a note.
    assert!(check_start(&Mail::Simulated).unwrap().is_some());
    // A list without the worker service: refused, naming the setting and its host.
    env.set(HOSTS, "person.declared.example");
    let why = check_start(&Mail::Simulated).unwrap_err();
    assert!(
        why.contains("CONTACT_RELATIONSHIP_MANAGEMENT_WORKER_SERVICE_URL (wpm.elsewhere.example)"),
        "{why}"
    );
    // Declared: allowed, with no note.
    env.set(HOSTS, "person.declared.example, *.elsewhere.example");
    assert_eq!(check_start(&Mail::Simulated), Ok(None));
    // A list that does not read is refused too.
    env.set(HOSTS, "https://wpm.elsewhere.example");
    assert!(
        check_start(&Mail::Simulated)
            .unwrap_err()
            .contains("not a host name")
    );
}
