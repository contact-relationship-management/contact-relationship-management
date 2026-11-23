//! The stored consent profile adapter (CRM-D21): database rows become pure rules, evidence is
//! gathered without copying identity data, and every channel reaches one auditable decision.

use axa_test::prelude::*;
use contact_relationship_management::app::App;
use contact_relationship_management::consent_profiles::{self, Decision};
use contact_relationship_management::db;
use contact_relationship_management::models::rows;
use contact_relationship_management::rules::consent_regime::{Basis, Blocked, Layer, Regime};
use contact_relationship_management::time::Date;
use serial_test::serial;
use uuid::Uuid;

use super::seed_contact;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
async fn stored_profiles_drive_one_auditable_decision_per_channel() {
    request::<App, _, _>(|request, ctx| async move {
        let pid = seed_contact!(&request, "Profile subject").await;
        let pid = Uuid::parse_str(&pid).unwrap();
        let mut tx = db::pool(&ctx).begin().await.unwrap();

        sqlx::query("UPDATE contacts SET consent_jurisdiction = 'ZZ-1' WHERE pid = $1")
            .bind(pid)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO consent_jurisdictions (code, layer, parent) VALUES \
             ('ZZ', 'national', NULL), ('ZZ-1', 'subnational', 'ZZ')",
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO consent_purposes \
             (key, position, names, descriptions, active, soft_opt_in_eligible) VALUES \
             ('marketing-profile-test', 999, '{\"en\":\"Marketing\"}', \
              '{\"en\":\"Product news\"}', true, true)",
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO consent_profile_rows \
             (code, key, value, effective_from, source, status) VALUES \
             ('ZZ', 'consent.regime.sms', '\"implied\"', '2020-01-01', 'statute', 'read'), \
             ('ZZ', 'consent.implied.expiry_days.inquiry', '30', '2020-01-01', 'statute', 'read'), \
             ('ZZ-1', 'message.postal_address', '\"required\"', '2020-01-01', 'guidance', 'read'), \
             ('ZZ-1', 'message.advertising_label', '{\"en\":\"Ad\"}', '2020-01-01', 'guidance', 'read'), \
             ('ZZ', 'unknown.parameter', 'true', '2020-01-01', 'bad import', 'read')",
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO consent_profile_acknowledgements (code, through_id) \
             SELECT 'ZZ', max(id) FROM consent_profile_rows WHERE code = 'ZZ'",
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        let evidence_pid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tickets (pid, contact_pid, title, priority, channel, status, opened_at) \
             VALUES ($1, $2, 'Question', 'normal', 'web', 'open', now())",
        )
        .bind(evidence_pid)
        .bind(pid)
        .execute(&mut *tx)
        .await
        .unwrap();

        let loaded = consent_profiles::load(&mut tx).await.unwrap();
        assert!(
            loaded
                .links
                .iter()
                .any(|link| link.code == "ZZ"
                    && link.layer == Layer::National
                    && link.parent.is_none())
        );
        assert!(!loaded.rows.iter().any(|row| row.key == "unknown.parameter"));
        assert!(loaded.acks.iter().any(|ack| ack.code == "ZZ"));

        let chain = loaded.chain_for(Some("ZZ-1")).unwrap();
        assert_eq!(chain.iter().map(|step| step.code.as_str()).collect::<Vec<_>>(), ["ZZ-1", "ZZ"]);
        assert_eq!(loaded.chain_for(None), None);
        let malformed = consent_profiles::Loaded {
            links: vec![
                contact_relationship_management::rules::consent_regime::Link {
                    code: "AA".into(),
                    layer: Layer::National,
                    parent: Some("AA".into()),
                },
            ],
            ..Default::default()
        };
        assert_eq!(malformed.chain_for(Some("AA")).unwrap().len(), 1);

        let day = Date::parse_ymd("2026-11-19").unwrap();
        let rules = loaded.rules("sms", Some("ZZ-1"), day);
        assert_eq!(rules.regime, Regime::Implied);
        assert_eq!(rules.expiry_days.get("inquiry"), Some(&30));
        let message = loaded.message_rules(Some("ZZ-1"), day);
        assert!(message.postal_address_required);
        assert_eq!(message.advertising_label.unwrap()["en"], "Ad");

        let all_purposes = consent_profiles::purposes(&mut *tx).await.unwrap();
        let marketing = consent_profiles::purpose(&mut *tx, "marketing-profile-test")
            .await
            .unwrap()
            .unwrap();
        assert!(all_purposes.iter().any(|purpose| purpose.key == marketing.key));
        assert_eq!(
            consent_profiles::purpose_at(&mut *tx, marketing.position)
                .await
                .unwrap()
                .unwrap()
                .key,
            "marketing-profile-test"
        );
        assert!(consent_profiles::purpose(&mut *tx, "missing").await.unwrap().is_none());
        assert!(consent_profiles::purpose_at(&mut *tx, -1).await.unwrap().is_none());

        let contact = rows::contact(&mut *tx, pid).await.unwrap().unwrap();
        assert_eq!(
            consent_profiles::state_on(&mut *tx, &contact, "marketing", "email")
                .await
                .unwrap(),
            "granted"
        );
        assert_eq!(
            consent_profiles::state_on(&mut *tx, &contact, "marketing", "sms")
                .await
                .unwrap(),
            "never"
        );
        let evidence = consent_profiles::evidence(&mut *tx, pid).await.unwrap();
        assert!(evidence.iter().any(|item| item.pid == evidence_pid.to_string()));

        let decision = consent_profiles::decide(
            &mut tx,
            &loaded,
            &contact,
            "marketing",
            "sms",
            None,
            // The service's clock, which follows the database that stamped the evidence: the host's
            // can be a few milliseconds behind it, which would put the evidence in the future.
            contact_relationship_management::clock::now(),
            day,
        )
        .await
        .unwrap();
        assert!(decision.sends(), "{:?}", decision.outcome);
        assert!(matches!(decision.outcome, Ok(Basis::Implied { .. })));
        let record = decision.record().unwrap();
        assert_eq!(record["basis"], "implied");
        assert_eq!(record["consent_jurisdiction"], "ZZ-1");
        assert_eq!(record["regime"], "implied");

        let blocked = Decision {
            code: None,
            rules,
            outcome: Err(Blocked::Withdrawn),
        };
        assert!(!blocked.sends());
        assert_eq!(blocked.record().unwrap()["blocked"], "withdrawn");

        tx.rollback().await.unwrap();
    })
    .await;
}
