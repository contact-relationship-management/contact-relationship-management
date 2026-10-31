//! Web form lead capture (CRM-R31): the origin check, the limits, the honeypot, double opt-in
//! before any mail, the nurture enrolment on confirmation, and the audit rows.

use axum::http::StatusCode;
use contact_relationship_management::app::App;
use contact_relationship_management::mail::{self, Recorder, Sender};
use contact_relationship_management::settings::{self, Settings, WebForm};
use contact_relationship_management::unsubscribe::{self, Unsubscriber};
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

const SECRET: &str = "0123456789abcdef0123456789abcdef";
const BASE: &str = "https://crm.example.org";
const ORIGIN: &str = "https://www.example.org";

/// Puts the process-wide settings and the sender back when a test ends, even by panic.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        settings::install(Settings::default());
        mail::set(Sender::Simulated);
        unsubscribe::install(None);
    }
}

fn arrange(forms: Vec<WebForm>) -> (Recorder, Restore) {
    let recorder = Recorder::default();
    mail::set(Sender::Recording(recorder.clone()));
    unsubscribe::install(Some(Unsubscriber::new(SECRET, BASE, true).unwrap()));
    settings::update(|s| {
        s.web_forms = forms;
        s.trust_forwarded_for = true;
    });
    (recorder, Restore)
}

fn form(slug: &str, purpose: &str, sequence: Option<uuid::Uuid>) -> WebForm {
    WebForm {
        slug: slug.to_string(),
        origins: vec![ORIGIN.to_string()],
        purpose: purpose.to_string(),
        sequence,
    }
}

fn path_of(message_text: &str) -> String {
    let start = message_text.find(BASE).expect("a link in the message");
    let link: String = message_text[start..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    link.strip_prefix(BASE).unwrap().to_string()
}

macro_rules! submit {
    ($request:expr, $slug:expr, $client:expr, $body:expr) => {
        $request
            .post(&format!("/forms/{}", $slug))
            .add_header("origin", ORIGIN)
            .add_header("x-forwarded-for", $client)
            .json(&$body)
            .await
    };
}

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_submission_makes_a_pending_lead_mails_one_confirmation_and_enrols_only_after_it() {
    request::<App, _, _>(|request, ctx| async move {
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({
                "name": "Welcome", "purpose": "newsletter",
                "steps": [{ "delay_hours": 0, "template_ref": "w1",
                           "subject": "Welcome", "body": "Hello there" }],
            }))
            .await
            .json();
        let sequence_pid: uuid::Uuid = sequence["pid"].as_str().unwrap().parse().unwrap();
        let (recorder, _restore) =
            arrange(vec![form("news-signup", "newsletter", Some(sequence_pid))]);
        let before = count(&ctx.db, "SELECT count(*) AS n FROM leads").await;

        let answer = submit!(
            request,
            "news-signup",
            "203.0.113.1",
            json!({ "name": "Ada Lovelace", "email": "ada@analytical.test" })
        );
        assert_eq!(answer.status_code(), StatusCode::ACCEPTED);
        assert_eq!(
            answer
                .header("access-control-allow-origin")
                .to_str()
                .unwrap(),
            ORIGIN
        );
        let first_answer: Value = answer.json();

        // A web lead, scored, linked to a contact, with no address stored anywhere in the CRM.
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads").await,
            before + 1
        );
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM leads WHERE source = 'web' AND email IS NULL \
                 AND email_domain = 'analytical.test' AND contact_pid IS NOT NULL \
                 AND display_name = 'Ada Lovelace'"
            )
            .await,
            1
        );
        let contact_pid: String = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT contact_pid::text AS p FROM leads WHERE source = 'web' LIMIT 1",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "p")
            .unwrap();
        let states: Value = request
            .get(&format!("/api/contacts/{contact_pid}/consents"))
            .await
            .json();
        let newsletter = states["purposes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["purpose"] == "newsletter")
            .unwrap();
        assert_eq!(newsletter["state"], "pending");

        // One mail, the confirmation, to the address in the request; it is not marketing.
        let sent = recorder.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "ada@analytical.test");
        assert!(sent[0].unsubscribe_url.is_none());
        let confirm = path_of(&sent[0].text);
        assert!(confirm.starts_with("/confirm/"));

        // No nurture mail and no enrolment before the visitor confirms.
        let advanced: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(advanced["sent"], 0);
        assert_eq!(recorder.sent().len(), 1);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM nurture_enrollments").await,
            0
        );

        // Opening the link changes nothing; pressing the button grants and enrols.
        request.get(&confirm).await.assert_status_ok();
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM nurture_enrollments").await,
            0
        );
        request.post(&confirm).await.assert_status_ok();
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM nurture_enrollments WHERE status = 'active'"
            )
            .await,
            1
        );
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM form_submissions WHERE confirmed_at IS NOT NULL"
            )
            .await,
            1
        );
        let advanced: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(advanced["sent"], 1);
        assert_eq!(recorder.sent().len(), 2);
        assert_eq!(recorder.sent()[1].to, "ada@analytical.test");

        // The same visitor again: the same answer, no second lead, no second confirmation (they
        // are already confirmed), and the granted purpose is untouched.
        let again = submit!(
            request,
            "news-signup",
            "203.0.113.1",
            json!({ "name": "Ada Lovelace", "email": "ADA@analytical.test" })
        );
        assert_eq!(again.status_code(), StatusCode::ACCEPTED);
        assert_eq!(again.json::<Value>(), first_answer);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM leads").await,
            before + 1
        );
        assert_eq!(recorder.sent().len(), 2);
        let states: Value = request
            .get(&format!("/api/contacts/{contact_pid}/consents"))
            .await
            .json();
        assert!(
            states["purposes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| { p["purpose"] == "newsletter" && p["state"] == "granted" })
        );

        // The audit row names the form and purpose and holds neither name nor address.
        let audit = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT actor, snapshot::text AS d FROM audit_logs \
                 WHERE action = 'web_form_submitted' ORDER BY id LIMIT 1",
            ))
            .await
            .unwrap()
            .expect("an audit row");
        let detail: String = audit.try_get("", "d").unwrap();
        assert!(detail.contains("news-signup") && detail.contains("newsletter"));
        assert!(!detail.contains("Ada") && !detail.contains("analytical"));
        let actor: String = audit.try_get("", "actor").unwrap();
        assert_eq!(actor, "web_form:news-signup");
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM audit_logs WHERE snapshot::text ILIKE '%analytical.test%'"
            )
            .await,
            0
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_origin_the_form_the_honeypot_and_the_input_are_checked() {
    request::<App, _, _>(|request, ctx| async move {
        let (recorder, _restore) = arrange(vec![form("contact-us", "marketing", None)]);
        let contacts_before = count(&ctx.db, "SELECT count(*) AS n FROM contacts").await;
        let body = json!({ "name": "Grace", "email": "grace@navy.test" });

        // No such form.
        assert_eq!(
            submit!(request, "nope", "198.51.100.1", body).status_code(),
            StatusCode::NOT_FOUND
        );
        // Another website, and no origin at all.
        let foreign = request
            .post("/forms/contact-us")
            .add_header("origin", "https://evil.example")
            .add_header("x-forwarded-for", "198.51.100.1")
            .json(&body)
            .await;
        assert_eq!(foreign.status_code(), StatusCode::FORBIDDEN);
        assert!(
            foreign
                .maybe_header("access-control-allow-origin")
                .is_none()
        );
        let none = request
            .post("/forms/contact-us")
            .add_header("x-forwarded-for", "198.51.100.1")
            .json(&body)
            .await;
        assert_eq!(none.status_code(), StatusCode::FORBIDDEN);

        // The preflight is answered for the allowed origin only.
        let allowed = request
            .method(axum::http::Method::OPTIONS, "/forms/contact-us")
            .add_header("origin", ORIGIN)
            .await;
        assert_eq!(allowed.status_code(), StatusCode::NO_CONTENT);
        assert_eq!(
            allowed
                .header("access-control-allow-origin")
                .to_str()
                .unwrap(),
            ORIGIN
        );
        let refused = request
            .method(axum::http::Method::OPTIONS, "/forms/contact-us")
            .add_header("origin", "https://evil.example")
            .await;
        assert_eq!(refused.status_code(), StatusCode::FORBIDDEN);

        // Unusable input is a 422 and creates nothing.
        for bad in [
            json!({ "name": "G", "email": "not-an-address" }),
            json!({ "name": "G\nBcc: x@y.z", "email": "g@navy.test" }),
            json!({ "name": " ", "email": "g@navy.test" }),
        ] {
            let answer = submit!(request, "contact-us", "198.51.100.2", bad);
            assert_eq!(answer.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }

        // A filled honeypot gets the normal answer and creates and sends nothing, even with an
        // address that would otherwise be refused.
        let trap = submit!(
            request,
            "contact-us",
            "198.51.100.3",
            json!({ "name": "Bot", "email": "bot@spam.test", "website": "http://spam.test" })
        );
        assert_eq!(trap.status_code(), StatusCode::ACCEPTED);
        let real = submit!(request, "contact-us", "198.51.100.4", body);
        assert_eq!(real.status_code(), StatusCode::ACCEPTED);
        assert_eq!(trap.json::<Value>(), real.json::<Value>());
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contacts").await,
            contacts_before + 1,
            "only the real submission made a contact"
        );
        assert_eq!(recorder.sent().len(), 1);
        assert_eq!(recorder.sent()[0].to, "grace@navy.test");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_client_and_a_form_are_limited_and_a_stranger_is_not_mailed_twice() {
    request::<App, _, _>(|request, ctx| async move {
        let (recorder, _restore) = arrange(vec![
            form("limited", "marketing", None),
            form("busy", "marketing", None),
        ]);
        settings::update(|s| {
            s.web_form_client_limit = 3;
            s.web_form_limit = 5;
        });
        // One client: three in, the fourth is told when to come back.
        for n in 0..3 {
            let answer = submit!(
                request,
                "limited",
                "192.0.2.10",
                json!({ "name": "Hal", "email": format!("hal{n}@example.test") })
            );
            assert_eq!(answer.status_code(), StatusCode::ACCEPTED, "{n}");
        }
        let fourth = submit!(
            request,
            "limited",
            "192.0.2.10",
            json!({ "name": "Hal", "email": "hal3@example.test" })
        );
        assert_eq!(fourth.status_code(), StatusCode::TOO_MANY_REQUESTS);
        assert!(
            fourth
                .header("retry-after")
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                >= 1
        );
        assert_eq!(
            fourth
                .header("access-control-allow-origin")
                .to_str()
                .unwrap(),
            ORIGIN,
            "the page can read the refusal"
        );
        // Another client is not affected, and another form is counted on its own.
        assert_eq!(
            submit!(
                request,
                "limited",
                "192.0.2.11",
                json!({ "name": "Hal", "email": "other@example.test" })
            )
            .status_code(),
            StatusCode::ACCEPTED
        );
        assert_eq!(
            submit!(
                request,
                "busy",
                "192.0.2.10",
                json!({ "name": "Hal", "email": "hal9@example.test" })
            )
            .status_code(),
            StatusCode::ACCEPTED
        );
        // The whole form: five an hour however many clients (four were counted above).
        assert_eq!(
            submit!(
                request,
                "limited",
                "192.0.2.12",
                json!({ "name": "Hal", "email": "fifth@example.test" })
            )
            .status_code(),
            StatusCode::ACCEPTED
        );
        assert_eq!(
            submit!(
                request,
                "limited",
                "192.0.2.13",
                json!({ "name": "Hal", "email": "sixth@example.test" })
            )
            .status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );

        // Someone else's address cannot be flooded: a second submission for the same address
        // within the hour creates a second submission record but no second mail.
        settings::update(|s| s.web_form_client_limit = 50);
        let sent_before = recorder.sent().len();
        for client in ["192.0.2.20", "192.0.2.21"] {
            let _ = submit!(
                request,
                "busy",
                client,
                json!({ "name": "Victim", "email": "victim@example.test" })
            );
        }
        let to_victim = recorder
            .sent()
            .iter()
            .skip(sent_before)
            .filter(|m| m.to == "victim@example.test")
            .count();
        assert_eq!(to_victim, 1);
        assert_eq!(
            count(
                &ctx.db,
                "SELECT count(*) AS n FROM leads WHERE source = 'web' AND display_name = 'Victim'"
            )
            .await,
            1,
            "one open web lead per contact"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unreachable_person_service_creates_nothing_and_erasure_removes_the_submissions() {
    request::<App, _, _>(|request, ctx| async move {
        let (recorder, _restore) = arrange(vec![form("signup", "marketing", None)]);
        settings::update(|s| {
            s.upstream_mode = contact_relationship_management::clients::Mode::Http
        });
        let contacts_before = count(&ctx.db, "SELECT count(*) AS n FROM contacts").await;
        let answer = submit!(
            request,
            "signup",
            "192.0.2.30",
            json!({ "name": "Alan", "email": "alan@bletchley.test" })
        );
        assert_eq!(answer.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            count(&ctx.db, "SELECT count(*) AS n FROM contacts").await,
            contacts_before
        );
        assert!(recorder.sent().is_empty());

        // Back to the stub: a submission, then erasure takes the record of it away.
        settings::update(|s| {
            s.upstream_mode = contact_relationship_management::clients::Mode::Stub
        });
        assert_eq!(
            submit!(
                request,
                "signup",
                "192.0.2.31",
                json!({ "name": "Alan", "email": "alan@bletchley.test" })
            )
            .status_code(),
            StatusCode::ACCEPTED
        );
        let contact_pid: String = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT contact_pid::text AS p FROM form_submissions ORDER BY id DESC LIMIT 1",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "p")
            .unwrap();
        request
            .post(&format!("/api/contacts/{contact_pid}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "test" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/contacts/{contact_pid}/erase"))
            .await
            .assert_status_ok();
        assert_eq!(
            count(
                &ctx.db,
                &format!(
                    "SELECT count(*) AS n FROM form_submissions WHERE contact_pid = '{contact_pid}'"
                )
            )
            .await,
            0
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_confirmation_mail_is_in_the_language_of_the_request_that_asked_for_it() {
    // CRM-T180: the form page's `language`, else the request's Accept-Language, else English;
    // the link opens the confirmation page in the same language; nothing about it is stored.
    request::<App, _, _>(|request, ctx| async move {
        let (recorder, _restore) = arrange(vec![form("lang-signup", "newsletter", None)]);

        // The browser asks for French: the answer and the mail are French.
        let answer = request
            .post("/forms/lang-signup")
            .add_header("origin", ORIGIN)
            .add_header("x-forwarded-for", "192.0.2.10")
            .add_header("accept-language", "fr-CA, en;q=0.5")
            .json(&json!({ "name": "Marie Curie", "email": "marie@radium.test" }))
            .await;
        assert_eq!(answer.status_code(), StatusCode::ACCEPTED);
        assert_eq!(answer.header("content-language").to_str().unwrap(), "fr");
        assert_eq!(answer.header("vary").to_str().unwrap(), "accept-language");
        let french: Value = answer.json();
        let words = contact_relationship_management::rules::public_text::text_for("fr").unwrap();
        assert_eq!(french["message"], words.form_received);
        assert_ne!(
            words.form_received,
            contact_relationship_management::rules::public_text::english().form_received
        );
        let sent = recorder.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject, "Veuillez confirmer votre abonnement");
        assert!(
            sent[0].text.starts_with("Bonjour Marie Curie,"),
            "{}",
            sent[0].text
        );
        assert!(sent[0].text.contains("La lettre d"), "{}", sent[0].text);
        let confirm = path_of(&sent[0].text);
        assert!(confirm.ends_with("?lang=fr"), "{confirm}");
        // The page the link opens is French, whatever the browser that opens it asks for.
        let page = request
            .get(&confirm)
            .add_header("accept-language", "de")
            .await;
        page.assert_status_ok();
        assert!(page.text().contains("<html lang=\"fr\""), "{}", page.text());

        // The form page names its language: it wins over the browser's.
        let answer = request
            .post("/forms/lang-signup")
            .add_header("origin", ORIGIN)
            .add_header("x-forwarded-for", "192.0.2.11")
            .add_header("accept-language", "de")
            .json(&json!({ "name": "Ibn Sina", "email": "ibn@sina.test", "language": "ar" }))
            .await;
        assert_eq!(answer.header("content-language").to_str().unwrap(), "ar");
        let sent = recorder.sent();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[1].subject, "يرجى تأكيد اشتراكك");
        assert!(path_of(&sent[1].text).ends_with("?lang=ar"));

        // A language the pages lack, and no header: English.
        let answer = request
            .post("/forms/lang-signup")
            .add_header("origin", ORIGIN)
            .add_header("x-forwarded-for", "192.0.2.12")
            .json(&json!({ "name": "Sei Shonagon", "email": "sei@heian.test", "language": "ja" }))
            .await;
        assert_eq!(answer.header("content-language").to_str().unwrap(), "en");
        let sent = recorder.sent();
        assert_eq!(sent.len(), 3);
        assert_eq!(sent[2].subject, "Please confirm your subscription");
        assert!(
            sent[2].text.contains("receive: The newsletter."),
            "{}",
            sent[2].text
        );
        assert!(path_of(&sent[2].text).ends_with("?lang=en"));

        // No language is kept: no column of the contact or of the submission holds one.
        let columns = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM information_schema.columns \
                 WHERE table_name IN ('contacts', 'form_submissions', 'link_tokens') \
                 AND column_name ILIKE '%lang%'",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "n")
            .unwrap();
        assert_eq!(columns, 0);
    })
    .await;
}
