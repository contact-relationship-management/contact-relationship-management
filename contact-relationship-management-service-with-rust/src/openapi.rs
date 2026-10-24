//! The `OpenAPI` 3 description of the CRM REST API, built from the service itself (CRM-T83).
//!
//! The paths and verbs come from the mounted router ([`crate::app::app_routes`]), so a route
//! cannot be missing. Each `/api/` operation's tag and one-line summary come from
//! [`crate::openapi_operations`]; the few public routes outside `/api/` are described by hand
//! in [`public_operations`] because they carry their own bodies and no authentication. Request
//! and response bodies come from the Rust types ([`crate::openapi_schemas`]). The field-by-field
//! meaning lives in the spec (`../spec/domain-model.md`).

use serde_json::{Value, json};

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

/// The public routes outside `/api/`: the unsubscribe page, the provider webhooks and the
/// metrics endpoint. They carry their own request bodies and no bearer token.
#[allow(clippy::too_many_lines)] // one literal
fn public_operations() -> Value {
    let ok = |desc: &str| json!({ "200": { "description": desc } });
    json!({
            "/unsubscribe/{token}": {
                "get": { "tags": ["marketing"], "summary": "Public: the page an email's unsubscribe link opens; changes nothing", "security": [], "responses": ok("HTML page") },
                "post": { "tags": ["marketing"], "summary": "Public: withdraw marketing consent through a signed link (RFC 8058 one-click); idempotent, body ignored", "security": [], "responses": ok("HTML page") }
            },
            "/confirm/{token}": {
                "get": { "tags": ["marketing"], "summary": "Public: the page a double opt-in message links to (CRM-R29); changes nothing", "security": [], "responses": ok("HTML page") },
                "post": { "tags": ["marketing"], "summary": "Public: confirm the purpose the signed link names (pending becomes granted, source double_opt_in); a link whose purpose was withdrawn or never requested grants nothing", "security": [], "responses": ok("HTML page") }
            },
            "/preferences/{token}": {
                "get": { "tags": ["marketing"], "summary": "Public: each consent purpose and its current state, with a button to start or stop it (CRM-R29)", "security": [], "responses": ok("HTML page") },
                "post": { "tags": ["marketing"], "summary": "Public: grant or withdraw one purpose; form fields purpose and action (granted or withdrawn); recorded as a consent event with the source preference_page", "security": [], "requestBody": { "required": true, "content": { "application/x-www-form-urlencoded": { "schema": { "type": "object", "required": ["purpose", "action"], "properties": { "purpose": { "type": "string", "enum": ["marketing", "newsletter", "events", "product_updates"] }, "action": { "type": "string", "enum": ["granted", "withdrawn"] } } } } } }, "responses": ok("HTML page") }
            },
            "/forms/{slug}": {
                "post": { "tags": ["marketing"], "summary": "Public web form (CRM-R31, CRM_WEB_FORMS): creates a contact and a source=web lead for the person the person service returns and asks for the form's consent purpose by double opt-in. The request's Origin must be on the form's list. Answers 202 with the same body whether or not anything was created. 403 origin not allowed, 404 no such form, 422 name or email unusable, 429 over the rate limit (Retry-After), 503 person service unavailable", "security": [], "parameters": [{ "name": "slug", "in": "path", "required": true, "schema": { "type": "string" } }], "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "required": ["name", "email"], "properties": { "name": { "type": "string", "maxLength": 200 }, "email": { "type": "string", "maxLength": 254 }, "website": { "type": "string", "description": "Honeypot: leave empty. A filled value creates nothing." } } } } } }, "responses": { "202": { "description": "Received" } } },
                "options": { "tags": ["marketing"], "summary": "Public: the browser preflight for the form, answered only for an allowed Origin", "security": [], "parameters": [{ "name": "slug", "in": "path", "required": true, "schema": { "type": "string" } }], "responses": { "204": { "description": "Allowed" } } }
            },
            "/webhooks/email": { "post": { "tags": ["marketing"], "summary": "Public, HMAC-signed (X-CRM-Timestamp, X-CRM-Signature): a provider outcome for a message (delivered, bounce_hard, bounce_soft, complaint). Hard bounces and complaints suppress the mailbox; a complaint also withdraws consent. Each event is applied once", "security": [], "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "required": ["message_id", "event"], "properties": { "message_id": { "type": "string" }, "event": { "type": "string", "enum": ["delivered", "bounce_hard", "bounce_soft", "complaint"] } } } } } }, "responses": ok("Result") } },
            "/webhooks/email/{provider}": { "post": { "tags": ["marketing"], "summary": "Public: a callback in the vendor's own format (provider is brevo, mailjet, ses, azure, mailgun or sendgrid). Authenticated by CRM_WEBHOOK_SECRET as a bearer token, basic-auth password or X-CRM-Webhook-Token; Mailgun by its own body signature (CRM_MAILGUN_SIGNING_KEY). Delivered, hard or soft bounce and complaints are applied as for /webhooks/email; other events are ignored", "security": [], "parameters": [{ "name": "provider", "in": "path", "required": true, "schema": { "type": "string", "enum": ["brevo", "mailjet", "ses", "azure", "mailgun", "sendgrid"] } }], "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object" } } } }, "responses": ok("Result") } },
            "/metrics.prom": { "get": { "tags": ["ops"], "summary": "Prometheus metrics (public)", "responses": ok("Exposition text") } }
    })
}

/// The operation for `(method, path)`, or `None` when the document does not describe it
/// (probes and the documentation endpoints themselves).
fn operation(method: &str, path: &str, public: &Value) -> Option<Value> {
    use crate::openapi_operations::{Kind, OPERATIONS};
    if !path.starts_with("/api/") {
        return public.get(path)?.get(method).cloned();
    }
    let shaped = shape(path);
    let (_, _, tag, summary, kind) = OPERATIONS
        .iter()
        .find(|(m, p, ..)| *m == method && shape(p) == shaped)?;
    let responses = match kind {
        Kind::Ok => json!({ "200": { "description": "OK" } }),
        Kind::Created => json!({
            "200": { "description": "Created: {pid}" },
            "422": { "description": "Validation failure" }
        }),
        Kind::Transition => json!({
            "200": { "description": "The updated record" },
            "422": { "description": "Illegal transition (names the current state)" }
        }),
    };
    Some(json!({ "tags": [tag], "summary": summary, "responses": responses }))
}

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    let public = public_operations();
    let mut paths = serde_json::Map::new();
    for route in crate::app::app_routes().collect() {
        for method in route.actions {
            let method = method.as_str().to_lowercase();
            if let Some(operation) = operation(&method, &route.uri, &public) {
                paths.entry(route.uri.clone()).or_insert_with(|| json!({}))[method] = operation;
            }
        }
    }
    let mut doc = json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Contact Relationship Management Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "CRM across four modules: sales automation (contacts/accounts over person:/organization: URNs, rule-scored leads, deal pipelines with Kanban, stage-weighted forecasting), consent-first marketing automation (segments, campaigns, nurture), support (tickets + SLA + knowledge base), analytics (derived, per-currency, honest ratios). Money is minor units + ISO-4217. Validation failures return 422. API version is negotiated with the Accepts-version header (1.0)."
        },
        "paths": paths,
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

    use super::shape;

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
    const BODYLESS: [&str; 14] = [
        "post /unsubscribe/{}",
        "post /api/contacts/{}/erase",
        "post /api/campaigns/{}/run",
        "post /api/nurture/advance",
        "post /api/retention/sweep",
        "post /api/forecast/snapshot",
        "post /api/sla/sweep",
        "put /api/activities/{}/done",
        "post /api/webhook-subscriptions/relay",
        "post /api/notifications/read-all",
        "post /api/notifications/sweep",
        "put /api/notifications/{}/read",
        "post /api/contacts/{}/preferences-link",
        "post /api/webhook-subscriptions/{}/rotate-secret",
    ];

    fn mounted() -> BTreeMap<String, BTreeSet<String>> {
        let mut routes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for route in crate::app::app_routes().collect() {
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
    #[test]
    fn every_mounted_route_is_documented_and_the_reverse() {
        let doc = super::spec();
        let mut documented: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (path, item) in doc["paths"].as_object().unwrap() {
            let entry = documented.entry(shape(path)).or_default();
            for method in item.as_object().unwrap().keys() {
                entry.insert(method.clone());
            }
        }
        let mounted = mounted();
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

    /// Every entry of the operations table names a mounted route, so a removed or renamed route
    /// leaves no stale summary behind. (The other direction, a route with no entry, is the test
    /// above: an operation with no entry is not in the document.)
    #[test]
    fn every_table_entry_is_a_mounted_route() {
        let mounted = mounted();
        let stale: Vec<String> = crate::openapi_operations::OPERATIONS
            .iter()
            .filter(|(method, path, ..)| {
                !mounted
                    .get(&shape(path))
                    .is_some_and(|methods| methods.contains(*method))
            })
            .map(|(method, path, ..)| format!("{method} {path}"))
            .collect();
        assert!(stale.is_empty(), "no such route: {stale:?}");
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
            "/api/deals/{pid}/position",
            "/api/activities/{pid}",
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
