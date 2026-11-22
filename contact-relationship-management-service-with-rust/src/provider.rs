//! Provider adapters for Brevo and Mailjet (CRM-T39): sending over their HTTP
//! APIs, and reading their webhook payloads.
//!
//! **Why HTTP and not their SMTP relays.** A provider's webhook reports the
//! message under the provider's own id, not the `Message-ID` we put on the
//! wire. Sending through the API lets us attach our id as a tag (Brevo) or a
//! `CustomID` (Mailjet), and both give it back on every event, which is how an
//! event finds its delivery row. The generic `smtp` mode still works with
//! either vendor's relay, but their events then cannot be matched.
//!
//! **What was checked.** The request and event shapes below follow the
//! vendors' published documentation (Brevo `POST /v3/smtp/email` and
//! transactional webhooks; Mailjet Send API v3.1 and event callbacks). They
//! have not been run against a live account: no credentials were available.
//! The unit tests prove our parsing of the documented shapes, nothing more.
//! Run one real message and one real event of each kind before relying on
//! this.

use crate::rules::absent::OrEmpty;
use std::time::Duration;

use lettre::message::Mailbox;
use serde_json::{Value, json};

use crate::mail::{Message, SendError};

/// The vendors we can talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Brevo (formerly Sendinblue).
    Brevo,
    /// Mailjet.
    Mailjet,
    /// Amazon SES (API v2; events through SNS).
    Ses,
    /// Azure Communication Services Email (events through Event Grid).
    Azure,
    /// Mailgun.
    Mailgun,
    /// Twilio `SendGrid`.
    SendGrid,
}

impl Kind {
    /// The name used in settings and in the webhook path.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Brevo => "brevo",
            Self::Mailjet => "mailjet",
            Self::Ses => "ses",
            Self::Azure => "azure",
            Self::Mailgun => "mailgun",
            Self::SendGrid => "sendgrid",
        }
    }

    /// The kind named in a webhook path, if it is one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "brevo" => Some(Self::Brevo),
            "mailjet" => Some(Self::Mailjet),
            "ses" => Some(Self::Ses),
            "azure" => Some(Self::Azure),
            "mailgun" => Some(Self::Mailgun),
            "sendgrid" => Some(Self::SendGrid),
            _ => None,
        }
    }
}

/// A sender that posts each message to a provider's API.
#[derive(Debug)]
pub struct ApiSender {
    kind: Kind,
    url: String,
    key: String,
    secret: String,
    /// SES region, or empty.
    region: String,
    /// SES session token, if any.
    session_token: Option<String>,
    /// SES configuration set (needed for events), if any.
    config_set: Option<String>,
    from: Mailbox,
    client: reqwest::Client,
}

impl ApiSender {
    /// The provider URL messages are posted to (CRM-R106 checks its host).
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The `From` mailbox.
    #[must_use]
    pub fn from_domain(&self) -> String {
        crate::mail::domain_of(self.from.email.as_ref())
    }

    /// Post one message. Returns the provider's own id for it when the
    /// provider gives one and its events use that id (Azure).
    ///
    /// # Errors
    ///
    /// [`SendError::Permanent`] when the provider rejects this message,
    /// [`SendError::Transient`] for an outage, a limit, or an account problem
    /// (a bad key or credit would fail every message, so those are retried
    /// rather than counted against the contact).
    pub async fn send(&self, message: &Message) -> Result<Option<String>, SendError> {
        let transient = |e: reqwest::Error| SendError::Transient(e.without_url().to_string());
        let request = self.request(message)?;
        let response = request.send().await.map_err(transient)?;
        let status = response.status().as_u16();
        let error_type = response
            .headers()
            .get("x-amzn-errortype")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let text = response.text().await.map_err(transient)?;
        match self.kind {
            Kind::Brevo => interpret_brevo(status, &text).map(|()| None),
            Kind::Mailjet => interpret_mailjet(status, &text).map(|()| None),
            Kind::Ses => {
                crate::provider_cloud::interpret_ses(status, &error_type, &text).map(|_| None)
            }
            Kind::Azure => crate::provider_cloud::interpret_azure(status, &text),
            Kind::Mailgun => crate::provider_api::interpret_mailgun(status, &text).map(|_| None),
            Kind::SendGrid => crate::provider_api::interpret_sendgrid(status, &text),
        }
    }

    /// Build the signed request for one message.
    fn request(&self, message: &Message) -> Result<reqwest::RequestBuilder, SendError> {
        let post = self.client.post(&self.url);
        let signed_json = |body: &Value| {
            serde_json::to_vec(body).map_err(|_| {
                SendError::Permanent("the request body could not be serialized".into())
            })
        };
        Ok(match self.kind {
            Kind::Brevo => post
                .header("api-key", &self.key)
                .json(&brevo_body(&self.from, message)),
            Kind::Mailjet => post
                .basic_auth(&self.key, Some(&self.secret))
                .json(&mailjet_body(&self.from, message)),
            Kind::SendGrid => post
                .bearer_auth(&self.key)
                .json(&crate::provider_api::sendgrid_body(&self.from, message)),
            Kind::Mailgun => post
                .basic_auth("api", Some(&self.key))
                .form(&crate::provider_api::mailgun_form(&self.from, message)),
            Kind::Ses => {
                let body = signed_json(&crate::provider_cloud::ses_body(
                    &self.from,
                    message,
                    self.config_set.as_deref(),
                ))?;
                let url = self.parsed_url()?;
                let creds = crate::provider_cloud::SesCredentials {
                    access_key: self.key.clone(),
                    secret_key: self.secret.clone(),
                    session_token: self.session_token.clone(),
                    region: self.region.clone(),
                };
                let headers = crate::provider_cloud::ses_headers(
                    &creds,
                    &host_header(&url),
                    url.path(),
                    &body,
                    crate::clock::now(),
                );
                with_headers(post.header("content-type", "application/json"), headers).body(body)
            }
            Kind::Azure => {
                let body = signed_json(&crate::provider_cloud::azure_body(&self.from, message))?;
                let url = self.parsed_url()?;
                let path = match url.query() {
                    Some(q) => format!("{}?{q}", url.path()),
                    None => url.path().to_string(),
                };
                let headers = crate::provider_cloud::azure_headers(
                    &self.secret,
                    &host_header(&url),
                    &path,
                    &body,
                    crate::clock::now(),
                )
                .map_err(SendError::Permanent)?;
                with_headers(post.header("content-type", "application/json"), headers).body(body)
            }
        })
    }

    fn parsed_url(&self) -> Result<reqwest::Url, SendError> {
        reqwest::Url::parse(&self.url)
            .map_err(|e| SendError::Permanent(format!("bad API URL: {e}")))
    }
}

/// The `Host` header value for a URL (the port only when it is not the default).
fn host_header(url: &reqwest::Url) -> String {
    let host = url.host_str().or_empty();
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

fn with_headers(
    mut request: reqwest::RequestBuilder,
    headers: Vec<(String, String)>,
) -> reqwest::RequestBuilder {
    for (name, value) in headers {
        request = request.header(name, value);
    }
    request
}

/// What a provider's settings add up to.
struct Credentials {
    key: String,
    secret: String,
    default_url: String,
    region: String,
    session_token: Option<String>,
    config_set: Option<String>,
}

/// Read one provider's keys and work out its endpoint.
fn credentials(kind: Kind, get: &dyn Fn(&str) -> Option<String>) -> Result<Credentials, String> {
    let need = |name: &str| get(name).ok_or_else(|| format!("{name} is required"));
    let mut region = String::new();
    let mut session_token = None;
    let mut config_set = None;
    let (key, secret, default_url) = match kind {
        Kind::Brevo => (
            need("CONTACT_RELATIONSHIP_MANAGEMENT_BREVO_API_KEY")?,
            String::new(),
            "https://api.brevo.com/v3/smtp/email".to_string(),
        ),
        Kind::Mailjet => (
            need("CONTACT_RELATIONSHIP_MANAGEMENT_MAILJET_API_KEY")?,
            need("CONTACT_RELATIONSHIP_MANAGEMENT_MAILJET_API_SECRET")?,
            "https://api.mailjet.com/v3.1/send".to_string(),
        ),
        Kind::SendGrid => (
            need("CONTACT_RELATIONSHIP_MANAGEMENT_SENDGRID_API_KEY")?,
            String::new(),
            "https://api.sendgrid.com/v3/mail/send".to_string(),
        ),
        Kind::Mailgun => {
            let domain = need("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_DOMAIN")?;
            if !domain
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
            {
                return Err(
                    "CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_DOMAIN is not a domain name"
                        .to_string(),
                );
            }
            let base = get("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_API_BASE")
                .unwrap_or_else(|| "https://api.mailgun.net".to_string());
            (
                need("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_API_KEY")?,
                String::new(),
                format!("{}/v3/{domain}/messages", base.trim_end_matches('/')),
            )
        }
        Kind::Ses => {
            region = need("CONTACT_RELATIONSHIP_MANAGEMENT_SES_REGION")?;
            if !region
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return Err(
                    "CONTACT_RELATIONSHIP_MANAGEMENT_SES_REGION is not a region name".to_string(),
                );
            }
            session_token = get("CONTACT_RELATIONSHIP_MANAGEMENT_SES_SESSION_TOKEN");
            config_set = get("CONTACT_RELATIONSHIP_MANAGEMENT_SES_CONFIGURATION_SET");
            (
                need("CONTACT_RELATIONSHIP_MANAGEMENT_SES_ACCESS_KEY_ID")?,
                need("CONTACT_RELATIONSHIP_MANAGEMENT_SES_SECRET_ACCESS_KEY")?,
                format!("https://email.{region}.amazonaws.com/v2/email/outbound-emails"),
            )
        }
        Kind::Azure => {
            let endpoint = need("CONTACT_RELATIONSHIP_MANAGEMENT_AZURE_ENDPOINT")?;
            (
                String::new(),
                need("CONTACT_RELATIONSHIP_MANAGEMENT_AZURE_ACCESS_KEY")?,
                format!(
                    "{}/emails:send?api-version=2023-03-31",
                    endpoint.trim_end_matches('/')
                ),
            )
        }
    };
    Ok(Credentials {
        key,
        secret,
        default_url,
        region,
        session_token,
        config_set,
    })
}

/// Build the sender from settings. `get` reads one setting (already trimmed
/// of nothing: this trims).
///
/// # Errors
///
/// A message when a key is missing, the `From` mailbox or URL is unusable, or
/// production is chosen without the acknowledgement.
pub fn configure(
    kind: Kind,
    get: &dyn Fn(&str) -> Option<String>,
    production: bool,
) -> Result<ApiSender, String> {
    let get = |name: &str| {
        get(name)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    if production
        && !matches!(
            get("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION")
                .map(|v| v.to_ascii_lowercase())
                .as_deref(),
            Some("1" | "true" | "yes" | "on")
        )
    {
        return Err(format!(
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE={} in production needs CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION=1: \
             the vendor agreement and legal review is open (CRM-T39)",
            kind.name()
        ));
    }
    let from: Mailbox = get("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM")
        .ok_or("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM is required")?
        .parse()
        .map_err(|e| {
            format!("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM is not a valid mailbox: {e}")
        })?;
    let Credentials {
        key,
        secret,
        default_url,
        region,
        session_token,
        config_set,
    } = credentials(kind, &get)?;
    let url = get("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_API_URL").unwrap_or(default_url);
    if production && !url.starts_with("https://") {
        return Err("the mail API URL must be https in production".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;
    Ok(ApiSender {
        kind,
        url,
        key,
        secret,
        region,
        session_token,
        config_set,
        from,
        client,
    })
}

/// Our `Message-ID` without the angle brackets: the tag/`CustomID` we send.
pub(crate) fn tag_of(message_id: &str) -> String {
    message_id
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string()
}

/// A tag read back from an event, as the `Message-ID` we stored.
fn id_of(tag: &str) -> String {
    format!("<{}>", tag_of(tag))
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The `List-Unsubscribe` header pair, empty when the message has no link.
pub(crate) fn unsubscribe_pairs(message: &Message) -> Vec<(String, String)> {
    message.unsubscribe_url.as_ref().map_or_default(|url| {
        vec![
            ("List-Unsubscribe".to_string(), format!("<{url}>")),
            (
                "List-Unsubscribe-Post".to_string(),
                "List-Unsubscribe=One-Click".to_string(),
            ),
        ]
    })
}

/// `Name <address>` when the name is plain ASCII, else just the address
/// (non-ASCII names would need RFC 2047 encoding, which is not done here).
pub(crate) fn person_text(name: &str, email: &str) -> String {
    let plain = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " .-_'".contains(c));
    if plain {
        format!("{name} <{email}>")
    } else {
        email.to_string()
    }
}

/// The `List-Unsubscribe` pair, when the message has a link.
fn unsubscribe_headers(message: &Message) -> Option<Value> {
    message.unsubscribe_url.as_ref().map(|url| {
        json!({
            "List-Unsubscribe": format!("<{url}>"),
            "List-Unsubscribe-Post": "List-Unsubscribe=One-Click",
        })
    })
}

fn person(email: &str, name: &str, email_key: &str, name_key: &str) -> Value {
    let mut out = serde_json::Map::new();
    out.insert(email_key.to_string(), json!(email));
    if !name.is_empty() {
        out.insert(name_key.to_string(), json!(name));
    }
    Value::Object(out)
}

/// The Brevo request body for `POST /v3/smtp/email`.
#[must_use]
pub fn brevo_body(from: &Mailbox, message: &Message) -> Value {
    let mut body = json!({
        "sender": person(from.email.as_ref(), from.name.as_deref().unwrap_or(""), "email", "name"),
        "to": [person(&message.to, &message.to_name, "email", "name")],
        "subject": message.subject,
        "textContent": message.text,
        // Brevo's reference marks htmlContent required without a template.
        "htmlContent": format!(
            "<html><body><pre style=\"white-space:pre-wrap;font-family:inherit\">{}</pre></body></html>",
            escape_html(&message.text)
        ),
        "tags": [tag_of(&message.message_id)],
    });
    if let Some(headers) = unsubscribe_headers(message) {
        crate::json::put(&mut body, &["headers"], headers);
    }
    body
}

/// The Mailjet Send API v3.1 request body.
#[must_use]
pub fn mailjet_body(from: &Mailbox, message: &Message) -> Value {
    let mut one = json!({
        "From": person(from.email.as_ref(), from.name.as_deref().unwrap_or(""), "Email", "Name"),
        "To": [person(&message.to, &message.to_name, "Email", "Name")],
        "Subject": message.subject,
        "TextPart": message.text,
        "CustomID": tag_of(&message.message_id),
    });
    if let Some(headers) = unsubscribe_headers(message) {
        crate::json::put(&mut one, &["Headers"], headers);
    }
    json!({ "Messages": [one] })
}

/// Which status codes say "this message is bad" rather than "try later".
pub(crate) fn classify(status: u16, detail: &str) -> SendError {
    match status {
        // 401/402/403: the account, not the message. 429: slow down.
        401 | 402 | 403 | 429 | 500..=599 => {
            SendError::Transient(format!("HTTP {status}: {detail}"))
        }
        _ => SendError::Permanent(format!("HTTP {status}: {detail}")),
    }
}

/// Read Brevo's answer: `201` (sent) or `202` (scheduled) is success.
///
/// # Errors
///
/// See [`classify`]; the text is Brevo's own `message` when it sent one.
pub fn interpret_brevo(status: u16, body: &str) -> Result<(), SendError> {
    if (200..300).contains(&status) {
        return Ok(());
    }
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_string))
        .or_empty();
    Err(classify(status, &detail))
}

/// Read Mailjet's answer. A `200` can still carry a per-message error.
///
/// # Errors
///
/// A permanent error when the message itself was refused.
pub fn interpret_mailjet(status: u16, body: &str) -> Result<(), SendError> {
    let parsed = serde_json::from_str::<Value>(body).ok();
    if (200..300).contains(&status) {
        let first = parsed
            .as_ref()
            .and_then(|v| v.get("Messages"))
            .and_then(|m| m.get(0));
        let ok = first
            .and_then(|m| m.get("Status"))
            .and_then(Value::as_str)
            .is_some_and(|s| s == "success");
        if ok {
            return Ok(());
        }
        let detail = first
            .and_then(|m| m.get("Errors"))
            .and_then(|e| e.get(0))
            .and_then(|e| e.get("ErrorMessage"))
            .and_then(Value::as_str)
            .unwrap_or("message not accepted")
            .to_string();
        return Err(SendError::Permanent(detail));
    }
    let detail = parsed
        .and_then(|v| {
            v.get("ErrorMessage")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_empty();
    Err(classify(status, &detail))
}

/// One outcome read from a webhook, in the generic vocabulary (`delivered`, `bounce_hard`,
/// `bounce_soft`, `complaint`, and the engagement events `opened` and `clicked`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Our `Message-ID`, with angle brackets.
    pub message_id: String,
    /// `delivered`, `bounce_hard`, `bounce_soft`, `complaint`, `opened` or `clicked`.
    pub event: &'static str,
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// Brevo transactional events. `delivered`, `soft_bounce`, `hard_bounce` and
/// `spam` map across; `invalid_email` and `blocked` count as hard bounces
/// (the address cannot be mailed); `opened` and `click` are engagement (CRM-R58); `request`,
/// `deferred`, `unsubscribed` and the rest are ignored.
#[must_use]
pub fn brevo_outcomes(body: &[u8]) -> Option<Vec<Outcome>> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let items = match &value {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let mut out = Vec::new();
    for item in &items {
        let event = match text(item, "event")? {
            "delivered" => "delivered",
            "soft_bounce" => "bounce_soft",
            "hard_bounce" | "invalid_email" | "blocked" => "bounce_hard",
            "spam" => "complaint",
            "opened" => "opened",
            "click" => "clicked",
            _ => continue,
        };
        // Our id travels as a tag; the vendor's own `message-id` is the
        // fallback (it matches only if the relay kept ours).
        let from_tag = match item.get("tags") {
            Some(Value::Array(tags)) => tags
                .iter()
                .filter_map(Value::as_str)
                .find(|t| t.contains('@'))
                .map(id_of),
            Some(Value::String(tag)) if tag.contains('@') => Some(id_of(tag)),
            _ => None,
        };
        if let Some(message_id) = from_tag.or_else(|| text(item, "message-id").map(id_of)) {
            out.push(Outcome { message_id, event });
        }
    }
    Some(out)
}

/// Mailjet event callbacks, as one object or an array. `sent` (Mailjet's word
/// for accepted by the recipient's server) counts as delivered; `bounce`
/// splits on `hard_bounce`; `blocked` (Mailjet did not send) is a soft
/// bounce, so it is recorded without suppressing the mailbox; `spam` is a
/// complaint. `open` and `click` are engagement (CRM-R58); `unsub` is ignored.
#[must_use]
pub fn mailjet_outcomes(body: &[u8]) -> Option<Vec<Outcome>> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let items = match &value {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let mut out = Vec::new();
    for item in &items {
        let event = match text(item, "event")? {
            "sent" => "delivered",
            "bounce" => {
                if item
                    .get("hard_bounce")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "bounce_hard"
                } else {
                    "bounce_soft"
                }
            }
            "blocked" => "bounce_soft",
            "spam" => "complaint",
            "open" => "opened",
            "click" => "clicked",
            _ => continue,
        };
        if let Some(id) = text(item, "CustomID").filter(|c| !c.is_empty()) {
            out.push(Outcome {
                message_id: id_of(id),
                event,
            });
        }
    }
    Some(out)
}

/// What a webhook request asks of us.
#[derive(Debug, Clone, PartialEq)]
pub enum Inbound {
    /// Outcomes to apply (possibly none).
    Outcomes(Vec<Outcome>),
    /// An SNS subscription to confirm by fetching this URL once.
    Subscribe(String),
    /// A handshake to answer with this JSON (Event Grid validation).
    Respond(Value),
}

/// Read a webhook body for a provider; `None` when it is not that provider's
/// format.
#[must_use]
pub fn inbound(kind: Kind, body: &[u8]) -> Option<Inbound> {
    use crate::{provider_api, provider_cloud};
    match kind {
        Kind::Brevo => brevo_outcomes(body).map(Inbound::Outcomes),
        Kind::Mailjet => mailjet_outcomes(body).map(Inbound::Outcomes),
        Kind::Mailgun => provider_api::mailgun_outcomes(body).map(Inbound::Outcomes),
        Kind::SendGrid => provider_api::sendgrid_outcomes(body).map(Inbound::Outcomes),
        Kind::Ses => provider_cloud::ses_inbound(body),
        Kind::Azure => provider_cloud::azure_inbound(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from() -> Mailbox {
        "CRM <crm@example.test>".parse().unwrap()
    }

    fn message() -> Message {
        Message {
            to: "a@b.test".into(),
            to_name: "Ada".into(),
            subject: "Hi".into(),
            text: "x < y & z".into(),
            message_id: "<abc@example.test>".into(),
            unsubscribe_url: Some("https://crm.test/unsubscribe/t".into()),
        }
    }

    #[test]
    fn the_brevo_body_carries_our_id_as_a_tag_and_the_unsubscribe_headers() {
        let body = brevo_body(&from(), &message());
        assert_eq!(body["tags"], json!(["abc@example.test"]));
        assert_eq!(body["sender"]["email"], "crm@example.test");
        assert_eq!(body["to"][0]["name"], "Ada");
        assert_eq!(body["textContent"], "x < y & z");
        assert!(
            body["htmlContent"]
                .as_str()
                .unwrap()
                .contains("x &lt; y &amp; z")
        );
        assert_eq!(
            body["headers"]["List-Unsubscribe-Post"],
            "List-Unsubscribe=One-Click"
        );
    }

    #[test]
    fn the_mailjet_body_carries_our_id_as_the_custom_id() {
        let body = mailjet_body(&from(), &message());
        let one = &body["Messages"][0];
        assert_eq!(one["CustomID"], "abc@example.test");
        assert_eq!(one["From"]["Email"], "crm@example.test");
        assert_eq!(one["TextPart"], "x < y & z");
        assert_eq!(
            one["Headers"]["List-Unsubscribe"],
            "<https://crm.test/unsubscribe/t>"
        );
    }

    #[test]
    fn an_empty_name_and_missing_link_are_left_out() {
        let mut m = message();
        m.to_name.clear();
        m.unsubscribe_url = None;
        let body = brevo_body(&from(), &m);
        assert!(body["to"][0].get("name").is_none());
        assert!(body.get("headers").is_none());
        assert!(
            mailjet_body(&from(), &m)["Messages"][0]
                .get("Headers")
                .is_none()
        );
    }

    #[test]
    fn brevo_answers_are_classified() {
        assert!(interpret_brevo(201, r#"{"messageId":"<1@x>"}"#).is_ok());
        assert!(interpret_brevo(202, "{}").is_ok());
        let bad = interpret_brevo(400, r#"{"code":"invalid_parameter","message":"bad to"}"#);
        assert!(matches!(bad, Err(SendError::Permanent(t)) if t.contains("bad to")));
        for status in [401, 402, 403, 429, 500, 503] {
            assert!(matches!(
                interpret_brevo(status, "{}"),
                Err(SendError::Transient(_))
            ));
        }
    }

    #[test]
    fn mailjet_answers_are_classified() {
        let ok = r#"{"Messages":[{"Status":"success","To":[{"MessageID":1}]}]}"#;
        assert!(interpret_mailjet(200, ok).is_ok());
        let refused =
            r#"{"Messages":[{"Status":"error","Errors":[{"ErrorMessage":"bad address"}]}]}"#;
        assert!(matches!(
            interpret_mailjet(200, refused),
            Err(SendError::Permanent(t)) if t.contains("bad address")
        ));
        assert!(matches!(
            interpret_mailjet(400, r#"{"ErrorMessage":"nope"}"#),
            Err(SendError::Permanent(_))
        ));
        assert!(matches!(
            interpret_mailjet(401, "{}"),
            Err(SendError::Transient(_))
        ));
        assert!(matches!(
            interpret_mailjet(429, "{}"),
            Err(SendError::Transient(_))
        ));
        assert!(matches!(
            interpret_mailjet(200, "not json"),
            Err(SendError::Permanent(_))
        ));
    }

    #[test]
    fn brevo_events_map_to_outcomes() {
        let body = |event: &str| {
            json!({"event": event, "email": "a@b.test", "message-id": "<v@relay>",
                   "tags": ["other", "abc@example.test"]})
            .to_string()
        };
        let one = |event: &str| brevo_outcomes(body(event).as_bytes()).unwrap();
        assert_eq!(
            one("hard_bounce"),
            vec![Outcome {
                message_id: "<abc@example.test>".into(),
                event: "bounce_hard"
            }]
        );
        assert_eq!(one("delivered")[0].event, "delivered");
        assert_eq!(one("soft_bounce")[0].event, "bounce_soft");
        assert_eq!(one("spam")[0].event, "complaint");
        assert_eq!(one("invalid_email")[0].event, "bounce_hard");
        assert_eq!(one("opened")[0].event, "opened");
        assert_eq!(one("click")[0].event, "clicked");
        // What is not engagement or an outcome is left out.
        assert!(one("request").is_empty());
        assert!(brevo_outcomes(b"nope").is_none());
    }

    #[test]
    fn a_brevo_event_without_our_tag_falls_back_to_the_vendor_id() {
        let body = json!({"event": "delivered", "message-id": "<v@relay>"}).to_string();
        assert_eq!(
            brevo_outcomes(body.as_bytes()).unwrap()[0].message_id,
            "<v@relay>"
        );
    }

    #[test]
    fn mailjet_events_map_to_outcomes_object_or_array() {
        let hard = json!({"event": "bounce", "hard_bounce": true, "CustomID": "abc@example.test"});
        let soft = json!({"event": "bounce", "hard_bounce": false, "CustomID": "abc@example.test"});
        let batch = json!([hard, soft, {"event": "sent", "CustomID": "abc@example.test"},
            {"event": "spam", "CustomID": "abc@example.test"},
            {"event": "open", "CustomID": "abc@example.test"},
            {"event": "click", "CustomID": "abc@example.test"},
            {"event": "unsub", "CustomID": "abc@example.test"},
            {"event": "sent", "CustomID": ""}]);
        let got = mailjet_outcomes(batch.to_string().as_bytes()).unwrap();
        let events: Vec<_> = got.iter().map(|o| o.event).collect();
        assert_eq!(
            events,
            [
                "bounce_hard",
                "bounce_soft",
                "delivered",
                "complaint",
                "opened",
                "clicked"
            ]
        );
        assert_eq!(got[0].message_id, "<abc@example.test>");
        let single = mailjet_outcomes(hard.to_string().as_bytes()).unwrap();
        assert_eq!(single.len(), 1);
    }

    #[test]
    fn configuration_is_checked() {
        axa::tls::install_provider();
        let all = |name: &str| match name {
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM" => {
                Some("CRM <crm@example.test>".to_string())
            }
            "CONTACT_RELATIONSHIP_MANAGEMENT_BREVO_API_KEY"
            | "CONTACT_RELATIONSHIP_MANAGEMENT_MAILJET_API_KEY" => Some("k".to_string()),
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAILJET_API_SECRET" => Some("s".to_string()),
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION" => Some("1".to_string()),
            _ => None,
        };
        assert!(configure(Kind::Brevo, &all, true).is_ok());
        assert!(configure(Kind::Mailjet, &all, true).is_ok());
        let no_secret = |n: &str| {
            if n == "CONTACT_RELATIONSHIP_MANAGEMENT_MAILJET_API_SECRET" {
                None
            } else {
                all(n)
            }
        };
        assert!(configure(Kind::Mailjet, &no_secret, false).is_err());
        let no_ack = |n: &str| {
            if n == "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION" {
                None
            } else {
                all(n)
            }
        };
        assert!(configure(Kind::Brevo, &no_ack, true).is_err());
        assert!(configure(Kind::Brevo, &no_ack, false).is_ok());
        let http = |n: &str| {
            if n == "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_API_URL" {
                Some("http://x.test".to_string())
            } else {
                all(n)
            }
        };
        assert!(configure(Kind::Brevo, &http, true).is_err());
        assert!(configure(Kind::Brevo, &http, false).is_ok());
    }

    #[test]
    fn every_provider_is_configured_from_its_own_settings() {
        axa::tls::install_provider();
        let vars = |pairs: Vec<(&'static str, &'static str)>| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_string())
            }
        };
        let from = (
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM",
            "CRM <crm@example.test>",
        );
        let ses = vars(vec![
            from,
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_REGION", "eu-west-1"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_ACCESS_KEY_ID", "AKID"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_SECRET_ACCESS_KEY", "s"),
        ]);
        let sender = configure(Kind::Ses, &ses, false).unwrap();
        assert_eq!(
            sender.url,
            "https://email.eu-west-1.amazonaws.com/v2/email/outbound-emails"
        );
        let bad_region = vars(vec![
            from,
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_REGION", "eu west"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_ACCESS_KEY_ID", "AKID"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_SECRET_ACCESS_KEY", "s"),
        ]);
        assert!(configure(Kind::Ses, &bad_region, false).is_err());
        let azure = vars(vec![
            from,
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_AZURE_ENDPOINT",
                "https://x.communication.azure.com/",
            ),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_AZURE_ACCESS_KEY", "a2V5"),
        ]);
        assert_eq!(
            configure(Kind::Azure, &azure, false).unwrap().url,
            "https://x.communication.azure.com/emails:send?api-version=2023-03-31"
        );
        let mailgun = vars(vec![
            from,
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_DOMAIN",
                "mg.example.test",
            ),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_API_KEY", "k"),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_API_BASE",
                "https://api.eu.mailgun.net/",
            ),
        ]);
        assert_eq!(
            configure(Kind::Mailgun, &mailgun, false).unwrap().url,
            "https://api.eu.mailgun.net/v3/mg.example.test/messages"
        );
        let bad_domain = vars(vec![
            from,
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_DOMAIN", "a/b"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_API_KEY", "k"),
        ]);
        assert!(configure(Kind::Mailgun, &bad_domain, false).is_err());
        let sendgrid = vars(vec![
            from,
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SENDGRID_API_KEY", "k"),
        ]);
        assert!(configure(Kind::SendGrid, &sendgrid, false).is_ok());
        assert!(configure(Kind::SendGrid, &vars(vec![from]), false).is_err());
    }
}

/// Each provider's request, sent for real to a stand-in on this machine (CRM-T292): the path, the
/// way it authenticates, the body it carries, and how the answer is read.
#[cfg(test)]
mod sent_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// One request the stand-in received.
    #[derive(Debug, Clone)]
    struct Seen {
        method: String,
        uri: String,
        headers: Vec<(String, String)>,
        body: String,
    }

    impl Seen {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        }
    }

    /// A server on a free local port that answers every request with `status` and `answer`.
    async fn stand_in(status: u16, answer: &'static str) -> (String, Arc<Mutex<Vec<Seen>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let kept = seen.clone();
        let app = axum::Router::new().fallback(move |request: axum::extract::Request| {
            let kept = kept.clone();
            async move {
                let (parts, body) = request.into_parts();
                let bytes = axum::body::to_bytes(body, 1 << 20)
                    .await
                    .unwrap_or_default();
                kept.lock().unwrap().push(Seen {
                    method: parts.method.to_string(),
                    uri: parts.uri.to_string(),
                    headers: parts
                        .headers
                        .iter()
                        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
                        .collect(),
                    body: String::from_utf8_lossy(&bytes).into_owned(),
                });
                (axum::http::StatusCode::from_u16(status).unwrap(), answer)
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        (format!("http://{address}"), seen)
    }

    fn message() -> Message {
        Message {
            to: "ada@example.test".into(),
            to_name: "Ada".into(),
            subject: "Hello".into(),
            text: "A note.".into(),
            message_id: "<m1@example.test>".into(),
            unsubscribe_url: Some("https://crm.example.test/unsubscribe/t".into()),
        }
    }

    /// Configure `kind` with `settings`, posting to `url`.
    fn sender(kind: Kind, url: &str, settings: &[(&str, &str)]) -> ApiSender {
        axa::tls::install_provider();
        let mut all: Vec<(String, String)> = settings
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        all.push((
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM".into(),
            "CRM <crm@example.test>".into(),
        ));
        all.push((
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_API_URL".into(),
            url.into(),
        ));
        let get = move |name: &str| all.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
        configure(kind, &get, false).unwrap()
    }

    async fn one(
        kind: Kind,
        path: &str,
        settings: &[(&str, &str)],
        status: u16,
        answer: &'static str,
    ) -> (Result<Option<String>, SendError>, Seen) {
        let (base, seen) = stand_in(status, answer).await;
        let result = sender(kind, &format!("{base}{path}"), settings)
            .send(&message())
            .await;
        let request = seen.lock().unwrap().first().cloned().expect("one request");
        (result, request)
    }

    #[tokio::test]
    async fn each_keyed_provider_posts_its_own_request_and_reads_its_own_answer() {
        let (sent, seen) = one(
            Kind::Brevo,
            "/v3/smtp/email",
            &[("CONTACT_RELATIONSHIP_MANAGEMENT_BREVO_API_KEY", "brevo-key")],
            201,
            r#"{"messageId":"<m1>"}"#,
        )
        .await;
        assert_eq!(sent.unwrap(), None);
        assert_eq!(
            (seen.method.as_str(), seen.uri.as_str()),
            ("POST", "/v3/smtp/email")
        );
        assert_eq!(seen.header("api-key"), Some("brevo-key"));
        assert!(seen.body.contains("ada@example.test"), "{}", seen.body);

        let (sent, seen) = one(
            Kind::Mailjet,
            "/v3.1/send",
            &[
                ("CONTACT_RELATIONSHIP_MANAGEMENT_MAILJET_API_KEY", "mj-key"),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_MAILJET_API_SECRET",
                    "mj-secret",
                ),
            ],
            200,
            r#"{"Messages":[{"Status":"success"}]}"#,
        )
        .await;
        assert_eq!(sent.unwrap(), None);
        assert!(seen.header("authorization").unwrap().starts_with("Basic "));

        let (sent, seen) = one(
            Kind::SendGrid,
            "/v3/mail/send",
            &[("CONTACT_RELATIONSHIP_MANAGEMENT_SENDGRID_API_KEY", "sg-key")],
            202,
            "",
        )
        .await;
        assert_eq!(sent.unwrap(), None);
        assert_eq!(seen.header("authorization"), Some("Bearer sg-key"));

        let (sent, seen) = one(
            Kind::Mailgun,
            "/v3/mg.example.test/messages",
            &[
                ("CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_API_KEY", "mg-key"),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_MAILGUN_DOMAIN",
                    "mg.example.test",
                ),
            ],
            200,
            r#"{"id":"<x@mg>","message":"Queued."}"#,
        )
        .await;
        assert_eq!(sent.unwrap(), None);
        assert!(seen.header("authorization").unwrap().starts_with("Basic "));
        assert!(
            seen.header("content-type")
                .unwrap()
                .starts_with("application/x-www-form-urlencoded")
        );
        assert!(
            seen.body.contains("to=ada%40example.test") || seen.body.contains("ada%40example.test"),
            "{}",
            seen.body
        );
    }

    #[tokio::test]
    async fn the_signed_providers_sign_each_request_and_keep_azure_s_id() {
        let (sent, seen) = one(
            Kind::Ses,
            "/v2/email/outbound-emails",
            &[
                ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_REGION", "eu-west-2"),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_SES_ACCESS_KEY_ID",
                    "AKIDEXAMPLE",
                ),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_SES_SECRET_ACCESS_KEY",
                    "secret",
                ),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_SES_SESSION_TOKEN",
                    "session",
                ),
            ],
            200,
            r#"{"MessageId":"0100abc"}"#,
        )
        .await;
        assert_eq!(sent.unwrap(), None);
        let signed = seen.header("authorization").unwrap();
        assert!(
            signed.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"),
            "{signed}"
        );
        assert!(signed.contains("/eu-west-2/ses/aws4_request"), "{signed}");
        assert_eq!(seen.header("x-amz-security-token"), Some("session"));
        assert_eq!(seen.header("content-type"), Some("application/json"));

        let (sent, seen) = one(
            Kind::Azure,
            "/emails:send?api-version=2023-03-31",
            &[
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_AZURE_ENDPOINT",
                    "https://x.communication.azure.com",
                ),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_AZURE_ACCESS_KEY",
                    "c2VjcmV0",
                ),
            ],
            202,
            r#"{"id":"op-1","status":"Running"}"#,
        )
        .await;
        assert_eq!(
            sent.unwrap(),
            Some("op-1".to_string()),
            "Azure's id is kept for its events"
        );
        assert_eq!(seen.uri, "/emails:send?api-version=2023-03-31");
        assert!(
            seen.header("authorization")
                .unwrap()
                .starts_with("HMAC-SHA256 ")
        );
    }

    #[tokio::test]
    async fn a_refusal_is_permanent_an_outage_is_transient_and_so_is_no_answer() {
        let brevo = [("CONTACT_RELATIONSHIP_MANAGEMENT_BREVO_API_KEY", "k")];
        let (refused, _) = one(
            Kind::Brevo,
            "/x",
            &brevo,
            400,
            r#"{"code":"invalid_parameter","message":"bad to"}"#,
        )
        .await;
        assert!(
            matches!(refused, Err(SendError::Permanent(_))),
            "{refused:?}"
        );
        let (outage, _) = one(Kind::Brevo, "/x", &brevo, 503, "{}").await;
        assert!(matches!(outage, Err(SendError::Transient(_))), "{outage:?}");
        // Nothing listening: transient, and the address is not in the text.
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = closed.local_addr().unwrap();
        drop(closed);
        let gone = sender(Kind::Brevo, &format!("http://{address}/x"), &brevo)
            .send(&message())
            .await;
        match gone {
            Err(SendError::Transient(why)) => assert!(!why.contains(&address.to_string()), "{why}"),
            other => panic!("{other:?}"),
        }
        // An API URL that does not parse is refused before anything is sent, for a signed provider.
        let mut bad = sender(
            Kind::Ses,
            "http://127.0.0.1/x",
            &[
                ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_REGION", "eu-west-2"),
                ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_ACCESS_KEY_ID", "a"),
                ("CONTACT_RELATIONSHIP_MANAGEMENT_SES_SECRET_ACCESS_KEY", "b"),
            ],
        );
        bad.url = "not a url".into();
        assert!(matches!(
            bad.send(&message()).await,
            Err(SendError::Permanent(_))
        ));
        assert_eq!(bad.url(), "not a url");
    }
}
