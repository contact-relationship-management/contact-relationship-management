//! Cloud-provider adapters: Amazon SES and Azure Communication Services Email
//! (CRM-T39). Sending, request signing, and the event payloads each cloud
//! delivers (SES through SNS, Azure through Event Grid).
//!
//! Built from the vendors' documentation, with the `SigV4` signer checked
//! against a published AWS test vector and an independent Python
//! implementation. Nothing has run against a live AWS or Azure account.
//! Read [`crate::provider`] for the shared caveats.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::mail::{Message, SendError};
use crate::provider::{Inbound, Outcome, classify, person_text, tag_of, unsubscribe_pairs};

/// The SES event tag name that carries our id.
const SES_TAG: &str = "crm_message_id";

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    #[allow(clippy::expect_used, reason = "HMAC accepts a key of any length")]
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

// ---------------------------------------------------------------- Amazon SES

/// SES tag values allow only letters, digits, `_`, `-` and `.`, so `@` is
/// carried as `_at_`.
#[must_use]
pub fn ses_tag(message_id: &str) -> String {
    tag_of(message_id).replacen('@', "_at_", 1)
}

fn ses_id(tag: &str) -> String {
    format!("<{}>", tag.replacen("_at_", "@", 1))
}

/// The `SendEmail` (v2) request body.
#[must_use]
pub fn ses_body(
    from: &lettre::message::Mailbox,
    message: &Message,
    config_set: Option<&str>,
) -> Value {
    let mut simple = json!({
        "Subject": { "Data": message.subject, "Charset": "UTF-8" },
        "Body": { "Text": { "Data": message.text, "Charset": "UTF-8" } },
    });
    let headers: Vec<Value> = unsubscribe_pairs(message)
        .into_iter()
        .map(|(name, value)| json!({ "Name": name, "Value": value }))
        .collect();
    if !headers.is_empty() {
        simple["Headers"] = Value::Array(headers);
    }
    let mut body = json!({
        "FromEmailAddress": person_text(from.name.as_deref().unwrap_or(""), from.email.as_ref()),
        "Destination": { "ToAddresses": [person_text(&message.to_name, &message.to)] },
        "Content": { "Simple": simple },
        "EmailTags": [{ "Name": SES_TAG, "Value": ses_tag(&message.message_id) }],
    });
    if let Some(set) = config_set {
        body["ConfigurationSetName"] = json!(set);
    }
    body
}

/// An AWS Signature Version 4 signature (hex) for the given request parts.
/// `headers` are lowercase `(name, value)` pairs, already sorted by name.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn sigv4_signature(
    method: &str,
    path: &str,
    query: &str,
    headers: &[(String, String)],
    body: &[u8],
    amz_date: &str,
    region: &str,
    service: &str,
    secret_key: &str,
) -> String {
    let canonical_headers = headers.iter().fold(String::new(), |mut out, (n, v)| {
        use std::fmt::Write as _;
        let _ = writeln!(out, "{n}:{}", v.trim());
        out
    });
    let signed = headers
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let canonical = format!(
        "{method}\n{path}\n{query}\n{canonical_headers}\n{signed}\n{}",
        sha256_hex(body)
    );
    let day = &amz_date[..8];
    let scope = format!("{day}/{region}/{service}/aws4_request");
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(canonical.as_bytes())
    );
    let k_date = hmac(format!("AWS4{secret_key}").as_bytes(), day.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, service.as_bytes());
    let k_signing = hmac(&k_service, b"aws4_request");
    hex(&hmac(&k_signing, to_sign.as_bytes()))
}

/// Everything SES signs with.
#[derive(Debug)]
pub struct SesCredentials {
    /// Access key id.
    pub access_key: String,
    /// Secret access key.
    pub secret_key: String,
    /// Temporary-credential session token, if any.
    pub session_token: Option<String>,
    /// AWS region, for example `eu-west-1`.
    pub region: String,
}

/// The headers to add to an SES request: `x-amz-date`, optional
/// `x-amz-security-token`, and `Authorization`.
#[must_use]
pub fn ses_headers(
    creds: &SesCredentials,
    host: &str,
    path: &str,
    body: &[u8],
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<(String, String)> {
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let mut signed = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("host".to_string(), host.to_string()),
        ("x-amz-date".to_string(), amz_date.clone()),
    ];
    if let Some(token) = &creds.session_token {
        signed.push(("x-amz-security-token".to_string(), token.clone()));
    }
    let signature = sigv4_signature(
        "POST",
        path,
        "",
        &signed,
        body,
        &amz_date,
        &creds.region,
        "ses",
        &creds.secret_key,
    );
    let names = signed
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let scope = format!("{}/{}/ses/aws4_request", &amz_date[..8], creds.region);
    let mut out = vec![
        ("x-amz-date".to_string(), amz_date),
        (
            "authorization".to_string(),
            format!(
                "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={names}, Signature={signature}",
                creds.access_key
            ),
        ),
    ];
    if let Some(token) = &creds.session_token {
        out.push(("x-amz-security-token".to_string(), token.clone()));
    }
    out
}

/// Read SES's answer. `error_type` is the `x-amzn-ErrorType` header, which
/// can carry a `:` suffix.
///
/// # Errors
///
/// Permanent for a message SES rejects; transient for account, sending-pause,
/// domain-verification, configuration and throttling problems, which would
/// fail every message.
pub fn interpret_ses(
    status: u16,
    error_type: &str,
    body: &str,
) -> Result<Option<String>, SendError> {
    const ACCOUNT_LEVEL: [&str; 5] = [
        "SendingPausedException",
        "AccountSuspendedException",
        "MailFromDomainNotVerifiedException",
        "ConfigurationSetSendingPausedException",
        "NotFoundException",
    ];

    let parsed: Option<Value> = serde_json::from_str(body).ok();
    if (200..300).contains(&status) {
        return Ok(parsed.and_then(|v| {
            v.get("MessageId")
                .and_then(Value::as_str)
                .map(str::to_string)
        }));
    }
    let kind = error_type.split(':').next().unwrap_or("");
    let detail = parsed
        .as_ref()
        .and_then(|v| v.get("message").or_else(|| v.get("Message")))
        .and_then(Value::as_str)
        .unwrap_or(kind)
        .to_string();
    if ACCOUNT_LEVEL.contains(&kind)
        || kind == "TooManyRequestsException"
        || kind == "LimitExceededException"
    {
        return Err(SendError::Transient(format!(
            "HTTP {status}: {kind}: {detail}"
        )));
    }
    Err(classify(status, &detail))
}

/// An SES event, directly or wrapped by SNS.
///
/// SNS delivers `SubscriptionConfirmation` first; its `SubscribeURL` must be
/// fetched once. The signature on SNS messages is **not** verified here (that
/// needs the signing certificate); the request's shared secret is.
#[must_use]
pub fn ses_inbound(body: &[u8]) -> Option<Inbound> {
    let value: Value = serde_json::from_slice(body).ok()?;
    match value.get("Type").and_then(Value::as_str) {
        Some("SubscriptionConfirmation") => {
            let url = value.get("SubscribeURL").and_then(Value::as_str)?;
            return Some(Inbound::Subscribe(url.to_string()));
        }
        Some("Notification") => {
            let inner: Value =
                serde_json::from_str(value.get("Message").and_then(Value::as_str)?).ok()?;
            return Some(Inbound::Outcomes(ses_event(&inner)));
        }
        Some(_) => return Some(Inbound::Outcomes(Vec::new())),
        None => {}
    }
    Some(Inbound::Outcomes(ses_event(&value)))
}

fn ses_event(event: &Value) -> Vec<Outcome> {
    let kind = event
        .get("eventType")
        .or_else(|| event.get("notificationType"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let outcome = match kind {
        "Delivery" => "delivered",
        "Complaint" => "complaint",
        "Open" => "opened",
        "Click" => "clicked",
        "Bounce" => {
            if event.pointer("/bounce/bounceType").and_then(Value::as_str) == Some("Permanent") {
                "bounce_hard"
            } else {
                "bounce_soft"
            }
        }
        _ => return Vec::new(),
    };
    let tag = event
        .pointer(&format!("/mail/tags/{SES_TAG}/0"))
        .and_then(Value::as_str);
    tag.map(|t| Outcome {
        message_id: ses_id(t),
        event: outcome,
    })
    .into_iter()
    .collect()
}

/// Whether an SNS `SubscribeURL` is one we may fetch: https, an AWS host.
#[must_use]
pub fn subscribe_url_ok(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str().is_some_and(|h| {
                h.starts_with("sns.")
                    && (h.ends_with(".amazonaws.com") || h.ends_with(".amazonaws.com.cn"))
            })
    })
}

// ------------------------------------------------------------ Azure (ACS)

/// The Azure Communication Services Email request body.
#[must_use]
pub fn azure_body(from: &lettre::message::Mailbox, message: &Message) -> Value {
    let mut to = json!({ "address": message.to });
    if !message.to_name.is_empty() {
        to["displayName"] = json!(message.to_name);
    }
    let mut body = json!({
        "senderAddress": from.email.to_string(),
        "content": { "subject": message.subject, "plainText": message.text },
        "recipients": { "to": [to] },
        "userEngagementTrackingDisabled": true,
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

/// The headers to add to an Azure request (HMAC-SHA256 with the access key):
/// `x-ms-date`, `x-ms-content-sha256`, `Authorization`.
///
/// # Errors
///
/// When the access key is not base64.
pub fn azure_headers(
    access_key: &str,
    host: &str,
    path_and_query: &str,
    body: &[u8],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<(String, String)>, String> {
    let key = STANDARD
        .decode(access_key)
        .map_err(|_| "CRM_AZURE_ACCESS_KEY is not base64".to_string())?;
    let date = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let content_hash = STANDARD.encode(Sha256::digest(body));
    let to_sign = format!("POST\n{path_and_query}\n{date};{host};{content_hash}");
    let signature = STANDARD.encode(hmac(&key, to_sign.as_bytes()));
    Ok(vec![
        ("x-ms-date".to_string(), date),
        ("x-ms-content-sha256".to_string(), content_hash),
        (
            "authorization".to_string(),
            format!(
                "HMAC-SHA256 SignedHeaders=x-ms-date;host;x-ms-content-sha256&Signature={signature}"
            ),
        ),
    ])
}

/// Read Azure's answer: `202` with the operation `id`, which is the id its
/// delivery events carry.
///
/// # Errors
///
/// Permanent for `400` (the message), transient for the rest.
pub fn interpret_azure(status: u16, body: &str) -> Result<Option<String>, SendError> {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    if (200..300).contains(&status) {
        return Ok(parsed.and_then(|v| v.get("id").and_then(Value::as_str).map(str::to_string)));
    }
    let detail = parsed
        .and_then(|v| {
            v.pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    Err(classify(status, &detail))
}

/// Event Grid events for delivery reports. The subscription handshake is
/// answered with its code; reports are matched on the operation id returned at
/// send. `Delivered` is delivered; `Bounced` and `Suppressed` are hard
/// bounces; `Failed`, `Quarantined` and `FilteredSpam` are soft (recorded,
/// not suppressed). Azure reports no spam complaints here.
#[must_use]
pub fn azure_inbound(body: &[u8]) -> Option<Inbound> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let items = match &value {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let mut out = Vec::new();
    for item in &items {
        let kind = item.get("eventType").and_then(Value::as_str).unwrap_or("");
        if kind == "Microsoft.EventGrid.SubscriptionValidationEvent" {
            let code = item
                .pointer("/data/validationCode")
                .and_then(Value::as_str)?;
            return Some(Inbound::Respond(json!({ "validationResponse": code })));
        }
        if kind != "Microsoft.Communication.EmailDeliveryReportReceived" {
            continue;
        }
        let event = match item.pointer("/data/status").and_then(Value::as_str) {
            Some("Delivered") => "delivered",
            Some("Bounced" | "Suppressed") => "bounce_hard",
            Some("Failed" | "Quarantined" | "FilteredSpam") => "bounce_soft",
            _ => continue,
        };
        if let Some(id) = item.pointer("/data/messageId").and_then(Value::as_str) {
            out.push(Outcome {
                message_id: id.to_string(),
                event,
            });
        }
    }
    Some(Inbound::Outcomes(out))
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
            message_id: "<abc-123@example.test>".into(),
            unsubscribe_url: Some("https://crm.test/unsubscribe/t".into()),
        }
    }

    /// AWS's published `get-vanilla` Signature V4 test case.
    #[test]
    fn sigv4_matches_the_published_test_vector() {
        let headers = vec![
            ("host".to_string(), "example.amazonaws.com".to_string()),
            ("x-amz-date".to_string(), "20150830T123600Z".to_string()),
        ];
        let sig = sigv4_signature(
            "GET",
            "/",
            "",
            &headers,
            b"",
            "20150830T123600Z",
            "us-east-1",
            "service",
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        );
        assert_eq!(
            sig,
            "5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
        );
    }

    #[test]
    fn ses_headers_name_the_credential_scope_and_the_signed_headers() {
        let creds = SesCredentials {
            access_key: "AKIDEXAMPLE".into(),
            secret_key: "secret".into(),
            session_token: Some("tok".into()),
            region: "eu-west-1".into(),
        };
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-13T01:02:03Z")
            .unwrap()
            .to_utc();
        let headers = ses_headers(
            &creds,
            "email.eu-west-1.amazonaws.com",
            "/v2/email/outbound-emails",
            b"{}",
            now,
        );
        let auth = &headers
            .iter()
            .find(|(n, _)| n == "authorization")
            .unwrap()
            .1;
        assert!(
            auth.starts_with(
                "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20261013/eu-west-1/ses/aws4_request, \
             SignedHeaders=content-type;host;x-amz-date;x-amz-security-token, Signature="
            ),
            "{auth}"
        );
        assert!(
            headers
                .iter()
                .any(|(n, v)| n == "x-amz-date" && v == "20261013T010203Z")
        );
    }

    #[test]
    fn the_ses_body_tags_our_id_in_a_form_ses_accepts() {
        let body = ses_body(&from(), &message(), Some("crm-events"));
        let tag = body["EmailTags"][0]["Value"].as_str().unwrap();
        assert_eq!(tag, "abc-123_at_example.test");
        assert!(
            tag.chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c))
        );
        assert_eq!(body["ConfigurationSetName"], "crm-events");
        assert_eq!(body["FromEmailAddress"], "CRM <crm@example.test>");
        assert_eq!(body["Destination"]["ToAddresses"][0], "Ada <a@b.test>");
        assert_eq!(
            body["Content"]["Simple"]["Headers"][0]["Name"],
            "List-Unsubscribe"
        );
        assert_eq!(ses_id(tag), "<abc-123@example.test>");
    }

    #[test]
    fn ses_answers_are_classified() {
        assert_eq!(
            interpret_ses(200, "", r#"{"MessageId":"0100abc"}"#).unwrap(),
            Some("0100abc".to_string())
        );
        let rejected = interpret_ses(
            400,
            "MessageRejected:http://x",
            r#"{"message":"Email address is not verified."}"#,
        );
        assert!(matches!(rejected, Err(SendError::Permanent(t)) if t.contains("not verified")));
        for kind in [
            "SendingPausedException",
            "AccountSuspendedException",
            "TooManyRequestsException",
        ] {
            assert!(
                matches!(interpret_ses(400, kind, "{}"), Err(SendError::Transient(_))),
                "{kind}"
            );
        }
        assert!(matches!(
            interpret_ses(403, "", "{}"),
            Err(SendError::Transient(_))
        ));
        assert!(matches!(
            interpret_ses(503, "", "{}"),
            Err(SendError::Transient(_))
        ));
    }

    fn outcomes(inbound: Option<Inbound>) -> Vec<Outcome> {
        match inbound {
            Some(Inbound::Outcomes(list)) => list,
            other => panic!("not outcomes: {other:?}"),
        }
    }

    #[test]
    fn ses_events_arrive_direct_or_inside_an_sns_envelope() {
        let event = json!({
            "eventType": "Bounce",
            "bounce": { "bounceType": "Permanent" },
            "mail": { "tags": { "crm_message_id": ["abc-123_at_example.test"] } }
        });
        let direct = outcomes(ses_inbound(event.to_string().as_bytes()));
        assert_eq!(
            direct,
            vec![Outcome {
                message_id: "<abc-123@example.test>".into(),
                event: "bounce_hard"
            }]
        );
        let wrapped = json!({ "Type": "Notification", "Message": event.to_string() });
        assert_eq!(
            outcomes(ses_inbound(wrapped.to_string().as_bytes())),
            direct
        );

        let soft = json!({ "notificationType": "Bounce", "bounce": { "bounceType": "Transient" },
            "mail": { "tags": { "crm_message_id": ["x_at_y"] } } });
        assert_eq!(
            outcomes(ses_inbound(soft.to_string().as_bytes()))[0].event,
            "bounce_soft"
        );
        let complaint = json!({ "eventType": "Complaint", "mail": { "tags": { "crm_message_id": ["x_at_y"] } } });
        assert_eq!(
            outcomes(ses_inbound(complaint.to_string().as_bytes()))[0].event,
            "complaint"
        );
        let delivery = json!({ "eventType": "Delivery", "mail": { "tags": { "crm_message_id": ["x_at_y"] } } });
        assert_eq!(
            outcomes(ses_inbound(delivery.to_string().as_bytes()))[0].event,
            "delivered"
        );
        let open =
            json!({ "eventType": "Open", "mail": { "tags": { "crm_message_id": ["x_at_y"] } } });
        assert_eq!(
            outcomes(ses_inbound(open.to_string().as_bytes()))[0].event,
            "opened"
        );
        let click =
            json!({ "eventType": "Click", "mail": { "tags": { "crm_message_id": ["x_at_y"] } } });
        assert_eq!(
            outcomes(ses_inbound(click.to_string().as_bytes()))[0].event,
            "clicked"
        );
        let send =
            json!({ "eventType": "Send", "mail": { "tags": { "crm_message_id": ["x_at_y"] } } });
        assert!(outcomes(ses_inbound(send.to_string().as_bytes())).is_empty());
    }

    #[test]
    fn an_sns_subscription_is_offered_for_confirmation_only_for_aws_hosts() {
        let body = json!({ "Type": "SubscriptionConfirmation",
            "SubscribeURL": "https://sns.eu-west-1.amazonaws.com/?Action=ConfirmSubscription&Token=t" });
        assert!(matches!(
            ses_inbound(body.to_string().as_bytes()),
            Some(Inbound::Subscribe(_))
        ));
        assert!(subscribe_url_ok("https://sns.eu-west-1.amazonaws.com/?x=1"));
        assert!(!subscribe_url_ok("http://sns.eu-west-1.amazonaws.com/"));
        assert!(!subscribe_url_ok(
            "https://evil.example/sns.eu-west-1.amazonaws.com"
        ));
        assert!(!subscribe_url_ok("https://sns.amazonaws.com.evil.example/"));
        assert!(!subscribe_url_ok("https://169.254.169.254/"));
        assert!(ses_inbound(b"nope").is_none());
    }

    #[test]
    fn the_azure_body_and_signature_have_the_documented_shape() {
        let body = azure_body(&from(), &message());
        assert_eq!(body["senderAddress"], "crm@example.test");
        assert_eq!(body["recipients"]["to"][0]["displayName"], "Ada");
        assert_eq!(body["content"]["plainText"], "body");
        assert_eq!(
            body["headers"]["List-Unsubscribe-Post"],
            "List-Unsubscribe=One-Click"
        );
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-13T01:02:03Z")
            .unwrap()
            .to_utc();
        let key = STANDARD.encode(b"0123456789abcdef");
        let headers = azure_headers(
            &key,
            "x.communication.azure.com",
            "/emails:send?api-version=2023-03-31",
            b"{}",
            now,
        )
        .unwrap();
        let value = |n: &str| headers.iter().find(|(h, _)| h == n).unwrap().1.clone();
        assert_eq!(value("x-ms-date"), "Tue, 13 Oct 2026 01:02:03 GMT");
        assert_eq!(
            value("x-ms-content-sha256"),
            STANDARD.encode(Sha256::digest(b"{}"))
        );
        assert!(value("authorization").starts_with(
            "HMAC-SHA256 SignedHeaders=x-ms-date;host;x-ms-content-sha256&Signature="
        ));
        // The signature is over the documented string, with the decoded key.
        let expected = STANDARD.encode(hmac(
            b"0123456789abcdef",
            format!(
                "POST\n/emails:send?api-version=2023-03-31\nTue, 13 Oct 2026 01:02:03 GMT;x.communication.azure.com;{}",
                value("x-ms-content-sha256")
            )
            .as_bytes(),
        ));
        assert!(value("authorization").ends_with(&expected));
        assert!(azure_headers("not base64!", "h", "/", b"", now).is_err());
    }

    #[test]
    fn azure_answers_and_events_are_read() {
        assert_eq!(
            interpret_azure(202, r#"{"id":"op-1","status":"Running"}"#).unwrap(),
            Some("op-1".to_string())
        );
        assert!(matches!(
            interpret_azure(400, r#"{"error":{"message":"bad"}}"#),
            Err(SendError::Permanent(_))
        ));
        assert!(matches!(
            interpret_azure(429, "{}"),
            Err(SendError::Transient(_))
        ));
        let report = |status: &str| {
            json!([{ "eventType": "Microsoft.Communication.EmailDeliveryReportReceived",
                     "data": { "messageId": "op-1", "status": status } }])
            .to_string()
        };
        let one = |status: &str| outcomes(azure_inbound(report(status).as_bytes()));
        assert_eq!(
            one("Delivered")[0],
            Outcome {
                message_id: "op-1".into(),
                event: "delivered"
            }
        );
        assert_eq!(one("Bounced")[0].event, "bounce_hard");
        assert_eq!(one("Suppressed")[0].event, "bounce_hard");
        assert_eq!(one("Failed")[0].event, "bounce_soft");
        assert!(one("Expanded").is_empty());
        let handshake = json!([{ "eventType": "Microsoft.EventGrid.SubscriptionValidationEvent",
                                 "data": { "validationCode": "abc" } }]);
        assert!(matches!(
            azure_inbound(handshake.to_string().as_bytes()),
            Some(Inbound::Respond(v)) if v["validationResponse"] == "abc"
        ));
    }
}
