//! The real send path (CRM-T39): per-recipient delivery rows, consent re-checked
//! at send time, retries that never mail a contact twice, and the sender seam.
//!
//! The sender is swapped for a recorder, so no network is used. A separate
//! ignored test in `tests/smtp_live.rs` talks to a real server.

use contact_relationship_management_service::app::App;
use contact_relationship_management_service::mail::{self, Recorder, Sender};
use contact_relationship_management_service::models::deliveries;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use super::seed_contact;

/// Puts the simulated sender back when a test ends, even by panic.
struct RestoreSender;

impl Drop for RestoreSender {
    fn drop(&mut self) {
        mail::set(Sender::Simulated);
    }
}

fn recording() -> (Recorder, RestoreSender) {
    let recorder = Recorder::default();
    mail::set(Sender::Recording(recorder.clone()));
    (recorder, RestoreSender)
}

/// Give a contact a mailbox and return `(person_ref, address)`.
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
        contact_relationship_management_service::clients::prime_email(&person_ref, $address);
        person_ref
    }};
}

/// Create an everyone-segment and a scheduled campaign over it. Expands to the
/// campaign pid.
macro_rules! scheduled_campaign {
    ($request:expr, $body:expr) => {{
        let segment: Value = $request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let mut payload = json!({
            "name": "Autumn news",
            "segment_pid": segment["pid"],
            "subject": "Autumn news for you",
        });
        if let Some(body) = $body {
            payload["body"] = json!(body);
        }
        let campaign: Value = $request.post("/api/campaigns").json(&payload).await.json();
        let pid = campaign["pid"].as_str().unwrap().to_string();
        $request
            .post(&format!("/api/campaigns/{pid}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status_ok();
        pid
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_real_send_mails_each_consented_contact_once_and_records_it() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        // Never consented: must not be mailed even though it has an address.
        let alan: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": super::a_person(), "display_name": "Alan Turing" }))
            .await
            .json();
        give_address!(&request, ada, "ada@example.test");
        give_address!(&request, grace, "grace@example.test");
        give_address!(&request, alan["pid"].as_str().unwrap(), "alan@example.test");

        let campaign = scheduled_campaign!(&request, Some("Hello from the CRM."));
        let run = request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await;
        run.assert_status_ok();
        let row: Value = run.json();
        assert_eq!(row["status"], "completed", "{row}");
        assert_eq!(row["recipients"], 2);
        // SMTP acceptance is not delivery: a real sender leaves these at zero.
        assert_eq!(row["delivered"], 0);

        let mut recipients: Vec<String> = recorder.sent().iter().map(|m| m.to.clone()).collect();
        recipients.sort();
        assert_eq!(recipients, ["ada@example.test", "grace@example.test"]);
        let message = &recorder.sent()[0];
        assert_eq!(message.subject, "Autumn news for you");
        assert_eq!(message.text, "Hello from the CRM.");
        assert!(message.message_id.starts_with('<') && message.message_id.ends_with('>'));

        let progress: Value = request
            .get(&format!("/api/campaigns/{campaign}/deliveries"))
            .await
            .json();
        assert_eq!(progress["by_state"]["sent"], 2, "{progress}");
        assert_eq!(progress["remaining"], 0);

        // Subject access lists the message, with the id but no address or body.
        let export: Value = request
            .get(&format!("/api/contacts/{ada}/subject-access"))
            .await
            .json();
        let deliveries = export["email_deliveries"].as_array().unwrap();
        assert_eq!(deliveries.len(), 1, "{export}");
        assert_eq!(deliveries[0]["state"], "sent");
        assert!(
            deliveries[0]["message_id"]
                .as_str()
                .unwrap()
                .starts_with('<')
        );
        assert!(!export.to_string().contains("ada@example.test"));

        // A finished campaign cannot be run again.
        assert_eq!(
            request
                .post(&format!("/api/campaigns/{campaign}/run"))
                .await
                .status_code(),
            422
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_failed_send_is_retried_and_nobody_is_mailed_twice() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        give_address!(&request, ada, "ada@example.test");
        give_address!(&request, grace, "grace@example.test");
        recorder.fail_for("ada@example.test");

        let campaign = scheduled_campaign!(&request, Some("Hello"));
        let first: Value = request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .json();
        assert_eq!(
            first["status"], "running",
            "an unfinished send stays open: {first}"
        );
        assert_eq!(first["recipients"], 1);
        let progress: Value = request
            .get(&format!("/api/campaigns/{campaign}/deliveries"))
            .await
            .json();
        assert_eq!(progress["by_state"]["sent"], 1);
        assert_eq!(progress["by_state"]["failed"], 1);
        assert_eq!(progress["remaining"], 1);

        recorder.heal("ada@example.test");
        let second: Value = request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .json();
        assert_eq!(second["status"], "completed", "{second}");
        assert_eq!(second["recipients"], 2);
        // Grace was mailed once, in the first run only.
        let mut to: Vec<String> = recorder.sent().iter().map(|m| m.to.clone()).collect();
        to.sort();
        assert_eq!(to, ["ada@example.test", "grace@example.test"]);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn consent_withdrawn_between_runs_is_honoured() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        give_address!(&request, ada, "ada@example.test");
        give_address!(&request, grace, "grace@example.test");
        recorder.fail_for("ada@example.test");
        let campaign = scheduled_campaign!(&request, Some("Hello"));
        request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .assert_status_ok();

        // Ada's send failed; before the retry she withdraws consent.
        request
            .post(&format!("/api/contacts/{ada}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "web form" }))
            .await
            .assert_status_ok();
        recorder.heal("ada@example.test");
        let second: Value = request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .json();
        assert_eq!(second["status"], "completed", "{second}");
        assert!(
            recorder.sent().iter().all(|m| m.to != "ada@example.test"),
            "a contact who withdrew was mailed"
        );
        let progress: Value = request
            .get(&format!("/api/campaigns/{campaign}/deliveries"))
            .await
            .json();
        assert_eq!(progress["by_state"]["suppressed"], 1, "{progress}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_contact_with_no_address_is_recorded_not_mailed() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let _ = ada; // consented, but no address is primed for her
        let campaign = scheduled_campaign!(&request, Some("Hello"));
        let row: Value = request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .json();
        assert!(recorder.sent().is_empty());
        assert_eq!(row["recipients"], 0);
        let progress: Value = request
            .get(&format!("/api/campaigns/{campaign}/deliveries"))
            .await
            .json();
        assert_eq!(progress["by_state"]["no_address"], 1, "{progress}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_real_sender_needs_a_body_but_the_simulated_one_does_not() {
    request::<App, _, _>(|request, _ctx| async move {
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        give_address!(&request, ada, "ada@example.test");
        let campaign = scheduled_campaign!(&request, None::<&str>);
        {
            let (recorder, _restore) = recording();
            let response = request
                .post(&format!("/api/campaigns/{campaign}/run"))
                .await;
            assert_eq!(response.status_code(), 422);
            assert!(recorder.sent().is_empty());
        }
        // Back to the simulated sender: no body needed, nothing leaves.
        let row: Value = request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .json();
        assert_eq!(row["status"], "completed");
        assert_eq!(
            row["delivered"], 1,
            "simulated engagement counters are kept"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn two_runs_claim_different_rows() {
    request::<App, _, _>(|request, ctx| async move {
        let campaign = Uuid::new_v4();
        let contacts: Vec<Uuid> = (0..20).map(|_| Uuid::new_v4()).collect();
        deliveries::ensure_rows(&ctx.db, campaign, &contacts)
            .await
            .unwrap();
        // Asking again adds nothing.
        deliveries::ensure_rows(&ctx.db, campaign, &contacts)
            .await
            .unwrap();
        let (a, b) = tokio::join!(
            deliveries::claim(&ctx.db, campaign, 20),
            deliveries::claim(&ctx.db, campaign, 20),
        );
        let (a, b) = (a.unwrap(), b.unwrap());
        let mut all: Vec<i32> = a.iter().chain(b.iter()).map(|c| c.id).collect();
        let claimed = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), claimed, "a row was claimed twice");
        assert_eq!(claimed, 20, "every row is claimed exactly once");
        // Everything is now `sending`, so a third claim finds nothing.
        assert!(
            deliveries::claim(&ctx.db, campaign, 20)
                .await
                .unwrap()
                .is_empty()
        );
        let _ = request;
    })
    .await;
}
