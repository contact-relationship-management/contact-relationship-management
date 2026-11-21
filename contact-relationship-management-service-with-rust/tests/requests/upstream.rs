//! The upstream lookups over HTTP (CRM-R1, CRM-R43, CRM-T39): finding or making a person or an
//! organization, a display name with its cache, and a mailbox at send time, against stand-ins on
//! this machine. Serial, since the mode and the service addresses are process-wide; no database.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use contact_relationship_management::clients::{
    self, Mode, active_workers, display_name, email_address, open_delivery_idea, overlap_check,
    resolve_organization, resolve_person,
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

#[tokio::test]
#[serial]
async fn with_no_upstream_an_organization_keeps_one_reference_per_name_and_a_mailbox_follows_the_dev_domain()
 {
    let _restore = Restore;
    // Stub mode makes an organization reference up, and the same name (trimmed, any case) gets the
    // same one for the life of the process.
    let acme = resolve_organization("  Acme Ltd ").await.unwrap();
    assert!(acme.starts_with("organization:"), "{acme}");
    assert!(uuid::Uuid::parse_str(acme.trim_start_matches("organization:")).is_ok());
    assert_eq!(resolve_organization("acme ltd").await.unwrap(), acme);
    assert_ne!(resolve_organization("Other Ltd").await.unwrap(), acme);

    // With no address on file a mailbox is `<id>@<dev domain>` only while that domain is set, and
    // only for a reference that parses.
    let id = uuid::Uuid::new_v4();
    let person = format!("person:{id}");
    assert_eq!(email_address(&person).await, None);
    settings::update(|s| s.mail_dev_domain = Some("mail.test".to_string()));
    assert_eq!(
        email_address(&person).await,
        Some(format!("{id}@mail.test"))
    );
    assert_eq!(email_address("not-a-reference").await, None);
}

#[tokio::test]
#[serial]
async fn a_delivery_idea_is_opened_in_the_portfolio_service_and_each_failure_is_named() {
    let _restore = Restore;
    axa::tls::install_provider();
    let idea = || open_delivery_idea("Win-back", "Bring lapsed buyers back", &["crm"]);
    assert!(
        idea()
            .await
            .unwrap_err()
            .contains("PPM_SERVICE_URL is not set"),
        "unconfigured"
    );
    let id = uuid::Uuid::new_v4().to_string();
    let good: &'static str = Box::leak(format!("{{\"pid\": \"{id}\"}}").into_boxed_str());
    let by_id: &'static str = Box::leak(format!("{{\"id\": \"{id}\"}}").into_boxed_str());
    for (status, body, expected) in [
        (200, good, Ok(id.as_str())),
        (201, by_id, Ok(id.as_str())),
        (500, "{}", Err("answered 500")),
        (200, "not json", Err("was not JSON")),
        (200, "{}", Err("gave no id")),
        (200, "{\"pid\": \"abc\"}", Err("not a UUID")),
    ] {
        let (url, calls) = answering(status, body).await;
        settings::update(|s| s.ppm_service_url = Some(url.clone()));
        let got = idea().await;
        match expected {
            Ok(want) => assert_eq!(got.as_deref(), Ok(want), "{status} {body}"),
            Err(part) => {
                let why = got.unwrap_err();
                assert!(why.contains(part), "{status} {body}: {why}");
                assert!(!why.contains("Win-back"), "no deal data in {why}");
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    settings::update(|s| s.ppm_service_url = Some("http://127.0.0.1:1".to_string()));
    assert!(idea().await.unwrap_err().contains("could not be reached"));
}

#[tokio::test]
#[serial]
async fn the_active_workers_are_read_page_by_page_and_each_failure_is_named() {
    let _restore = Restore;
    axa::tls::install_provider();
    assert!(
        active_workers()
            .await
            .unwrap_err()
            .contains("WORKER_SERVICE_URL is not set"),
        "unconfigured"
    );
    // A row without a `pid` is not a worker row; the department is trimmed; a person is optional.
    let (url, calls) = answering(
        200,
        r#"[{"pid":"w1","department":" Sales ","person_ref":"person:p1"},{"pid":"w2"},{"department":"x"}]"#,
    )
    .await;
    settings::update(|s| s.worker_service_url = Some(url));
    let workers = active_workers().await.unwrap();
    assert_eq!(workers.len(), 2, "{workers:?}");
    assert_eq!(workers[0].department, "Sales");
    assert_eq!(workers[0].person_ref.as_deref(), Some("person:p1"));
    assert_eq!(workers[1].department, "");
    assert_eq!(workers[1].person_ref, None);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "a short page ends the read"
    );

    for (status, body, part) in [
        (503, "[]", "answered 503"),
        (200, "nope", "was not JSON"),
        (200, "{}", "was not a list"),
    ] {
        let (url, _) = answering(status, body).await;
        settings::update(|s| s.worker_service_url = Some(url));
        let why = active_workers().await.unwrap_err();
        assert!(why.contains(part), "{status} {body}: {why}");
    }
    // A service that never runs out of full pages is cut off, not read for ever.
    let full: &'static str =
        Box::leak(format!("[{}]", vec![r#"{"pid":"w"}"#; 200].join(",")).into_boxed_str());
    let (url, calls) = answering(200, full).await;
    settings::update(|s| s.worker_service_url = Some(url));
    let why = active_workers().await.unwrap_err();
    assert!(why.contains("more workers than one import reads"), "{why}");
    assert_eq!(calls.load(Ordering::SeqCst), 51);
    settings::update(|s| s.worker_service_url = Some("http://127.0.0.1:1".to_string()));
    assert!(
        active_workers()
            .await
            .unwrap_err()
            .contains("could not be reached")
    );
}

#[tokio::test]
#[serial]
async fn the_overlap_check_sends_only_references_and_names_each_failure() {
    let _restore = Restore;
    axa::tls::install_provider();
    let refs = vec![format!("person:{}", uuid::Uuid::new_v4())];
    let org = format!("organization:{}", uuid::Uuid::new_v4());
    assert!(
        overlap_check(&org, &refs)
            .await
            .unwrap_err()
            .contains("WORKER_SERVICE_URL is not set")
    );
    let (url, calls) =
        answering(200, r#"{"results": [{"person_ref": "x", "worker": true}]}"#).await;
    settings::update(|s| s.worker_service_url = Some(url));
    let answer = overlap_check(&org, &refs).await.unwrap();
    assert_eq!(answer["results"][0]["worker"], true);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for (status, body, part) in [(403, "{}", "answered 403"), (200, "nope", "was not JSON")] {
        let (url, _) = answering(status, body).await;
        settings::update(|s| s.worker_service_url = Some(url));
        let why = overlap_check(&org, &refs).await.unwrap_err();
        assert!(why.contains(part), "{status} {body}: {why}");
        assert!(!why.contains(&refs[0]) && !why.contains(&org), "{why}");
    }
    settings::update(|s| s.worker_service_url = Some("http://127.0.0.1:1".to_string()));
    assert!(
        overlap_check(&org, &refs)
            .await
            .unwrap_err()
            .contains("could not be reached")
    );
}
