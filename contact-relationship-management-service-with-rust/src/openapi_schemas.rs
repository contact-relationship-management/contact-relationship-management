//! Request and response schemas for the `OpenAPI` document (CRM-T49 follow-up).
//!
//! Request schemas mirror the `*Payload` structs in the controllers: a field is
//! required exactly when the struct field has no `#[serde(default)]` (in any
//! form) and is not an `Option`; a defaulted field is optional here and the
//! service fills it in. Resource schemas describe what a list returns; they keep
//! `additionalProperties: true` because several reads add derived fields (live
//! breach flags, scores), and they list as `required` only fields every row
//! carries. The DB-gated test `responses_match_the_documented_schemas`
//! validates real responses against them, so drift fails a test.
//!
//! Not schematised: the insight views, dashboards, forecast, audit and
//! retention reports, and the bundled detail reads (contact, account, lead,
//! ticket, group). They are documented as objects with a description only.

use serde_json::{Value, json};

fn string() -> Value {
    json!({ "type": "string" })
}
fn uuid() -> Value {
    json!({ "type": "string", "format": "uuid" })
}
fn integer() -> Value {
    json!({ "type": "integer" })
}
fn boolean() -> Value {
    json!({ "type": "boolean" })
}
fn date() -> Value {
    json!({ "type": "string", "format": "date" })
}
fn date_time() -> Value {
    json!({ "type": "string", "format": "date-time" })
}
fn nullable(mut schema: Value) -> Value {
    schema["nullable"] = json!(true);
    schema
}
fn object(required: &[&str], properties: &Value) -> Value {
    json!({ "type": "object", "required": required, "properties": properties })
}
fn resource(required: &[&str], properties: &Value) -> Value {
    let mut schema = object(required, properties);
    schema["additionalProperties"] = json!(true);
    schema
}
fn reference(name: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{name}") })
}

/// `components.schemas`.
#[must_use]
#[allow(clippy::too_many_lines)] // one literal table of schemas
pub fn schemas() -> Value {
    json!({
        "Error": object(&["error", "description"], &json!({
            "error": string(), "description": string() })),
        "Created": resource(&["pid"], &json!({ "pid": uuid() })),

        // ---- requests
        "ContactPayload": object(&["person_ref", "display_name"], &json!({
            "person_ref": string(), "display_name": string(), "account_pid": uuid(),
            "owner_ref": string(), "job_title": string(), "preferred_channel": string() })),
        "AccountPayload": object(&["organization_ref", "display_name"], &json!({
            "organization_ref": string(), "display_name": string(), "tier": string(),
            "owner_ref": string(), "industry": string() })),
        "RepointPayload": object(&["person_ref", "reason"], &json!({
            "person_ref": string(), "reason": string() })),
        "ActivityPayload": object(&["subject_kind", "subject_pid", "kind", "summary"], &json!({
            "subject_kind": string(), "subject_pid": uuid(), "kind": string(), "summary": string(),
            "occurred_at": date_time(), "actor_ref": string(), "due_on": date(),
            "sentiment": string() })),
        "ConsentPayload": object(&["action", "source"], &json!({
            "action": string(), "source": string() })),
        "ContactStakeholderPayload": object(&[], &json!({
            "role": string(), "influence": integer(), "interest": integer() })),
        "AccountStakeholderPayload": object(&[], &json!({ "role": string() })),
        "LeadPayload": object(&["display_name", "source"], &json!({
            "display_name": string(), "source": string(), "email": string(),
            "campaign_pid": uuid(), "contact_pid": uuid() })),
        "LeadStatusPayload": object(&["to"], &json!({
            "to": string(), "person_ref": string(), "deal": reference("DealPayload") })),
        "StageInput": object(&["name", "probability_percent"], &json!({
            "name": string(), "probability_percent": integer(),
            "is_won": boolean(), "is_lost": boolean() })),
        "PipelinePayload": object(&["name", "stages"], &json!({
            "name": string(), "stages": { "type": "array", "items": reference("StageInput") } })),
        "DealPayload": object(&["name", "pipeline_pid", "amount_minor", "currency"], &json!({
            "name": string(), "pipeline_pid": uuid(),
            "amount_minor": { "type": "integer", "format": "int64",
                "description": "Money in minor units (for example pence)" },
            "currency": { "type": "string", "description": "ISO 4217 code" },
            "account_pid": uuid(), "primary_contact_pid": uuid(), "owner_ref": string(),
            "expected_close_on": date(), "source_campaign_pid": uuid() })),
        "DealStagePayload": object(&["stage_pid"], &json!({
            "stage_pid": uuid(), "kanban_position": integer(), "lost_reason": string() })),
        "ReopenPayload": object(&["reason"], &json!({ "reason": string() })),
        "SegmentPayload": object(&["name"], &json!({
            "name": string(), "filter": { "type": "object" } })),
        "CampaignPayload": object(&["name"], &json!({
            "name": string(), "kind": string(), "currency": string(),
            "cost_minor": { "type": "integer", "format": "int64" }, "segment_pid": uuid() })),
        "StatusPayload": object(&["to"], &json!({ "to": string() })),
        "SequenceStep": object(&["delay_hours", "template_ref"], &json!({
            "delay_hours": integer(), "template_ref": string() })),
        "SequencePayload": object(&["name", "steps"], &json!({
            "name": string(), "steps": { "type": "array", "items": reference("SequenceStep") } })),
        "EnrollPayload": object(&["contact_pid"], &json!({ "contact_pid": uuid() })),
        "PolicyPayload": object(&["priority", "first_response_minutes", "resolution_minutes"], &json!({
            "priority": string(), "first_response_minutes": integer(),
            "resolution_minutes": integer() })),
        "TicketPayload": object(&["title", "priority"], &json!({
            "title": string(), "priority": string(), "channel": string(),
            "contact_pid": uuid(), "assignee_ref": string() })),
        "PriorityPayload": object(&["priority", "reason"], &json!({
            "priority": string(), "reason": string() })),
        "ArticlePayload": object(&["title", "body"], &json!({
            "title": string(), "body": string(), "keywords": string() })),
        "GrantPayload": object(&["reason"], &json!({
            "reason": { "type": "string", "minLength": 20, "maxLength": 500 },
            "minutes": { "type": "integer", "minimum": 1, "maximum": 60 } })),
        "ReviewPayload": object(&["note"], &json!({
            "note": { "type": "string", "minLength": 10 } })),
        "PartnershipPayload": object(&["kind", "summary"], &json!({
            "kind": string(), "summary": string(), "started_on": date() })),
        "MembershipPayload": object(&["joined_on"], &json!({
            "joined_on": date(), "status": string(), "renewal_on": date() })),
        "GroupPayload": object(&["name"], &json!({ "name": string(), "purpose": string() })),
        "MemberPayload": object(&["contact_pid"], &json!({ "contact_pid": uuid() })),

        // ---- resources (what the lists return)
        "Contact": resource(&["pid", "person_ref", "display_name", "status", "marketing_consent"], &json!({
            "pid": uuid(), "person_ref": string(), "account_pid": nullable(uuid()),
            "owner_ref": nullable(string()), "display_name": string(), "status": string(),
            "job_title": nullable(string()), "preferred_channel": string(),
            "marketing_consent": string() })),
        "Account": resource(&["pid", "organization_ref", "display_name", "tier"], &json!({
            "pid": uuid(), "organization_ref": string(), "owner_ref": nullable(string()),
            "display_name": string(), "tier": string(), "industry": nullable(string()) })),
        "Activity": resource(&["pid", "subject_kind", "subject_pid", "kind", "occurred_at", "done"], &json!({
            "pid": uuid(), "subject_kind": string(), "subject_pid": uuid(), "kind": string(),
            "occurred_at": date_time(), "actor_ref": nullable(string()), "summary": string(),
            "due_on": nullable(date()), "done": boolean(), "sentiment": nullable(string()) })),
        "Lead": resource(&["pid", "display_name", "source", "score", "status"], &json!({
            "pid": uuid(), "display_name": string(), "source": string(),
            "email": nullable(string()), "score": integer(), "status": string(),
            "campaign_pid": nullable(uuid()), "contact_pid": nullable(uuid()) })),
        "PipelineRecord": resource(&["pid", "name"], &json!({ "pid": uuid(), "name": string() })),
        "StageRecord": resource(&["pid", "pipeline_pid", "name", "position", "probability_percent",
            "is_won", "is_lost"], &json!({
            "pid": uuid(), "pipeline_pid": uuid(), "name": string(), "position": integer(),
            "probability_percent": integer(), "is_won": boolean(), "is_lost": boolean() })),
        "Pipeline": object(&["pipeline", "stages"], &json!({
            "pipeline": reference("PipelineRecord"),
            "stages": { "type": "array", "items": reference("StageRecord") } })),
        "Deal": resource(&["pid", "name", "pipeline_pid", "stage_pid", "currency"], &json!({
            "pid": uuid(), "name": string(), "pipeline_pid": uuid(), "stage_pid": uuid(),
            "amount_minor": { "type": "integer", "format": "int64", "nullable": true,
                "description": "Null when masked for the caller" },
            "currency": string(), "won": boolean(), "closed_at": nullable(date_time()),
            "kanban_position": integer() })),
        "Ticket": resource(&["pid", "title", "priority", "channel", "status"], &json!({
            "pid": uuid(), "title": string(), "priority": string(), "channel": string(),
            "status": string(), "contact_pid": nullable(uuid()),
            "assignee_ref": nullable(string()), "opened_at": date_time(),
            "live_first_response_breached": boolean(),
            "live_resolution_breached": boolean() })),
        "Article": resource(&["pid", "title", "body", "status", "version"], &json!({
            "pid": uuid(), "title": string(), "body": string(), "keywords": nullable(string()),
            "status": string(), "version": integer() })),
        "Campaign": resource(&["pid", "name", "kind", "status", "currency"], &json!({
            "pid": uuid(), "name": string(), "kind": string(), "status": string(),
            "cost_minor": { "type": "integer", "format": "int64" }, "currency": string(),
            "segment_pid": nullable(uuid()), "recipients": integer(), "delivered": integer(),
            "opened": integer(), "clicked": integer(), "unsubscribed": integer() })),
        "Segment": resource(&["pid", "name"], &json!({
            "pid": uuid(), "name": string(), "filter": { "type": "object" } })),
        "SlaPolicy": resource(&["pid", "priority", "first_response_minutes", "resolution_minutes"], &json!({
            "pid": uuid(), "priority": string(), "first_response_minutes": integer(),
            "resolution_minutes": integer() })),
    })
}

/// The request schema for `(method, path)`, if the operation takes a body.
#[must_use]
pub fn request_schema(method: &str, path: &str) -> Option<&'static str> {
    Some(match (method, path) {
        ("post", "/api/contacts") => "ContactPayload",
        ("post", "/api/contacts/{pid}/repoint") => "RepointPayload",
        ("post", "/api/contacts/{pid}/consent") => "ConsentPayload",
        ("put", "/api/contacts/{pid}/stakeholder") => "ContactStakeholderPayload",
        ("put", "/api/accounts/{pid}/stakeholder") => "AccountStakeholderPayload",
        ("post", "/api/accounts") => "AccountPayload",
        ("post", "/api/activities") => "ActivityPayload",
        ("post", "/api/leads") => "LeadPayload",
        ("post", "/api/leads/{pid}/status") => "LeadStatusPayload",
        ("post", "/api/pipelines") => "PipelinePayload",
        ("post", "/api/deals") => "DealPayload",
        ("post", "/api/deals/{pid}/stage") => "DealStagePayload",
        ("post", "/api/deals/{pid}/reopen") => "ReopenPayload",
        ("post", "/api/segments") => "SegmentPayload",
        ("post", "/api/campaigns") => "CampaignPayload",
        (
            "post",
            "/api/campaigns/{pid}/status"
            | "/api/tickets/{pid}/status"
            | "/api/articles/{pid}/status"
            | "/api/partnerships/{pid}/stage",
        ) => "StatusPayload",
        ("post", "/api/nurture-sequences") => "SequencePayload",
        ("post", "/api/nurture-sequences/{pid}/enrollments") => "EnrollPayload",
        ("post", "/api/sla-policies") => "PolicyPayload",
        ("post", "/api/tickets") => "TicketPayload",
        ("put", "/api/tickets/{pid}/priority") => "PriorityPayload",
        ("post", "/api/articles") | ("put", "/api/articles/{pid}") => "ArticlePayload",
        ("post", "/api/break-glass") => "GrantPayload",
        ("post", "/api/break-glass/{pid}/review") => "ReviewPayload",
        ("post", "/api/accounts/{pid}/partnerships") => "PartnershipPayload",
        ("put", "/api/accounts/{pid}/membership") => "MembershipPayload",
        ("post", "/api/groups") => "GroupPayload",
        ("post", "/api/groups/{pid}/members") => "MemberPayload",
        _ => return None,
    })
}

/// The schema of each row a list returns, for `GET path`.
#[must_use]
pub fn list_schema(path: &str) -> Option<&'static str> {
    Some(match path {
        "/api/contacts" => "Contact",
        "/api/accounts" => "Account",
        "/api/activities" => "Activity",
        "/api/leads" => "Lead",
        "/api/pipelines" => "Pipeline",
        "/api/deals" => "Deal",
        "/api/tickets" => "Ticket",
        "/api/articles" => "Article",
        "/api/campaigns" => "Campaign",
        "/api/segments" => "Segment",
        "/api/sla-policies" => "SlaPolicy",
        _ => return None,
    })
}

/// Attach `requestBody`, typed list responses, the `Error` body and the
/// `Created` response to the operations in `doc`.
pub fn attach(doc: &mut Value) {
    doc["components"]["schemas"] = schemas();
    let json_body = |schema: Value| json!({ "application/json": { "schema": schema } });
    let Some(paths) = doc["paths"].as_object_mut() else {
        return;
    };
    for (path, item) in paths.iter_mut() {
        let Some(item) = item.as_object_mut() else {
            continue;
        };
        for (method, operation) in item.iter_mut() {
            if let Some(name) = request_schema(method, path) {
                operation["requestBody"] = json!({
                    "required": true, "content": json_body(reference(name)) });
                if method == "post" && path.matches('{').count() == 0 && name.ends_with("Payload") {
                    operation["responses"]["200"]["content"] = json_body(reference("Created"));
                }
            }
            if method == "get"
                && let Some(name) = list_schema(path)
            {
                operation["responses"]["200"]["content"] = json_body(json!({
                    "type": "array", "items": reference(name) }));
            }
            if path.starts_with("/api/")
                && let Some(responses) = operation["responses"].as_object_mut()
            {
                for (status, response) in responses.iter_mut() {
                    if status.len() == 3
                        && !status.starts_with('2')
                        && response.get("content").is_none()
                    {
                        response["content"] = json_body(reference("Error"));
                    }
                }
            }
        }
    }
}
