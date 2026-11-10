//! The move from `SeaORM` to `sqlx` (CRM-D24): while the two share a database, what one writes the
//! other must read, and one audit chain takes both. These go with the code they prove: when
//! `SeaORM` goes, so do they. (The tests of `db::Bridge` went with it in CRM-T216.)

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use contact_relationship_management::db;
use contact_relationship_management::models::_entities::{activities, contacts};
use contact_relationship_management::models::audit_logs::{self, Model as Audit, verify_chain};
use contact_relationship_management::models::live::FindLive;
use contact_relationship_management::models::rows;
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use super::seed_contact;

/// What `jsonb` rewrites: key order, a number's text, an escape. The hash is of the stored form.
fn snapshot() -> Value {
    json!({ "zeta": 1, "alpha": { "b": "é\n", "a": [1, 2.5, null] }, "n": null, "big": 1e3 })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_chain_written_by_both_libraries_verifies() {
    request::<App, _, _>(|_request, ctx| async move {
        let pid = Uuid::new_v4();
        // `SeaORM`'s writer is gone (CRM-T216); a row written by it earlier still chains, which the
        // migrated chains of a deployed database prove. Here, three writes in two transactions.
        let mut txn = db::pool(&ctx).begin().await.unwrap();
        let first = Audit::record_on(&mut txn, "test", pid, "sqlx", Some("a"), Some(snapshot()))
            .await
            .unwrap();
        txn.commit().await.unwrap();

        let mut txn = db::pool(&ctx).begin().await.unwrap();
        let second = Audit::record_on(&mut txn, "test", pid, "sqlx", None, Some(snapshot()))
            .await
            .unwrap();
        let third = Audit::record_on(&mut txn, "test", pid, "sqlx", Some("a"), None)
            .await
            .unwrap();
        txn.commit().await.unwrap();

        let links = [&first, &second, &third];
        for pair in links.windows(2) {
            assert_eq!(pair[1].prev_hash, pair[0].hash);
        }
        let verdict = verify_chain(db::pool(&ctx)).await.unwrap();
        assert_eq!(verdict.first_break, None, "{verdict:?}");
        assert_eq!(verdict.head, third.hash);
    })
    .await;
}

/// Two pools, so the libraries really do race for the chain's lock: a test's own pool has one
/// connection. A writer that skips the lock forks the chain here, and only here.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn both_libraries_can_write_the_chain_at_once() {
    request::<App, _, _>(|_request, ctx| async move {
        let url = std::env::var("DB_URL").unwrap();
        let racing = sqlx::postgres::PgPoolOptions::new()
            .max_connections(8)
            .connect(&url)
            .await
            .unwrap();
        let by_sqlx = (0..40).map(|_| {
            let racing = racing.clone();
            tokio::spawn(async move {
                let mut txn = racing.begin().await.unwrap();
                Audit::record_on(
                    &mut txn,
                    "test",
                    Uuid::new_v4(),
                    "sqlx",
                    None,
                    Some(snapshot()),
                )
                .await
                .unwrap();
                txn.commit().await.unwrap();
            })
        });
        // A second batch of writers on the same racing pool: `SeaORM`'s writer is gone (CRM-T216),
        // so all 80 race through `record_on` for the one lock.
        let by_more = (0..40).map(|_| {
            let racing = racing.clone();
            tokio::spawn(async move {
                let mut txn = racing.begin().await.unwrap();
                Audit::record_on(&mut txn, "test", Uuid::new_v4(), "sqlx", None, None)
                    .await
                    .unwrap();
                txn.commit().await.unwrap();
            })
        });
        for writer in by_sqlx.chain(by_more).collect::<Vec<_>>() {
            writer.await.unwrap();
        }
        let verdict = verify_chain(db::pool(&ctx)).await.unwrap();
        assert_eq!(verdict.first_break, None, "{verdict:?}");
        assert!(verdict.rows >= 80, "{verdict:?}");
    })
    .await;
}

/// The row a `sqlx` handler returns serializes as the same text as `SeaORM`'s read of it, so a
/// client sees no timestamp change as a handler moves, though the macros read a `timestamptz` as
/// `DateTime<Utc>` and the entities as `DateTime<FixedOffset>`. Both write `Z` at offset zero.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn a_row_reads_the_same_through_both_libraries() {
    request::<App, _, _>(|request, ctx| async move {
        let mut txn = db::pool(&ctx).begin().await.unwrap();
        let written = Audit::record_on(
            &mut txn,
            "test",
            Uuid::new_v4(),
            "sqlx",
            None,
            Some(snapshot()),
        )
        .await
        .unwrap();
        txn.commit().await.unwrap();
        let read = audit_logs::Entity::find_by_id(written.id)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(written, read);
        let text = serde_json::to_value(&written).unwrap();
        assert_eq!(text, serde_json::to_value(&read).unwrap());
        assert!(
            text["created_at"].as_str().unwrap().ends_with('Z'),
            "{text}"
        );

        let set: Value = request.get("/api/settings/organization").await.json();
        assert!(set["updated_at"].as_str().unwrap().ends_with('Z'), "{set}");
    })
    .await;
}

/// CRM-T207: a `sqlx` finder reads what `find_live` reads, row for row: the sealed columns opened,
/// a soft-deleted row left out, and the same filter.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn the_sqlx_finders_read_what_find_live_reads() {
    use base64::Engine;
    use contact_relationship_management::crypto::{self, Keyring};
    use sea_orm::ColumnTrait;
    request::<App, _, _>(|request, ctx| async move {
        let key = base64::engine::general_purpose::STANDARD.encode([7u8; 32]);
        crypto::install(Some(Keyring::from_spec(&format!("k1:{key}")).unwrap()));
        let kept = seed_contact!(&request, "Kept Contact").await;
        let gone = seed_contact!(&request, "Gone Contact").await;
        for (summary, due_on) in [
            ("Call back about renewal", Some("2030-01-10")),
            ("A note", None),
        ] {
            request
                .post("/api/activities")
                .json(&json!({ "subject_kind": "contact", "subject_pid": kept,
                               "kind": "task", "summary": summary, "due_on": due_on }))
                .await
                .assert_status_ok();
        }
        ctx.db
            .execute_unprepared(&format!(
                "UPDATE contacts SET deleted_at = now() WHERE pid = '{gone}'"
            ))
            .await
            .unwrap();

        let through_sqlx = rows::contacts(db::pool(&ctx)).await.unwrap();
        let through_seaorm = contacts::Entity::find_live()
            .order_by_asc(contacts::Column::Id)
            .all(&ctx.db)
            .await
            .unwrap();
        assert_eq!(through_sqlx, through_seaorm);
        let pids: Vec<String> = through_sqlx.iter().map(|c| c.pid.to_string()).collect();
        assert!(pids.contains(&kept) && !pids.contains(&gone), "{pids:?}");
        let stored: String =
            sqlx::query_scalar("SELECT preferred_channel FROM contacts WHERE pid = $1")
                .bind(Uuid::parse_str(&kept).unwrap())
                .fetch_one(db::pool(&ctx))
                .await
                .unwrap();
        assert!(stored.starts_with("enc:v1:k1:"), "{stored}");
        let channel = through_sqlx
            .iter()
            .find(|c| c.pid.to_string() == kept)
            .unwrap()
            .preferred_channel
            .to_string();
        assert!(!channel.starts_with("enc:"), "{channel}");

        let open = rows::open_dated_activities(db::pool(&ctx), Some("task"))
            .await
            .unwrap();
        let open_seaorm = activities::Entity::find_live()
            .filter(activities::Column::Done.eq(false))
            .filter(activities::Column::DueOn.is_not_null())
            .filter(activities::Column::Kind.eq("task"))
            .order_by_asc(activities::Column::Id)
            .all(&ctx.db)
            .await
            .unwrap();
        assert_eq!(open, open_seaorm);
        let mine: Vec<String> = open
            .iter()
            .filter(|a| a.subject_pid.to_string() == kept)
            .map(|a| a.summary.to_string())
            .collect();
        assert_eq!(mine, ["Call back about renewal"]);
        crypto::install(None);
    })
    .await;
}
