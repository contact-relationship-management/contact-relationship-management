//! Shared by the request and enforcement tests: validate a real response
//! against a schema of the served `OpenAPI` document.

use serde_json::{Value, json};

/// OpenAPI 3.0 marks optional nulls with `nullable`; JSON Schema wants a type
/// union. Rewrite so a standard validator understands the document.
pub fn to_json_schema(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.remove("nullable") == Some(Value::Bool(true)) {
                if let Some(Value::String(kind)) = map.get("type").cloned() {
                    map.insert("type".into(), json!([kind, "null"]));
                } else if map.contains_key("allOf") || map.contains_key("oneOf") {
                    // `nullable` beside a composition: any of it, or null.
                    let inner = Value::Object(std::mem::take(map));
                    map.insert("anyOf".into(), json!([inner, { "type": "null" }]));
                }
            }
            for child in map.values_mut() {
                to_json_schema(child);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(to_json_schema),
        _ => {}
    }
}

pub fn check(doc: &Value, schema: &str, instance: &Value, what: &str) {
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
