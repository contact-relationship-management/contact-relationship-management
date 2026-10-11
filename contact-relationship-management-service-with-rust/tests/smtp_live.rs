//! Send one message through the configured SMTP server and read it back.
//!
//! Ignored by default: it needs a mail server. Run it with
//! `scripts/email-dev.sh live`, which starts the local Stalwart server and sets
//! the `CRM_SMTP_*` variables. Without `CRM_MAIL_MODE=smtp` it does nothing.

use contact_relationship_management_service::mail::{self, Message, Sender};

#[tokio::test]
#[ignore = "needs a mail server; run with scripts/email-dev.sh live"]
async fn the_smtp_sender_delivers_to_a_real_server() {
    let sender = mail::from_settings(&|name| std::env::var(name).ok(), false).unwrap();
    if !matches!(sender, Sender::Smtp(_)) {
        eprintln!("CRM_MAIL_MODE is not smtp; skipping");
        return;
    }
    let jmap =
        std::env::var("CRM_TEST_JMAP_URL").unwrap_or_else(|_| "http://127.0.0.1:8081".into());
    let user = std::env::var("CRM_TEST_MAILBOX_USER").unwrap_or_else(|_| "crm@crm.test".into());
    let password =
        std::env::var("CRM_TEST_MAILBOX_PASSWORD").unwrap_or_else(|_| "crm-dev-password".into());

    let subject = format!("live check {}", uuid::Uuid::new_v4());
    let message = Message {
        // The dev domain has a catch-all, so any local part reaches the mailbox.
        to: format!("{}@crm.test", uuid::Uuid::new_v4()),
        to_name: "Live Check".into(),
        subject: subject.clone(),
        text: "Sent by tests/smtp_live.rs".into(),
        message_id: sender.new_message_id(),
    };
    sender
        .send(&message)
        .await
        .expect("the server accepts the message");

    let http = reqwest::Client::new();
    let session = http
        .get(format!("{jmap}/jmap/session"))
        .basic_auth(&user, Some(&password))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    // Stalwart 0.16's session document is not strictly valid JSON, so read the
    // account id with a pattern.
    let marker = "\"urn:ietf:params:jmap:mail\":\"";
    let start = session.find(marker).expect("a mail account in the session") + marker.len();
    let account = &session[start..start + session[start..].find('"').unwrap()];

    let query = serde_json::json!({
        "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
        "methodCalls": [["Email/query",
            { "accountId": account, "filter": { "subject": subject } }, "q"]],
    });
    for _ in 0..20 {
        let reply: serde_json::Value = http
            .post(format!("{jmap}/jmap"))
            .basic_auth(&user, Some(&password))
            .json(&query)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if reply["methodResponses"][0][1]["ids"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    panic!("the message was accepted but never arrived: {subject}");
}
