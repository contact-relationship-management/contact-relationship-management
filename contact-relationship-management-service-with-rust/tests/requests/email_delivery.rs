//! The real send path (CRM-T39): per-recipient delivery rows, consent re-checked
//! at send time, retries that never mail a contact twice, and the sender seam.
//!
//! The sender is swapped for a recorder, so no network is used. A separate
//! ignored test in `tests/smtp_live.rs` talks to a real server.

use contact_relationship_management_service::app::App;
use contact_relationship_management_service::mail::{self, Recorder, Sender};
use contact_relationship_management_service::models::deliveries;
use contact_relationship_management_service::unsubscribe::{self, Unsubscriber};
use contact_relationship_management_service::webhook;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use super::seed_contact;

const SECRET: &str = "0123456789abcdef0123456789abcdef";

/// Puts the simulated sender back when a test ends, even by panic.
struct RestoreSender;

impl Drop for RestoreSender {
    fn drop(&mut self) {
        mail::set(Sender::Simulated);
        unsubscribe::install(None);
    }
}

fn recording() -> (Recorder, RestoreSender) {
    let recorder = Recorder::default();
    mail::set(Sender::Recording(recorder.clone()));
    unsubscribe::install(Some(
        Unsubscriber::new(SECRET, "https://crm.example.org", true).unwrap(),
    ));
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
        assert!(message.text.starts_with("Hello from the CRM."));
        let link = message.unsubscribe_url.as_deref().expect("a link");
        assert!(link.starts_with("https://crm.example.org/unsubscribe/"));
        assert!(message.text.contains(link), "the footer shows the link");
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
            .assert_status_success();

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

/// The path of a link under the public URL.
fn link_path(url: &str) -> String {
    url.strip_prefix("https://crm.example.org")
        .expect("under the public url")
        .to_string()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_unsubscribe_link_withdraws_consent_and_stops_the_next_send() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        give_address!(&request, ada, "ada@example.test");
        give_address!(&request, grace, "grace@example.test");
        // Grace's first send fails, so a second run is still to come.
        recorder.fail_for("grace@example.test");
        let campaign = scheduled_campaign!(&request, Some("Hello"));
        request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .assert_status_success();

        // Grace follows no link; Ada's message has one.
        let ada_mail = recorder
            .sent()
            .into_iter()
            .find(|m| m.to == "ada@example.test")
            .unwrap();
        let path = link_path(ada_mail.unsubscribe_url.as_deref().unwrap());

        // Opening the link (a mail scanner would) changes nothing.
        let page = request.get(&path).await;
        page.assert_status_ok();
        assert!(page.text().contains("<form method=\"post\""));
        let before: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        assert_eq!(before["contact"]["marketing_consent"], "granted");

        // The one-click POST withdraws, with no credentials.
        request.post(&path).await.assert_status_ok();
        let after: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        assert_eq!(
            after["contact"]["marketing_consent"], "withdrawn",
            "{after}"
        );
        let history: Value = request
            .get(&format!("/api/contacts/{ada}/consent"))
            .await
            .json();
        let last = history.as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["action"], "withdrawn");
        assert_eq!(last["source"], "unsubscribe-link");

        // A repeat is harmless and records nothing more.
        request.post(&path).await.assert_status_ok();
        let again: Value = request
            .get(&format!("/api/contacts/{ada}/consent"))
            .await
            .json();
        assert_eq!(
            again.as_array().unwrap().len(),
            history.as_array().unwrap().len()
        );

        // A forged or damaged link is refused, and nothing else changes.
        let bad = format!("{}x", &path);
        request.post(&bad).await.assert_status_not_found();
        request
            .get("/unsubscribe/garbage")
            .await
            .assert_status_not_found();
        let grace_now: Value = request.get(&format!("/api/contacts/{grace}")).await.json();
        assert_eq!(grace_now["contact"]["marketing_consent"], "granted");

        // Ada is not mailed again.
        recorder.heal("grace@example.test");
        request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .assert_status_ok();
        let to_ada = recorder
            .sent()
            .iter()
            .filter(|m| m.to == "ada@example.test")
            .count();
        assert_eq!(to_ada, 1, "Ada was mailed after unsubscribing");
    })
    .await;
}

const WEBHOOK_SECRET: &str = "webhook-secret-0123456789abcdef-xyz";

/// Post a signed provider event; expands to the response.
macro_rules! provider_event {
    ($request:expr, $message_id:expr, $event:expr) => {{
        let body = json!({ "message_id": $message_id, "event": $event }).to_string();
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let signature = webhook::sign(WEBHOOK_SECRET, &timestamp, body.as_bytes());
        $request
            .post("/webhooks/email")
            .add_header("x-crm-timestamp", timestamp)
            .add_header("x-crm-signature", signature)
            .add_header("content-type", "application/json")
            .bytes(body.into())
            .await
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn provider_events_count_deliveries_and_suppress_bounces_and_complaints() {
    // SAFETY: serial tests; nothing else reads the environment concurrently.
    unsafe { std::env::set_var("CRM_WEBHOOK_SECRET", WEBHOOK_SECRET) };
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        let alan = seed_contact!(&request, "Alan Turing").await;
        give_address!(&request, ada, "ada@example.test");
        give_address!(&request, grace, "grace@example.test");
        give_address!(&request, alan, "alan@example.test");
        let first = scheduled_campaign!(&request, Some("Hello"));
        request
            .post(&format!("/api/campaigns/{first}/run"))
            .await
            .assert_status_ok();
        let id_of = |to: &str| {
            recorder
                .sent()
                .into_iter()
                .find(|m| m.to == to)
                .unwrap()
                .message_id
        };

        // Unsigned, badly signed and stale requests are refused.
        request
            .post("/webhooks/email")
            .json(&json!({ "message_id": id_of("ada@example.test"), "event": "delivered" }))
            .await
            .assert_status_unauthorized();

        // A delivery is counted once, however often it is reported.
        let ada_id = id_of("ada@example.test");
        let first_report: Value = provider_event!(&request, &ada_id, "delivered").json();
        assert_eq!(first_report["result"], "recorded");
        let repeat: Value = provider_event!(&request, &ada_id, "delivered").json();
        assert_eq!(repeat["result"], "duplicate");
        let campaigns: Value = request.get("/api/campaigns").await.json();
        let row = campaigns
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["pid"] == first)
            .unwrap();
        assert_eq!(row["delivered"], 1, "{row}");

        // A message we never sent is acknowledged and ignored.
        let unknown: Value = provider_event!(&request, "<nope@crm.invalid>", "bounce_hard").json();
        assert_eq!(unknown["result"], "unknown_message");

        // A hard bounce suppresses the mailbox; a complaint also withdraws consent.
        provider_event!(&request, &id_of("grace@example.test"), "bounce_hard").assert_status_ok();
        provider_event!(&request, &id_of("alan@example.test"), "complaint").assert_status_ok();
        let alan_now: Value = request.get(&format!("/api/contacts/{alan}")).await.json();
        assert_eq!(alan_now["contact"]["marketing_consent"], "withdrawn");
        let grace_now: Value = request.get(&format!("/api/contacts/{grace}")).await.json();
        assert_eq!(grace_now["contact"]["marketing_consent"], "granted");

        // A later campaign mails only Ada.
        recorder.clear();
        let second = scheduled_campaign!(&request, Some("Again"));
        request
            .post(&format!("/api/campaigns/{second}/run"))
            .await
            .assert_status_ok();
        let to: Vec<String> = recorder.sent().iter().map(|m| m.to.clone()).collect();
        assert_eq!(to, ["ada@example.test"], "a suppressed mailbox was mailed");
    })
    .await;
    unsafe { std::env::remove_var("CRM_WEBHOOK_SECRET") };
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn brevo_and_mailjet_callbacks_are_authenticated_and_applied() {
    use base64::Engine as _;
    unsafe { std::env::set_var("CRM_WEBHOOK_SECRET", WEBHOOK_SECRET) };
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        give_address!(&request, ada, "ada@example.test");
        give_address!(&request, grace, "grace@example.test");
        let first = scheduled_campaign!(&request, Some("Hello"));
        request
            .post(&format!("/api/campaigns/{first}/run"))
            .await
            .assert_status_ok();
        let bare = |to: &str| {
            recorder
                .sent()
                .into_iter()
                .find(|m| m.to == to)
                .unwrap()
                .message_id
                .trim_matches(|c| c == '<' || c == '>')
                .to_string()
        };

        // No credential, or a wrong one, is refused; an unknown vendor is 404.
        let brevo = json!({ "event": "hard_bounce", "tags": [bare("ada@example.test")] });
        request
            .post("/webhooks/email/brevo")
            .json(&brevo)
            .await
            .assert_status_unauthorized();
        request
            .post("/webhooks/email/brevo")
            .add_header("authorization", "Bearer wrong")
            .json(&brevo)
            .await
            .assert_status_unauthorized();
        request
            .post("/webhooks/email/acme")
            .add_header("authorization", format!("Bearer {WEBHOOK_SECRET}"))
            .json(&brevo)
            .await
            .assert_status_not_found();

        // Brevo, bearer token: a hard bounce suppresses; a repeat changes nothing.
        let ok: Value = request
            .post("/webhooks/email/brevo")
            .add_header("authorization", format!("Bearer {WEBHOOK_SECRET}"))
            .json(&brevo)
            .await
            .json();
        assert_eq!(ok["recorded"], 1, "{ok}");
        let again: Value = request
            .post("/webhooks/email/brevo")
            .add_header("x-crm-webhook-token", WEBHOOK_SECRET)
            .json(&brevo)
            .await
            .json();
        assert_eq!(again["recorded"], 0);
        let state: Value = request
            .get(&format!("/api/contacts/{ada}/email-suppression"))
            .await
            .json();
        assert_eq!(state["suppressed"], true, "{state}");

        // Mailjet, basic auth, an array: a complaint withdraws consent; open is ignored.
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("crm:{WEBHOOK_SECRET}"));
        let batch = json!([
            { "event": "spam", "CustomID": bare("grace@example.test") },
            { "event": "open", "CustomID": bare("grace@example.test") }
        ]);
        let done: Value = request
            .post("/webhooks/email/mailjet")
            .add_header("authorization", format!("Basic {basic}"))
            .json(&batch)
            .await
            .json();
        assert_eq!(done["received"], 1, "{done}");
        assert_eq!(done["recorded"], 1);
        let grace_now: Value = request.get(&format!("/api/contacts/{grace}")).await.json();
        assert_eq!(grace_now["contact"]["marketing_consent"], "withdrawn");
    })
    .await;
    unsafe { std::env::remove_var("CRM_WEBHOOK_SECRET") };
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_suppression_can_be_read_and_lifted_and_the_contact_is_mailed_again() {
    unsafe { std::env::set_var("CRM_WEBHOOK_SECRET", WEBHOOK_SECRET) };
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        give_address!(&request, ada, "ada@example.test");
        let first = scheduled_campaign!(&request, Some("Hello"));
        request
            .post(&format!("/api/campaigns/{first}/run"))
            .await
            .assert_status_ok();
        let id = recorder.sent()[0].message_id.clone();
        provider_event!(&request, &id, "bounce_hard").assert_status_ok();

        let path = format!("/api/contacts/{ada}/email-suppression");
        let state: Value = request.get(&path).await.json();
        assert_eq!(state["suppressed"], true, "{state}");
        assert_eq!(state["reason"], "bounce_hard");

        request.delete(&path).await.assert_status_ok();
        let after: Value = request.get(&path).await.json();
        assert_eq!(after["suppressed"], false);
        request.delete(&path).await.assert_status_not_found();

        recorder.clear();
        let second = scheduled_campaign!(&request, Some("Again"));
        request
            .post(&format!("/api/campaigns/{second}/run"))
            .await
            .assert_status_ok();
        assert_eq!(
            recorder.sent().len(),
            1,
            "a lifted suppression still blocked"
        );

        let trail: Value = request.get(&format!("/api/audits/{ada}")).await.json();
        assert!(
            trail.to_string().contains("email_suppression_cleared"),
            "{trail}"
        );
    })
    .await;
    unsafe { std::env::remove_var("CRM_WEBHOOK_SECRET") };
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn one_run_call_walks_a_long_audience_in_batches_and_a_stall_answers_202() {
    unsafe { std::env::set_var("CRM_MAIL_BATCH", "1") };
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        for (i, name) in ["Ada", "Grace", "Alan"].iter().enumerate() {
            let pid = seed_contact!(&request, *name).await;
            give_address!(&request, pid, &format!("c{i}@example.test"));
        }
        // Batches of one, yet a single call finishes the whole audience.
        let campaign = scheduled_campaign!(&request, Some("Hello"));
        let run = request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await;
        run.assert_status_ok();
        let row: Value = run.json();
        assert_eq!(row["status"], "completed", "{row}");
        assert_eq!(recorder.sent().len(), 3);

        // When every remaining send fails the job stops instead of spinning:
        // the call answers 202 and the campaign stays running for a later run.
        recorder.clear();
        recorder.fail_for("c0@example.test");
        recorder.fail_for("c1@example.test");
        recorder.fail_for("c2@example.test");
        let stalled = scheduled_campaign!(&request, Some("Again"));
        let run = request.post(&format!("/api/campaigns/{stalled}/run")).await;
        assert_eq!(run.status_code().as_u16(), 202);
        let row: Value = run.json();
        assert_eq!(row["status"], "running", "{row}");
        assert!(recorder.sent().is_empty());
    })
    .await;
    unsafe { std::env::remove_var("CRM_MAIL_BATCH") };
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn nurture_steps_are_really_mailed_once_and_a_failure_is_held_for_the_next_sweep() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let grace = seed_contact!(&request, "Grace Hopper").await;
        give_address!(&request, ada, "ada@example.test");
        give_address!(&request, grace, "grace@example.test");

        // With real mail on, a step with no body is refused at creation.
        request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Bad", "steps": [{ "delay_hours": 0, "template_ref": "t" }] }))
            .await
            .assert_status_unprocessable_entity();

        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Welcome", "steps": [
                { "delay_hours": 0, "template_ref": "welcome-1",
                  "subject": "Welcome", "body": "Glad you are here." },
                { "delay_hours": 0, "template_ref": "welcome-2", "body": "Second note." },
            ] }))
            .await
            .json();
        let sequence_pid = sequence["pid"].as_str().unwrap().to_string();
        for contact in [&ada, &grace] {
            request
                .post(&format!(
                    "/api/nurture-sequences/{sequence_pid}/enrollments"
                ))
                .json(&json!({ "contact_pid": contact }))
                .await
                .assert_status_ok();
        }

        // Grace's mailbox is down: Ada is mailed, Grace is held, not skipped.
        recorder.fail_for("grace@example.test");
        let first: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(first["sent"], 1, "{first}");
        assert_eq!(first["held"], 1, "{first}");
        let sent = recorder.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "ada@example.test");
        assert_eq!(sent[0].subject, "Welcome");
        assert!(sent[0].text.starts_with("Glad you are here."));
        assert!(sent[0].unsubscribe_url.is_some());

        // Healed: the next sweep sends Grace step 1 and Ada step 2, each once.
        recorder.heal("grace@example.test");
        let second: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(second["sent"], 2, "{second}");
        let mut got: Vec<(String, String)> = recorder
            .sent()
            .iter()
            .map(|m| (m.to.clone(), m.subject.clone()))
            .collect();
        got.sort();
        assert_eq!(
            got,
            [
                ("ada@example.test".to_string(), "Welcome".to_string()),
                ("ada@example.test".to_string(), "welcome-2".to_string()),
                ("grace@example.test".to_string(), "Welcome".to_string()),
            ]
        );

        // Nothing is mailed twice by a further sweep that has nothing new due
        // beyond Grace's second step.
        let third: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(third["sent"], 1, "{third}");
        assert_eq!(recorder.sent().len(), 4);
        let fourth: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(fourth["sent"], 0, "{fourth}");

        // Subject access lists the nurture messages with the campaign ones.
        let export: Value = request
            .get(&format!("/api/contacts/{ada}/subject-access"))
            .await
            .json();
        assert_eq!(
            export["email_deliveries"].as_array().unwrap().len(),
            2,
            "{export}"
        );
    })
    .await;
}
