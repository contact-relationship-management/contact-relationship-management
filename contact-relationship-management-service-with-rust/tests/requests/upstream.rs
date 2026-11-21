//! The upstream lookups over HTTP (CRM-R1, CRM-R43, CRM-T39): finding or making a person or an
//! organization, a display name with its cache, and a mailbox at send time, against stand-ins on
//! this machine. Serial, since the mode and the service addresses are process-wide; no database.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use contact_relationship_management::clients::{
    self, Mode, display_name, email_address, resolve_organization, resolve_person,
};
use contact_relationship_management::settings::{self, Settings};
use entity_ref::EntityRef;
use serial_test::serial;

use crate::scoped_env::ScopedEnv;

/// Puts the default settings back when a test ends, even by panic.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        settings::install(Settings::default());
    }
}

fn over_http() -> Restore {
    axa::tls::install_provider();
    settings::update(|s| s.upstream_mode = Mode::Http);
    Restore
}

/// A server answering every request `status` and `body`, counting the requests.
async fn answering(status: u16, body: &'static str) -> (String, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let app = Router::new().fallback(move || {
        let counted = counted.clone();
        async move {
            counted.fetch_add(1, Ordering::SeqCst);
            (
                axum::http::StatusCode::from_u16(status).unwrap(),
                [("content-type", "application/json")],
                body,
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, calls)
}

/// An address with nothing listening on it.
async fn nobody() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    url
}

const PERSON: &str = "CONTACT_RELATIONSHIP_MANAGEMENT_PERSON_SERVICE_URL";
const ORGANIZATION: &str = "CONTACT_RELATIONSHIP_MANAGEMENT_ORGANIZATION_SERVICE_URL";
const ID: &str = "0c4f1e2a-0000-4000-8000-000000000001";

#[tokio::test]
#[serial]
async fn a_person_and_an_organization_are_found_or_made_over_http_and_each_failure_is_named() {
    let _restore = over_http();
    let mut env = ScopedEnv::default();
    for (var, find) in [(PERSON, true), (ORGANIZATION, false)] {
        let ask = |var: &'static str| async move {
            if var == PERSON {
                resolve_person("Ada", "ada@example.test").await
            } else {
                resolve_organization("Acme").await
            }
        };
        let kind = if find { "person" } else { "organization" };
        unsafe_remove(var);
        assert!(ask(var).await.unwrap_err().ends_with("is not set"));

        for (status, body, expected) in [
            (
                200,
                r#"{"pid":"0c4f1e2a-0000-4000-8000-000000000001"}"#,
                Ok(format!("{kind}:{ID}")),
            ),
            (
                201,
                r#"{"id":"0c4f1e2a-0000-4000-8000-000000000001"}"#,
                Ok(format!("{kind}:{ID}")),
            ),
            (500, "{}", Err(format!("the {kind} service answered 500"))),
            (
                200,
                "not json",
                Err(format!("the {kind} service answer was not JSON")),
            ),
            (200, "{}", Err(format!("the {kind} service gave no id"))),
            (
                200,
                r#"{"pid":"a b"}"#,
                Err(format!(
                    "the {kind} service gave an id that is not a reference"
                )),
            ),
        ] {
            let (url, _) = answering(status, body).await;
            env.set(var, &url);
            assert_eq!(ask(var).await, expected, "{kind} {status} {body}");
        }
        env.set(var, &nobody().await);
        assert_eq!(
            ask(var).await,
            Err(format!("the {kind} service could not be reached"))
        );
    }
}

/// Removing a variable the guard did not set: put it back to unset for the case that needs it.
fn unsafe_remove(var: &str) {
    // Safe here: the test is `#[serial]`, so no other thread reads the environment.
    unsafe { std::env::remove_var(var) };
}

#[tokio::test]
#[serial]
async fn a_display_name_is_looked_up_once_and_kept_and_a_failed_lookup_is_none() {
    let _restore = over_http();
    let mut env = ScopedEnv::default();
    let (url, calls) = answering(200, r#"{"display_name":"Ada Lovelace"}"#).await;
    env.set(PERSON, &url);
    let ada: EntityRef = format!("person:{}", uuid::Uuid::new_v4()).parse().unwrap();
    assert_eq!(display_name(&ada).await.as_deref(), Some("Ada Lovelace"));
    assert_eq!(display_name(&ada).await.as_deref(), Some("Ada Lovelace"));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the second came from the cache"
    );

    let (url, _) = answering(200, r#"{"title":"Acme Ltd"}"#).await;
    env.set(ORGANIZATION, &url);
    let acme: EntityRef = format!("organization:{}", uuid::Uuid::new_v4())
        .parse()
        .unwrap();
    assert_eq!(
        display_name(&acme).await.as_deref(),
        Some("Acme Ltd"),
        "a title serves as a name"
    );

    env.set(PERSON, &nobody().await);
    let grace: EntityRef = format!("person:{}", uuid::Uuid::new_v4()).parse().unwrap();
    assert_eq!(display_name(&grace).await, None);
    let (url, _) = answering(200, r#"{"other":"x"}"#).await;
    env.set(PERSON, &url);
    assert_eq!(display_name(&grace).await, None, "no name field, no name");
}

#[tokio::test]
#[serial]
async fn a_mailbox_is_read_from_the_person_service_at_send_time() {
    let _restore = over_http();
    let mut env = ScopedEnv::default();
    let (url, _) = answering(
        200,
        r#"{"emails":[{"address":"work@example.test"},{"address":"home@example.test","primary":true}]}"#,
    )
    .await;
    env.set(PERSON, &url);
    let person = format!("person:{}", uuid::Uuid::new_v4());
    assert_eq!(
        email_address(&person).await.as_deref(),
        Some("home@example.test")
    );
    env.set(PERSON, &nobody().await);
    assert_eq!(email_address(&person).await, None);
    assert_eq!(email_address("not a reference").await, None);
    // A primed address wins over the service.
    clients::prime_email(&person, "primed@example.test");
    assert_eq!(
        email_address(&person).await.as_deref(),
        Some("primed@example.test")
    );
}
