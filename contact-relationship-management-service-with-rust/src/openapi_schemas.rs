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
//! Responses of the reads that are not plain lists (insight views, dashboards,
//! forecast, audit, retention, the bundled detail reads) are described by
//! `view_schemas`. Fields a role can mask (amounts, cost, ROI, totals) are
//! `nullable`; a masked value is absent, never zero. Operations whose response
//! is just the updated record (`transition` in `openapi.rs`) stay untyped.

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

/// Request schemas generated from the payload structs, so a request body and
/// its documentation cannot drift apart. The rest are still written by hand
/// below; the move is one controller at a time (CRM-T83).
fn generated() -> serde_json::Map<String, Value> {
    use crate::controllers::{
        board, break_glass, engagement, external_links, marketing, merge, relationships, sales,
        support, unmerge,
    };
    let mut generator = schemars::generate::SchemaSettings::openapi3().into_generator();
    macro_rules! document {
        ($($ty:ty),* $(,)?) => {$( generator.subschema_for::<$ty>(); )*};
    }
    document!(
        relationships::ContactPayload,
        relationships::AccountPayload,
        relationships::RepointPayload,
        relationships::ActivityPayload,
        relationships::ContactDetail,
        relationships::AccountDetail,
        relationships::ContactDuplicates,
        merge::ContactMerge,
        sales::LeadDetail,
        sales::PipelineWithStages,
        sales::Forecast,
        crate::models::_entities::pipelines::Model,
        crate::models::_entities::pipeline_stages::Model,
        crate::models::_entities::forecast_snapshots::Model,
        unmerge::ContactUnmerge,
        unmerge::ContactMerges,
        external_links::ExternalLinks,
        crate::models::_entities::external_links::Model,
        sales::LeadPayload,
        sales::LeadStatusPayload,
        sales::PipelinePayload,
        sales::DealPayload,
        sales::DealStagePayload,
        sales::ReopenPayload,
        board::DealEditPayload,
        board::LeadEditPayload,
        board::TicketEditPayload,
        board::PositionPayload,
        board::ActivityEditPayload,
        crate::models::_entities::contacts::Model,
        crate::models::_entities::accounts::Model,
        crate::models::_entities::activities::Model,
        crate::models::_entities::leads::Model,
        crate::models::_entities::deals::Model,
        crate::models::_entities::tickets::Model,
        crate::models::_entities::articles::Model,
        crate::models::_entities::campaigns::Model,
        crate::models::_entities::segments::Model,
        crate::models::_entities::sla_policies::Model,
        crate::models::_entities::email_deliveries::Model,
        support::PolicyPayload,
        support::TicketPayload,
        support::PriorityPayload,
        support::ArticlePayload,
        engagement::ContactStakeholderPayload,
        engagement::AccountStakeholderPayload,
        engagement::PartnershipPayload,
        engagement::MembershipPayload,
        engagement::GroupPayload,
        engagement::MemberPayload,
        merge::MergePayload,
        unmerge::UnmergePayload,
        external_links::LinkPayload,
        break_glass::GrantPayload,
        break_glass::ReviewPayload,
        marketing::consent::ConsentPayload,
        marketing::segment::SegmentPayload,
        marketing::nurture::SequencePayload,
        marketing::nurture::StepPayload,
        marketing::nurture::EnrollPayload,
        marketing::campaign::CampaignPayload,
    );
    generator.take_definitions(true)
}

/// `components.schemas`.
#[must_use]
pub fn schemas() -> Value {
    let mut all = base_schemas();
    if let Some(all) = all.as_object_mut() {
        all.extend(generated());
    }
    if let (Some(all), Some(views)) = (all.as_object_mut(), view_schemas().as_object()) {
        for (name, schema) in views {
            all.insert(name.clone(), schema.clone());
        }
    }
    all
}

#[allow(clippy::too_many_lines)] // one literal table of schemas
fn base_schemas() -> Value {
    json!({
        "Error": resource(&["type", "title", "status", "detail", "instance", "error", "description"], &json!({
            "type": { "type": "string", "description": "`about:blank`, or a `urn:crm:problem:` URN for the machine code" },
            "title": string(), "status": integer(), "detail": string(),
            "instance": { "type": "string", "description": "The request path" },
            "error": { "type": "string", "description": "Stable machine code (kept from the earlier body)" },
            "description": { "type": "string", "description": "Same text as `detail` (kept from the earlier body)" },
            "errors": { "type": "array", "description": "On a `422` validation error: each problem, with the member it is about",
                "items": { "type": "object", "required": ["detail"], "properties": {
                    "pointer": { "type": "string", "description": "JSON Pointer (RFC 6901) to the request member, such as `/display_name`; a list entry is `/stages/0/name` for a body the parser refused, and `/stages/name` for a check that does not know the entry. Absent when the problem is about the request as a whole" },
                    "detail": string() } } } })),
        "Created": resource(&["pid"], &json!({ "pid": uuid() })),

        // ---- requests
        "StatusPayload": object(&["to"], &json!({ "to": string() })),
        "SequenceStep": object(&["delay_hours", "template_ref"], &json!({
            "delay_hours": integer(), "template_ref": string(),
            "subject": { "type": "string", "description": "Defaults to the template reference" },
            "body": { "type": "string", "maxLength": 20000,
                "description": "Plain text. Required for every step before a real sender sends it" } })),
        // ---- resources (what the lists return)
    })
}

fn number() -> Value {
    json!({ "type": "number" })
}
fn counts() -> Value {
    json!({ "type": "object", "additionalProperties": { "type": "integer" } })
}
fn money_by_currency() -> Value {
    json!({ "type": "object",
        "additionalProperties": { "type": "integer", "format": "int64" },
        "description": "Minor units per ISO 4217 code; never summed across currencies" })
}
fn masked_money_by_currency() -> Value {
    let mut schema = money_by_currency();
    schema["nullable"] = json!(true);
    schema["description"] = json!("Minor units per ISO 4217 code; null when masked for the caller");
    schema
}
fn list_of(name: &str) -> Value {
    json!({ "type": "array", "items": reference(name) })
}
fn strings() -> Value {
    json!({ "type": "array", "items": { "type": "string" } })
}
/// Every derived view carries `as_of` (stamped at response time).
fn view(required: &[&str], properties: &Value) -> Value {
    let mut required: Vec<&str> = required.to_vec();
    required.push("as_of");
    let mut properties = properties.clone();
    properties["as_of"] = date_time();
    object(&required, &properties)
}

/// Schemas for the reads that are not plain lists.
#[allow(clippy::too_many_lines)] // one literal table of schemas
fn view_schemas() -> Value {
    json!({
        "Ratio": object(&["numerator", "denominator", "value"], &json!({
            "numerator": integer(), "denominator": integer(),
            "value": nullable(number()) })),

        // ---- detail reads
        "TicketDetail": object(&["ticket", "activities"], &json!({
            "ticket": reference("Ticket"), "activities": list_of("Activity") })),
        "Group": object(&["group", "roster", "feed", "feed_note"], &json!({
            "group": { "type": "object" },
            "roster": { "type": "array", "items": object(&["pid", "display_name"], &json!({
                "pid": uuid(), "display_name": string(), "account_pid": nullable(uuid()),
                "stakeholder_role": nullable(string()) })) },
            "feed": list_of("Activity"), "feed_note": string() })),

        // ---- sales and analytics
        "Clv": object(&["account_pid", "won_deals", "clv_minor", "as_of"], &json!({
            "account_pid": uuid(), "won_deals": integer(), "clv_minor": money_by_currency(),
            "as_of": date_time() })),
        "SalesDashboard": view(&["win_rate", "open_deals", "pipeline_by_stage"], &json!({
            "win_rate": reference("Ratio"), "open_deals": integer(),
            "pipeline_by_stage": { "type": "object", "additionalProperties":
                object(&["count", "totals_minor"], &json!({
                    "count": integer(), "totals_minor": masked_money_by_currency() })) } })),
        "SlaDashboard": view(&["open_tickets", "by_priority"], &json!({
            "open_tickets": integer(),
            "by_priority": { "type": "array", "items": object(&["priority", "open", "breached"],
                &json!({ "priority": string(), "open": integer(), "breached": integer() })) } })),
        "ActivityDashboard": object(&["window_days", "total", "by_kind", "as_of"], &json!({
            "window_days": integer(), "total": integer(), "by_kind": counts(),
            "as_of": date_time() })),

        // ---- marketing and support
        "Preview": object(&["count", "sample"], &json!({ "count": integer(), "sample": strings() })),
        "CampaignFunnel": object(&["campaign", "leads", "won_deals", "won_revenue_minor", "roi"], &json!({
            "campaign": reference("Campaign"), "leads": integer(), "won_deals": integer(),
            "won_revenue_minor": { "type": "integer", "format": "int64", "nullable": true },
            "won_revenue_other_currencies": masked_money_by_currency(),
            "roi": { "allOf": [reference("Ratio")], "nullable": true } })),
        "Deliveries": object(&["campaign_pid", "total", "remaining", "by_state"], &json!({
            "campaign_pid": uuid(), "total": integer(),
            "remaining": { "type": "integer",
                "description": "Rows a further run could still send" },
            "by_state": counts() })),
        "NurtureSweep": object(&["sent", "completed"], &json!({
            "sent": integer(), "completed": integer() })),
        "SlaSweep": object(&["new_breaches"], &json!({ "new_breaches": integer() })),
        "SequenceView": object(&["sequence", "steps", "active_enrollments"], &json!({
            "sequence": { "type": "object" }, "steps": { "type": "array", "items": { "type": "object" } },
            "active_enrollments": integer() })),

        // ---- privacy
        "SubjectAccessExport": object(&["as_of", "contact", "consent_history", "activities",
            "leads", "deals_as_primary_contact", "tickets", "nurture_enrollments", "email_deliveries",
            "exclusions"], &json!({
            "as_of": date_time(), "contact": reference("Contact"),
            "consent_history": { "type": "array", "items": { "type": "object" } },
            "activities": list_of("Activity"), "leads": list_of("Lead"),
            "deals_as_primary_contact": list_of("Deal"), "tickets": list_of("Ticket"),
            "nurture_enrollments": { "type": "array", "items": { "type": "object" } },
            "email_deliveries": list_of("EmailDelivery"),
            "exclusions": strings() })),
        "Erased": object(&["erased", "note"], &json!({ "erased": uuid(), "note": string() })),
        "RetentionReport": object(&["as_of", "horizon_days", "soft_deleted_past_horizon",
            "withdrawn_consent_past_horizon", "derivation"], &json!({
            "as_of": date_time(), "horizon_days": integer(),
            "soft_deleted_past_horizon": counts(),
            "withdrawn_consent_past_horizon": integer(), "derivation": string() })),
        "RetentionSweep": object(&["horizon_days", "deleted", "rows_deleted"], &json!({
            "horizon_days": integer(), "deleted": counts(), "rows_deleted": integer() })),

        // ---- break-glass
        "Grant": object(&["pid", "expires_at", "minutes"], &json!({
            "pid": uuid(), "expires_at": date_time(), "minutes": integer() })),
        "GrantRecord": object(&["pid", "subject", "reason", "granted_at", "expires_at"], &json!({
            "pid": uuid(), "subject": string(), "reason": string(), "granted_at": date_time(),
            "expires_at": date_time(), "reviewed_at": nullable(date_time()),
            "reviewed_by": nullable(string()), "review_note": nullable(string()) })),
        "Grants": object(&["unreviewed", "grants"], &json!({
            "unreviewed": integer(), "grants": list_of("GrantRecord") })),
        "Reviewed": object(&["pid", "reviewed_by"], &json!({
            "pid": uuid(), "reviewed_by": string() })),

        // ---- audit and events
        "AuditEntry": resource(&["id", "entity", "entity_pid", "action", "created_at"], &json!({
            "id": { "type": "integer", "format": "int64" }, "entity": string(),
            "entity_pid": uuid(), "action": string(), "actor": nullable(string()),
            "snapshot": { "nullable": true, "description": "Free-form facts of the change" },
            "prev_hash": nullable(string()), "hash": nullable(string()),
            "created_at": date_time(), "updated_at": date_time() })),
        "ChainVerdict": object(&["ok", "rows", "legacy_rows", "head", "first_break"], &json!({
            "ok": boolean(), "rows": integer(), "legacy_rows": integer(),
            "head": nullable(string()),
            "first_break": { "nullable": true, "type": "object",
                "required": ["id", "reason"],
                "properties": { "id": integer(), "reason": string() } } })),
        "Event": { "oneOf": [
            object(&["kind", "pid", "name", "seq"], &json!({
                "kind": string(), "pid": string(), "name": string(), "seq": integer() })),
            resource(&["event_id", "entity", "entity_pid", "kind", "occurred_at"], &json!({
                "event_id": uuid(), "entity": string(), "entity_pid": uuid(), "kind": string(),
                "occurred_at": date_time(), "actor": nullable(string()),
                "published_at": nullable(date_time()) })),
        ] },

        // ---- insights
        "StaleDeals": view(&["derivation", "threshold_days", "open_deals", "stale_deals", "deals"], &json!({
            "derivation": string(), "threshold_days": integer(), "open_deals": integer(),
            "stale_deals": integer(),
            "deals": { "type": "array", "items": object(&["pid", "name", "days_in_stage", "stale"], &json!({
                "pid": uuid(), "name": string(), "stage": nullable(string()),
                "owner_ref": nullable(string()),
                "amount_minor": { "type": "integer", "format": "int64", "nullable": true },
                "currency": string(), "days_in_stage": integer(), "stale": boolean() })) } })),
        "Followups": view(&["note", "overdue", "upcoming_30d", "open_by_recorder"], &json!({
            "note": string(), "overdue": list_of("Followup"), "upcoming_30d": list_of("Followup"),
            "open_by_recorder": counts() })),
        "Followup": object(&["pid", "kind", "summary", "subject_kind", "subject_pid", "due_on"], &json!({
            "pid": uuid(), "kind": string(), "summary": string(), "subject_kind": string(),
            "subject_pid": uuid(), "actor_ref": nullable(string()), "due_on": date(),
            "overdue_days": nullable(integer()) })),
        "HygieneFindings": view(&["threshold_days", "findings"], &json!({
            "threshold_days": integer(),
            "findings": { "type": "array", "items": resource(&["rule", "detail"], &json!({
                "rule": string(), "detail": string(),
                "deal": object(&["pid", "name"], &json!({ "pid": uuid(), "name": string() })),
                "lead_pid": uuid(), "score": integer(), "expected_close_on": date() })) } })),
        "ExecutivePack": view(&["window", "deals_won", "deals_lost", "won_value_by_currency_minor",
            "lost_reasons", "new_leads", "tickets_opened", "tickets_resolved", "campaigns_started",
            "activities_logged", "consent_withdrawals", "note"], &json!({
            "window": object(&["from", "to"], &json!({ "from": date_time(), "to": date_time() })),
            "deals_won": integer(), "deals_lost": integer(),
            "won_value_by_currency_minor": masked_money_by_currency(),
            "lost_reasons": counts(), "new_leads": integer(), "tickets_opened": integer(),
            "tickets_resolved": integer(), "campaigns_started": integer(),
            "activities_logged": integer(), "consent_withdrawals": integer(), "note": string() })),
        "ForecastTrends": view(&["note", "series"], &json!({
            "note": string(),
            "series": { "type": "array", "items": object(&["taken_on", "totals"], &json!({
                "taken_on": date(), "totals": masked_money_by_currency() })) } })),
        "SlaRegister": view(&["derivation", "breaches", "workload"], &json!({
            "derivation": string(),
            "breaches": { "type": "array", "items": object(&["pid", "title", "priority", "status",
                "breached", "overdue_hours"], &json!({
                "pid": uuid(), "title": string(), "priority": string(), "status": string(),
                "assignee_ref": nullable(string()), "breached": string(),
                "overdue_hours": integer() })) },
            "workload": { "type": "array", "items": object(&["assignee_ref", "open", "breached",
                "at_risk_4h"], &json!({ "assignee_ref": string(), "open": integer(),
                "breached": integer(), "at_risk_4h": integer() })) } })),
        "DpoView": view(&["note", "contacts", "consent_coverage", "window_days",
            "withdrawals_in_window", "consent_events_by_source", "duplicate_person_refs"], &json!({
            "note": string(), "contacts": integer(), "consent_coverage": counts(),
            "window_days": integer(), "withdrawals_in_window": integer(),
            "consent_events_by_source": counts(),
            "duplicate_person_refs": { "type": "array", "items": object(&["person_ref", "contacts"], &json!({
                "person_ref": string(),
                "contacts": { "type": "array", "items": object(&["pid", "display_name"], &json!({
                    "pid": uuid(), "display_name": string() })) } })) } })),
        "Cadence": view(&["derivation", "threshold_days", "untouched_contacts",
            "untouched_accounts", "contacts_without_next_touch"], &json!({
            "derivation": string(), "threshold_days": integer(),
            "untouched_contacts": { "type": "array", "items": object(&["pid", "display_name",
                "days_since_touch", "has_next_touch"], &json!({
                "pid": uuid(), "display_name": string(), "stakeholder_role": nullable(string()),
                "days_since_touch": integer(), "has_next_touch": boolean() })) },
            "untouched_accounts": { "type": "array", "items": object(&["pid", "display_name",
                "days_since_touch"], &json!({
                "pid": uuid(), "display_name": string(), "stakeholder_role": nullable(string()),
                "days_since_touch": integer() })) },
            "contacts_without_next_touch": integer() })),
        "EngagementView": view(&["window_days", "touches", "per_recorder_month", "per_kind",
            "sentiment", "note"], &json!({
            "window_days": integer(), "touches": integer(), "per_recorder_month": counts(),
            "per_kind": counts(), "sentiment": counts(), "note": string() })),
        "StageFunnel": view(&["pipeline", "derivation", "stages"], &json!({
            "pipeline": object(&["pid", "name"], &json!({ "pid": uuid(), "name": string() })),
            "derivation": string(),
            "stages": { "type": "array", "items": object(&["stage", "position", "is_won",
                "is_lost", "entered", "conversion_from_previous"], &json!({
                "stage": string(), "position": integer(), "is_won": boolean(),
                "is_lost": boolean(), "entered": integer(),
                "conversion_from_previous": { "allOf": [reference("Ratio")], "nullable": true } })) } })),
        "MembersView": view(&["derivation", "threshold_days", "accounts", "silent_accounts"], &json!({
            "derivation": string(), "threshold_days": integer(), "silent_accounts": integer(),
            "accounts": { "type": "array", "items": object(&["pid", "display_name", "tier",
                "contacts", "days_since_touch", "silent", "open_followups", "open_tickets"], &json!({
                "pid": uuid(), "display_name": string(), "tier": string(),
                "stakeholder_role": nullable(string()),
                "membership": { "nullable": true, "type": "object", "properties": {
                    "status": string(), "joined_on": date(), "renewal_on": nullable(date()) } },
                "contacts": integer(), "days_since_touch": integer(), "silent": boolean(),
                "open_followups": integer(), "open_tickets": integer() })) } })),
        "ConsentByAccount": view(&["window_days", "note", "accounts"], &json!({
            "window_days": integer(), "note": string(),
            "accounts": { "type": "array", "items": object(&["pid", "display_name",
                "consent_coverage", "withdrawals_in_window"], &json!({
                "pid": uuid(), "display_name": string(), "consent_coverage": counts(),
                "withdrawals_in_window": integer() })) } })),
        "StakeholderRegister": view(&["note", "by_role", "grid", "stakeholders_without_grid_scores",
            "undeclared_contacts", "account_roles"], &json!({
            "note": string(),
            "by_role": { "type": "object", "additionalProperties": { "type": "array", "items":
                object(&["pid", "display_name", "marketing_consent", "days_since_touch"], &json!({
                    "pid": uuid(), "display_name": string(), "marketing_consent": string(),
                    "influence": nullable(integer()), "interest": nullable(integer()),
                    "days_since_touch": integer() })) } },
            "grid": counts(), "stakeholders_without_grid_scores": integer(),
            "undeclared_contacts": integer(),
            "account_roles": { "type": "array", "items": object(&["pid", "display_name", "role"], &json!({
                "pid": uuid(), "display_name": string(), "role": string() })) } })),
        "PartnershipsRegister": view(&["by_kind", "by_stage", "register"], &json!({
            "by_kind": counts(), "by_stage": counts(),
            "register": { "type": "array", "items": object(&["pid", "account_pid", "kind", "stage"], &json!({
                "pid": uuid(), "account_pid": uuid(), "account": nullable(string()),
                "kind": string(), "stage": string(), "summary": nullable(string()),
                "started_on": nullable(date()) })) } })),
        "MembershipsView": view(&["window_days", "memberships", "renewals_due", "lapsed"], &json!({
            "window_days": integer(), "memberships": integer(),
            "renewals_due": list_of("MembershipRow"), "lapsed": list_of("MembershipRow") })),
        "MembershipRow": object(&["pid", "account_pid", "status", "joined_on"], &json!({
            "pid": uuid(), "account_pid": uuid(), "account": nullable(string()), "status": string(),
            "joined_on": date(), "renewal_on": nullable(date()) })),
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
        ("put", "/api/deals/{pid}") => "DealEditPayload",
        ("put", "/api/leads/{pid}") => "LeadEditPayload",
        ("put", "/api/tickets/{pid}") => "TicketEditPayload",
        (
            "put",
            "/api/deals/{pid}/position"
            | "/api/leads/{pid}/position"
            | "/api/tickets/{pid}/position",
        ) => "PositionPayload",
        ("put", "/api/activities/{pid}") => "ActivityEditPayload",
        ("post", "/api/articles") | ("put", "/api/articles/{pid}") => "ArticlePayload",
        ("post", "/api/break-glass") => "GrantPayload",
        ("post", "/api/break-glass/{pid}/review") => "ReviewPayload",
        ("post", "/api/accounts/{pid}/partnerships") => "PartnershipPayload",
        ("put", "/api/accounts/{pid}/membership") => "MembershipPayload",
        ("post", "/api/groups") => "GroupPayload",
        ("post", "/api/groups/{pid}/members") => "MemberPayload",
        ("post", "/api/contacts/{pid}/merge") => "MergePayload",
        ("post", "/api/contacts/{pid}/unmerge") => "UnmergePayload",
        (
            "post",
            "/api/contacts/{pid}/external-links"
            | "/api/accounts/{pid}/external-links"
            | "/api/deals/{pid}/external-links"
            | "/api/leads/{pid}/external-links",
        ) => "ExternalLinkPayload",
        _ => return None,
    })
}

/// The schema of a non-list `200` response for `(method, path)`.
#[must_use]
pub fn response_schema(method: &str, path: &str) -> Option<&'static str> {
    Some(match (method, path) {
        ("get", "/api/contacts/{pid}") => "ContactDetail",
        ("get", "/api/contacts/{pid}/duplicates") => "ContactDuplicates",
        ("post", "/api/contacts/{pid}/merge") => "ContactMerge",
        ("post", "/api/contacts/{pid}/unmerge") => "ContactUnmerge",
        ("get", "/api/contacts/{pid}/merges") => "ContactMerges",
        (
            "post",
            "/api/contacts/{pid}/external-links"
            | "/api/accounts/{pid}/external-links"
            | "/api/deals/{pid}/external-links"
            | "/api/leads/{pid}/external-links",
        ) => "ExternalLink",
        (
            "get",
            "/api/contacts/{pid}/external-links"
            | "/api/accounts/{pid}/external-links"
            | "/api/deals/{pid}/external-links"
            | "/api/leads/{pid}/external-links",
        ) => "ExternalLinks",
        ("get", "/api/accounts/{pid}") => "AccountDetail",
        ("get", "/api/accounts/{pid}/clv") => "Clv",
        ("get", "/api/leads/{pid}") => "LeadDetail",
        ("get", "/api/tickets/{pid}") => "TicketDetail",
        ("get", "/api/groups/{pid}") => "Group",
        ("get", "/api/forecast") => "Forecast",
        ("post", "/api/forecast/snapshot") => "Snapshot",
        ("get", "/api/dashboards/sales") => "SalesDashboard",
        ("get", "/api/dashboards/sla") => "SlaDashboard",
        ("get", "/api/dashboards/activity") => "ActivityDashboard",
        ("get", "/api/segments/{pid}/preview") => "Preview",
        ("get", "/api/campaigns/{pid}/deliveries") => "Deliveries",
        ("get", "/api/campaigns/{pid}/funnel") => "CampaignFunnel",
        ("post", "/api/nurture/advance") => "NurtureSweep",
        ("post", "/api/sla/sweep") => "SlaSweep",
        ("get", "/api/contacts/{pid}/subject-access") => "SubjectAccessExport",
        ("post", "/api/contacts/{pid}/erase") => "Erased",
        ("get", "/api/retention") => "RetentionReport",
        ("post", "/api/retention/sweep") => "RetentionSweep",
        ("post", "/api/break-glass") => "Grant",
        ("get", "/api/break-glass") => "Grants",
        ("post", "/api/break-glass/{pid}/review") => "Reviewed",
        ("get", "/api/audits/verify") => "ChainVerdict",
        ("get", "/api/insights/stale-deals") => "StaleDeals",
        ("get", "/api/insights/followups") => "Followups",
        ("get", "/api/insights/pipeline-hygiene") => "HygieneFindings",
        ("get", "/api/insights/executive") => "ExecutivePack",
        ("get", "/api/insights/forecast-trends") => "ForecastTrends",
        ("get", "/api/insights/sla") => "SlaRegister",
        ("get", "/api/insights/dpo") => "DpoView",
        ("get", "/api/insights/cadence") => "Cadence",
        ("get", "/api/insights/engagement") => "EngagementView",
        ("get", "/api/insights/funnel") => "StageFunnel",
        ("get", "/api/insights/members") => "MembersView",
        ("get", "/api/insights/consent-by-account") => "ConsentByAccount",
        ("get", "/api/insights/stakeholders") => "StakeholderRegister",
        ("get", "/api/insights/partnerships") => "PartnershipsRegister",
        ("get", "/api/insights/memberships") => "MembershipsView",
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
        "/api/nurture-sequences" => "SequenceView",
        "/api/audits/recent" | "/api/audits" | "/api/audits/{entity_pid}" => "AuditEntry",
        "/api/events/recent" => "Event",
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
            if let Some(name) = response_schema(method, path) {
                operation["responses"]["200"]["content"] = json_body(reference(name));
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
                        response["content"] = json!({
                            "application/problem+json": { "schema": reference("Error") } });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generated;

    #[test]
    fn request_schemas_come_from_the_payload_structs() {
        let all = generated();
        let contact = &all["ContactPayload"];
        assert_eq!(
            contact["required"],
            serde_json::json!(["person_ref", "display_name"])
        );
        assert_eq!(
            contact["additionalProperties"], false,
            "deny_unknown_fields"
        );
        // A defaulted member is optional and documents its default.
        assert_eq!(contact["properties"]["preferred_channel"]["type"], "string");
        assert!(contact["properties"]["preferred_channel"]["default"].is_string());
        assert_eq!(contact["properties"]["account_pid"]["format"], "uuid");
        assert_eq!(
            all["AccountPayload"]["required"],
            serde_json::json!(["organization_ref", "display_name"])
        );
    }

    /// Every `$ref` anywhere in the schemas names a schema that exists.
    #[test]
    fn every_reference_resolves() {
        fn refs(value: &serde_json::Value, found: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    for (key, inner) in map {
                        match (key.as_str(), inner) {
                            ("$ref", serde_json::Value::String(r)) => found.push(r.clone()),
                            _ => refs(inner, found),
                        }
                    }
                }
                serde_json::Value::Array(items) => items.iter().for_each(|v| refs(v, found)),
                _ => {}
            }
        }
        let all = super::schemas();
        let mut found = Vec::new();
        refs(&all, &mut found);
        assert!(found.iter().any(|r| r.ends_with("/StageInput")));
        for r in found {
            let name = r
                .strip_prefix("#/components/schemas/")
                .unwrap_or_else(|| panic!("{r}"));
            assert!(all.get(name).is_some(), "dangling reference {r}");
        }
    }
}
