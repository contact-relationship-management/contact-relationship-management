//! List pagination (CRM-T51): a `Link: rel="next"` replaces silent
//! truncation, following it visits every row once, bad input is refused, and
//! article search runs before the page is cut.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashSet;

use super::seed_contact;

/// The path-and-query inside a `Link: <...>; rel="next"` header value.
/// (Takes the header, so a test need not name `axum-test`'s response type.)
fn next_of(link: Option<&axum::http::HeaderValue>) -> Option<String> {
    let link = link?.to_str().ok()?;
    let inner = link.strip_prefix('<')?.split('>').next()?.to_string();
    link.contains("rel=\"next\"").then_some(inner)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn following_next_visits_every_row_once() {
    request::<App, _, _>(|request, _ctx| async move {
        for i in 0..5 {
            seed_contact!(&request, format!("Paged {i}")).await;
        }
        let mut seen = HashSet::new();
        let mut url = "/api/contacts?limit=2".to_string();
        let mut pages = 0;
        loop {
            let response = request.get(&url).await;
            response.assert_status_ok();
            let rows: Value = response.json();
            let rows = rows.as_array().unwrap();
            assert!(rows.len() <= 2);
            for row in rows {
                assert!(
                    seen.insert(row["pid"].as_str().unwrap().to_string()),
                    "repeat"
                );
            }
            pages += 1;
            match next_of(response.headers().get("link")) {
                Some(next) => url = next,
                None => break,
            }
            assert!(pages < 20, "runaway pagination");
        }
        assert_eq!(seen.len(), 5);
        assert_eq!(pages, 3);

        // One page that holds everything has no next link.
        let all = request.get("/api/contacts").await;
        assert!(next_of(all.headers().get("link")).is_none());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn bad_paging_input_is_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        for url in [
            "/api/contacts?limit=0",
            "/api/contacts?limit=1001",
            "/api/contacts?cursor=garbage",
            "/api/tickets?limit=5000",
            "/api/deals?limit=0",
            "/api/contacts?limit=two",
            "/api/contacts?limit=1&limit=2",
            // The unsigned cursor this service handed out before CRM-T223, `o:250`.
            "/api/contacts?cursor=bzoyNTA",
        ] {
            assert_eq!(request.get(url).await.status_code(), 422, "{url}");
        }
    })
    .await;
}

/// A cursor is signed over its list and its filters (CRM-T223): it works where it was handed
/// out, with any page size, and nowhere else.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_cursor_works_only_for_the_list_and_filters_it_came_from() {
    request::<App, _, _>(|request, _ctx| async move {
        for i in 0..3 {
            seed_contact!(&request, format!("Signed {i}")).await;
        }
        let first = request.get("/api/contacts?limit=1").await;
        let next = next_of(first.headers().get("link")).expect("a next link");
        let cursor = next.split("cursor=").nth(1).unwrap().to_string();
        assert_eq!(request.get(&next).await.status_code(), 200, "{next}");
        let bigger = format!("/api/contacts?limit=2&cursor={cursor}");
        assert_eq!(request.get(&bigger).await.status_code(), 200, "{bigger}");
        let detail: Value = request
            .get(&format!("/api/deals?cursor={cursor}"))
            .await
            .json();
        assert_eq!(detail["detail"], "cursor is not valid", "{detail}");
        for url in [
            format!("/api/deals?cursor={cursor}"),
            format!("/api/contacts?limit=1&tag=vip&cursor={cursor}"),
        ] {
            assert_eq!(request.get(&url).await.status_code(), 422, "{url}");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn article_search_is_not_cut_by_the_page() {
    request::<App, _, _>(|request, _ctx| async move {
        for title in ["Alpha one", "Alpha two", "Needle in the stack"] {
            request
                .post("/api/articles")
                .json(&json!({ "title": title, "body": "b", "keywords": "k" }))
                .await
                .assert_status_ok();
        }
        // The match is the third row; a page of one must still find it.
        let found: Value = request.get("/api/articles?q=needle&limit=1").await.json();
        assert_eq!(found.as_array().unwrap().len(), 1, "{found}");
        assert_eq!(found[0]["title"], "Needle in the stack");

        // Two matches, a page of one: one row now, the other behind `next`.
        let first = request.get("/api/articles?q=alpha&limit=1").await;
        let next = next_of(first.headers().get("link")).expect("a next link");
        let first: Value = first.json();
        assert_eq!(first[0]["title"], "Alpha one");
        let second: Value = request.get(&next).await.json();
        assert_eq!(second[0]["title"], "Alpha two");

        // A wildcard in the query is literal, not a pattern.
        let none: Value = request.get("/api/articles?q=%25").await.json();
        assert!(none.as_array().unwrap().is_empty(), "{none}");
    })
    .await;
}

/// The activity dashboard counts every activity, not a capped sample.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn activity_dashboard_counts_past_the_old_cap() {
    request::<App, _, _>(|request, ctx| async move {
        ctx.db
            .execute_unprepared(
                "INSERT INTO activities (created_at, updated_at, pid, subject_kind, subject_pid, \
                 kind, occurred_at, summary, done) \
                 SELECT now(), now(), gen_random_uuid(), 'contact', gen_random_uuid(), \
                 CASE WHEN g % 2 = 0 THEN 'call' ELSE 'note' END, now(), 'bulk', false \
                 FROM generate_series(1, 5003) AS g",
            )
            .await
            .unwrap();
        let dashboard: Value = request.get("/api/dashboards/activity?days=1").await.json();
        assert_eq!(dashboard["total"], 5003, "{dashboard}");
        assert_eq!(dashboard["by_kind"]["call"], 2501);
        assert_eq!(dashboard["by_kind"]["note"], 2502);
        assert!(dashboard.get("truncated").is_none());
    })
    .await;
}
