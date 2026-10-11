//! The documented schemas match the running service: real requests satisfy
//! the request schemas, and real list responses satisfy the resource schemas
//! (OpenAPI schemas drift silently unless something checks them).

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_worker, seed_contact, seed_pipeline};

/// OpenAPI 3.0 marks optional nulls with `nullable`; JSON Schema wants a type
/// union. Rewrite so a standard validator understands the document.
fn to_json_schema(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.remove("nullable") == Some(Value::Bool(true))
                && let Some(Value::String(kind)) = map.get("type").cloned()
            {
                map.insert("type".into(), json!([kind, "null"]));
            }
            for child in map.values_mut() {
                to_json_schema(child);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(to_json_schema),
        _ => {}
    }
}

fn check(doc: &Value, schema: &str, instance: &Value, what: &str) {
    let root = json!({
        "$ref": format!("#/components/schemas/{schema}"),
        "components": doc["components"],
    });
    let validator = jsonschema::validator_for(&root).expect("schema compiles");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{} at {}", e, e.instance_path))
        .collect();
    assert!(
        errors.is_empty(),
        "{what} does not satisfy {schema}: {errors:?}\n{instance}"
    );
}

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
