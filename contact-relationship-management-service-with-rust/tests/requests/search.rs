//! Search (CRM-R33): accents, typos, prefixes, every record kind, paging, and the audit row.

use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use serial_test::serial;

use super::a_person;

macro_rules! find {
    ($request:expr, $q:expr) => {{
        let answer = $request.get(&format!("/api/search?q={}", $q)).await;
        answer.assert_status_ok();
        answer.json::<Vec<Value>>()
    }};
}

fn titles(hits: &[Value]) -> Vec<String> {
    hits.iter()
        .map(|h| h["title"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn search_ignores_accents_forgives_typos_and_covers_every_kind() {
    request::<App, _, _>(|request, ctx| async move {
        let contact: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": a_person(), "display_name": "Zoë Ångström-Quillfeather",
                           "job_title": "Head of Procurement" }))
            .await
            .json();
        let contact_pid = contact["pid"].as_str().unwrap().to_string();
        request
            .post("/api/accounts")
            .json(&json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                           "display_name": "Quillfeather Brewing", "industry": "Beverages" }))
            .await
            .assert_status_ok();
        request
            .post("/api/leads")
            .json(&json!({ "display_name": "Quillfeather enquiry", "source": "web",
                           "email": "x@quillmail.test" }))
            .await
            .assert_status_ok();
        request
            .post("/api/tickets")
            .json(&json!({ "title": "Quillfeather portal down" }))
            .await
            .assert_status_ok();
        request
            .post("/api/articles")
            .json(&json!({ "title": "Resetting a portal", "body": "Quillfeather customers reset here",
                           "keywords": "portal" }))
            .await
            .assert_status_ok();

        // Accents folded both ways; case ignored; a prefix is enough.
        for q in ["zoe", "ZOË", "angstrom", "ångström", "zoe%20angs", "procure"] {
            let hits = find!(request, q);
            assert!(
                hits.iter().any(|h| h["pid"] == contact_pid.as_str()),
                "{q}: {hits:?}"
            );
        }
        // A typo still finds it, and a different word does not.
        assert!(
            find!(request, "quilfeather")
                .iter()
                .any(|h| h["kind"] == "account"),
            "a typo"
        );
        assert!(find!(request, "zzyzx").is_empty());
        // Every kind that mentions the word, with its descriptor.
        let kinds: std::collections::BTreeSet<String> = find!(request, "quillfeather")
            .iter()
            .map(|h| h["kind"].as_str().unwrap().to_string())
            .collect();
        for kind in ["contact", "account", "lead", "ticket", "article"] {
            assert!(kinds.contains(kind), "{kind} in {kinds:?}");
        }
        let account = find!(request, "brewing")
            .into_iter()
            .find(|h| h["kind"] == "account")
            .unwrap();
        assert_eq!(account["detail"], "Beverages");
        // The lead's address domain is searchable; the address itself is not stored.
        assert!(find!(request, "quillmail").iter().any(|h| h["kind"] == "lead"));

        // A removed record is not found.
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET deleted_at = now() WHERE pid = '{contact_pid}'"
            ))
            .await
            .unwrap();
        assert!(
            !find!(request, "zoe")
                .iter()
                .any(|h| h["pid"] == contact_pid.as_str())
        );

        // Bad queries are 422; the audit holds a count and never the text.
        for q in ["", "a", "%26%7C%21", &"x".repeat(101)] {
            assert_eq!(
                request.get(&format!("/api/search?q={q}")).await.status_code(),
                422,
                "{q}"
            );
        }
        let audit = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM audit_logs WHERE entity = 'search' \
                 AND snapshot::text ILIKE '%quillfeather%'",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(audit.try_get::<i64>("", "n").unwrap(), 0);
        let rows = ctx
            .db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM audit_logs WHERE entity = 'search' AND action = 'search'",
            ))
            .await
            .unwrap()
            .unwrap();
        assert!(rows.try_get::<i64>("", "n").unwrap() >= 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn results_are_paged_best_first() {
    request::<App, _, _>(|request, _ctx| async move {
        for n in 0..5 {
            request
                .post("/api/contacts")
                .json(&json!({ "person_ref": a_person(), "display_name": format!("Paged Person {n}") }))
                .await
                .assert_status_ok();
        }
        let first = request.get("/api/search?q=paged&limit=2").await;
        first.assert_status_ok();
        let link = first
            .header("link")
            .to_str()
            .expect("a next link")
            .to_string();
        let first: Vec<Value> = first.json();
        assert_eq!(first.len(), 2);
        let cursor = link
            .split("cursor=")
            .nth(1)
            .unwrap()
            .split('>')
            .next()
            .unwrap()
            .to_string();
        let second: Vec<Value> = request
            .get(&format!("/api/search?q=paged&limit=2&cursor={cursor}"))
            .await
            .json();
        assert_eq!(second.len(), 2);
        let mut all = titles(&first);
        all.extend(titles(&second));
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 4, "no repeats across pages: {all:?}");
    })
    .await;
}

/// `q`, percent-encoded for the query string.
fn encoded(q: &str) -> String {
    url::form_urlencoded::byte_serialize(q.as_bytes()).collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn search_finds_part_of_a_name_in_a_script_without_spaces() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut pids = std::collections::BTreeMap::new();
        for name in [
            "王芳",
            "田中 博",
            "たなか ひろし",
            "สมศรี ใจดี",
            "김민준",
            "प्रिया वेंकटरमन",
            "نور حداد",
            "北京王芳科技有限公司",
        ] {
            let made: Value = request
                .post("/api/contacts")
                .json(&json!({ "person_ref": a_person(), "display_name": name }))
                .await
                .json();
            pids.insert(name, made["pid"].as_str().unwrap().to_string());
        }
        let found =
            |hits: &[Value], name: &str| hits.iter().any(|h| h["pid"] == pids[name].as_str());
        // A part of a name, at its start, in its middle or at its end; one ideograph is enough.
        for (q, name) in [
            ("芳", "王芳"),
            ("王芳", "王芳"),
            ("芳科技", "北京王芳科技有限公司"),
            ("有限公司", "北京王芳科技有限公司"),
            ("田中", "田中 博"),
            ("なか", "たなか ひろし"),
            ("ศรี", "สมศรี ใจดี"),
            ("ใจดี", "สมศรี ใจดี"),
            ("민준", "김민준"),
            ("प्रिया", "प्रिया वेंकटरमन"),
            ("वेंकट", "प्रिया वेंकटरमन"),
            ("حداد", "نور حداد"),
        ] {
            let hits = find!(request, encoded(q));
            assert!(found(&hits, name), "{q} finds {name}: {hits:?}");
        }
        // Pairs must all be there: a name holding only one of the query's characters is not found.
        let hits = find!(request, encoded("芳华"));
        assert!(!found(&hits, "王芳"), "{hits:?}");
        assert!(
            find!(request, encoded("科学"))
                .iter()
                .all(|h| h["pid"] != pids["北京王芳科技有限公司"].as_str())
        );
        // One letter of an alphabet is still too short; one ideograph is not.
        assert_eq!(
            request
                .get(&format!("/api/search?q={}", encoded("ส")))
                .await
                .status_code(),
            422
        );
        request
            .get(&format!("/api/search?q={}", encoded("博")))
            .await
            .assert_status_ok();
    })
    .await;
}
