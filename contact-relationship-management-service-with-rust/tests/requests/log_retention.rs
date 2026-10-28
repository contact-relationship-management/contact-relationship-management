//! Retention for the delivery and event logs (CRM-R81): rows that are finished with go once they
//! are old enough, and nothing that something may still act on is touched.

use contact_relationship_management::app::App;
use contact_relationship_management::settings;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;

async fn exec(db: &sea_orm::DatabaseConnection, sql: String) {
    db.execute_raw(Statement::from_string(
        DatabaseBackend::Postgres,
        sql.clone(),
    ))
    .await
    .unwrap_or_else(|e| panic!("{e}: {sql}"));
}

async fn count(db: &sea_orm::DatabaseConnection, table: &str, filter: &str) -> i64 {
    db.query_one_raw(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("SELECT count(*) AS n FROM {table} WHERE {filter}"),
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

fn past(report: &Value, table: &str) -> i64 {
    report["logs_past_horizon"][table].as_i64().unwrap_or(0)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn finished_delivery_and_event_rows_go_by_age_and_nothing_still_owed_does() {
    request::<App, _, _>(|request, ctx| async move {
        let db = &ctx.db;
        let (done, live, unknown) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let tag = Uuid::new_v4().simple().to_string();
        for (pid, status) in [(done, "completed"), (live, "running")] {
            exec(
                db,
                format!(
                    "INSERT INTO campaigns (pid, kind, name, status, currency) \
                     VALUES ('{pid}', 'email', 'c-{tag}', '{status}', 'GBP')"
                ),
            )
            .await;
        }
        // (campaign, state, attempts, days old): a delivery is judged by what it is and its campaign.
        let deliveries = [
            (done, "sent", 1, 800),    // 1  goes
            (live, "sent", 1, 800),    // 2  stays: a running campaign
            (done, "failed", 5, 800),  // 3  goes: failed for good
            (done, "failed", 2, 800),  // 4  stays: it can still be retried
            (done, "pending", 0, 800), // 5  stays: still to send
            (done, "sent", 1, 10),     // 6  stays: too recent
            (unknown, "sent", 1, 800), // 7  goes: a nurture step has no campaign
        ];
        for (campaign, state, attempts, days) in deliveries {
            exec(
                db,
                format!(
                    "INSERT INTO email_deliveries (pid, campaign_pid, contact_pid, state, attempts, \
                     sent_at, updated_at) VALUES ('{}', '{campaign}', '{}', '{state}', {attempts}, \
                     now() - interval '{days} days', now() - interval '{days} days')",
                    Uuid::new_v4(),
                    Uuid::new_v4()
                ),
            )
            .await;
        }
        for days in [800, 10] {
            exec(
                db,
                format!(
                    "INSERT INTO email_events (message_id, event, received_at) \
                     VALUES ('m-{tag}-{days}', 'opened', now() - interval '{days} days')"
                ),
            )
            .await;
        }
        // (published days ago or never, fanned out): only published and handed on, long ago, goes.
        let outbox = [
            (Some(40), true),  // goes
            (Some(40), false), // stays: still owed to the webhooks
            (None, false),     // stays: never published
            (Some(5), true),   // stays: too recent
        ];
        for (published, fanned) in outbox {
            let when = published.map_or("NULL".to_string(), |d| format!("now() - interval '{d} days'"));
            let fan = if fanned { "now()" } else { "NULL" };
            exec(
                db,
                format!(
                    "INSERT INTO event_outbox (event_id, entity, entity_pid, kind, occurred_at, \
                     schema_version, payload, published_at, webhook_fanned_out_at) \
                     VALUES ('{}', 'deal', '{}', 'k-{tag}', now() - interval '40 days', 1, '{{}}', {when}, {fan})",
                    Uuid::new_v4(),
                    Uuid::new_v4()
                ),
            )
            .await;
        }
        exec(
            db,
            format!(
                "INSERT INTO webhook_subscriptions (pid, url, events, secret) \
                 VALUES ('{}', 'https://example.test/{tag}', '[]', 'x')",
                Uuid::new_v4()
            ),
        )
        .await;
        let sub: i32 = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT id FROM webhook_subscriptions WHERE url = 'https://example.test/{tag}'"),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "id")
            .unwrap();
        for (status, days) in [("delivered", 40), ("pending", 40), ("failed", 40), ("delivered", 3)] {
            exec(
                db,
                format!(
                    "INSERT INTO webhook_deliveries (subscription_id, event_id, entity_pid, status, \
                     updated_at) VALUES ({sub}, '{}', '{}', '{status}', now() - interval '{days} days')",
                    Uuid::new_v4(),
                    Uuid::new_v4()
                ),
            )
            .await;
        }

        // The report says what would go, and which horizons it used.
        let report: Value = request.get("/api/retention").await.json();
        assert_eq!(report["event_horizon_days"], 30, "{report}");
        assert_eq!(report["mail_horizon_days"], 730, "{report}");
        for (table, want) in [
            ("email_deliveries", 3),
            ("email_events", 1),
            ("event_outbox", 1),
            ("webhook_deliveries", 2),
        ] {
            assert!(past(&report, table) >= want, "{table}: {report}");
        }

        let swept: Value = request.post("/api/retention/sweep").await.json();
        for table in ["email_deliveries", "email_events", "event_outbox", "webhook_deliveries"] {
            assert!(swept["logs_deleted"][table].as_u64().unwrap_or(0) >= 1, "{table}: {swept}");
        }

        // What stayed is exactly what something may still act on or what is recent.
        let mine = format!("campaign_pid IN ('{done}', '{live}', '{unknown}')");
        assert_eq!(count(db, "email_deliveries", &mine).await, 4);
        assert_eq!(
            count(db, "email_deliveries", &format!("{mine} AND state = 'pending'")).await,
            1
        );
        assert_eq!(count(db, "email_events", &format!("message_id LIKE 'm-{tag}-%'")).await, 1);
        assert_eq!(count(db, "event_outbox", &format!("kind = 'k-{tag}'")).await, 3);
        assert_eq!(
            count(db, "event_outbox", &format!("kind = 'k-{tag}' AND published_at IS NULL")).await,
            1
        );
        assert_eq!(
            count(db, "webhook_deliveries", &format!("subscription_id = {sub}")).await,
            2
        );
        assert_eq!(
            count(
                db,
                "webhook_deliveries",
                &format!("subscription_id = {sub} AND status = 'pending'")
            )
            .await,
            1
        );

        // Run again: nothing more is old enough and finished.
        let again: Value = request.post("/api/retention/sweep").await.json();
        for table in ["email_deliveries", "email_events", "event_outbox", "webhook_deliveries"] {
            assert!(again["logs_deleted"][table].is_null(), "{table}: {again}");
        }

        // The horizons cannot be set below their floors: a day is a week, and a month is not enough
        // for mail (attribution reaches back a year).
        settings::update(|s| {
            s.event_retention_days = Some("1".into());
            s.mail_retention_days = Some("30".into());
        });
        let floored: Value = request.get("/api/retention").await.json();
        assert_eq!(floored["event_horizon_days"], 7, "{floored}");
        assert_eq!(floored["mail_horizon_days"], 400, "{floored}");
        settings::update(|s| {
            s.event_retention_days = None;
            s.mail_retention_days = None;
        });
    })
    .await;
}
