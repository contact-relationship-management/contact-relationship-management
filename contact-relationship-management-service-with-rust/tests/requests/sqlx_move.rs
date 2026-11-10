//! The move from `SeaORM` to `sqlx` (CRM-D24): while the two share a database, what one writes the
//! other must read, one audit chain takes both, and a `SeaORM` helper can run inside a `sqlx`
//! transaction. These go with the code they prove: when `SeaORM` goes, so do they.

use axa::AppContext;
use axa_test::prelude::*;
use contact_relationship_management::app::App;
use contact_relationship_management::controllers::organization;
use contact_relationship_management::db::{self, Bridge};
use contact_relationship_management::models::_entities::{activities, contacts};
use contact_relationship_management::models::audit_logs::{self, Model as Audit, verify_chain};
use contact_relationship_management::models::live::FindLive;
use contact_relationship_management::models::rows;
use contact_relationship_management::streaming;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, RuntimeErr, Statement, TransactionTrait,
};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use super::seed_contact;

/// What `jsonb` rewrites: key order, a number's text, an escape. The hash is of the stored form.
fn snapshot() -> Value {
    json!({ "zeta": 1, "alpha": { "b": "é\n", "a": [1, 2.5, null] }, "n": null, "big": 1e3 })
}

async fn count(ctx: &AppContext, sql: &'static str, pid: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(pid)
        .fetch_one(db::pool(ctx))
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_chain_written_by_both_libraries_verifies() {
    request::<App, _, _>(|_request, ctx| async move {
        let pid = Uuid::new_v4();
        let txn = ctx.db.begin().await.unwrap();
        let first = Audit::record(&txn, "test", pid, "seaorm", Some("a"), Some(snapshot()))
            .await
            .unwrap();
        txn.commit().await.unwrap();

        let mut txn = db::pool(&ctx).begin().await.unwrap();
        let second = Audit::record_on(&mut txn, "test", pid, "sqlx", None, Some(snapshot()))
            .await
            .unwrap();
        // A block, not `drop`: whether `Bridge` has drop glue depends on the platform, and on
        // Linux clippy refuses `drop` of a value without it.
        let third = {
            let bridge = Bridge::new(&mut txn);
            Audit::record(&bridge, "test", pid, "bridge", Some("a"), None)
                .await
                .unwrap()
        };
        let fourth = Audit::record_on(&mut txn, "test", pid, "sqlx", Some("a"), None)
            .await
            .unwrap();
        txn.commit().await.unwrap();

        let links = [&first, &second, &third, &fourth];
        for pair in links.windows(2) {
            assert_eq!(pair[1].prev_hash, pair[0].hash);
        }
        let verdict = verify_chain(&ctx.db).await.unwrap();
        assert_eq!(verdict.first_break, None, "{verdict:?}");
        assert_eq!(verdict.head, fourth.hash);
    })
    .await;
}

/// Two pools, so the libraries really do race for the chain's lock: a test's own pool has one
/// connection. A writer that skips the lock forks the chain here, and only here.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn both_libraries_can_write_the_chain_at_once() {
    request::<App, _, _>(|_request, ctx| async move {
        let url = std::env::var("DATABASE_URL").unwrap();
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
        let by_seaorm = (0..40).map(|_| {
            let db = sea_orm::DatabaseConnection::from(racing.clone());
            tokio::spawn(async move {
                let txn = db.begin().await.unwrap();
                Audit::record(
                    &txn,
                    "test",
                    Uuid::new_v4(),
                    "seaorm",
                    None,
                    Some(snapshot()),
                )
                .await
                .unwrap();
                txn.commit().await.unwrap();
            })
        });
        for writer in by_sqlx.chain(by_seaorm).collect::<Vec<_>>() {
            writer.await.unwrap();
        }
        let verdict = verify_chain(&ctx.db).await.unwrap();
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
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
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

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn a_seaorm_helper_commits_and_rolls_back_with_the_sqlx_transaction() {
    request::<App, _, _>(|_request, ctx| async move {
        const AUDITED: &str = "SELECT count(*) FROM audit_logs WHERE entity_pid = $1";
        const NOTED: &str = "SELECT count(*) FROM activities WHERE subject_pid = $1";
        let pid = Uuid::new_v4();
        for (keep, audited, noted) in [(false, 0, 0), (true, 1, 1)] {
            let mut txn = db::pool(&ctx).begin().await.unwrap();
            {
                let bridge = Bridge::new(&mut txn);
                Audit::record(&bridge, "test", pid, "bridge", None, None)
                    .await
                    .unwrap();
                note(pid).insert(&bridge).await.unwrap();
            }
            if keep {
                txn.commit().await.unwrap();
            } else {
                txn.rollback().await.unwrap();
            }
            assert_eq!(count(&ctx, AUDITED, pid).await, audited);
            assert_eq!(count(&ctx, NOTED, pid).await, noted);
        }
    })
    .await;
}

fn note(subject: Uuid) -> activities::ActiveModel {
    activities::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set("contact".to_string()),
        subject_pid: ActiveValue::set(subject),
        kind: ActiveValue::set("note".to_string()),
        occurred_at: ActiveValue::set(contact_relationship_management::clock::now().into()),
        actor_ref: ActiveValue::set(None),
        summary: ActiveValue::set("through the bridge".into()),
        due_on: ActiveValue::set(Some(chrono::NaiveDate::from_ymd_opt(2026, 11, 7).unwrap())),
        remind_at: ActiveValue::set(None),
        ..Default::default()
    }
}

/// What the unmoved helpers use, each on a transaction `sqlx` owns: entity insert with `RETURNING`,
/// partial update, row lock, typed binds and `NULL`s, raw statements, and the real helpers.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
async fn the_bridge_carries_what_seaorm_code_does() {
    request::<App, _, _>(|_request, ctx| async move {
        let subject = Uuid::new_v4();
        let mut txn = db::pool(&ctx).begin().await.unwrap();
        let bridge = Bridge::new(&mut txn);

        let inserted = note(subject).insert(&bridge).await.unwrap();
        assert!(inserted.id > 0, "RETURNING gave the row");
        assert_eq!(inserted.summary.as_str(), "through the bridge");
        assert_eq!(inserted.actor_ref, None);
        assert_eq!(
            inserted.due_on,
            chrono::NaiveDate::from_ymd_opt(2026, 11, 7)
        );

        let locked = activities::Entity::find_by_id(inserted.id)
            .lock_exclusive()
            .one(&bridge)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(locked, inserted);
        let missing = activities::Entity::find_by_id(-1).one(&bridge).await;
        assert_eq!(missing.unwrap(), None);

        let mut change = inserted.clone().into_active_model();
        change.sentiment = ActiveValue::set(Some("positive".to_string()));
        let changed = change.update(&bridge).await.unwrap();
        assert_eq!(changed.sentiment.as_deref(), Some("positive"));
        assert_eq!(changed.summary, inserted.summary);

        let rows = bridge
            .query_all_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT $1::uuid AS a, $2::bool AS b, $3::jsonb AS c, $4::text AS d, $5::int2 AS e",
                [
                    subject.into(),
                    true.into(),
                    snapshot().into(),
                    Option::<String>::None.into(),
                    7_i16.into(),
                ],
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].try_get::<Uuid>("", "a").unwrap(), subject);
        assert!(rows[0].try_get::<bool>("", "b").unwrap());
        assert_eq!(rows[0].try_get::<Value>("", "c").unwrap(), snapshot());
        assert_eq!(rows[0].try_get::<Option<String>>("", "d").unwrap(), None);
        assert_eq!(rows[0].try_get::<i16>("", "e").unwrap(), 7);
        let touched = bridge
            .execute_unprepared("SET LOCAL lock_timeout = '1s'")
            .await
            .unwrap();
        assert_eq!(touched.rows_affected(), 0);
        let stated = bridge
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE activities SET done = $1 WHERE id = $2",
                [true.into(), inserted.id.into()],
            ))
            .await
            .unwrap();
        assert_eq!(stated.rows_affected(), 1);

        // Real helpers, the way a moved handler will call them.
        streaming::emit_on(
            &bridge,
            "activity",
            "noted",
            &subject.to_string(),
            "A note",
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            organization::periods(&bridge)
                .await
                .unwrap()
                .fiscal_start_month,
            1
        );
        assert_eq!(organization::stalled_after_days(&bridge).await.unwrap(), 14);

        // A failed statement is `SeaORM`'s error, so a helper handles it the same.
        let refused = bridge
            .execute_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT * FROM no_such_table",
            ))
            .await;
        assert!(
            matches!(refused, Err(DbErr::Exec(RuntimeErr::SqlxError(_)))),
            "{refused:?}"
        );
        // `bridge` is not used again, so its borrow of `txn` has ended.
        txn.rollback().await.unwrap();
    })
    .await;
}

/// CRM-T207: a `sqlx` finder reads what `find_live` reads, row for row: the sealed columns opened,
/// a soft-deleted row left out, and the same filter.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test -- --ignored`"]
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
