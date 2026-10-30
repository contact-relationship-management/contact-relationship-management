//! Auth-activation persona matrix over the live routes (CRM-R15).
//!
//! Each test changes the settings (`require_auth`, the key set, the policy) for its own length
//! with [`scoped`], which puts them back, so these share a process with the other request tests.
//! A throwaway Ed25519 key mints PASETO tokens + the matching key set in-process (no auth service
//! needed).
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via `config/test.yaml` / `DATABASE_URL`).
#![cfg(feature = "paseto")]

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use contact_relationship_management::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

use super::openapi_schemas::schema_check;
use crate::scoped_auth::scoped;

const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";
/// Throwaway Ed25519 seed — mints test tokens only, never a secret.
const SEED: [u8; 32] = [5; 32];

/// The published-key-set JSON + `kid` for the test key.
fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a PASETO `v4.public` with a given `sub` and ABAC attributes.
fn sign_as(kid: &str, sub: &str, attrs: &[(&str, &[&str])]) -> String {
    let attrs_map: serde_json::Map<String, Value> = attrs
        .iter()
        .map(|(key, values)| ((*key).to_string(), json!(values)))
        .collect();
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": sub,
        "email": "rep@example.com", "name": "Test Rep",
        "iss": ISSUER, "aud": AUDIENCE,
        "exp": iat + 10_000_000_000_i64, "iat": iat,
        "sid": "test-sid", "attrs": attrs_map,
    })
    .to_string();
    let keypair = SigningKey::from_bytes(&SEED).to_keypair_bytes();
    let key = Key::<64>::from(keypair);
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    builder.try_sign(&private).expect("sign")
}

/// The deployment policy (spec `auth.md` personas): machine peers do
/// everything; `access=write` writes; the record's own owner (`$sub`
/// matching `resource.owner`, CRM-T24) reads unmasked; any other
/// authenticated read is allowed but carries the `mask` obligation —
/// first-match-wins, so an unowned record (no `resource.owner` at
/// all) falls through the owner rule to this masked catch-all.
fn test_policy() -> String {
    json!({ "rules": [
        { "effect": "allow",
          "actions": ["read", "write", "delete", "destructive"],
          "when": { "svc": ["true"] } },
        { "effect": "deny", "actions": ["read"], "when": { "resource.status": ["inactive"] } },
        { "effect": "allow", "actions": ["read"], "when": { "resource.owner": ["$sub"] } },
        { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } },
        { "effect": "allow", "actions": ["read"], "when": { "env.break_glass": ["true"] } },
        { "effect": "allow", "actions": ["read"], "when": {}, "obligations": ["mask"] }
    ] })
    .to_string()
}

/// The activation matrix in one boot: public paths stay open, missing
/// tokens are 401, ABAC gates mutations, and the writer persona runs
/// a real consent-gated flow end-to-end under enforcement.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_gates_the_real_stack() {
    let (keys, kid) = keys_and_kid();
    // Settings, not the environment: the verifier and the policy are built from
    // them on first use, so this runs before anything touches either.
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let reader = sign_as(&kid, "reader-user", &[]);
    let writer = sign_as(&kid, "writer-user", &[("access", &["write"])]);
    let service = sign_as(&kid, "svc-user", &[("svc", &["true"])]);

    // Break-glass grants outlive a run when Redis is configured (they last 30
    // minutes there); a leftover grant would lift this test's masks.
    if let Ok(url) = std::env::var("CRM_REDIS_URL")
        && !url.is_empty()
    {
        let client = redis::Client::open(url).expect("CRM_REDIS_URL");
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .expect("redis connection");
        let _: () = redis::cmd("DEL")
            .arg("crm:bg:reader-user")
            .arg("crm:bg:svc-user")
            .arg("crm:bg:writer-user")
            .query_async(&mut conn)
            .await
            .expect("clear leftover grants");
    }

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let mut doc: Value = request.get("/api-docs/openapi.json").await.json();
        schema_check::to_json_schema(&mut doc);

        // A refusal is a problem document and names the accepted scheme.
        let refused = request.get("/api/contacts").await;
        assert_eq!(refused.status_code(), 401);
        assert_eq!(
            refused.headers().get("www-authenticate").map(|v| v.to_str().unwrap()),
            Some("Bearer")
        );
        let problem: Value = refused.json();
        schema_check::check(&doc, "Error", &problem, "401 without a token");
        assert_eq!(problem["status"], 401);

        // Public allow-list stays open without a token.
        assert_eq!(request.get("/_health").await.status_code(), 200);
        assert_eq!(request.get("/metrics.prom").await.status_code(), 200);

        // Protected: no token ⇒ 401; junk ⇒ 401.
        assert_eq!(request.get("/api/contacts").await.status_code(), 401);
        assert_eq!(
            request
                .get("/api/contacts")
                .add_header("authorization", "Bearer v4.public.junk")
                .await
                .status_code(),
            401
        );

        // Reader: GET allowed, POST 403.
        assert_eq!(
            request
                .get("/api/contacts")
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .post("/api/contacts")
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "person_ref": "person:x" }))
                .await
                .status_code(),
            403
        );

        // Writer: creates a contact, grants consent, reads history.
        let contact = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                "display_name": "Enforced Contact",
            }))
            .await;
        assert_eq!(contact.status_code(), 200);
        let contact_pid = contact.json::<Value>()["pid"].as_str().unwrap().to_string();
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/consent"))
                .add_header("authorization", bearer(&writer))
                .json(&json!({ "action": "granted", "source": "form" }))
                .await
                .status_code(),
            200
        );
        let history = request
            .get(&format!("/api/contacts/{contact_pid}/consent"))
            .add_header("authorization", bearer(&reader))
            .await;
        assert_eq!(history.status_code(), 200);
        assert_eq!(history.json::<Value>().as_array().unwrap().len(), 1);

        // CRM-T24: record-level ABAC on the contact/deal read paths.
        // The deal/contact's own owner reads unmasked; a stranger
        // (the `reader` persona above, whose `sub` matches no
        // `resource.owner`) reads the same records masked — `reader`
        // itself proves the "GET allowed" case is unaffected, and the
        // sub-below (`owner-user`) proves ownership actually changes
        // the outcome, not just the default policy shape.
        let owner_sub = uuid::Uuid::new_v4();
        let owner = sign_as(&kid, &owner_sub.to_string(), &[]);
        let owner_ref = format!("person:{owner_sub}");

        // A contact owned by `owner_sub`: the owner sees the real
        // `preferred_channel`; the stranger sees it masked.
        let owned_contact = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                "display_name": "Owned Contact",
                "owner_ref": owner_ref,
                "preferred_channel": "phone",
            }))
            .await;
        assert_eq!(owned_contact.status_code(), 200);
        let owned_contact_pid = owned_contact.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        let as_owner = request
            .get(&format!("/api/contacts/{owned_contact_pid}"))
            .add_header("authorization", bearer(&owner))
            .await;
        assert_eq!(as_owner.status_code(), 200);
        assert_eq!(
            as_owner.json::<Value>()["contact"]["preferred_channel"],
            "phone"
        );

        let as_stranger = request
            .get(&format!("/api/contacts/{owned_contact_pid}"))
            .add_header("authorization", bearer(&reader))
            .await;
        assert_eq!(as_stranger.status_code(), 200);
        assert_eq!(
            as_stranger.json::<Value>()["contact"]["preferred_channel"],
            contact_relationship_management::auth::MASKED
        );

        // A deal owned by `owner_sub`: the owner's list view sees the
        // real `amount_minor`; the stranger's sees it nulled.
        let pipeline = request
            .post("/api/pipelines")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "name": "Enforced Pipeline",
                "stages": [
                    { "name": "Open", "probability_percent": 50 },
                    { "name": "Won", "probability_percent": 100, "is_won": true },
                ],
            }))
            .await;
        assert_eq!(pipeline.status_code(), 200);
        let pipeline_pid = pipeline.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        let owned_deal = request
            .post("/api/deals")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "name": "Owned Deal",
                "pipeline_pid": pipeline_pid,
                "amount_minor": 500_000,
                "currency": "GBP",
                "owner_ref": owner_ref,
            }))
            .await;
        assert_eq!(owned_deal.status_code(), 200);
        let owned_deal_pid = owned_deal.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        let find_amount = |rows: &Value| -> Value {
            rows.as_array()
                .unwrap()
                .iter()
                .find(|row| row["pid"] == owned_deal_pid)
                .expect("the owned deal is in the list")["amount_minor"]
                .clone()
        };

        let deals_as_owner = request
            .get(&format!("/api/deals?pipeline={pipeline_pid}"))
            .add_header("authorization", bearer(&owner))
            .await;
        assert_eq!(deals_as_owner.status_code(), 200);
        assert_eq!(find_amount(&deals_as_owner.json::<Value>()), json!(500_000));

        let deals_as_stranger = request
            .get(&format!("/api/deals?pipeline={pipeline_pid}"))
            .add_header("authorization", bearer(&reader))
            .await;
        assert_eq!(deals_as_stranger.status_code(), 200);
        assert!(find_amount(&deals_as_stranger.json::<Value>()).is_null());

        // CRM-T32: an account's detail bundles its deals and contacts; they
        // follow the same masking as their own endpoints.
        let account = request
            .post("/api/accounts")
            .add_header("authorization", bearer(&writer))
            .json(&json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Bundled Account" }))
            .await;
        assert_eq!(account.status_code(), 200);
        let account_pid = account.json::<Value>()["pid"].as_str().unwrap().to_string();
        let bundled = request
            .post("/api/deals")
            .add_header("authorization", bearer(&writer))
            .json(&json!({ "name": "Bundled Deal", "pipeline_pid": pipeline_pid,
                           "amount_minor": 700_000, "currency": "GBP",
                           "account_pid": account_pid }))
            .await;
        assert_eq!(bundled.status_code(), 200);
        let masked_account: Value = request
            .get(&format!("/api/accounts/{account_pid}"))
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert_eq!(masked_account["deals"].as_array().unwrap().len(), 1);
        schema_check::check(&doc, "AccountDetail", &masked_account, "masked account");
        assert!(
            masked_account["deals"][0]["amount_minor"].is_null(),
            "{masked_account}"
        );
        let full_account: Value = request
            .get(&format!("/api/accounts/{account_pid}"))
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert_eq!(full_account["deals"][0]["amount_minor"], 700_000);

        // CRM-T38: the audit trail, event stream, and retention report are
        // admin/service-tier reads. An ordinary reader or writer is refused;
        // a service credential is not.
        for path in [
            "/api/audits/recent",
            "/api/audits/verify",
            "/api/events/recent",
            "/api/retention",
        ] {
            for token in [&reader, &writer] {
                assert_eq!(
                    request
                        .get(path)
                        .add_header("authorization", bearer(token))
                        .await
                        .status_code(),
                    403,
                    "{path} must be refused to a non-admin caller"
                );
            }
            assert_eq!(
                request
                    .get(path)
                    .add_header("authorization", bearer(&service))
                    .await
                    .status_code(),
                200,
                "{path} must be open to a service credential"
            );
        }

        // CRM-T32: aggregate money is masked for a masked reader (absent,
        // never zero) and visible to a service credential.
        let forecast_masked: Value = request
            .get("/api/forecast")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert!(
            forecast_masked["totals_minor"].is_null(),
            "{forecast_masked}"
        );
        assert!(forecast_masked["open_deals"].is_number());
        let forecast_full: Value = request
            .get("/api/forecast")
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert!(forecast_full["totals_minor"].is_object(), "{forecast_full}");
        schema_check::check(&doc, "Forecast", &forecast_full, "service forecast");
        let dash_masked: Value = request
            .get("/api/dashboards/sales")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        for stage in dash_masked["pipeline_by_stage"]
            .as_object()
            .unwrap()
            .values()
        {
            assert!(stage["totals_minor"].is_null(), "{stage}");
            assert!(stage["count"].is_number());
        }
        schema_check::check(&doc, "SalesDashboard", &dash_masked, "masked sales dashboard");

        // CRM-T32: the insights views follow the same masking.
        let exec_masked: Value = request
            .get("/api/insights/executive")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert!(
            exec_masked["won_value_by_currency_minor"].is_null(),
            "{exec_masked}"
        );
        assert!(exec_masked["deals_won"].is_number());
        schema_check::check(&doc, "ExecutivePack", &exec_masked, "masked executive pack");
        let exec_full: Value = request
            .get("/api/insights/executive")
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert!(
            exec_full["won_value_by_currency_minor"].is_object(),
            "{exec_full}"
        );
        assert_eq!(
            request
                .get("/api/insights/forecast-trends")
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            200
        );

        // CRM-T44: break-glass. A reasoned, time-limited grant lifts the mask
        // for reads; the grant is listed and reviewed by someone else.
        let short = request
            .post("/api/break-glass")
            .add_header("authorization", bearer(&reader))
            .json(&json!({ "reason": "because" }))
            .await;
        assert_eq!(short.status_code(), 422, "a thin reason is refused");
        let too_long = request
            .post("/api/break-glass")
            .add_header("authorization", bearer(&reader))
            .json(&json!({ "reason": "Patient collapsed, covering colleague absent", "minutes": 600 }))
            .await;
        assert_eq!(too_long.status_code(), 422, "the duration is capped");
        let before: Value = request
            .get("/api/forecast")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert!(before["totals_minor"].is_null(), "masked before the grant");
        // A masked response still satisfies the documented schema.
        schema_check::check(&doc, "Forecast", &before, "masked /api/forecast");
        let granted = request
            .post("/api/break-glass")
            .add_header("authorization", bearer(&reader))
            .json(&json!({ "reason": "Patient collapsed, covering colleague absent" }))
            .await;
        assert_eq!(granted.status_code(), 200, "a plain reader may take break-glass");
        let grant: Value = granted.json();
        schema_check::check(&doc, "Grant", &grant, "POST /api/break-glass");
        let unmasked: Value = request
            .get("/api/forecast")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert!(unmasked["totals_minor"].is_object(), "elevated read is unmasked: {unmasked}");
        schema_check::check(&doc, "Forecast", &unmasked, "unmasked /api/forecast");
        // Elevation is read-only: a reader still cannot write.
        assert_eq!(
            request
                .post("/api/contacts")
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "person_ref": "person:x" }))
                .await
                .status_code(),
            403
        );
        // CRM-R24: a merge is destructive-classified, so a writer is refused
        // and a service credential is not.
        let mut pids = Vec::new();
        for name in ["Merge One", "Merge Two"] {
            let created: Value = request
                .post("/api/contacts")
                .add_header("authorization", bearer(&service))
                .json(&json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                                "display_name": name }))
                .await
                .json();
            pids.push(created["pid"].as_str().unwrap().to_string());
        }
        let merge_body = json!({ "into_pid": pids[1], "reason": "same person" });
        assert_eq!(
            request
                .post(&format!("/api/contacts/{}/merge", pids[0]))
                .add_header("authorization", bearer(&writer))
                .json(&merge_body)
                .await
                .status_code(),
            403,
            "a writer cannot merge"
        );
        let merged: Value = request
            .post(&format!("/api/contacts/{}/merge", pids[0]))
            .add_header("authorization", bearer(&service))
            .json(&merge_body)
            .await
            .json();
        assert_eq!(merged["survivor_pid"], pids[1], "{merged}");
        let unmerge_body = json!({ "reason": "wrong person" });
        assert_eq!(
            request
                .post(&format!("/api/contacts/{}/unmerge", pids[0]))
                .add_header("authorization", bearer(&writer))
                .json(&unmerge_body)
                .await
                .status_code(),
            403,
            "a writer cannot unmerge"
        );
        let undone: Value = request
            .post(&format!("/api/contacts/{}/unmerge", pids[0]))
            .add_header("authorization", bearer(&service))
            .json(&unmerge_body)
            .await
            .json();
        assert_eq!(undone["merged_pid"], pids[0], "{undone}");
        // The review surface is admin/service tier.
        assert_eq!(
            request
                .get("/api/break-glass")
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            403
        );
        let listing: Value = request
            .get("/api/break-glass")
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert!(listing["unreviewed"].as_u64().unwrap() >= 1, "{listing}");
        schema_check::check(&doc, "Grants", &listing, "GET /api/break-glass");
        let pid = grant["pid"].as_str().unwrap();
        // The grantee cannot review their own grant.
        let own = request
            .post("/api/break-glass")
            .add_header("authorization", bearer(&service))
            .json(&json!({ "reason": "Service account incident drill, supervised" }))
            .await
            .json::<Value>();
        assert_eq!(
            request
                .post(&format!("/api/break-glass/{}/review", own["pid"].as_str().unwrap()))
                .add_header("authorization", bearer(&service))
                .json(&json!({ "note": "reviewing my own grant" }))
                .await
                .status_code(),
            403
        );
        // Someone else reviews it, once.
        let review_path = format!("/api/break-glass/{pid}/review");
        let reviewed = request
            .post(&review_path)
            .add_header("authorization", bearer(&service))
            .json(&json!({ "note": "Confirmed against the incident log" }))
            .await;
        assert_eq!(reviewed.status_code(), 200);
        schema_check::check(&doc, "Reviewed", &reviewed.json::<Value>(), "review");
        assert_eq!(
            request
                .post(&review_path)
                .add_header("authorization", bearer(&service))
                .json(&json!({ "note": "Confirmed against the incident log" }))
                .await
                .status_code(),
            422
        );
    })
    .await;
}

/// MCP (CRM-R30): a tool call is decided as the caller, because it is the same REST request through
/// the same stack. A reader cannot write, a stranger sees masked values, nobody without a token
/// gets data, and every call is audited with the agent marker.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn mcp_tool_calls_are_decided_as_the_caller() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
        s.mcp_writes = true;
    });
    let reader = sign_as(&kid, "mcp-reader", &[]);
    let writer = sign_as(&kid, "mcp-writer", &[("access", &["write"])]);
    let owner_sub = uuid::Uuid::new_v4();
    let owner = sign_as(&kid, &owner_sub.to_string(), &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let call = |token: Option<String>, name: &str, arguments: Value| {
            let mut req = request
                .post("/mcp")
                .add_header("mcp-protocol-version", "2026-07-28")
                .add_header("mcp-method", "tools/call")
                .add_header("mcp-name", name.to_string());
            if let Some(token) = token {
                req = req.add_header("authorization", bearer(&token));
            }
            req.json(&json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {
                    "name": name, "arguments": arguments,
                    "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" },
                },
            }))
        };

        // A contact owned by `owner`, created by the writer through the REST API.
        let created: Value = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                "display_name": "Mcp Owned",
                "owner_ref": format!("person:{owner_sub}"),
                "preferred_channel": "phone",
            }))
            .await
            .json();
        let pid = created["pid"].as_str().unwrap().to_string();

        // The owner sees the real value; a stranger sees it masked, as over REST.
        let as_owner: Value = call(Some(owner.clone()), "get_contact", json!({ "pid": pid }))
            .await
            .json();
        assert_eq!(as_owner["result"]["isError"], false, "{as_owner}");
        assert_eq!(
            as_owner["result"]["structuredContent"]["contact"]["preferred_channel"],
            "phone"
        );
        let as_stranger: Value = call(Some(reader.clone()), "get_contact", json!({ "pid": pid }))
            .await
            .json();
        assert_eq!(
            as_stranger["result"]["structuredContent"]["contact"]["preferred_channel"],
            contact_relationship_management::auth::MASKED
        );

        // Results are marked as data.
        let text = as_owner["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.starts_with("NOTICE: what follows is record data"),
            "{text}"
        );

        // A reader cannot write through MCP, a writer can, and nobody without a token gets data.
        let add = |subject: &str| {
            json!({ "subject_kind": "contact", "subject_pid": subject,
                                          "kind": "note", "summary": "from an agent" })
        };
        let refused: Value = call(Some(reader.clone()), "add_activity", add(&pid))
            .await
            .json();
        assert_eq!(refused["result"]["isError"], true, "{refused}");
        assert!(
            refused["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("403")
        );
        let done: Value = call(Some(writer.clone()), "add_activity", add(&pid))
            .await
            .json();
        assert_eq!(done["result"]["isError"], false, "{done}");
        let anonymous: Value = call(None, "list_contacts", json!({})).await.json();
        assert_eq!(anonymous["result"]["isError"], true);
        assert!(
            anonymous["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("401")
        );

        // A preview of a merge is allowed for a reader to ask, and still needs the access a merge needs.
        let other = uuid::Uuid::new_v4().to_string();
        let preview: Value = call(
            Some(reader.clone()),
            "preview_merge_contacts",
            json!({ "merged_pid": pid, "into_pid": other, "reason": "same person" }),
        )
        .await
        .json();
        assert_eq!(
            preview["result"]["isError"], true,
            "a reader cannot even preview a merge: {preview}"
        );

        // Every call left an audit row naming the caller and the agent marker.
        let svc = sign_as(&kid, "mcp-svc", &[("svc", &["true"])]);
        let audits: Value = request
            .get("/api/audits/recent")
            .add_header("authorization", bearer(&svc))
            .await
            .json();
        let rows: Vec<&Value> = audits
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["action"] == "mcp_tool_called")
            .collect();
        assert!(rows.len() >= 6, "{audits}");
        assert!(rows.iter().any(|a| a["actor"] == "mcp-reader"));
        assert!(
            rows.iter().all(|a| a["snapshot"]["agent"] == true),
            "{rows:?}"
        );
        assert!(
            rows.iter().any(
                |a| a["snapshot"]["tool"] == "add_activity" && a["snapshot"]["kind"] == "write"
            )
        );
        assert!(
            !audits.to_string().contains("from an agent"),
            "the free text an agent passed is not audited"
        );
    })
    .await;
}

/// CRM-R32: a reminder is made only if the assignee may read the record, the inbox is the caller's
/// own, and a record that becomes unreadable drops out of the inbox. The policy denies reading an
/// inactive contact to everyone but machine peers.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn notifications_are_made_and_shown_only_for_records_the_recipient_may_read() {
    use sea_orm::ConnectionTrait;
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let assignee_sub = uuid::Uuid::new_v4();
    let assignee = sign_as(&kid, &assignee_sub.to_string(), &[]);
    let reader = sign_as(&kid, "inbox-reader", &[]);
    let writer = sign_as(&kid, "inbox-writer", &[("access", &["write"])]);
    let service = sign_as(&kid, "inbox-svc", &[("svc", &["true"])]);

    request::<App, _, _>(|request, ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let mut contacts = Vec::new();
        for name in ["Readable", "Hidden"] {
            let created: Value = request
                .post("/api/contacts")
                .add_header("authorization", bearer(&writer))
                .json(&json!({
                    "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                    "display_name": name,
                }))
                .await
                .json();
            let pid = created["pid"].as_str().unwrap().to_string();
            let remind = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
            request
                .post("/api/activities")
                .add_header("authorization", bearer(&writer))
                .json(&json!({
                    "subject_kind": "contact", "subject_pid": pid, "kind": "task",
                    "summary": "Follow up", "assignee_ref": format!("person:{assignee_sub}"),
                    "remind_at": remind,
                }))
                .await
                .assert_status_ok();
            contacts.push(pid);
        }
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET status = 'inactive' WHERE pid = '{}'",
                contacts[1]
            ))
            .await
            .unwrap();

        // Sweeping is an admin action; the assignee, with no attributes, may not read the
        // inactive contact, so only the readable one is notified.
        assert_eq!(
            request
                .post("/api/notifications/sweep")
                .add_header("authorization", bearer(&writer))
                .await
                .status_code(),
            403
        );
        let swept: Value = request
            .post("/api/notifications/sweep")
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert_eq!(swept, json!({ "created": 1, "suppressed": 1 }));

        assert_eq!(request.get("/api/notifications").await.status_code(), 401);
        let mine: Vec<Value> = request
            .get("/api/notifications")
            .add_header("authorization", bearer(&assignee))
            .await
            .json();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0]["subject_pid"], contacts[0]);
        // A reader may read their own (empty) inbox and mark things read, but sees nothing of
        // the assignee's, and cannot mark theirs read.
        let theirs: Vec<Value> = request
            .get("/api/notifications")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert!(theirs.is_empty());
        let pid = mine[0]["pid"].as_str().unwrap();
        assert_eq!(
            request
                .put(&format!("/api/notifications/{pid}/read"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            404
        );

        // The policy is asked again at read time: once the record becomes unreadable the
        // notification is gone from the list and the count.
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET status = 'inactive' WHERE pid = '{}'",
                contacts[0]
            ))
            .await
            .unwrap();
        let after: Vec<Value> = request
            .get("/api/notifications")
            .add_header("authorization", bearer(&assignee))
            .await
            .json();
        assert!(after.is_empty());
        let unread: Value = request
            .get("/api/notifications/unread-count")
            .add_header("authorization", bearer(&assignee))
            .await
            .json();
        assert_eq!(unread["unread"], 0);
        // Reading marks it read for the one who owns it, and is allowed to a reader-tier caller.
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET status = 'active' WHERE pid = '{}'",
                contacts[0]
            ))
            .await
            .unwrap();
        request
            .put(&format!("/api/notifications/{pid}/read"))
            .add_header("authorization", bearer(&assignee))
            .await
            .assert_status_ok();
    })
    .await;
}

/// CRM-R33: search leaves out a contact the policy denies the caller, shows a masked caller only
/// names (never the masked field), and needs a token.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn search_leaves_out_what_the_caller_may_not_read() {
    use sea_orm::ConnectionTrait;
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let reader = sign_as(&kid, "search-reader", &[]);
    let writer = sign_as(&kid, "search-writer", &[("access", &["write"])]);
    let service = sign_as(&kid, "search-svc", &[("svc", &["true"])]);

    request::<App, _, _>(|request, ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let mut pids = Vec::new();
        for name in ["Wexford Open", "Wexford Hidden"] {
            let created: Value = request
                .post("/api/contacts")
                .add_header("authorization", bearer(&writer))
                .json(
                    &json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                               "display_name": name, "preferred_channel": "phone" }),
                )
                .await
                .json();
            pids.push(created["pid"].as_str().unwrap().to_string());
        }
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET status = 'inactive' WHERE pid = '{}'",
                pids[1]
            ))
            .await
            .unwrap();
        assert_eq!(
            request.get("/api/search?q=wexford").await.status_code(),
            401
        );
        let as_reader: Vec<Value> = request
            .get("/api/search?q=wexford")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        let found: Vec<&str> = as_reader
            .iter()
            .map(|h| h["pid"].as_str().unwrap())
            .collect();
        assert_eq!(found, [pids[0].as_str()], "the denied contact is left out");
        assert!(
            !as_reader[0].to_string().contains("phone"),
            "no masked field is returned"
        );
        // A machine peer may read both.
        let as_service: Vec<Value> = request
            .get("/api/search?q=wexford")
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert_eq!(as_service.len(), 2);
        // Searching for the masked field's value finds nothing: it is not indexed.
        let by_channel: Vec<Value> = request
            .get("/api/search?q=phone")
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert!(by_channel.is_empty());
    })
    .await;
}

/// CRM-R34: importing needs write access, and duplicate advice never names a contact the caller
/// may not read.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn lead_import_needs_write_and_advice_hides_unreadable_contacts() {
    use sea_orm::ConnectionTrait;
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let reader = sign_as(&kid, "import-reader", &[]);
    let writer = sign_as(&kid, "import-writer", &[("access", &["write"])]);
    let service = sign_as(&kid, "import-svc", &[("svc", &["true"])]);

    request::<App, _, _>(|request, ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let created: Value = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&writer))
            .json(
                &json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                           "display_name": "Quentin Vanderbilt" }),
            )
            .await
            .json();
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET status = 'inactive' WHERE pid = '{}'",
                created["pid"].as_str().unwrap()
            ))
            .await
            .unwrap();
        let body =
            json!({ "csv": "Name\nQuentin Vanderbilt\n", "mapping": { "display_name": "Name" } });
        assert_eq!(
            request
                .post("/api/lead-imports/preview")
                .add_header("authorization", bearer(&reader))
                .json(&body)
                .await
                .status_code(),
            403,
            "a reader cannot import"
        );
        let as_writer: Value = request
            .post("/api/lead-imports/preview")
            .add_header("authorization", bearer(&writer))
            .json(&body)
            .await
            .json();
        assert!(
            as_writer["report"][0]["likely_duplicates"]
                .as_array()
                .unwrap()
                .is_empty(),
            "the writer may not read that contact: {as_writer}"
        );
        let as_service: Value = request
            .post("/api/lead-imports/preview")
            .add_header("authorization", bearer(&service))
            .json(&body)
            .await
            .json();
        assert_eq!(
            as_service["report"][0]["likely_duplicates"][0]["kind"], "contact",
            "{as_service}"
        );
    })
    .await;
}

/// CRM-R35: a shared view is read by another user, a private one is not, only the owner changes
/// a view, a reader may keep views of their own, and applying a shared view gives a masked reader
/// nothing the list would not.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn views_are_shared_without_sharing_access() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let writer = sign_as(&kid, "views-writer", &[("access", &["write"])]);
    let reader = sign_as(&kid, "views-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let view = |name: &str, shared: bool| {
            json!({ "kind": "contact", "name": name, "filter": { "status": ["active"] },
                    "columns": ["display_name", "status"], "shared": shared })
        };
        assert_eq!(request.get("/api/views").await.status_code(), 401);
        let shared: Value = request
            .post("/api/views")
            .add_header("authorization", bearer(&writer))
            .json(&view("Team contacts", true))
            .await
            .json();
        let private: Value = request
            .post("/api/views")
            .add_header("authorization", bearer(&writer))
            .json(&view("Mine only", false))
            .await
            .json();
        let (shared, private) = (
            shared["pid"].as_str().unwrap().to_string(),
            private["pid"].as_str().unwrap().to_string(),
        );

        // The reader sees the shared view, marked as not theirs, and not the private one.
        let seen: Vec<Value> = request
            .get("/api/views?kind=contact")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        let pids: Vec<&str> = seen.iter().map(|v| v["pid"].as_str().unwrap()).collect();
        assert!(pids.contains(&shared.as_str()) && !pids.contains(&private.as_str()));
        assert_eq!(
            seen.iter().find(|v| v["pid"] == shared.as_str()).unwrap()["mine"],
            false
        );

        // Not theirs to change: 403 for the shared one, 404 for the private one.
        let attempt = view("Hijacked", true);
        assert_eq!(
            request
                .put(&format!("/api/views/{shared}"))
                .add_header("authorization", bearer(&reader))
                .json(&attempt)
                .await
                .status_code(),
            403
        );
        assert_eq!(
            request
                .delete(&format!("/api/views/{private}"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .delete(&format!("/api/views/{shared}"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            403
        );

        // A reader may keep views of their own, even though they cannot write data.
        request
            .post("/api/views")
            .add_header("authorization", bearer(&reader))
            .json(&view("Mine only", false))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post("/api/contacts")
                .add_header("authorization", bearer(&reader))
                .json(
                    &json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                               "display_name": "Nope" })
                )
                .await
                .status_code(),
            403
        );

        // The view carries no record data, and the list the reader applies it to is still masked.
        assert!(!seen.iter().any(|v| v.to_string().contains("phone")));
        let created: Value = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&writer))
            .json(
                &json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                           "display_name": "View Subject", "preferred_channel": "phone" }),
            )
            .await
            .json();
        let listed: Vec<Value> = request
            .get("/api/contacts")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        let row = listed
            .iter()
            .find(|c| c["pid"] == created["pid"])
            .expect("the reader lists the contact");
        assert_ne!(row["preferred_channel"], "phone", "still masked: {row}");

        // The owner changes and deletes their own.
        request
            .put(&format!("/api/views/{shared}"))
            .add_header("authorization", bearer(&writer))
            .json(&view("Team contacts", false))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/views/{shared}"))
            .add_header("authorization", bearer(&writer))
            .await
            .assert_status_ok();
    })
    .await;
}

/// CRM-R36: writing a template is a write; previewing a draft stores nothing, so a reader may.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_reader_may_preview_a_template_but_not_save_one() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let writer = sign_as(&kid, "templates-writer", &[("access", &["write"])]);
    let reader = sign_as(&kid, "templates-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let draft = json!({ "name": "Autumn", "subject": "Hi {{given_name}}",
                            "body": "Hello\n{{unsubscribe_url}}" });
        assert_eq!(request.get("/api/email-templates").await.status_code(), 401);
        assert_eq!(
            request
                .post("/api/email-templates")
                .add_header("authorization", bearer(&reader))
                .json(&draft)
                .await
                .status_code(),
            403
        );
        let preview = request
            .post("/api/email-templates/preview")
            .add_header("authorization", bearer(&reader))
            .json(&draft_without_name(&draft))
            .await;
        preview.assert_status_ok();
        assert_eq!(preview.json::<Value>()["subject"], "Hi Alex");
        request
            .post("/api/email-templates")
            .add_header("authorization", bearer(&writer))
            .json(&draft)
            .await
            .assert_status_ok();
        // A reader can read what the writer saved.
        let listed: Vec<Value> = request
            .get("/api/email-templates")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert_eq!(listed.len(), 1);
    })
    .await;
}

fn draft_without_name(draft: &Value) -> Value {
    json!({ "subject": draft["subject"], "body": draft["body"] })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_reader_may_list_workflows_but_not_save_or_try_one() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let writer = sign_as(&kid, "workflows-writer", &[("access", &["write"])]);
    let reader = sign_as(&kid, "workflows-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let draft = json!({ "name": "Greet", "trigger": "contact_created",
                            "actions": [{ "type": "create_task", "summary": "Say hello" }] });
        assert_eq!(request.get("/api/workflows").await.status_code(), 401);
        assert_eq!(
            request
                .post("/api/workflows")
                .add_header("authorization", bearer(&reader))
                .json(&draft)
                .await
                .status_code(),
            403
        );
        let made: Value = request
            .post("/api/workflows")
            .add_header("authorization", bearer(&writer))
            .json(&draft)
            .await
            .json();
        let pid = made["pid"].as_str().unwrap();
        request
            .get("/api/workflows")
            .add_header("authorization", bearer(&reader))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/workflows/{pid}/dry-run"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            403
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_masked_read_hides_personal_custom_fields_and_a_reader_cannot_set_any() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "fields-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "fields-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        for (key, sensitivity) in [("plan", "business"), ("hobby", "personal")] {
            request
                .post("/api/custom-fields")
                .add_header("authorization", bearer(&service))
                .json(
                    &json!({ "entity": "contact", "key": key, "label": key, "kind": "text",
                               "sensitivity": sensitivity }),
                )
                .await
                .assert_status_ok();
        }
        let made: Value = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&service))
            .json(
                &json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                           "display_name": "Masked Fields" }),
            )
            .await
            .json();
        let url = format!(
            "/api/contacts/{}/custom-fields",
            made["pid"].as_str().unwrap()
        );
        request
            .put(&url)
            .add_header("authorization", bearer(&service))
            .json(&json!({ "values": { "plan": "Pro", "hobby": "sailing" } }))
            .await
            .assert_status_ok();

        let seen = |token: String| {
            let request = &request;
            let url = url.clone();
            async move {
                request
                    .get(&url)
                    .add_header("authorization", bearer(&token))
                    .await
                    .json::<Vec<Value>>()
            }
        };
        let masked = seen(reader.clone()).await;
        let by = |rows: &[Value], key: &str| rows.iter().find(|v| v["key"] == key).unwrap().clone();
        assert_eq!(by(&masked, "plan")["value"], "Pro");
        assert_eq!(by(&masked, "hobby")["value"], Value::Null);
        assert_eq!(by(&masked, "hobby")["masked"], true);
        let full = seen(service.clone()).await;
        assert_eq!(by(&full, "hobby")["value"], "sailing");
        assert_eq!(by(&full, "hobby")["masked"], false);

        // A reader sets nothing, declares nothing.
        assert_eq!(
            request
                .put(&url)
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "values": { "plan": "Free" } }))
                .await
                .status_code(),
            403
        );
        assert_eq!(
            request
                .post("/api/custom-fields")
                .add_header("authorization", bearer(&reader))
                .json(
                    &json!({ "entity": "contact", "key": "x", "label": "x", "kind": "text",
                               "sensitivity": "business" })
                )
                .await
                .status_code(),
            403
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_masked_reader_cannot_list_or_fetch_files_and_cannot_attach_any() {
    let dir = std::env::temp_dir().join(format!("crm-attach-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let dir_text = dir.to_string_lossy().to_string();
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
        s.attachment_dir = Some(dir_text);
    });
    let service = sign_as(&kid, "files-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "files-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let made: Value = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&service))
            .json(
                &json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                           "display_name": "Files Behind Mask" }),
            )
            .await
            .json();
        let contact = made["pid"].as_str().unwrap().to_string();
        let url = format!("/api/contacts/{contact}/attachments?name=note.txt");
        let stored: Value = request
            .post(&url)
            .add_header("authorization", bearer(&service))
            .content_type("application/octet-stream")
            .bytes(axum::body::Bytes::from_static(b"private words"))
            .await
            .json();
        let pid = stored["pid"].as_str().unwrap().to_string();

        for (method, target) in [
            ("get", format!("/api/contacts/{contact}/attachments")),
            ("get", format!("/api/attachments/{pid}/content")),
        ] {
            let r = request
                .get(&target)
                .add_header("authorization", bearer(&reader))
                .await;
            assert_eq!(r.status_code(), 403, "{method} {target}");
        }
        assert_eq!(
            request
                .post(&url)
                .add_header("authorization", bearer(&reader))
                .content_type("application/octet-stream")
                .bytes(axum::body::Bytes::from_static(b"nope"))
                .await
                .status_code(),
            403
        );
        let own = request
            .get(&format!("/api/attachments/{pid}/content"))
            .add_header("authorization", bearer(&service))
            .await;
        own.assert_status_ok();
        assert_eq!(own.as_bytes().as_ref(), b"private words");
    })
    .await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_masked_reader_cannot_see_tags_and_a_tag_filter_does_not_show_masked_records() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "tags-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "tags-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let made: Value = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&service))
            .json(
                &json!({ "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                           "display_name": "Tagged Behind Mask" }),
            )
            .await
            .json();
        let contact = made["pid"].as_str().unwrap().to_string();
        request
            .put(&format!("/api/contacts/{contact}/tags"))
            .add_header("authorization", bearer(&service))
            .json(&json!({ "tags": ["sensitive-label"] }))
            .await
            .assert_status_ok();

        // The unmasked caller sees the tag and finds the record by it.
        let own: Vec<Value> = request
            .get("/api/contacts?tag=sensitive-label")
            .add_header("authorization", bearer(&service))
            .await
            .json();
        assert_eq!(own.len(), 1);

        // The masked reader sees neither the tag nor, by filtering, which records carry it.
        assert_eq!(
            request
                .get(&format!("/api/contacts/{contact}/tags"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            403
        );
        assert_eq!(
            request
                .put(&format!("/api/contacts/{contact}/tags"))
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "tags": ["x"] }))
                .await
                .status_code(),
            403
        );
        let hidden: Vec<Value> = request
            .get("/api/contacts?tag=sensitive-label")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert!(hidden.is_empty(), "{hidden:?}");
        // Unfiltered, the reader still gets the masked list as before.
        let masked: Vec<Value> = request
            .get("/api/contacts")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert!(!masked.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_masked_reader_may_count_but_not_sum_or_use_tags_and_shared_reports_stay_the_owners() {
    let _plain = super::no_suppression();
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "reports-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "reports-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");
        let run = |token: String, spec: Value| {
            let request = &request;
            async move {
                request
                    .post("/api/reports/run")
                    .add_header("authorization", bearer(&token))
                    .json(&spec)
                    .await
                    .status_code()
            }
        };
        let count = json!({ "kind": "contact", "group_by": "status" });
        let sum = json!({ "kind": "deal", "group_by": "state", "measure": "sum_amount" });
        let tagged = json!({ "kind": "contact", "group_by": "tag" });
        assert_eq!(run(reader.clone(), count.clone()).await, 200);
        assert_eq!(run(reader.clone(), sum.clone()).await, 403);
        assert_eq!(run(reader.clone(), tagged.clone()).await, 403);
        assert_eq!(run(service.clone(), sum).await, 200);
        assert_eq!(run(service.clone(), tagged).await, 200);

        // A reader may save a report (it is not CRM data), and a shared one is run but not changed
        // by someone else; a private one is not even seen.
        let shared = request
            .post("/api/reports")
            .add_header("authorization", bearer(&service))
            .json(&json!({ "name": "Shared", "spec": count, "shared": true }))
            .await;
        shared.assert_status_ok();
        let shared = shared.json::<Value>()["pid"].as_str().unwrap().to_string();
        let private = request
            .post("/api/reports")
            .add_header("authorization", bearer(&service))
            .json(&json!({ "name": "Private", "spec": count, "shared": false }))
            .await
            .json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let seen: Vec<Value> = request
            .get("/api/reports")
            .add_header("authorization", bearer(&reader))
            .await
            .json();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0]["mine"], false);
        request
            .get(&format!("/api/reports/{shared}/run"))
            .add_header("authorization", bearer(&reader))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .get(&format!("/api/reports/{private}/run"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .delete(&format!("/api/reports/{shared}"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            403
        );
        let own = request
            .post("/api/reports")
            .add_header("authorization", bearer(&reader))
            .json(
                &json!({ "name": "Mine", "spec": json!({ "kind": "lead", "group_by": "source" }) }),
            )
            .await;
        own.assert_status_ok();
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn attribution_is_money_so_a_masked_reader_is_refused_and_an_unmasked_one_is_answered() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "attribution-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "attribution-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let status = |token: String| {
            let request = &request;
            async move {
                request
                    .get("/api/attribution")
                    .add_header("authorization", format!("Bearer {token}"))
                    .await
                    .status_code()
            }
        };
        assert_eq!(status(reader).await, 403);
        assert_eq!(status(service).await, 200);
        request
            .get("/api/attribution")
            .await
            .assert_status(axum::http::StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_reader_may_see_exchange_rates_but_only_a_writer_may_change_them() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "fx-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "fx-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let body = json!({ "from": "USD", "to": "GBP", "rate": "0.8",
                           "effective_on": "2026-01-01", "source": "test table" });
        let put = |token: String| {
            let request = &request;
            let body = body.clone();
            async move {
                request
                    .put("/api/fx-rates")
                    .add_header("authorization", format!("Bearer {token}"))
                    .json(&body)
                    .await
            }
        };
        assert_eq!(put(reader.clone()).await.status_code(), 403);
        let made = put(service.clone()).await;
        assert_eq!(made.status_code(), 200);
        let pid = made.json::<Value>()["pid"].as_str().unwrap().to_string();
        let listed = request
            .get("/api/fx-rates")
            .add_header("authorization", format!("Bearer {reader}"))
            .await;
        assert_eq!(listed.status_code(), 200);
        assert_eq!(listed.json::<Vec<Value>>().len(), 1);
        let removed = request
            .delete(&format!("/api/fx-rates/{pid}"))
            .add_header("authorization", format!("Bearer {reader}"))
            .await;
        assert_eq!(removed.status_code(), 403);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_accounts_clv_is_money_so_a_masked_reader_sees_null_and_an_unmasked_one_the_figures() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "clv-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "clv-reader", &[]);

    request::<App, _, _>(|request, _ctx| async move {
        let made: Value = request
            .post("/api/accounts")
            .add_header("authorization", format!("Bearer {service}"))
            .json(
                &json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Masked Money Ltd" }),
            )
            .await
            .json();
        let url = format!("/api/accounts/{}/clv", made["pid"].as_str().unwrap());
        let read = |token: String| {
            let request = &request;
            let url = url.clone();
            async move {
                request
                    .get(&url)
                    .add_header("authorization", format!("Bearer {token}"))
                    .await
            }
        };
        let masked = read(reader).await;
        masked.assert_status_ok();
        let masked: Value = masked.json();
        assert!(masked["clv_minor"].is_null(), "{masked}");
        assert!(masked["converted_minor"].is_null(), "{masked}");
        assert!(masked["won_deals"].is_number(), "counts stay: {masked}");
        let open: Value = read(service).await.json();
        assert!(open["clv_minor"].is_object(), "{open}");
        request
            .get(&url)
            .await
            .assert_status(axum::http::StatusCode::UNAUTHORIZED);
    })
    .await;
}

/// Money must not leave through any read a masked caller may make. The service seeds records
/// that carry a distinctive amount; a masked reader then asks **every** `GET` path in the
/// OpenAPI document (`{pid}` filled with a record of the right kind), and the amount must appear
/// in none of the answers. A new endpoint that returns money without masking fails here.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn no_read_a_masked_caller_may_make_returns_the_seeded_amount() {
    const MARKER: i64 = 7_654_321;
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "sweep-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "sweep-reader", &[]);

    request::<App, _, _>(|mut request, _ctx| async move {
        request.add_header("authorization", format!("Bearer {service}"));
        // Seed, as the service, records that carry the amount.
        let (pipeline, stages) = super::seed_pipeline!(&request).await;
        let contact = super::seed_contact!(&request, "Sweep Buyer").await;
        let account: Value = request
            .post("/api/accounts")
            .json(&json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Sweep Org" }))
            .await
            .json();
        let account = account["pid"].as_str().unwrap().to_string();
        let segment: Value = request
            .post("/api/segments")
            .json(&json!({ "name": "Everyone", "filter": {} }))
            .await
            .json();
        let campaign: Value = request
            .post("/api/campaigns")
            .json(&json!({ "name": "Sweep Push", "cost_minor": MARKER, "currency": "GBP",
                           "segment_pid": segment["pid"] }))
            .await
            .json();
        let campaign = campaign["pid"].as_str().unwrap().to_string();
        let mut deal = String::new();
        for (name, to_won) in [("Open", false), ("Won", true)] {
            let made: Value = request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline, "amount_minor": MARKER,
                               "currency": "GBP", "account_pid": account,
                               "primary_contact_pid": contact, "source_campaign_pid": campaign }))
                .await
                .json();
            if to_won {
                request
                    .post(&format!("/api/deals/{}/stage", made["pid"].as_str().unwrap()))
                    .json(&json!({ "stage_pid": stages[2] }))
                    .await
                    .assert_status_ok();
            } else {
                deal = made["pid"].as_str().unwrap().to_string();
            }
        }
        // What a worker is paid: a quota and a plan, also carrying the amount.
        let plan: Value = request
            .put("/api/commission-plans")
            .json(&json!({ "name": "Sweep plan", "tiers": [{ "from_percent": 0, "rate_bps": 500 }] }))
            .await
            .json();
        request
            .put("/api/quotas")
            .json(&json!({ "owner_ref": "person:00000000-0000-0000-0000-0000000000aa",
                           "period_from": "2026-01-01", "period_to": "2026-12-31",
                           "currency": "GBP", "amount_minor": MARKER, "plan_pid": plan["pid"] }))
            .await
            .assert_status_ok();
        let lead: Value = request
            .post("/api/leads")
            .json(&json!({ "contact_pid": contact, "source": "sweep" }))
            .await
            .json();
        let report: Value = request
            .post("/api/reports")
            .json(&json!({ "name": "Sweep sums", "spec": { "kind": "deal", "group_by": "state",
                           "measure": "sum_amount" } }))
            .await
            .json();
        let for_segment = |first: &str| -> Option<String> {
            Some(match first {
                "accounts" | "account-imports" => account.clone(),
                "contacts" | "contact-imports" => contact.clone(),
                "deals" => deal.clone(),
                "campaigns" => campaign.clone(),
                "leads" | "lead-imports" => lead["pid"].as_str()?.to_string(),
                "reports" => report["pid"].as_str()?.to_string(),
                "segments" => segment["pid"].as_str()?.to_string(),
                _ => return None,
            })
        };
        let document: Value = request.get("/api-docs/openapi.json").await.json();
        let mut paths = Vec::new();
        for (path, item) in document["paths"].as_object().unwrap() {
            if item.get("get").is_none()
                || ["/confirm", "/preferences", "/unsubscribe", "/metrics", "/api/audits"]
                    .iter()
                    .any(|skip| path.starts_with(skip))
                || path.ends_with("/content")
            {
                continue;
            }
            let filled = if path.contains("{pid}") {
                let first = path.trim_start_matches("/api/").split('/').next().unwrap_or("");
                match for_segment(first) {
                    Some(pid) => path.replace("{pid}", &pid),
                    None => continue,
                }
            } else {
                path.clone()
            };
            paths.push(filled);
        }

        let marker = MARKER.to_string();
        let mut seen_by_service = 0;
        for path in &paths {
            let body = request.get(path).await.text();
            if body.contains(&marker) {
                seen_by_service += 1;
            }
        }
        assert!(
            seen_by_service >= 6,
            "the sweep is vacuous: only {seen_by_service} of {} answers show the amount to the service",
            paths.len()
        );

        request.clear_headers();
        let mut leaks = Vec::new();
        for path in &paths {
            let answer = request
                .get(path)
                .add_header("authorization", format!("Bearer {reader}"))
                .await;
            assert!(
                answer.status_code().as_u16() < 500,
                "{path}: {}",
                answer.status_code()
            );
            if answer.text().contains(&marker) {
                leaks.push(path.clone());
            }
        }
        assert!(leaks.is_empty(), "masked reader sees the amount on: {leaks:#?}");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn quotas_and_commission_plans_are_for_admin_and_service_callers_only() {
    let (keys, kid) = keys_and_kid();
    let _scope = scoped(|s| {
        s.require_auth = true;
        s.paseto_keys = Some(keys.to_string());
        s.abac_policy = Some(test_policy());
    });
    let service = sign_as(&kid, "quota-svc", &[("svc", &["true"])]);
    let reader = sign_as(&kid, "quota-reader", &[]);
    let writer = sign_as(&kid, "quota-writer", &[("access", &["write"])]);

    request::<App, _, _>(|request, _ctx| async move {
        let call = |method: &'static str, path: &'static str, token: String| {
            let request = &request;
            async move {
                let body = json!({ "owner_ref": "person:00000000-0000-0000-0000-000000000009",
                                   "period_from": "2026-01-01", "period_to": "2026-03-31",
                                   "currency": "GBP", "amount_minor": 1 });
                let plan = json!({ "name": "P", "tiers": [{ "from_percent": 0, "rate_bps": 1 }] });
                let builder = match method {
                    "GET" => request.get(path),
                    "PUT" if path == "/api/quotas" => request.put(path).json(&body),
                    _ => request.put(path).json(&plan),
                };
                builder
                    .add_header("authorization", format!("Bearer {token}"))
                    .await
                    .status_code()
            }
        };
        for (method, path) in [
            ("GET", "/api/quotas"),
            ("GET", "/api/quotas/attainment"),
            ("GET", "/api/commission-plans"),
            ("PUT", "/api/quotas"),
            ("PUT", "/api/commission-plans"),
        ] {
            // A reader cannot read what people are paid, and a writer cannot set it.
            assert_eq!(
                call(method, path, reader.clone()).await,
                403,
                "reader {method} {path}"
            );
            assert_eq!(
                call(method, path, writer.clone()).await,
                403,
                "writer {method} {path}"
            );
            assert_eq!(
                call(method, path, service.clone()).await,
                200,
                "service {method} {path}"
            );
        }
        // How a deal is split is what people are paid too (CRM-R65): the service gets past the
        // policy to a 404 for a deal that does not exist, the others are refused first.
        let clawbacks = "/api/quotas/clawbacks/00000000-0000-0000-0000-000000000001";
        for (token, want, who) in [
            (&reader, 403, "reader"),
            (&writer, 403, "writer"),
            (&service, 404, "service"),
        ] {
            assert_eq!(
                call("GET", clawbacks, token.clone()).await,
                want,
                "{who} clawbacks"
            );
        }
        let splits = "/api/quotas/splits/00000000-0000-0000-0000-000000000001";
        assert_eq!(
            call("GET", splits, reader.clone()).await,
            403,
            "reader splits"
        );
        assert_eq!(
            call("GET", splits, writer.clone()).await,
            403,
            "writer splits"
        );
        assert_eq!(
            call("GET", splits, service.clone()).await,
            404,
            "service splits"
        );
        // The bridges send data out and read people in (CRM-R72, CRM-R73): never for a reader or a
        // writer, and for a service only as far as the stub, which sends nothing.
        for method in ["GET"] {
            for path in ["/api/bridges/wpm/teams-import"] {
                assert_eq!(
                    call(method, path, reader.clone()).await,
                    403,
                    "reader {path}"
                );
                assert_eq!(
                    call(method, path, writer.clone()).await,
                    403,
                    "writer {path}"
                );
            }
        }
        // So is a team's roll-up (CRM-R67).
        for path in [
            "/api/quotas/teams",
            "/api/quotas/team-attainment",
            "/api/quotas/territories",
            "/api/quotas/territory-attainment",
        ] {
            assert_eq!(
                call("GET", path, reader.clone()).await,
                403,
                "reader {path}"
            );
            assert_eq!(
                call("GET", path, writer.clone()).await,
                403,
                "writer {path}"
            );
            assert_eq!(
                call("GET", path, service.clone()).await,
                200,
                "service {path}"
            );
        }
        request
            .get("/api/quotas")
            .await
            .assert_status(axum::http::StatusCode::UNAUTHORIZED);
    })
    .await;
}
