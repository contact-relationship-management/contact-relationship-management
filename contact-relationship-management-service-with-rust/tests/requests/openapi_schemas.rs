//! The documented schemas match the running service: real requests satisfy
//! the request schemas, and real list responses satisfy the resource schemas
//! (OpenAPI schemas drift silently unless something checks them).

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_worker, seed_contact, seed_pipeline};

#[path = "../support/schema_check.rs"]
mod schema_check;
use schema_check::{check, to_json_schema};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn responses_match_the_documented_schemas() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        to_json_schema(&mut doc);

        let contact_pid = seed_contact!(&request, "Schema Subject").await;
        let (pipeline_pid, _stages) = seed_pipeline!(&request).await;

        // Each request body validates against its documented request schema,
        // and the service accepts it.
        let posts: Vec<(&str, &str, Value)> = vec![
            (
                "/api/contacts",
                "ContactPayload",
                json!({
                "person_ref": a_person(), "display_name": "Minimal Contact" }),
            ),
            (
                "/api/accounts",
                "AccountPayload",
                json!({
                "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                "display_name": "Schema Org" }),
            ),
            (
                "/api/leads",
                "LeadPayload",
                json!({
                "display_name": "Schema Lead", "source": "referral",
                "email": "lead@schema.example" }),
            ),
            (
                "/api/deals",
                "DealPayload",
                json!({
                "name": "Schema Deal", "pipeline_pid": pipeline_pid,
                "amount_minor": 1000_i64, "currency": "GBP" }),
            ),
            (
                "/api/activities",
                "ActivityPayload",
                json!({
                "subject_kind": "contact", "subject_pid": contact_pid,
                "kind": "note", "summary": "Schema note", "actor_ref": a_worker() }),
            ),
            (
                "/api/sla-policies",
                "PolicyPayload",
                json!({
                "priority": "high", "first_response_minutes": 5, "resolution_minutes": 60 }),
            ),
            (
                "/api/tickets",
                "TicketPayload",
                json!({
                "title": "Schema ticket", "priority": "high", "contact_pid": contact_pid }),
            ),
            (
                "/api/articles",
                "ArticlePayload",
                json!({
                "title": "Schema article", "body": "Body", "keywords": "schema" }),
            ),
            (
                "/api/segments",
                "SegmentPayload",
                json!({ "name": "Schema segment", "filter": {} }),
            ),
            (
                "/api/campaigns",
                "CampaignPayload",
                json!({ "name": "Schema campaign" }),
            ),
        ];
        for (path, schema, body) in &posts {
            check(&doc, schema, body, path);
            let response = request.post(path).json(body).await;
            response.assert_status_ok();
            check(&doc, "Created", &response.json::<Value>(), path);
        }

        // Each list's rows validate against the documented resource schema.
        let lists = [
            ("/api/contacts", "Contact"),
            ("/api/accounts", "Account"),
            ("/api/activities", "Activity"),
            ("/api/leads", "Lead"),
            ("/api/pipelines", "Pipeline"),
            ("/api/deals", "Deal"),
            ("/api/tickets", "Ticket"),
            ("/api/articles", "Article"),
            ("/api/campaigns", "Campaign"),
            ("/api/segments", "Segment"),
            ("/api/sla-policies", "SlaPolicy"),
        ];
        for (path, schema) in lists {
            let rows: Value = request.get(path).await.json();
            let rows = rows
                .as_array()
                .unwrap_or_else(|| panic!("{path} is not an array"));
            assert!(!rows.is_empty(), "{path} returned no rows to check");
            for row in rows {
                check(&doc, schema, row, path);
            }
        }
    })
    .await;
}

/// Every non-list read, and the sweeps, validate against their documented
/// response schemas, with rows in every table so no section is empty.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded walk over every documented read
async fn read_responses_match_the_documented_schemas() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        to_json_schema(&mut doc);

        // ── Seed one of everything.
        let account: Value = request
            .post("/api/accounts")
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                "display_name": "Schema Account" }),
            )
            .await
            .json();
        let account_pid = account["pid"].as_str().unwrap().to_string();
        let contact: Value = request
            .post("/api/contacts")
            .json(
                &json!({ "person_ref": a_person(), "display_name": "Schema Reader",
                "account_pid": account_pid }),
            )
            .await
            .json();
        let contact_pid = contact["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/contacts/{contact_pid}/consent"))
            .json(&json!({ "action": "granted", "source": "web form" }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/contacts/{contact_pid}/stakeholder"))
            .json(&json!({ "role": "partner", "influence": 4, "interest": 5 }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/accounts/{account_pid}/stakeholder"))
            .json(&json!({ "role": "partner" }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/accounts/{account_pid}/membership"))
            .json(&json!({ "joined_on": "2024-01-01", "renewal_on": "2099-01-01" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/accounts/{account_pid}/partnerships"))
            .json(&json!({ "kind": "university", "summary": "Joint lab" }))
            .await
            .assert_status_ok();
        let (pipeline_pid, stages) = seed_pipeline!(&request).await;
        let deal: Value = request
            .post("/api/deals")
            .json(
                &json!({ "name": "Schema Deal", "pipeline_pid": pipeline_pid,
                "amount_minor": 5000_i64, "currency": "GBP", "account_pid": account_pid,
                "primary_contact_pid": contact_pid }),
            )
            .await
            .json();
        let deal_pid = deal["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stages[1] }))
            .await
            .assert_status_ok();
        request
            .post("/api/activities")
            .json(
                &json!({ "subject_kind": "contact", "subject_pid": contact_pid,
                "kind": "task", "summary": "Call back", "due_on": "2000-01-01",
                "sentiment": "positive", "actor_ref": a_worker() }),
            )
            .await
            .assert_status_ok();
        let lead: Value = request
            .post("/api/leads")
            .json(&json!({ "display_name": "Schema Lead", "source": "referral" }))
            .await
            .json();
        let lead_pid = lead["pid"].as_str().unwrap().to_string();
        let ticket: Value = request
            .post("/api/tickets")
            .json(&json!({ "title": "Schema ticket", "priority": "high",
                "contact_pid": contact_pid, "assignee_ref": a_worker() }))
            .await
            .json();
        let ticket_pid = ticket["pid"].as_str().unwrap().to_string();
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let segment_pid = segment["pid"].as_str().unwrap().to_string();
        let campaign: Value = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Schema campaign", "segment_pid": segment_pid }))
            .await
            .json();
        let campaign_pid = campaign["pid"].as_str().unwrap().to_string();
        let group: Value = request
            .post("/api/groups")
            .json(&json!({ "name": "Schema WG" }))
            .await
            .json();
        let group_pid = group["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/groups/{group_pid}/members"))
            .json(&json!({ "contact_pid": contact_pid }))
            .await
            .assert_status_ok();
        request
            .post("/api/forecast/snapshot")
            .await
            .assert_status_ok();

        // ── Reads: (path, schema).
        let reads = [
            (format!("/api/contacts/{contact_pid}"), "ContactDetail"),
            (format!("/api/accounts/{account_pid}"), "AccountDetail"),
            (format!("/api/accounts/{account_pid}/clv"), "Clv"),
            (format!("/api/leads/{lead_pid}"), "LeadDetail"),
            (format!("/api/tickets/{ticket_pid}"), "TicketDetail"),
            (format!("/api/groups/{group_pid}"), "Group"),
            ("/api/forecast".into(), "Forecast"),
            ("/api/dashboards/sales".into(), "SalesDashboard"),
            ("/api/dashboards/sla".into(), "SlaDashboard"),
            ("/api/dashboards/activity".into(), "ActivityDashboard"),
            (format!("/api/segments/{segment_pid}/preview"), "Preview"),
            (
                format!("/api/campaigns/{campaign_pid}/funnel"),
                "CampaignFunnel",
            ),
            (
                format!("/api/contacts/{contact_pid}/subject-access"),
                "SubjectAccessExport",
            ),
            ("/api/retention".into(), "RetentionReport"),
            ("/api/audits/verify".into(), "ChainVerdict"),
            ("/api/insights/stale-deals".into(), "StaleDeals"),
            ("/api/insights/followups".into(), "Followups"),
            ("/api/insights/pipeline-hygiene".into(), "HygieneFindings"),
            ("/api/insights/executive".into(), "ExecutivePack"),
            ("/api/insights/forecast-trends".into(), "ForecastTrends"),
            ("/api/insights/sla".into(), "SlaRegister"),
            ("/api/insights/dpo".into(), "DpoView"),
            ("/api/insights/cadence?days=0".into(), "Cadence"),
            ("/api/insights/engagement".into(), "EngagementView"),
            (
                format!("/api/insights/funnel?pipeline={pipeline_pid}"),
                "StageFunnel",
            ),
            ("/api/insights/members".into(), "MembersView"),
            (
                "/api/insights/consent-by-account".into(),
                "ConsentByAccount",
            ),
            ("/api/insights/stakeholders".into(), "StakeholderRegister"),
            ("/api/insights/partnerships".into(), "PartnershipsRegister"),
            (
                "/api/insights/memberships?days=36500".into(),
                "MembershipsView",
            ),
        ];
        for (path, schema) in &reads {
            let response = request.get(path).await;
            response.assert_status_ok();
            check(&doc, schema, &response.json::<Value>(), path);
        }

        // ── Lists with their own element schemas.
        let element_lists = [
            ("/api/audits/recent", "AuditEntry"),
            ("/api/events/recent", "Event"),
            ("/api/nurture-sequences", "SequenceView"),
        ];
        for (path, schema) in element_lists {
            let rows: Value = request.get(path).await.json();
            for row in rows
                .as_array()
                .unwrap_or_else(|| panic!("{path} is not an array"))
            {
                check(&doc, schema, row, path);
            }
        }
        let trail: Value = request
            .get(&format!("/api/audits/{contact_pid}"))
            .await
            .json();
        assert!(!trail.as_array().unwrap().is_empty());
        for row in trail.as_array().unwrap() {
            check(&doc, "AuditEntry", row, "/api/audits/{entity_pid}");
        }

        // ── Writes that return a typed body.
        let snapshot: Value = request.post("/api/forecast/snapshot").await.json();
        check(&doc, "Snapshot", &snapshot, "/api/forecast/snapshot");
        let advance: Value = request.post("/api/nurture/advance").await.json();
        check(&doc, "NurtureSweep", &advance, "/api/nurture/advance");
        let sweep: Value = request.post("/api/sla/sweep").await.json();
        check(&doc, "SlaSweep", &sweep, "/api/sla/sweep");
        let retention: Value = request.post("/api/retention/sweep").await.json();
        check(&doc, "RetentionSweep", &retention, "/api/retention/sweep");
        let erased: Value = request
            .post(&format!(
                "/api/contacts/{}/erase",
                seed_contact!(&request, "Erase Me").await
            ))
            .await
            .json();
        check(&doc, "Erased", &erased, "/api/contacts/{pid}/erase");
    })
    .await;
}

/// Every error, whoever produced it, is an RFC 9457 problem document that
/// satisfies the documented `Error` schema and still carries the earlier
/// `error` and `description` members.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn errors_are_problem_documents() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        to_json_schema(&mut doc);
        let missing = uuid::Uuid::new_v4();

        let cases = [
            (
                "validation (422, controller)",
                request
                    .post("/api/contacts")
                    .json(&json!({ "person_ref": "nope", "display_name": "" }))
                    .await,
                422,
                "urn:crm:problem:validation",
            ),
            (
                "unknown record (404, controller)",
                request.get(&format!("/api/contacts/{missing}")).await,
                404,
                "about:blank",
            ),
            (
                "unknown version (406, middleware)",
                request
                    .get("/api/contacts")
                    .add_header("accepts-version", "9.9")
                    .await,
                406,
                "urn:crm:problem:unsupported-api-version",
            ),
            (
                "unknown route (404, router)",
                request.get("/api/no-such-thing").await,
                404,
                "about:blank",
            ),
            (
                "body over the limit (413, loco layer)",
                request
                    .post("/api/contacts")
                    .add_header("content-type", "application/json")
                    .bytes(vec![b' '; 3 * 1024 * 1024].into())
                    .await,
                413,
                "about:blank",
            ),
            (
                "wrong method (405, router)",
                request.delete("/api/contacts").await,
                405,
                "about:blank",
            ),
        ];
        for (what, response, status, kind) in cases {
            assert_eq!(response.status_code(), status, "{what}");
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            assert!(
                content_type.starts_with("application/problem+json"),
                "{what}: content-type {content_type}"
            );
            let body: Value = response.json();
            check(&doc, "Error", &body, what);
            assert_eq!(body["status"], status, "{what}");
            assert_eq!(body["type"], kind, "{what}: {body}");
            assert_eq!(body["detail"], body["description"], "{what}");
        }
    })
    .await;
}
