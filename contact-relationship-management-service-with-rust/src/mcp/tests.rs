//! The parts of the endpoint that need no database: version rules, header checks and shapes.

use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};

fn meta(version: &str) -> Value {
    json!({ "_meta": { "io.modelcontextprotocol/protocolVersion": version } })
}

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
    }
    map
}

#[test]
fn the_supported_versions_are_listed_newest_first() {
    assert_eq!(supported(), ["2026-07-28", "2025-11-25", "2025-06-18"]);
}

#[test]
fn discovery_names_versions_capabilities_and_identity() {
    let d = discover();
    assert_eq!(d["supportedVersions"][0], MODERN);
    assert!(d["capabilities"]["tools"].is_object());
    assert_eq!(
        d["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "contact-relationship-management"
    );
    // This endpoint offers tools only: it claims nothing it does not do.
    assert!(d["capabilities"].get("resources").is_none());
    assert!(d["capabilities"].get("prompts").is_none());
}

#[test]
fn a_modern_request_must_agree_with_its_headers() {
    let id = json!(1);
    let ok = headers(&[("mcp-method", "tools/list")]);
    assert!(check_modern(&ok, "tools/list", &meta(MODERN), &id).is_ok());
    // No version in _meta, or a different one.
    assert!(check_modern(&ok, "tools/list", &json!({}), &id).is_err());
    assert!(check_modern(&ok, "tools/list", &meta("2025-11-25"), &id).is_err());
    // No Mcp-Method, or one that disagrees with the body.
    assert!(check_modern(&headers(&[]), "tools/list", &meta(MODERN), &id).is_err());
    assert!(
        check_modern(
            &headers(&[("mcp-method", "tools/call")]),
            "tools/list",
            &meta(MODERN),
            &id
        )
        .is_err()
    );
}

#[test]
fn a_tool_call_must_name_the_same_tool_in_the_header_as_in_the_body() {
    let id = json!(1);
    let mut params = meta(MODERN);
    params["name"] = json!("get_contact");
    let h = |name: &str| headers(&[("mcp-method", "tools/call"), ("mcp-name", name)]);
    assert!(check_modern(&h("get_contact"), "tools/call", &params, &id).is_ok());
    assert!(check_modern(&h("list_contacts"), "tools/call", &params, &id).is_err());
    assert!(
        check_modern(
            &headers(&[("mcp-method", "tools/call")]),
            "tools/call",
            &params,
            &id
        )
        .is_err()
    );
    // The base64 sentinel is decoded before comparing.
    let encoded = format!("=?base64?{}?=", STANDARD.encode("get_contact"));
    assert!(check_modern(&h(&encoded), "tools/call", &params, &id).is_ok());
    assert_eq!(decoded("plain").as_deref(), Some("plain"));
    assert_eq!(decoded("=?base64?!!!?="), None);
}

#[test]
fn a_legacy_initialize_is_answered_with_a_version_this_endpoint_speaks_and_no_session() {
    let body = |response: Response| async move {
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), 100_000)
            .await
            .unwrap();
        (
            status,
            headers,
            serde_json::from_slice::<Value>(&bytes).unwrap(),
        )
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(async {
        let (status, headers, answer) = body(initialize(
            &json!({ "protocolVersion": "2025-06-18" }),
            &json!(7),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(answer["result"]["protocolVersion"], "2025-06-18");
        assert!(
            headers.get("mcp-session-id").is_none(),
            "no session is issued"
        );
        assert!(
            answer["result"].get("resultType").is_none(),
            "that is a modern-only member"
        );
        // A version it does not speak is answered with the newest it does.
        let (_, _, answer) = body(initialize(
            &json!({ "protocolVersion": "1999-01-01" }),
            &json!(8),
        ))
        .await;
        assert_eq!(answer["result"]["protocolVersion"], "2025-11-25");
    });
}

#[test]
fn the_list_hides_writes_unless_they_are_enabled() {
    let names = |v: &Value| -> Vec<String> {
        v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert!(!names(&list(false)).contains(&"add_activity".to_string()));
    assert!(names(&list(true)).contains(&"add_activity".to_string()));
    // Previews are always there; no tool erases.
    let all = names(&list(true));
    assert!(all.contains(&"preview_merge_contacts".to_string()));
    assert!(all.iter().all(|n| !n.contains("erase")));
}

#[test]
fn a_modern_result_says_it_is_complete_and_a_legacy_one_does_not() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(async {
        for (era, has) in [(Era::Modern, true), (Era::Legacy, false)] {
            let response = rpc_result(&json!(1), json!({ "ok": true }), era);
            let bytes = axum::body::to_bytes(response.into_body(), 10_000)
                .await
                .unwrap();
            let v: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(v["result"].get("resultType").is_some(), has);
            assert_eq!(v["id"], 1);
        }
    });
}
