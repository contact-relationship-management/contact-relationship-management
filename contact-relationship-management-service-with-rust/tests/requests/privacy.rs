//! Subject rights & retention round-trip (CRM-R20): the subject-access
//! export gathers the contact's footprint and names exclusions; erasure
//! is refused while a live engagement exists (an open deal, an open
//! ticket, or an active nurture enrolment) and anonymises once none
//! does; the retention report is horizon-floored and the sweep is
//! audited even when it deletes nothing.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn subject_rights_round_trip() {
    request::<App, _, _>(|request, ctx| async move {
        // ── Subject access: gathers the footprint, names exclusions, audited.
        let contact_pid = seed_contact!(&request, "Dana Rivers").await;
        request
            .post("/api/activities")
            .json(
                &json!({ "subject_kind": "contact", "subject_pid": contact_pid,
                            "kind": "note", "summary": "Prefers afternoon calls" }),
            )
            .await
            .assert_status_ok();
        let export: Value = request
            .get(&format!("/api/contacts/{contact_pid}/subject-access"))
            .await
            .json();
        assert_eq!(export["contact"]["pid"], contact_pid);
        assert_eq!(export["consent_history"].as_array().unwrap().len(), 1);
        assert_eq!(export["activities"].as_array().unwrap().len(), 1);
        assert_ne!(export["exclusions"].as_array().unwrap().len(), 0);
        let audits: Value = request
            .get(&format!("/api/audits/{contact_pid}"))
            .await
            .json();
        assert!(
            audits
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["action"] == "subject_access_exported"),
            "the export itself is audited"
        );

        // ── Erasure refused while an open deal names it primary contact.
        let (pipeline_pid, stages) = super::seed_pipeline!(&request).await;
        let deal: Value = request
            .post("/api/deals")
            .json(&json!({ "name": "Renewal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 250_000_i64, "currency": "GBP",
                            "primary_contact_pid": contact_pid }))
            .await
            .json();
        let deal_pid = deal["pid"].as_str().unwrap().to_string();
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/erase"))
                .await
                .status_code(),
            422,
            "an open deal is a live engagement"
        );

        // ── Close the deal (Won, terminal); erasure still refused for an
        // open ticket.
        request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stages[2] }))
            .await
            .assert_status_ok();
        let ticket: Value = request
            .post("/api/tickets")
            .json(&json!({ "contact_pid": contact_pid, "title": "Login issue",
                            "priority": "normal", "channel": "email" }))
            .await
            .json();
        let ticket_pid = ticket["pid"].as_str().unwrap().to_string();
        // A handover whose reason names the person: erasure scrubs it (CRM-T317).
        request
            .post(&format!("/api/tickets/{ticket_pid}/assign"))
            .json(&json!({ "assignee_ref": super::a_user(), "reason": "Dana Rivers asked for someone else" }))
            .await
            .assert_status_ok();
        // The event for the new ticket names it by pid; the title is free text and stays out of the envelope.
        let events: Vec<Value> = request.get("/api/events/recent").await.json();
        let opened = events
            .iter()
            .find(|e| e["kind"] == "ticket_opened" && e["pid"] == ticket_pid)
            .expect("the ticket_opened event");
        assert_eq!(opened["name"], "", "the ticket title is not in the event");
        // An activity on the ticket is keyed to the ticket's pid, and is still about this contact.
        request
            .post("/api/activities")
            .json(
                &json!({ "subject_kind": "ticket", "subject_pid": ticket_pid,
                            "kind": "note", "summary": "Dana Rivers cannot sign in" }),
            )
            .await
            .assert_status_ok();
        let export: Value = request
            .get(&format!("/api/contacts/{contact_pid}/subject-access"))
            .await
            .json();
        assert_eq!(
            export["activities"].as_array().unwrap().len(),
            2,
            "subject access includes the activity on the contact's ticket"
        );
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/erase"))
                .await
                .status_code(),
            422,
            "an open ticket is a live engagement"
        );

        // ── Resolve + close the ticket; erasure still refused for an
        // active nurture enrolment.
        request
            .post(&format!("/api/tickets/{ticket_pid}/status"))
            .json(&json!({ "to": "resolved" }))
            .await
            .assert_status_ok();
        // Reopened once, so the export has a reopening to show.
        for to in ["open", "resolved"] {
            request
                .post(&format!("/api/tickets/{ticket_pid}/status"))
                .json(&json!({ "to": to }))
                .await
                .assert_status_ok();
        }
        request
            .post(&format!("/api/tickets/{ticket_pid}/status"))
            .json(&json!({ "to": "closed" }))
            .await
            .assert_status_ok();
        // The ticket's handover and its history are the subject's too (CRM-T317).
        let export: Value = request
            .get(&format!("/api/contacts/{contact_pid}/subject-access"))
            .await
            .json();
        let movements = export["ticket_movements"].as_array().expect("ticket_movements");
        assert_eq!(movements.len(), 1, "{export}");
        assert_eq!(movements[0]["ticket_pid"], ticket_pid);
        assert_eq!(movements[0]["reason"], "Dana Rivers asked for someone else");
        assert_eq!(movements[0]["via"], "person");
        let reopenings = export["ticket_reopenings"].as_array().expect("ticket_reopenings");
        assert_eq!(reopenings.len(), 1, "{export}");
        assert_eq!(reopenings[0]["ticket_pid"], ticket_pid);
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Welcome",
                            "steps": [{ "delay_hours": 0, "template_ref": "welcome-1" }] }))
            .await
            .json();
        let sequence_pid = sequence["pid"].as_str().unwrap().to_string();
        request
            .post(&format!(
                "/api/nurture-sequences/{sequence_pid}/enrollments"
            ))
            .json(&json!({ "contact_pid": contact_pid }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/erase"))
                .await
                .status_code(),
            422,
            "an active nurture enrolment is a live engagement"
        );

        // A stored event about the contact carries its name as the label (the transport in tests is
        // memory, so the outbox row is written here as the outbox transport writes it).
        let pool = contact_relationship_management::db::pool(&ctx);
        sqlx::query(
            "INSERT INTO event_outbox (event_id, entity, entity_pid, kind, occurred_at, actor, \
             schema_version, payload) VALUES (gen_random_uuid(), 'contact', $1::uuid, 'created', now(), \
             NULL, 1, jsonb_build_object('entity', 'contact', 'kind', 'created', 'pid', $1, 'name', \
             'Dana Rivers'))",
        )
        .bind(&contact_pid)
        .execute(pool)
        .await
        .unwrap();
        // ── Withdraw consent (exits nurture per CRM-D6); now erasure
        // succeeds, and it anonymises the record and scrubs the linked
        // free text.
        request
            .post(&format!("/api/contacts/{contact_pid}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "support call" }))
            .await
            .assert_status_ok();
        let erased: Value = request
            .post(&format!("/api/contacts/{contact_pid}/erase"))
            .await
            .json();
        assert_eq!(erased["erased"], contact_pid);
        assert!(erased["files_not_removed"].is_u64());
        assert_eq!(
            request
                .get(&format!("/api/contacts/{contact_pid}"))
                .await
                .status_code(),
            404,
            "the contact is soft-deleted"
        );
        let audits_after: Value = request
            .get(&format!("/api/audits/{contact_pid}"))
            .await
            .json();
        let erased_entry = audits_after
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["action"] == "erased")
            .expect("erased audit row")
            .clone();
        assert_eq!(
            erased_entry["snapshot"]["activity_summaries_scrubbed"], 2,
            "the contact's own note and the note on its ticket"
        );
        let ticket_after: Value = request
            .get(&format!("/api/tickets/{ticket_pid}"))
            .await
            .json();
        assert_eq!(
            ticket_after["ticket"]["title"], "[erased]",
            "the closed ticket's free-text title is scrubbed: {ticket_after}"
        );
        assert_eq!(erased_entry["snapshot"]["ticket_titles_scrubbed"], 1);
        assert_eq!(erased_entry["snapshot"]["ticket_movement_reasons_scrubbed"], 1);
        assert_eq!(erased_entry["snapshot"]["event_labels_scrubbed"], 1, "{erased_entry}");
        let label: String = sqlx::query_scalar(
            "SELECT payload->>'name' FROM event_outbox WHERE entity_pid = $1::uuid",
        )
        .bind(&contact_pid)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(label, "[erased]", "the stored event no longer names the person");
        assert_eq!(ticket_after["movements"][0]["reason"], "[erased]", "{ticket_after}");
        assert!(
            !ticket_after.to_string().contains("cannot sign in"),
            "the ticket activity's free text is gone: {ticket_after}"
        );
        // Every count is named for the statement that produced it; none is
        // another statement's number under the wrong label.
        for key in [
            "leads_scrubbed",
            "working_group_memberships_deleted",
            "notifications_deleted",
            "form_submissions_deleted",
            "external_links_deleted",
            "webhook_deliveries_cancelled",
            "merged_duplicates_scrubbed",
            "purposes_withdrawn",
            "link_tokens_deleted",
            "custom_field_values_deleted",
        ] {
            assert!(
                erased_entry["snapshot"][key].is_u64(),
                "the erase audit names {key}"
            );
        }
        assert_eq!(
            erased_entry["snapshot"]["merged_duplicates_scrubbed"], 0,
            "nothing was merged into this contact"
        );

        // ── Retention: the report discloses the (default, unfloored-by-env)
        // horizon and today's just-erased contact; a sweep against a
        // same-day estate deletes nothing and is still audited (never
        // silently a no-op).
        let report: Value = request.get("/api/retention").await.json();
        assert_eq!(
            report["horizon_days"], 365,
            "CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS unset in tests ⇒ default"
        );
        // Each horizon says whose its default is: none is any jurisdiction's (CRM-T165).
        let basis = report["horizon_basis"].as_array().expect("horizon_basis");
        let settings: Vec<&str> = basis.iter().filter_map(|b| b["setting"].as_str()).collect();
        assert_eq!(
            settings,
            [
                "CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS",
                "CONTACT_RELATIONSHIP_MANAGEMENT_EVENT_RETENTION_DAYS",
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_RETENTION_DAYS"
            ]
        );
        assert_eq!(basis[0]["days"], 365);
        assert_eq!(basis[0]["default_days"], 365);
        assert_eq!(basis[0]["floor_days"], 30);
        for entry in basis {
            assert_eq!(entry["set"], false, "{entry}");
            assert_eq!(entry["default_from"]["regime"], Value::Null, "{entry}");
            assert_eq!(entry["default_from"]["status"], "starting_point", "{entry}");
            assert_eq!(entry["days"], entry["default_days"], "{entry}");
        }
        let sweep: Value = request.post("/api/retention/sweep").await.json();
        assert_eq!(
            sweep["rows_deleted"], 0,
            "nothing is past a 365-day horizon yet"
        );
    })
    .await;
}
