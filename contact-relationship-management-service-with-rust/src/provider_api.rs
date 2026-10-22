//! Mailgun and `SendGrid` adapters (CRM-T39): HTTP send and webhook payloads.
//!
//! Built from the vendors' documentation; nothing has run against a live
//! account. Read [`crate::provider`] for the shared caveats.
//!
//! Mailgun signs its webhooks (HMAC-SHA256 over `timestamp + token` with the
//! account's webhook signing key), and that signature is verified here.
//! `SendGrid`'s signed Event Webhook uses ECDSA; that check is **not built**, so
//! a `SendGrid` callback is authenticated by the shared secret instead.

use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha2::Sha256;

use crate::mail::{Message, SendError};
use crate::provider::{Outcome, classify, person_text, tag_of, unsubscribe_pairs};

/// The custom variable / argument that carries our id.
const VAR: &str = "crm_message_id";

/// How far a Mailgun timestamp may be from now, in seconds.
pub const MAILGUN_TOLERANCE_SECS: i64 = 300;

/// The form fields for `POST /v3/{domain}/messages`.
#[must_use]
pub fn mailgun_form(from: &lettre::message::Mailbox, message: &Message) -> Vec<(String, String)> {
    let mut form = vec![
        (
            "from".to_string(),
            person_text(from.name.as_deref().unwrap_or(""), from.email.as_ref()),
        ),
        ("to".to_string(), person_text(&message.to_name, &message.to)),
        ("subject".to_string(), message.subject.clone()),
        ("text".to_string(), message.text.clone()),
        ("o:tracking".to_string(), "no".to_string()),
        (format!("v:{VAR}"), tag_of(&message.message_id)),
    ];
    for (name, value) in unsubscribe_pairs(message) {
        form.push((format!("h:{name}"), value));
    }
    form
}

/// Read Mailgun's answer (`200` with the queued message `id`).
///
/// # Errors
///
/// Permanent for `400`; transient for the rest, including `404` (an unknown
/// sending domain is a setting, not the contact's fault).
pub fn interpret_mailgun(status: u16, body: &str) -> Result<Option<String>, SendError> {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    if (200..300).contains(&status) {
        return Ok(parsed.and_then(|v| v.get("id").and_then(Value::as_str).map(str::to_string)));
    }
    let detail = parsed
        .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    if status == 404 {
        return Err(SendError::Transient(format!("HTTP 404: {detail}")));
    }
    Err(classify(status, &detail))
}

/// Whether a Mailgun webhook is signed with `key` and recent.
#[must_use]
pub fn mailgun_verified(key: &str, body: &[u8], now: i64) -> bool {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return false;
    };
    let field = |name: &str| {
        value
            .pointer(&format!("/signature/{name}"))
            .and_then(Value::as_str)
    };
    let (Some(timestamp), Some(token), Some(signature)) =
        (field("timestamp"), field("token"), field("signature"))
    else {
        return false;
    };
    let Ok(sent_at) = timestamp.parse::<i64>() else {
        return false;
    };
    if (now - sent_at).abs() > MAILGUN_TOLERANCE_SECS || token.is_empty() {
        return false;
    }
    let Some(given) = unhex(signature) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(key.as_bytes()) else {
        return false;
    };
    mac.update(timestamp.as_bytes());
    mac.update(token.as_bytes());
    mac.verify_slice(&given).is_ok()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

/// The signing key for Mailgun webhooks, if configured.
#[must_use]
pub fn mailgun_signing_key() -> Option<String> {
    crate::settings::current().mailgun_signing_key.clone()
}

/// Mailgun events: `delivered`; `failed` split on `severity` (`permanent` is
/// a hard bounce, anything else soft); `complained`. Opens, clicks and
/// unsubscribes are ignored.
#[must_use]
pub fn mailgun_outcomes(body: &[u8]) -> Option<Vec<Outcome>> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let data = value.get("event-data")?;
    let event = match data.get("event").and_then(Value::as_str)? {
        "delivered" => "delivered",
        "failed" => {
            if data.get("severity").and_then(Value::as_str) == Some("permanent") {
                "bounce_hard"
            } else {
                "bounce_soft"
            }
        }
        "complained" => "complaint",
        _ => return Some(Vec::new()),
    };
    let tag = data
        .pointer(&format!("/user-variables/{VAR}"))
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty());
    Some(
        tag.map(|t| Outcome {
            message_id: format!("<{t}>"),
            event,
        })
        .into_iter()
        .collect(),
    )
}

/// The `POST /v3/mail/send` body.
#[must_use]
pub fn sendgrid_body(from: &lettre::message::Mailbox, message: &Message) -> Value {
    let mut to = json!({ "email": message.to });
    if !message.to_name.is_empty() {
        to["name"] = json!(message.to_name);
    }
    let mut sender = json!({ "email": from.email.to_string() });
    if let Some(name) = from.name.as_deref().filter(|n| !n.is_empty()) {
        sender["name"] = json!(name);
    }
    let mut body = json!({
        "personalizations": [{ "to": [to], "custom_args": { VAR: tag_of(&message.message_id) } }],
        "from": sender,
        "subject": message.subject,
        "content": [{ "type": "text/plain", "value": message.text }],
        // Link rewriting would break the signed unsubscribe link.
        "tracking_settings": {
            "click_tracking": { "enable": false },
            "open_tracking": { "enable": false },
        },
    });
    let headers: serde_json::Map<String, Value> = unsubscribe_pairs(message)
        .into_iter()
        .map(|(name, value)| (name, Value::String(value)))
        .collect();
    if !headers.is_empty() {
        body["headers"] = Value::Object(headers);
    }
    body
}

/// Read `SendGrid`'s answer (`202`, empty body).
///
/// # Errors
///
/// Permanent for `400` (and other message errors); transient for a bad key,
/// a limit or an outage.
pub fn interpret_sendgrid(status: u16, body: &str) -> Result<Option<String>, SendError> {
    if (200..300).contains(&status) {
        return Ok(None);
    }
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.pointer("/errors/0/message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    Err(classify(status, &detail))
}

/// `SendGrid` Event Webhook events (an array). `delivered`; `bounce` (type
/// `bounce` is hard, `blocked` is soft); `dropped` (`SendGrid` refused to send:
/// an invalid, previously bounced, complained or unsubscribed address) is hard;
/// `spamreport` is a complaint. The rest are ignored.
#[must_use]
pub fn sendgrid_outcomes(body: &[u8]) -> Option<Vec<Outcome>> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let items = match &value {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let mut out = Vec::new();
    for item in &items {
        let event = match item.get("event").and_then(Value::as_str)? {
            "delivered" => "delivered",
            "bounce" => {
                if item.get("type").and_then(Value::as_str) == Some("blocked") {
                    "bounce_soft"
                } else {
                    "bounce_hard"
                }
            }
            "dropped" => "bounce_hard",
            "spamreport" => "complaint",
            _ => continue,
        };
        if let Some(tag) = item
            .get(VAR)
            .and_then(Value::as_str)
            .filter(|t| !t.is_empty())
        {
            out.push(Outcome {
                message_id: format!("<{tag}>"),
                event,
            });
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from() -> lettre::message::Mailbox {
        "CRM <crm@example.test>".parse().unwrap()
    }

    fn message() -> Message {
        Message {
            to: "a@b.test".into(),
            to_name: "Ada".into(),
            subject: "Hi".into(),
            text: "body".into(),
            message_id: "<abc@example.test>".into(),
            unsubscribe_url: Some("https://crm.test/unsubscribe/t".into()),
        }
    }

    fn field<'a>(form: &'a [(String, String)], name: &str) -> &'a str {
        &form.iter().find(|(n, _)| n == name).unwrap().1
    }

    #[test]
    fn the_mailgun_form_carries_our_id_and_the_unsubscribe_headers() {
        let form = mailgun_form(&from(), &message());
        assert_eq!(field(&form, "v:crm_message_id"), "abc@example.test");
        assert_eq!(field(&form, "to"), "Ada <a@b.test>");
        assert_eq!(field(&form, "o:tracking"), "no");
        assert_eq!(
            field(&form, "h:List-Unsubscribe-Post"),
            "List-Unsubscribe=One-Click"
        );
    }

    #[test]
    fn mailgun_answers_are_classified() {
        assert_eq!(
            interpret_mailgun(200, r#"{"id":"<x@mg>","message":"Queued."}"#).unwrap(),
            Some("<x@mg>".to_string())
        );
        assert!(matches!(
            interpret_mailgun(400, r#"{"message":"bad"}"#),
            Err(SendError::Permanent(_))
        ));
        for status in [401, 402, 404, 429, 500] {
            assert!(
                matches!(
                    interpret_mailgun(status, "{}"),
                    Err(SendError::Transient(_))
                ),
                "{status}"
            );
        }
    }

    fn signed(key: &str, timestamp: &str, token: &str, event: &Value) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap();
        mac.update(timestamp.as_bytes());
        mac.update(token.as_bytes());
        let signature = mac
            .finalize()
            .into_bytes()
            .iter()
            .fold(String::new(), |mut out, b| {
                use std::fmt::Write as _;
                let _ = write!(out, "{b:02x}");
                out
            });
        json!({ "signature": { "timestamp": timestamp, "token": token, "signature": signature },
                "event-data": event })
        .to_string()
    }

    #[test]
    fn a_mailgun_signature_is_checked_with_a_window() {
        let key = "signing-key";
        let body = signed(key, "1000", "tok", &json!({"event": "delivered"}));
        assert!(mailgun_verified(key, body.as_bytes(), 1000));
        assert!(mailgun_verified(
            key,
            body.as_bytes(),
            1000 + MAILGUN_TOLERANCE_SECS
        ));
        assert!(!mailgun_verified(
            key,
            body.as_bytes(),
            1000 + MAILGUN_TOLERANCE_SECS + 1
        ));
        assert!(!mailgun_verified("other", body.as_bytes(), 1000));
        let tampered = body.replace("tok", "tok2");
        assert!(!mailgun_verified(key, tampered.as_bytes(), 1000));
        assert!(!mailgun_verified(key, b"{}", 1000));
        assert!(!mailgun_verified(key, b"nope", 1000));
    }

    #[test]
    fn mailgun_events_map_to_outcomes() {
        let event = |name: &str, severity: &str| {
            json!({ "event-data": { "event": name, "severity": severity,
                    "user-variables": { "crm_message_id": "abc@example.test" } } })
            .to_string()
        };
        let one = |name: &str, severity: &str| {
            mailgun_outcomes(event(name, severity).as_bytes()).unwrap()
        };
        assert_eq!(
            one("delivered", "")[0],
            Outcome {
                message_id: "<abc@example.test>".into(),
                event: "delivered"
            }
        );
        assert_eq!(one("failed", "permanent")[0].event, "bounce_hard");
        assert_eq!(one("failed", "temporary")[0].event, "bounce_soft");
        assert_eq!(one("complained", "")[0].event, "complaint");
        assert!(one("opened", "").is_empty());
        let unmatched = json!({ "event-data": { "event": "delivered" } }).to_string();
        assert!(mailgun_outcomes(unmatched.as_bytes()).unwrap().is_empty());
    }

    #[test]
    fn the_sendgrid_body_carries_our_id_and_turns_tracking_off() {
        let body = sendgrid_body(&from(), &message());
        assert_eq!(
            body["personalizations"][0]["custom_args"]["crm_message_id"],
            "abc@example.test"
        );
        assert_eq!(body["from"]["name"], "CRM");
        assert_eq!(body["tracking_settings"]["click_tracking"]["enable"], false);
        assert_eq!(
            body["headers"]["List-Unsubscribe"],
            "<https://crm.test/unsubscribe/t>"
        );
        assert_eq!(body["content"][0]["type"], "text/plain");
    }

    #[test]
    fn sendgrid_answers_and_events_are_read() {
        assert_eq!(interpret_sendgrid(202, "").unwrap(), None);
        let bad = interpret_sendgrid(400, r#"{"errors":[{"message":"bad from"}]}"#);
        assert!(matches!(bad, Err(SendError::Permanent(t)) if t.contains("bad from")));
        assert!(matches!(
            interpret_sendgrid(401, "{}"),
            Err(SendError::Transient(_))
        ));
        assert!(matches!(
            interpret_sendgrid(429, "{}"),
            Err(SendError::Transient(_))
        ));
        let batch = json!([
            { "event": "delivered", "crm_message_id": "abc@example.test" },
            { "event": "bounce", "type": "bounce", "crm_message_id": "abc@example.test" },
            { "event": "bounce", "type": "blocked", "crm_message_id": "abc@example.test" },
            { "event": "dropped", "crm_message_id": "abc@example.test" },
            { "event": "spamreport", "crm_message_id": "abc@example.test" },
            { "event": "open", "crm_message_id": "abc@example.test" },
            { "event": "delivered" }
        ]);
        let got = sendgrid_outcomes(batch.to_string().as_bytes()).unwrap();
        let events: Vec<_> = got.iter().map(|o| o.event).collect();
        assert_eq!(
            events,
            [
                "delivered",
                "bounce_hard",
                "bounce_soft",
                "bounce_hard",
                "complaint"
            ]
        );
        assert!(sendgrid_outcomes(b"nope").is_none());
    }
}
