//! Request and response schemas for the `OpenAPI` document (CRM-T49, CRM-T83).
//!
//! Every schema but one is generated from the Rust type that is sent or
//! received: the `*Payload` structs for requests, the `SeaORM` models for
//! records, and the typed response structs in the controllers for everything
//! else (see [`generated`]). A field is required when the type has no default
//! and is not an `Option`; a member that masking can blank (an amount, a total,
//! a campaign cost) is an `Option` and is `null` when masked, never absent and
//! never zero. The exception is `Error`, the problem document, which the
//! `problem_mw` layer builds from whatever error was produced.
//!
//! The DB-gated tests in `tests/requests/openapi_schemas.rs` validate real
//! requests and responses against these schemas, so drift fails a test.

use serde_json::{Value, json};

fn string() -> Value {
    json!({ "type": "string" })
}
fn integer() -> Value {
    json!({ "type": "integer" })
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
#[allow(clippy::too_many_lines)] // one list of the documented types
fn generated() -> serde_json::Map<String, Value> {
    use crate::controllers::{
        audits, board, break_glass, dashboards, engagement, external_links, insights, lead_imports,
        marketing, merge, notifications, privacy, relationships, sales, search, support, unmerge,
        views, webhooks,
    };
    // A request is read as the service deserializes it (an optional member may be left out); a
    // response is written as it serializes (an optional member is always there, as `null`).
    let mut requests = schemars::generate::SchemaSettings::openapi3()
        .for_deserialize()
        .into_generator();
    let mut generator = schemars::generate::SchemaSettings::openapi3()
        .for_serialize()
        .into_generator();
    macro_rules! document {
        ($generator:ident: $($ty:ty),* $(,)?) => {$( $generator.subschema_for::<$ty>(); )*};
    }
    document!(
        requests:
        relationships::ContactPayload,
        relationships::AccountPayload,
        relationships::RepointPayload,
        relationships::ActivityPayload,
        crate::controllers::StatusPayload,
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
        webhooks::SubscriptionPayload,
        marketing::consent::ConsentRequestPayload,
        break_glass::ReviewPayload,
        marketing::consent::ConsentPayload,
        marketing::segment::SegmentPayload,
        marketing::nurture::SequencePayload,
        marketing::nurture::StepPayload,
        marketing::nurture::EnrollPayload,
        marketing::campaign::CampaignPayload,
    );
    document!(
        generator:
        relationships::ContactDetail,
        relationships::AccountDetail,
        relationships::ContactDuplicates,
        merge::ContactMerge,
        sales::LeadDetail,
        privacy::Erased,
        privacy::RetentionReport,
        privacy::RetentionSweep,
        break_glass::Grant,
        marketing::consent::ConsentPurposes,
        marketing::consent::ConsentRequested,
        marketing::consent::PreferencesLink,
        crate::consents::PurposeOutcome,
        webhooks::SubscriptionCreated,
        webhooks::SecretRotated,
        crate::webhooks::Report,
        webhooks::SubscriptionView,
        webhooks::DeliveryView,
        break_glass::Grants,
        break_glass::Reviewed,
        audits::ChainVerdict,
        marketing::segment::Preview,
        marketing::nurture::NurtureSweep,
        marketing::campaign::Deliveries,
        marketing::campaign::CampaignFunnel,
        dashboards::ActivityDashboard,
        insights::sales::StaleDeals,
        crate::controllers::PidRef,
        marketing::nurture::SequenceView,
        privacy::SubjectAccessExport,
        crate::streaming::EventView,
        crate::models::_entities::consent_events::Model,
        crate::models::_entities::nurture_enrollments::Model,
        crate::models::_entities::nurture_sequences::Model,
        crate::models::_entities::nurture_steps::Model,
        insights::relationships::StakeholderRegister,
        insights::relationships::PartnershipsRegister,
        insights::relationships::MembershipsView,
        insights::support::SlaRegister,
        insights::compliance::DpoView,
        insights::compliance::ConsentByAccount,
        insights::outreach::Cadence,
        insights::outreach::EngagementView,
        insights::outreach::StageFunnel,
        insights::outreach::MembersView,
        insights::sales::Followups,
        insights::sales::HygieneFindings,
        insights::sales::ExecutivePack,
        insights::sales::ForecastTrends,
        dashboards::SalesDashboard,
        dashboards::SlaDashboard,
        dashboards::Clv,
        crate::models::_entities::audit_logs::Model,
        crate::rules::analytics::Ratio,
        support::TicketDetail,
        support::SlaSweep,
        notifications::Notification,
        search::SearchHit,
        views::View,
        views::ViewPayload,
        views::SortSpec,
        lead_imports::Preview,
        lead_imports::ImportRecord,
        lead_imports::Undone,
        lead_imports::PreviewPayload,
        lead_imports::CommitPayload,
        notifications::UnreadCount,
        notifications::MarkedRead,
        crate::notifications::Outcome,
        engagement::Group,
        crate::models::_entities::working_groups::Model,
        sales::PipelineWithStages,
        sales::Forecast,
        crate::models::_entities::pipelines::Model,
        crate::models::_entities::pipeline_stages::Model,
        crate::models::_entities::forecast_snapshots::Model,
        unmerge::ContactUnmerge,
        unmerge::ContactMerges,
        external_links::ExternalLinks,
        crate::models::_entities::external_links::Model,
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
    );
    let mut definitions = requests.take_definitions(true);
    definitions.extend(generator.take_definitions(true));
    definitions
}

/// `components.schemas`.
#[must_use]
pub fn schemas() -> Value {
    let mut all = base_schemas();
    if let Some(all) = all.as_object_mut() {
        all.extend(generated());
    }
    all
}

/// The one schema that is not generated: the problem document is built by the
/// `problem_mw` layer from whatever error a handler, guard or extractor produced.
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
        // ---- requests
        // ---- resources (what the lists return)
    })
}

/// The request schema for `(method, path)`, if the operation takes a body.
#[must_use]
pub fn request_schema(method: &str, path: &str) -> Option<&'static str> {
    Some(match (method, path) {
        ("post", "/api/contacts") => "ContactPayload",
        ("post", "/api/contacts/{pid}/repoint") => "RepointPayload",
        ("post", "/api/contacts/{pid}/consent") => "ConsentPayload",
        ("post", "/api/contacts/{pid}/consent/request") => "ConsentRequestPayload",
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
        ("post", "/api/views") | ("put", "/api/views/{pid}") => "ViewPayload",
        ("post", "/api/lead-imports/preview") => "LeadImportPreviewPayload",
        ("post", "/api/lead-imports") => "LeadImportCommitPayload",
        ("post", "/api/articles") | ("put", "/api/articles/{pid}") => "ArticlePayload",
        ("post", "/api/break-glass") => "GrantPayload",
        ("post", "/api/webhook-subscriptions") => "WebhookSubscriptionPayload",
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
        ("post", "/api/notifications/sweep") => "NotificationSweep",
        ("post", "/api/lead-imports/preview") => "LeadImportPreview",
        ("post", "/api/lead-imports") | ("get", "/api/lead-imports/{pid}") => "LeadImport",
        ("post", "/api/lead-imports/{pid}/undo") => "LeadImportUndone",
        ("get", "/api/notifications/unread-count") => "UnreadCount",
        ("post", "/api/notifications/read-all") => "MarkedRead",
        ("put", "/api/notifications/{pid}/read") => "Notification",
        ("get", "/api/contacts/{pid}/subject-access") => "SubjectAccessExport",
        ("post", "/api/contacts/{pid}/erase") => "Erased",
        ("get", "/api/retention") => "RetentionReport",
        ("post", "/api/retention/sweep") => "RetentionSweep",
        ("post", "/api/break-glass") => "Grant",
        ("get", "/api/contacts/{pid}/consents") => "ConsentPurposes",
        ("post", "/api/contacts/{pid}/consent/request") => "ConsentRequested",
        ("post", "/api/contacts/{pid}/preferences-link") => "PreferencesLink",
        ("post", "/api/webhook-subscriptions") => "WebhookSubscriptionCreated",
        ("post", "/api/webhook-subscriptions/relay") => "WebhookRelay",
        ("post", "/api/webhook-subscriptions/{pid}/rotate-secret") => "WebhookSecretRotated",
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
        "/api/notifications" => "Notification",
        "/api/search" => "SearchHit",
        "/api/views" => "View",
        "/api/lead-imports" => "LeadImport",
        "/api/webhook-subscriptions" => "WebhookSubscription",
        "/api/webhook-subscriptions/{pid}/deliveries" => "WebhookDelivery",
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
