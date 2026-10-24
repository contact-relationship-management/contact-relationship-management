//! Print the service's OpenAPI document (the one `GET /api/openapi.json` serves) on stdout, for
//! generating the UI's types without starting the service.
//!
//!     cargo run --example openapi > openapi.json
fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&contact_relationship_management::openapi::spec())
            .unwrap_or_default()
    );
}
