//! Hand-written `OpenAPI` 3 description of the CRM REST API.
//!
//! Summary-level by design: every path and verb is present with its
//! request/response essentials; the full field-by-field shapes live in
//! the spec (`../spec/domain-model.md`).

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
#[allow(clippy::too_many_lines)] // one literal document
pub fn spec() -> Value {
    let ok = |desc: &str| json!({ "200": { "description": desc } });
    let created = json!({
        "200": { "description": "Created: {pid}" },
        "422": { "description": "Validation failure" }
    });
    let transition = json!({
        "200": { "description": "The updated record" },
        "422": { "description": "Illegal transition (names the current state)" }
    });
    let mut doc = json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Contact Relationship Management Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "CRM across four modules: sales automation (contacts/accounts over person:/organization: URNs, rule-scored leads, deal pipelines with Kanban, stage-weighted forecasting), consent-first marketing automation (segments, campaigns, nurture), support (tickets + SLA + knowledge base), analytics (derived, per-currency, honest ratios). Money is minor units + ISO-4217. Validation failures return 422. API version is negotiated with the Accepts-version header (1.0)."
        },
        "paths": {
            "/api/contacts": {
                "post": { "tags": ["relationships"], "summary": "Create a contact (person: URN wrapper; consent starts never)", "responses": created },
                "get": { "tags": ["relationships"], "summary": "List contacts", "responses": ok("Contacts") }
            },
            "/api/contacts/{pid}": { "get": { "tags": ["relationships"], "summary": "Contact + merged timeline (activities, deals, tickets)", "responses": ok("ContactDetail") } },
            "/api/contacts/{pid}/repoint": { "post": { "tags": ["relationships"], "summary": "Repoint the wrapper after an upstream merge (reasoned, audited)", "responses": ok("Contact") } },
            "/api/accounts": {
                "post": { "tags": ["relationships"], "summary": "Create an account (organization: URN wrapper)", "responses": created },
                "get": { "tags": ["relationships"], "summary": "List accounts", "responses": ok("Accounts") }
            },
            "/api/accounts/{pid}": { "get": { "tags": ["relationships"], "summary": "Account + contacts + deals", "responses": ok("AccountDetail") } },
            "/api/accounts/{pid}/clv": { "get": { "tags": ["analytics"], "summary": "CLV per currency over won deals", "responses": ok("Clv") } },
            "/api/activities": {
                "post": { "tags": ["relationships"], "summary": "Log an interaction (stamps ticket first-response for the assignee)", "responses": created },
                "get": { "tags": ["relationships"], "summary": "Activity feed (?subject_kind=&subject_pid=)", "responses": ok("Activities") }
            },
            "/api/activities/{pid}/done": { "put": { "tags": ["relationships"], "summary": "Tick a task activity", "responses": ok("Activity") } },
            "/api/leads": {
                "post": { "tags": ["sales"], "summary": "Capture a lead (scored immediately, breakdown returned)", "responses": created },
                "get": { "tags": ["sales"], "summary": "The queue, score-sorted", "responses": ok("Leads") }
            },
            "/api/leads/{pid}": { "get": { "tags": ["sales"], "summary": "Lead + live score breakdown", "responses": ok("LeadDetail") } },
            "/api/leads/{pid}/status": { "post": { "tags": ["sales"], "summary": "Lead transition; converted creates/links the contact (+ optional deal) in one transaction", "responses": transition } },
            "/api/pipelines": {
                "post": { "tags": ["sales"], "summary": "Create a pipeline with ordered stages (probabilities + terminal flags)", "responses": created },
                "get": { "tags": ["sales"], "summary": "Pipelines with stages", "responses": ok("Pipelines") }
            },
            "/api/deals": {
                "post": { "tags": ["sales"], "summary": "Open a deal in the pipeline's first stage (minor units)", "responses": created },
                "get": { "tags": ["sales"], "summary": "Deals (?pipeline=)", "responses": ok("Deals") }
            },
            "/api/deals/{pid}/stage": { "post": { "tags": ["sales"], "summary": "Kanban stage move (pipeline-membership checked; terminal closes; lost needs a reason; row-locked)", "responses": transition } },
            "/api/deals/{pid}/reopen": { "post": { "tags": ["sales"], "summary": "Reasoned reopen of a closed deal", "responses": transition } },
            "/api/forecast": { "get": { "tags": ["sales"], "summary": "Live stage-weighted forecast per currency (derived, never typed)", "responses": ok("Forecast") } },
            "/api/forecast/snapshot": { "post": { "tags": ["sales"], "summary": "Freeze the current roll-up", "responses": ok("Snapshot") } },
            "/api/contacts/{pid}/consent": {
                "post": { "tags": ["marketing"], "summary": "Record consent (withdrawal exits nurture + blocks sends)", "responses": ok("Recorded") },
                "get": { "tags": ["marketing"], "summary": "The append-only history (audited read)", "responses": ok("ConsentEvents") }
            },
            "/api/segments": {
                "post": { "tags": ["marketing"], "summary": "Create a segment (consent ANDed structurally)", "responses": created },
                "get": { "tags": ["marketing"], "summary": "List segments", "responses": ok("Segments") }
            },
            "/api/segments/{pid}/preview": { "get": { "tags": ["marketing"], "summary": "Count + sample before scheduling", "responses": ok("Preview") } },
            "/api/campaigns": {
                "post": { "tags": ["marketing"], "summary": "Create a campaign (email kind; cost in minor units)", "responses": created },
                "get": { "tags": ["marketing"], "summary": "List campaigns", "responses": ok("Campaigns") }
            },
            "/api/campaigns/{pid}/status": { "post": { "tags": ["marketing"], "summary": "Campaign transition (draft→scheduled→running→completed|cancelled)", "responses": transition } },
            "/api/campaigns/{pid}/run": { "post": { "tags": ["marketing"], "summary": "Simulated send (demo mode): consent re-checked at send time; touch activities; deterministic counters", "responses": transition } },
            "/api/campaigns/{pid}/funnel": { "get": { "tags": ["marketing"], "summary": "Funnel + ROI (null on zero cost; per-currency honesty)", "responses": ok("Funnel") } },
            "/api/nurture-sequences": {
                "post": { "tags": ["marketing"], "summary": "Create a drip sequence (ordered delayed steps)", "responses": created },
                "get": { "tags": ["marketing"], "summary": "Sequences + steps + active enrolments", "responses": ok("Sequences") }
            },
            "/api/nurture-sequences/{pid}/enrollments": { "post": { "tags": ["marketing"], "summary": "Enrol a consented contact", "responses": created } },
            "/api/nurture/advance": { "post": { "tags": ["marketing"], "summary": "The idempotent advance sweep (send due steps; complete; exit unconsented)", "responses": ok("Sweep result") } },
            "/api/sla-policies": {
                "post": { "tags": ["support"], "summary": "Set a priority's SLA targets", "responses": created },
                "get": { "tags": ["support"], "summary": "List policies", "responses": ok("Policies") }
            },
            "/api/tickets": {
                "post": { "tags": ["support"], "summary": "Open a ticket (deadlines derive from the priority's policy)", "responses": created },
                "get": { "tags": ["support"], "summary": "The queue with live breach flags (?status=)", "responses": ok("Tickets") }
            },
            "/api/tickets/{pid}": { "get": { "tags": ["support"], "summary": "Ticket + activities", "responses": ok("TicketDetail") } },
            "/api/tickets/{pid}/status": { "post": { "tags": ["support"], "summary": "Ticket transition (resolved stamps resolved_at; reopen clears it)", "responses": transition } },
            "/api/tickets/{pid}/priority": { "put": { "tags": ["support"], "summary": "Audited priority change (re-derives the deadlines)", "responses": ok("Ticket") } },
            "/api/sla/sweep": { "post": { "tags": ["support"], "summary": "Persist breach facts + emit sla_breached once per breach", "responses": ok("Sweep result") } },
            "/api/articles": {
                "post": { "tags": ["support"], "summary": "Draft a KB article", "responses": created },
                "get": { "tags": ["support"], "summary": "Search articles (?q=)", "responses": ok("Articles") }
            },
            "/api/articles/{pid}": { "put": { "tags": ["support"], "summary": "Edit (published edits bump the version)", "responses": ok("Article") } },
            "/api/articles/{pid}/status": { "post": { "tags": ["support"], "summary": "Publish / archive", "responses": transition } },
            "/api/dashboards/sales": { "get": { "tags": ["analytics"], "summary": "Win rate + pipeline by stage (per currency; ETag conditional)", "responses": ok("SalesDashboard") } },
            "/api/dashboards/sla": { "get": { "tags": ["analytics"], "summary": "Open tickets by priority × breach (ETag conditional)", "responses": ok("SlaDashboard") } },
            "/api/dashboards/activity": { "get": { "tags": ["analytics"], "summary": "Activity counts by kind (?days=)", "responses": ok("ActivityDashboard") } },
            "/api/contacts/{pid}/subject-access": { "get": { "tags": ["privacy"], "summary": "GDPR subject-access export: every table keyed to the contact, in one audited document (exclusions named)", "responses": ok("SubjectAccessExport") } },
            "/api/contacts/{pid}/erase": { "post": { "tags": ["privacy"], "summary": "Erase (anonymise): refused 422 while an open deal, open ticket, or active nurture enrolment exists; destructive-classified", "responses": transition } },
            "/api/retention": { "get": { "tags": ["privacy"], "summary": "Retention report: soft-deleted rows past the floored horizon, per table", "responses": ok("RetentionReport") } },
            "/api/retention/sweep": { "post": { "tags": ["privacy"], "summary": "Hard-delete soft-deleted rows past the horizon; destructive-classified, audited with counts", "responses": ok("Sweep result") } },
            "/api/break-glass": { "post": { "tags": ["break-glass"], "summary": "Take a time-limited read elevation: written reason (20+ chars), 1-60 minutes (default 30); audited; any authenticated caller", "responses": ok("Grant") }, "get": { "tags": ["break-glass"], "summary": "Grants newest first with the unreviewed count; admin/service tier", "responses": ok("Grants") } },
            "/api/break-glass/{pid}/review": { "post": { "tags": ["break-glass"], "summary": "Record the after-the-fact review; refused 403 for the grantee, 422 if already reviewed; admin/service tier", "responses": transition } },
            "/api/audits/recent": { "get": { "tags": ["audit"], "summary": "Recent audit entries", "responses": ok("Audit entries") } },
            "/api/audits": { "get": { "tags": ["audit"], "summary": "Owner-scoped trail (?owner=&since=)", "responses": ok("Audit entries") } },
            "/api/audits/{entity_pid}": { "get": { "tags": ["audit"], "summary": "One record's audit trail", "responses": ok("Audit entries") } },
            "/api/events/recent": { "get": { "tags": ["events"], "summary": "Recent events (memory ring or outbox)", "responses": ok("Events") } },
            "/api/contacts/{pid}/stakeholder": { "put": { "tags": ["engagement"], "summary": "Declare a contact's stakeholder role, influence and interest", "responses": ok("Contact") } },
            "/api/accounts/{pid}/stakeholder": { "put": { "tags": ["engagement"], "summary": "Declare an account's stakeholder role", "responses": ok("Account") } },
            "/api/accounts/{pid}/partnerships": {
                "post": { "tags": ["engagement"], "summary": "Open a partnership with an account", "responses": created },
                "get": { "tags": ["engagement"], "summary": "An account's partnerships", "responses": ok("Partnerships") }
            },
            "/api/partnerships/{pid}/stage": { "post": { "tags": ["engagement"], "summary": "Partnership stage transition", "responses": transition } },
            "/api/accounts/{pid}/membership": { "put": { "tags": ["engagement"], "summary": "Create or update an account's membership", "responses": ok("Membership") } },
            "/api/groups": {
                "post": { "tags": ["engagement"], "summary": "Create a group", "responses": created },
                "get": { "tags": ["engagement"], "summary": "List groups", "responses": ok("Groups") }
            },
            "/api/groups/{pid}": { "get": { "tags": ["engagement"], "summary": "A group and its members", "responses": ok("Group") } },
            "/api/groups/{pid}/members": { "post": { "tags": ["engagement"], "summary": "Add a member to a group", "responses": created } },
            "/api/audits/verify": { "get": { "tags": ["audit"], "summary": "Recompute the audit hash chain; returns the row count, head hash and first break; admin/service tier", "responses": ok("Chain verdict") } },
            "/api/insights/stale-deals": { "get": { "tags": ["insights"], "summary": "Open deals by days in stage (?days=); audited read; cached 30 s", "responses": ok("Stale deals") } },
            "/api/insights/followups": { "get": { "tags": ["insights"], "summary": "Overdue and upcoming follow-ups", "responses": ok("Follow-ups") } },
            "/api/insights/pipeline-hygiene": { "get": { "tags": ["insights"], "summary": "Deals with missing amounts or close dates", "responses": ok("Hygiene findings") } },
            "/api/insights/executive": { "get": { "tags": ["insights"], "summary": "Executive pack (ETag conditional)", "responses": ok("Executive pack") } },
            "/api/insights/forecast-trends": { "get": { "tags": ["insights"], "summary": "Forecast snapshot series", "responses": ok("Trends") } },
            "/api/insights/sla": { "get": { "tags": ["insights"], "summary": "SLA register", "responses": ok("SLA register") } },
            "/api/insights/dpo": { "get": { "tags": ["insights"], "summary": "Data-protection view: consent coverage, withdrawals, duplicates (ETag conditional)", "responses": ok("DPO view") } },
            "/api/insights/cadence": { "get": { "tags": ["insights"], "summary": "Contact cadence", "responses": ok("Cadence") } },
            "/api/insights/engagement": { "get": { "tags": ["insights"], "summary": "Engagement view", "responses": ok("Engagement") } },
            "/api/insights/funnel": { "get": { "tags": ["insights"], "summary": "Stage funnel (?pipeline= required)", "responses": ok("Funnel") } },
            "/api/insights/members": { "get": { "tags": ["insights"], "summary": "Membership view", "responses": ok("Members") } },
            "/api/insights/consent-by-account": { "get": { "tags": ["insights"], "summary": "Consent coverage by account", "responses": ok("Consent by account") } },
            "/api/insights/stakeholders": { "get": { "tags": ["insights"], "summary": "Stakeholder register", "responses": ok("Stakeholders") } },
            "/api/insights/partnerships": { "get": { "tags": ["insights"], "summary": "Partnerships register", "responses": ok("Partnerships register") } },
            "/api/insights/memberships": { "get": { "tags": ["insights"], "summary": "Memberships view", "responses": ok("Memberships") } },
            "/metrics.prom": { "get": { "tags": ["ops"], "summary": "Prometheus metrics (public)", "responses": ok("Exposition text") } }
        },
        "components": { "securitySchemes": { "bearerAuth": {
            "type": "http", "scheme": "bearer", "bearerFormat": "PASETO v4.public",
            "description": "Required when the service runs with CRM_REQUIRE_AUTH (always in production); probes, metrics and docs are public."
        } } },
        "security": [{ "bearerAuth": [] }]
    });
    add_shared_errors(&mut doc);
    add_paging(&mut doc);
    add_idempotency(&mut doc);
    crate::openapi_schemas::attach(&mut doc);
    doc
}

/// The paginated lists (CRM-T51) and the largest page each allows.
const PAGED: [(&str, u64); 7] = [
    ("/api/contacts", 1000),
    ("/api/accounts", 500),
    ("/api/activities", 200),
    ("/api/leads", 1000),
    ("/api/deals", 2000),
    ("/api/tickets", 1000),
    ("/api/articles", 500),
];

/// Document the optional `Idempotency-Key` header (CRM-T52) on every `POST`
/// under `/api/`, and the statuses it adds.
fn add_idempotency(doc: &mut Value) {
    let Some(paths) = doc["paths"].as_object_mut() else {
        return;
    };
    for (path, item) in paths.iter_mut() {
        if !path.starts_with("/api/") {
            continue;
        }
        // `get_mut`, not indexing: indexing a missing key would insert a null.
        let Some(operation) = item.get_mut("post").filter(|o| o.is_object()) else {
            continue;
        };
        let mut parameters = operation["parameters"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        parameters.push(json!({ "name": "Idempotency-Key", "in": "header", "required": false,
            "description": "Opaque key, 1 to 255 visible ASCII characters. A retry with the same key and the same request replays the first successful response (header Idempotent-Replayed: true) instead of repeating the work; keys are scoped to the caller and kept 24 hours. Same key with a different request is 422; same key while the first is still running is 409 with Retry-After.",
            "schema": { "type": "string", "minLength": 1, "maxLength": 255 } }));
        operation["parameters"] = Value::Array(parameters);
        operation["responses"]["409"] = json!({ "description": "Idempotency-Key: the original request is still running; retry shortly" });
    }
}

/// Document `limit`, `cursor` and the `Link: rel="next"` header on each
/// paginated list.
fn add_paging(doc: &mut Value) {
    for (path, cap) in PAGED {
        // `get_mut`, not indexing: indexing a missing key would insert a null.
        let Some(operation) = doc["paths"]
            .get_mut(path)
            .and_then(|item| item.get_mut("get"))
            .filter(|o| o.is_object())
        else {
            continue;
        };
        let mut parameters = operation["parameters"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        parameters.push(json!({ "name": "limit", "in": "query", "required": false,
            "description": format!("Rows per page, 1 to {cap}; default {cap}. Out of range is 422."),
            "schema": { "type": "integer", "minimum": 1, "maximum": cap } }));
        parameters.push(json!({ "name": "cursor", "in": "query", "required": false,
            "description": "Opaque position from a previous Link rel=next. Anything else is 422.",
            "schema": { "type": "string" } }));
        operation["parameters"] = Value::Array(parameters);
        operation["responses"]["200"]["headers"] = json!({ "Link": {
            "description": "Present when more rows exist: </path?...&cursor=...>; rel=\"next\". Follow it until it is absent.",
            "schema": { "type": "string" } } });
    }
}

/// Every operation can answer the cross-cutting statuses the middleware
/// produces; stating them once here keeps the paths above readable.
fn add_shared_errors(doc: &mut Value) {
    let shared = [
        ("401", "Missing, expired or invalid bearer token"),
        ("403", "Denied by the authorization policy"),
        ("406", "Unsupported Accepts-version"),
        ("429", "Rate limited or locked out; see Retry-After"),
    ];
    let Some(paths) = doc["paths"].as_object_mut() else {
        return;
    };
    for (path, item) in paths.iter_mut() {
        if !path.starts_with("/api/") {
            continue;
        }
        let Some(item) = item.as_object_mut() else {
            continue;
        };
        for operation in item.values_mut() {
            if let Some(responses) = operation["responses"].as_object_mut() {
                for (status, description) in shared {
                    responses
                        .entry(status)
                        .or_insert_with(|| json!({ "description": description }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use loco_rs::app::Hooks;

    /// `{pid}` and `{entity_pid}` are the same route shape.
    fn shape(path: &str) -> String {
        let mut out = String::new();
        let mut inside = false;
        for c in path.chars() {
            match c {
                '{' => {
                    inside = true;
                    out.push_str("{}");
                }
                '}' => inside = false,
                _ if !inside => out.push(c),
                _ => {}
            }
        }
        out
    }

    /// Routes that exist but are not part of the documented API surface:
    /// Loco's probes and the documentation endpoints themselves.
    const UNDOCUMENTED: [&str; 5] = [
        "/_ping",
        "/_health",
        "/_readiness",
        "/api-docs/openapi.json",
        "/swagger-ui",
    ];

    /// `POST`/`PUT` operations that deliberately take no body.
    const BODYLESS: [&str; 7] = [
        "post /api/contacts/{}/erase",
        "post /api/campaigns/{}/run",
        "post /api/nurture/advance",
        "post /api/retention/sweep",
        "post /api/forecast/snapshot",
        "post /api/sla/sweep",
        "put /api/activities/{}/done",
    ];

    async fn mounted() -> BTreeMap<String, BTreeSet<String>> {
        let ctx = loco_rs::tests_cfg::app::get_app_context().await;
        let mut routes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for route in crate::app::App::routes(&ctx).collect() {
            let entry = routes.entry(shape(&route.uri)).or_default();
            for method in route.actions {
                entry.insert(method.as_str().to_lowercase());
            }
        }
        routes
    }

    /// The route-coverage test: every mounted route and verb is in the
    /// document, and every documented operation is mounted, so neither side
    /// can drift without this failing.
    #[tokio::test]
    async fn every_mounted_route_is_documented_and_the_reverse() {
        let doc = super::spec();
        let mut documented: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (path, item) in doc["paths"].as_object().unwrap() {
            let entry = documented.entry(shape(path)).or_default();
            for method in item.as_object().unwrap().keys() {
                entry.insert(method.clone());
            }
        }
        let mounted = mounted().await;
        let mut problems = Vec::new();
        for (path, methods) in &mounted {
            if UNDOCUMENTED.contains(&path.as_str()) {
                continue;
            }
            for method in methods {
                if !documented.get(path).is_some_and(|m| m.contains(method)) {
                    problems.push(format!("mounted but undocumented: {method} {path}"));
                }
            }
        }
        for (path, methods) in &documented {
            for method in methods {
                if !mounted.get(path).is_some_and(|m| m.contains(method)) {
                    problems.push(format!("documented but not mounted: {method} {path}"));
                }
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// Every mutating operation under `/api/` has a request schema, or is on
    /// the explicit list of bodyless ones, so a new endpoint forces a decision.
    #[test]
    fn mutating_operations_have_a_request_schema_or_are_bodyless() {
        let doc = super::spec();
        let mut problems = Vec::new();
        for (path, item) in doc["paths"].as_object().unwrap() {
            if !path.starts_with("/api/") {
                continue;
            }
            for (method, operation) in item.as_object().unwrap() {
                if method != "post" && method != "put" {
                    continue;
                }
                let key = format!("{method} {}", shape(path));
                let has_body = operation["requestBody"].is_object();
                let bodyless = BODYLESS.contains(&key.as_str());
                if has_body == bodyless {
                    problems.push(format!(
                        "{key}: {}",
                        if has_body {
                            "has a request body but is listed as bodyless"
                        } else {
                            "no request schema and not listed as bodyless"
                        }
                    ));
                }
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// Every `$ref` points at a schema that exists.
    #[test]
    fn every_schema_reference_resolves() {
        fn walk(value: &serde_json::Value, names: &BTreeSet<String>, bad: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    for (key, child) in map {
                        if key == "$ref"
                            && let Some(target) = child.as_str()
                        {
                            let name = target.rsplit('/').next().unwrap_or("");
                            if !names.contains(name) {
                                bad.push(target.to_string());
                            }
                        }
                        walk(child, names, bad);
                    }
                }
                serde_json::Value::Array(items) => {
                    for child in items {
                        walk(child, names, bad);
                    }
                }
                _ => {}
            }
        }
        let doc = super::spec();
        let names: BTreeSet<String> = doc["components"]["schemas"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let mut bad = Vec::new();
        walk(&doc, &names, &mut bad);
        assert!(bad.is_empty(), "dangling references: {bad:?}");
    }

    use serde_json::json;

    /// The document parses, declares `OpenAPI` 3, and covers the mounted
    /// API surface (spot-checked against the route table).
    #[test]
    fn spec_shape() {
        let doc = super::spec();
        assert_eq!(doc["openapi"], "3.0.3");
        let paths = doc["paths"].as_object().unwrap();
        for p in [
            "/api/contacts",
            "/api/leads/{pid}/status",
            "/api/deals/{pid}/stage",
            "/api/campaigns/{pid}/run",
            "/api/nurture/advance",
            "/api/tickets/{pid}/priority",
            "/api/dashboards/sales",
            "/api/accounts/{pid}/clv",
            "/api/contacts/{pid}/subject-access",
            "/api/contacts/{pid}/erase",
            "/api/retention",
            "/api/retention/sweep",
            "/api/insights/stale-deals",
            "/api/insights/funnel",
            "/api/groups",
            "/api/partnerships/{pid}/stage",
            "/api/audits/verify",
        ] {
            assert!(paths.contains_key(p), "missing {p}");
        }
        // No operation may be null (indexing a missing key inserts one).
        for (path, item) in paths {
            for (method, operation) in item.as_object().unwrap() {
                assert!(operation.is_object(), "{method} {path} is not an object");
            }
        }
        assert_eq!(doc["security"][0]["bearerAuth"], json!([]));
        let contacts = &doc["paths"]["/api/contacts"]["get"];
        assert_eq!(contacts["parameters"][0]["name"], "limit");
        assert!(contacts["responses"]["200"]["headers"]["Link"].is_object());
        assert!(doc["paths"]["/api/pipelines"]["get"]["parameters"].is_null());
        assert_eq!(
            doc["paths"]["/api/contacts"]["post"]["parameters"][0]["name"],
            "Idempotency-Key"
        );
        assert!(doc["paths"]["/api/contacts"]["get"]["responses"]["409"].is_null());
        assert_eq!(
            doc["paths"]["/api/contacts"]["get"]["responses"]["429"]["description"],
            "Rate limited or locked out; see Retry-After"
        );
        assert!(doc["paths"]["/metrics.prom"]["get"]["responses"]["401"].is_null());
    }
}
