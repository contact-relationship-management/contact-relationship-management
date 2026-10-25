//! Email templates with merge tokens (CRM-R36): the closed token set, the unsubscribe link every
//! body must carry, the synthetic preview, and campaigns and nurture steps that use a template.

use contact_relationship_management::app::App;
use contact_relationship_management::mail::{self, Recorder, Sender};
use contact_relationship_management::unsubscribe::{self, Unsubscriber};
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

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

fn template(name: &str) -> Value {
    json!({
        "name": name,
        "subject": "News for {{given_name}}",
        "body": "Hi {{given_name}} at {{account_name}},\n\nFrom {{rep_name}}.\n\nStop: {{unsubscribe_url}}",
        "fallbacks": { "account_name": "your team" },
    })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_template_is_saved_checked_previewed_and_deleted() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/email-templates")
            .json(&template("Autumn"))
            .await
            .json();
        let pid = created["pid"].as_str().unwrap().to_string();

        // A body without the unsubscribe link, an unknown token and an unclosed one are refused.
        for (field, body) in [
            ("body", "Buy now"),
            ("body", "Hi {{nickname}} {{unsubscribe_url}}"),
            ("body", "Hi {{given_name {{unsubscribe_url}}"),
        ] {
            let mut bad = template("Other");
            bad["body"] = json!(body);
            let response = request.post("/api/email-templates").json(&bad).await;
            response.assert_status_unprocessable_entity();
            assert!(response.text().contains(field), "{}", response.text());
        }
        // The subject cannot carry the link, and a fallback must be for a real token.
        let mut bad = template("Other");
        bad["subject"] = json!("{{unsubscribe_url}}");
        bad["fallbacks"] = json!({ "unsubscribe_url": "x", "colour": "red" });
        let response = request.post("/api/email-templates").json(&bad).await;
        response.assert_status_unprocessable_entity();
        let text = response.text();
        assert!(text.contains("fallbacks"), "{text}");

        // The name is unique whatever its case.
        request
            .post("/api/email-templates")
            .json(&template("AUTUMN"))
            .await
            .assert_status_unprocessable_entity();

        let listed: Vec<Value> = request.get("/api/email-templates").await.json();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["in_use"], 0);
        let one: Value = request
            .get(&format!("/api/email-templates/{pid}"))
            .await
            .json();
        assert_eq!(one["fallbacks"]["account_name"], "your team");

        let tokens: Vec<Value> = request.get("/api/email-templates/tokens").await.json();
        let names: Vec<&str> = tokens.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"given_name") && names.contains(&"unsubscribe_url"));
        let link = tokens
            .iter()
            .find(|t| t["name"] == "unsubscribe_url")
            .unwrap();
        assert_eq!(link["required"], true);
        assert!(link["fallback"].is_null());

        // A preview uses a made-up contact, escapes, and shows the fallbacks when asked.
        let draft = json!({
            "subject": "Hi {{given_name}}",
            "body": "<b>{{account_name}}</b> by {{rep_name}}\n{{unsubscribe_url}}",
            "fallbacks": { "account_name": "<your team>" },
            "blank": ["account_name"],
        });
        let preview: Value = request
            .post("/api/email-templates/preview")
            .json(&draft)
            .await
            .json();
        assert_eq!(preview["subject"], "Hi Alex");
        assert!(
            preview["text"]
                .as_str()
                .unwrap()
                .starts_with("<b><your team></b> by Sam Rep")
        );
        let html = preview["html"].as_str().unwrap();
        assert!(
            html.starts_with("&lt;b&gt;&lt;your team&gt;&lt;/b&gt; by Sam Rep<br>"),
            "{html}"
        );
        assert_eq!(preview["fell_back"], json!(["account_name"]));
        assert_eq!(preview["problems"], json!([]));
        // A draft still missing its link previews, and says what would stop it being saved.
        let mut open = draft.clone();
        open["body"] = json!("No link yet");
        let preview: Value = request
            .post("/api/email-templates/preview")
            .json(&open)
            .await
            .json();
        assert!(
            preview["problems"][0]
                .as_str()
                .unwrap()
                .contains("unsubscribe_url")
        );
        // One that cannot be parsed has nothing to show.
        open["body"] = json!("{{nickname}}");
        request
            .post("/api/email-templates/preview")
            .json(&open)
            .await
            .assert_status_unprocessable_entity();

        // Replace, then delete (nothing uses it yet).
        let mut next = template("Autumn");
        next["subject"] = json!("Autumn, {{name}}");
        request
            .put(&format!("/api/email-templates/{pid}"))
            .json(&next)
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/email-templates/{pid}"))
            .await
            .assert_status_ok();
        request
            .get(&format!("/api/email-templates/{pid}"))
            .await
            .assert_status_not_found();
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_campaign_and_a_nurture_step_are_made_from_a_template() {
    request::<App, _, _>(|request, _ctx| async move {
        let (recorder, _restore) = recording();
        let ada = seed_contact!(&request, "Ada Lovelace").await;
        let detail: Value = request.get(&format!("/api/contacts/{ada}")).await.json();
        let person_ref = detail["contact"]["person_ref"].as_str().unwrap().to_string();
        contact_relationship_management::clients::prime_email(&person_ref, "ada@example.test");

        let made: Value = request
            .post("/api/email-templates")
            .json(&template("Autumn"))
            .await
            .json();
        let tpl = made["pid"].as_str().unwrap().to_string();

        // A template and its own subject or body together are refused, as is a missing template.
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        for extra in [json!({ "body": "x" }), json!({ "subject": "x" })] {
            let mut both = json!({ "name": "C", "segment_pid": segment["pid"], "template_pid": tpl });
            both.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            request
                .post("/api/campaigns")
                .json(&both)
                .await
                .assert_status_unprocessable_entity();
        }
        request
            .post("/api/campaigns")
            .json(&json!({ "name": "C", "template_pid": uuid::Uuid::new_v4() }))
            .await
            .assert_status_unprocessable_entity();

        let campaign: Value = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Autumn news", "segment_pid": segment["pid"], "template_pid": tpl }))
            .await
            .json();
        let campaign = campaign["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/campaigns/{campaign}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .assert_status_ok();

        let sent = recorder.sent();
        assert_eq!(sent.len(), 1);
        let message = &sent[0];
        assert_eq!(message.subject, "News for Ada");
        // No account or owner on this contact, so those fall back (the template's own, then the default).
        assert!(message.text.starts_with("Hi Ada at your team,\n\nFrom the team."), "{}", message.text);
        let link = message.unsubscribe_url.as_deref().expect("a link");
        assert_eq!(message.text.matches(link).count(), 1, "the template's link, no extra footer");
        assert!(!message.text.contains("To stop receiving"));

        // A template in use cannot be deleted.
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Welcome", "steps": [{ "delay_hours": 0, "template_pid": tpl }] }))
            .await
            .json();
        let sequence = sequence["pid"].as_str().unwrap().to_string();
        let listed: Vec<Value> = request.get("/api/email-templates").await.json();
        assert_eq!(listed[0]["in_use"], 2);
        request
            .delete(&format!("/api/email-templates/{tpl}"))
            .await
            .assert_status(axum::http::StatusCode::CONFLICT);

        // The step is mailed from the template too, and a later edit reaches it.
        request
            .post(&format!("/api/nurture-sequences/{sequence}/enrollments"))
            .json(&json!({ "contact_pid": ada }))
            .await
            .assert_status_ok();
        let mut edited = template("Autumn");
        edited["subject"] = json!("Step for {{name}}");
        request
            .put(&format!("/api/email-templates/{tpl}"))
            .json(&edited)
            .await
            .assert_status_ok();
        recorder.clear();
        let swept: Value = request.post("/api/nurture/advance").await.json();
        assert_eq!(swept["sent"], 1, "{swept}");
        assert_eq!(recorder.sent()[0].subject, "Step for Ada Lovelace");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_family_name_fills_its_token_is_never_guessed_and_is_erased() {
    request::<App, _, _>(|request, ctx| async move {
        use sea_orm::ConnectionTrait;
        let (recorder, _restore) = recording();
        let mut pids = Vec::new();
        for (name, family) in [("Ada Lovelace", Some("Lovelace")), ("Grace Hopper", None)] {
            let mut body = json!({ "person_ref": super::a_person(), "display_name": name });
            if family.is_some() {
                body["given_name"] = json!("Addie");
            }
            if let Some(family) = family {
                body["family_name"] = json!(family);
            }
            let created: Value = request.post("/api/contacts").json(&body).await.json();
            let pid = created["pid"].as_str().unwrap().to_string();
            request
                .post(&format!("/api/contacts/{pid}/consent"))
                .json(&json!({ "action": "granted", "source": "web form" }))
                .await
                .assert_status_ok();
            let detail: Value = request.get(&format!("/api/contacts/{pid}")).await.json();
            assert_eq!(detail["contact"]["family_name"], json!(family));
            let person_ref = detail["contact"]["person_ref"].as_str().unwrap();
            contact_relationship_management::clients::prime_email(
                person_ref,
                &format!(
                    "{}@example.test",
                    name.split(' ').next().unwrap().to_lowercase()
                ),
            );
            pids.push(pid);
        }
        let made: Value = request
            .post("/api/email-templates")
            .json(
                &json!({ "name": "Formal", "subject": "Dear {{given_name}} {{family_name}}",
                           "body": "Dear {{family_name}}\n{{unsubscribe_url}}" }),
            )
            .await
            .json();
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let campaign: Value = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Formal", "segment_pid": segment["pid"],
                           "template_pid": made["pid"] }))
            .await
            .json();
        let campaign = campaign["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/campaigns/{campaign}/status"))
            .json(&json!({ "to": "scheduled" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/campaigns/{campaign}/run"))
            .await
            .assert_status_ok();
        let mut subjects: Vec<String> = recorder.sent().iter().map(|m| m.subject.clone()).collect();
        subjects.sort();
        // Ada's given name on file wins over the first word of her display name. Grace has none
        // recorded: her given name is the first word, and her family name is not guessed.
        assert_eq!(subjects, ["Dear Addie Lovelace", "Dear Grace customer"]);

        // Erasure clears it.
        request
            .post(&format!("/api/contacts/{}/erase", pids[0]))
            .await
            .assert_status_ok();
        let row = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT given_name, family_name FROM contacts WHERE pid = '{}'",
                    pids[0]
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.try_get::<Option<String>>("", "given_name").unwrap(),
            None
        );
        assert_eq!(
            row.try_get::<Option<String>>("", "family_name").unwrap(),
            None
        );
    })
    .await;
}
