//! Outbound email (CRM-T39): the sender seam and its implementations.
//!
//! A campaign send goes through [`Sender`], chosen once per process by
//! `CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE`:
//!
//! - `simulated` (the default): nothing leaves the process. The send is
//!   recorded as done and the campaign's engagement counters are the
//!   deterministic demo values. This is what the demo and the test suite use.
//! - `smtp`: real submission to an SMTP server (`CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_*`), for example the
//!   local Stalwart server from `scripts/email-dev.sh`.
//!
//! It is an enum rather than a trait object so a test can swap in
//! [`Sender::Recording`] without boxing, and so every variant is visible here.
//!
//! - `brevo` / `mailjet`: the provider's HTTP API and webhook (see
//!   [`crate::provider`]), which can match events to messages.
//!
//! Until a provider is chosen and reviewed, every real mode refuses to start in
//! production unless `CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION=1` acknowledges that.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Duration;

use lettre::message::Mailbox;
use lettre::message::header::{ContentType, Header, HeaderName, HeaderValue};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{
    Address, AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor,
};
use uuid::Uuid;

/// `List-Unsubscribe` (RFC 2369).
#[derive(Debug, Clone)]
struct ListUnsubscribe(String);

impl Header for ListUnsubscribe {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str("List-Unsubscribe")
    }
    fn parse(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self(s.to_string()))
    }
    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// `List-Unsubscribe-Post` (RFC 8058): one-click unsubscribe by `POST`.
#[derive(Debug, Clone)]
struct ListUnsubscribePost(String);

impl Header for ListUnsubscribePost {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str("List-Unsubscribe-Post")
    }
    fn parse(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self(s.to_string()))
    }
    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// One message to one recipient.
#[derive(Debug, Clone)]
pub struct Message {
    /// The recipient address.
    pub to: String,
    /// The recipient's display name.
    pub to_name: String,
    /// The subject line.
    pub subject: String,
    /// The plain-text body.
    pub text: String,
    /// The `Message-ID` we assign (with angle brackets), recorded on the
    /// delivery row so a complaint can be traced to a send.
    pub message_id: String,
    /// The signed unsubscribe link. When set, the wire message carries
    /// `List-Unsubscribe` and `List-Unsubscribe-Post` (RFC 8058) headers.
    pub unsubscribe_url: Option<String>,
}

/// Why a send failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    /// The server or the address will never accept this (a `5xx`, a bad
    /// address). Not retried.
    Permanent(String),
    /// A timeout, a connection failure or a `4xx`. Worth retrying.
    Transient(String),
}

impl SendError {
    /// The short text stored on the delivery row (no recipient address).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Permanent(text) => format!("permanent: {}", clip(text)),
            Self::Transient(text) => format!("transient: {}", clip(text)),
        }
    }
}

/// Keep an error short and single-line before it is stored.
fn clip(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

/// How the SMTP connection is secured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    /// Connect in the clear, then require `STARTTLS` (port 587).
    StartTls,
    /// TLS from the first byte (port 465).
    Implicit,
    /// No TLS. Only for a server on the same host.
    None,
}

/// A real SMTP sender.
#[derive(Debug)]
pub struct SmtpSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    /// The server's host name, for the residency check (CRM-R106).
    host: String,
}

/// A sender that records instead of sending, for tests.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    sent: Arc<Mutex<Vec<Message>>>,
    fail: Arc<Mutex<HashSet<String>>>,
    refuse: Arc<Mutex<HashSet<String>>>,
}

impl Recorder {
    /// The messages "sent" so far.
    #[must_use]
    pub fn sent(&self) -> Vec<Message> {
        lock(&self.sent).clone()
    }

    /// Forget the messages recorded so far.
    pub fn clear(&self) {
        lock(&self.sent).clear();
    }

    /// Make every send to `address` fail with a transient error until
    /// [`Recorder::heal`].
    pub fn fail_for(&self, address: &str) {
        lock(&self.fail).insert(address.to_string());
    }

    /// Stop failing sends to `address`.
    pub fn heal(&self, address: &str) {
        lock(&self.fail).remove(address);
    }

    /// Make every send to `address` fail permanently, as a server refusing the mailbox would.
    pub fn refuse(&self, address: &str) {
        lock(&self.refuse).insert(address.to_string());
    }
}

/// Lock a mutex, ignoring poisoning (a test panic must not hide later ones).
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How campaign mail is delivered.
#[derive(Debug)]
pub enum Sender {
    /// Nothing leaves the process (the demo default).
    Simulated,
    /// Real SMTP submission.
    Smtp(Box<SmtpSender>),
    /// A provider's HTTP API (Brevo, Mailjet); see [`crate::provider`].
    Api(Box<crate::provider::ApiSender>),
    /// Records messages; for tests.
    Recording(Recorder),
}

impl Sender {
    /// The outside host this sender hands mail to, if it sends any (CRM-R106).
    #[must_use]
    pub fn outside_host(&self) -> Option<String> {
        match self {
            Self::Smtp(smtp) => crate::rules::residency::host_of(&smtp.host),
            Self::Api(api) => crate::rules::residency::host_of(api.url()),
            Self::Simulated | Self::Recording(_) => None,
        }
    }

    /// Whether a send needs a real recipient address.
    #[must_use]
    pub fn needs_address(&self) -> bool {
        !matches!(self, Self::Simulated)
    }

    /// Whether the engagement counters are the deterministic demo values.
    #[must_use]
    pub fn is_simulated(&self) -> bool {
        matches!(self, Self::Simulated)
    }

    /// A fresh `Message-ID` under the sender's own domain.
    #[must_use]
    pub fn new_message_id(&self) -> String {
        match self {
            Self::Smtp(smtp) => new_message_id(&domain_of(smtp.from.email.as_ref())),
            Self::Api(api) => new_message_id(&api.from_domain()),
            _ => new_message_id("crm.invalid"),
        }
    }

    /// Hand one message to the transport.
    ///
    /// # Errors
    ///
    /// [`SendError::Permanent`] for an address or server refusal that retrying
    /// cannot fix, [`SendError::Transient`] otherwise.
    pub async fn send(&self, message: &Message) -> Result<Option<String>, SendError> {
        match self {
            Self::Simulated => Ok(None),
            Self::Recording(recorder) => {
                if lock(&recorder.fail).contains(&message.to) {
                    return Err(SendError::Transient("simulated outage".to_string()));
                }
                if lock(&recorder.refuse).contains(&message.to) {
                    return Err(SendError::Permanent("simulated refusal".to_string()));
                }
                lock(&recorder.sent).push(message.clone());
                Ok(None)
            }
            Self::Smtp(smtp) => smtp.send(message).await.map(|()| None),
            Self::Api(api) => api.send(message).await,
        }
    }
}

impl SmtpSender {
    /// Build the sender.
    ///
    /// # Errors
    ///
    /// A message when the host, TLS parameters or `From` address are unusable.
    pub fn new(
        host: &str,
        port: u16,
        security: Security,
        credentials: Option<Credentials>,
        from: &str,
        accept_invalid_certs: bool,
    ) -> Result<Self, String> {
        let from: Mailbox = from.parse().map_err(|e| {
            format!("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM is not a valid mailbox: {e}")
        })?;
        let tls_params = || {
            TlsParameters::builder(host.to_string())
                .dangerous_accept_invalid_certs(accept_invalid_certs)
                .dangerous_accept_invalid_hostnames(accept_invalid_certs)
                .build_rustls()
                .map_err(|e| format!("TLS parameters: {e}"))
        };
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .timeout(Some(Duration::from_secs(30)));
        builder = match security {
            Security::StartTls => builder.tls(Tls::Required(tls_params()?)),
            Security::Implicit => builder.tls(Tls::Wrapper(tls_params()?)),
            Security::None => builder.tls(Tls::None),
        };
        if let Some(credentials) = credentials {
            builder = builder.credentials(credentials);
        }
        Ok(Self {
            transport: builder.build(),
            from,
            host: host.to_string(),
        })
    }

    /// Build the wire message (public so a test can inspect it).
    ///
    /// # Errors
    ///
    /// [`SendError::Permanent`] when the recipient is not a valid address.
    pub fn build(&self, message: &Message) -> Result<LettreMessage, SendError> {
        let address: Address = message
            .to
            .parse()
            .map_err(|_| SendError::Permanent("recipient is not a valid address".to_string()))?;
        let to = Mailbox::new(Some(message.to_name.clone()), address);
        let mut builder = LettreMessage::builder()
            .from(self.from.clone())
            .to(to)
            .subject(message.subject.clone())
            .message_id(Some(message.message_id.clone()));
        if let Some(url) = &message.unsubscribe_url {
            builder =
                builder
                    .header(ListUnsubscribe(format!("<{url}>")))
                    .header(ListUnsubscribePost(
                        "List-Unsubscribe=One-Click".to_string(),
                    ));
        }
        builder
            .header(ContentType::TEXT_PLAIN)
            .body(message.text.clone())
            .map_err(|e| SendError::Permanent(format!("message could not be built: {e}")))
    }

    async fn send(&self, message: &Message) -> Result<(), SendError> {
        let wire = self.build(message)?;
        match self.transport.send(wire).await {
            Ok(_) => Ok(()),
            Err(error) if error.is_permanent() => Err(SendError::Permanent(error.to_string())),
            Err(error) => Err(SendError::Transient(error.to_string())),
        }
    }
}

/// The `Message-ID` for a send: unique, and under our own domain.
#[must_use]
pub fn new_message_id(from_domain: &str) -> String {
    format!("<{}@{}>", Uuid::new_v4(), from_domain)
}

/// The domain of a `From` address, for `Message-ID`s.
#[must_use]
pub fn domain_of(from: &str) -> String {
    from.rsplit('@')
        .next()
        .unwrap_or("localhost")
        .trim_end_matches('>')
        .to_string()
}

/// Build the sender from configuration. `var` reads one setting.
///
/// # Errors
///
/// A message when the settings are inconsistent, and in production when `smtp`
/// mode is chosen without the acknowledgement, or the TLS check is disabled.
pub fn from_settings(
    var: &dyn Fn(&str) -> Option<String>,
    production: bool,
) -> Result<Sender, String> {
    let get = |name: &str| {
        var(name)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let flag = |name: &str| {
        matches!(
            get(name).map(|v| v.to_ascii_lowercase()).as_deref(),
            Some("1" | "true" | "yes" | "on")
        )
    };
    match get("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE")
        .map(|v| v.to_ascii_lowercase())
        .as_deref()
    {
        None | Some("simulated") => Ok(Sender::Simulated),
        Some(name) if crate::provider::Kind::from_name(name).is_some() => {
            let kind =
                crate::provider::Kind::from_name(name).unwrap_or(crate::provider::Kind::Brevo);
            Ok(Sender::Api(Box::new(crate::provider::configure(
                kind, var, production,
            )?)))
        }
        Some("smtp") => {
            let accept_invalid = flag("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_ACCEPT_INVALID_CERTS");
            if production {
                if accept_invalid {
                    return Err(
                        "CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_ACCEPT_INVALID_CERTS must not be set in production".to_string(),
                    );
                }
                if !flag("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION") {
                    return Err(
                        "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE=smtp in production needs CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION=1: \
                         the vendor, agreement and legal review is open (CRM-T39)"
                            .to_string(),
                    );
                }
            }
            let host = get("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_HOST")
                .ok_or("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_HOST is required in smtp mode")?;
            let from = get("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM")
                .ok_or("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM is required in smtp mode")?;
            let security = match get("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_TLS")
                .map(|v| v.to_ascii_lowercase())
                .as_deref()
            {
                None | Some("starttls") => Security::StartTls,
                Some("implicit") => Security::Implicit,
                Some("none") => Security::None,
                Some(other) => {
                    return Err(format!(
                        "CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_TLS must be starttls, implicit or none, not {other:?}"
                    ));
                }
            };
            if production && security == Security::None {
                return Err(
                    "CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_TLS=none is not allowed in production"
                        .to_string(),
                );
            }
            let port = match get("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_PORT") {
                Some(text) => text.parse::<u16>().map_err(|_| {
                    format!("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_PORT is not a port: {text:?}")
                })?,
                None => match security {
                    Security::Implicit => 465,
                    _ => 587,
                },
            };
            let credentials = match (
                get("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_USERNAME"),
                get("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_PASSWORD"),
            ) {
                (Some(user), Some(password)) => Some(Credentials::new(user, password)),
                (None, None) => None,
                _ => {
                    return Err(
                        "CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_USERNAME and CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_PASSWORD must be set together".to_string(),
                    );
                }
            };
            Ok(Sender::Smtp(Box::new(SmtpSender::new(
                &host,
                port,
                security,
                credentials,
                &from,
                accept_invalid,
            )?)))
        }
        Some(other) => Err(format!(
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE must be simulated, smtp, brevo or mailjet, not {other:?}"
        )),
    }
}

/// The process-wide sender.
fn slot() -> &'static RwLock<Arc<Sender>> {
    static SLOT: OnceLock<RwLock<Arc<Sender>>> = OnceLock::new();
    SLOT.get_or_init(|| RwLock::new(Arc::new(Sender::Simulated)))
}

/// The sender campaign mail goes through.
#[must_use]
pub fn current() -> Arc<Sender> {
    slot()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Replace the process-wide sender (start-up, and tests).
pub fn set(sender: Sender) {
    *slot()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(sender);
}

/// Read the settings from the environment and install the sender.
///
/// # Errors
///
/// As [`from_settings`].
pub fn init(production: bool) -> Result<(), String> {
    let sender = from_settings(&|name| std::env::var(name).ok(), production)?;
    if matches!(sender, Sender::Smtp(_)) {
        tracing::warn!(
            "campaign mail is sent over SMTP (CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE=smtp)"
        );
    }
    set(sender);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn settings(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    const SMTP: &[(&str, &str)] = &[
        ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE", "smtp"),
        (
            "CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_HOST",
            "mail.example.test",
        ),
        (
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM",
            "CRM <crm@example.test>",
        ),
    ];

    #[test]
    fn the_default_is_simulated_and_needs_no_address() {
        let sender = from_settings(&settings(&[]), true).unwrap();
        assert!(sender.is_simulated());
        assert!(!sender.needs_address());
    }

    #[test]
    fn smtp_mode_needs_a_host_and_a_from_address() {
        let no_host = [
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE", "smtp"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM", "a@b.test"),
        ];
        assert!(from_settings(&settings(&no_host), false).is_err());
        let no_from = [
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE", "smtp"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_HOST", "h.test"),
        ];
        assert!(from_settings(&settings(&no_from), false).is_err());
        let bad_from = [
            ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE", "smtp"),
            ("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_HOST", "h.test"),
            (
                "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM",
                "not an address",
            ),
        ];
        assert!(from_settings(&settings(&bad_from), false).is_err());
        assert!(
            from_settings(&settings(SMTP), false)
                .unwrap()
                .needs_address()
        );
    }

    #[test]
    fn smtp_in_production_needs_the_acknowledgement() {
        let refused = from_settings(&settings(SMTP), true).unwrap_err();
        assert!(
            refused.contains("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION"),
            "{refused}"
        );
        let mut acknowledged = SMTP.to_vec();
        acknowledged.push(("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION", "1"));
        assert!(from_settings(&settings(&acknowledged), true).is_ok());
    }

    #[test]
    fn production_refuses_an_unchecked_certificate_or_no_tls() {
        let mut insecure = SMTP.to_vec();
        insecure.push(("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION", "1"));
        insecure.push((
            "CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_ACCEPT_INVALID_CERTS",
            "1",
        ));
        assert!(from_settings(&settings(&insecure), true).is_err());
        // Allowed outside production, where the dev server is self-signed.
        assert!(from_settings(&settings(&insecure), false).is_ok());
        let mut plain = SMTP.to_vec();
        plain.push(("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_ALLOW_PRODUCTION", "1"));
        plain.push(("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_TLS", "none"));
        assert!(from_settings(&settings(&plain), true).is_err());
    }

    #[test]
    fn credentials_come_as_a_pair_and_the_port_is_checked() {
        let mut half = SMTP.to_vec();
        half.push(("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_USERNAME", "u"));
        assert!(from_settings(&settings(&half), false).is_err());
        let mut port = SMTP.to_vec();
        port.push(("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_PORT", "ninety"));
        assert!(from_settings(&settings(&port), false).is_err());
        let unknown = [(
            "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE",
            "carrier-pigeon",
        )];
        assert!(from_settings(&settings(&unknown), false).is_err());
    }

    #[test]
    fn the_wire_message_carries_our_message_id_and_headers() {
        let sender = SmtpSender::new(
            "localhost",
            2587,
            Security::None,
            None,
            "CRM <crm@example.test>",
            false,
        )
        .unwrap();
        let id = new_message_id("example.test");
        let message = Message {
            to: "ada@example.test".into(),
            to_name: "Ada Lovelace".into(),
            subject: "Hello".into(),
            text: "Body".into(),
            message_id: id.clone(),
            unsubscribe_url: None,
        };
        let wire = String::from_utf8(sender.build(&message).unwrap().formatted()).unwrap();
        assert!(wire.contains(&format!("Message-ID: {id}")), "{wire}");
        assert!(wire.contains("Subject: Hello"), "{wire}");
        assert!(wire.contains("ada@example.test"), "{wire}");
        assert!(wire.contains("text/plain"), "{wire}");
    }

    #[test]
    fn the_wire_message_carries_rfc_8058_unsubscribe_headers() {
        let sender = SmtpSender::new(
            "localhost",
            2587,
            Security::None,
            None,
            "CRM <crm@example.test>",
            false,
        )
        .unwrap();
        let message = Message {
            to: "ada@example.test".into(),
            to_name: "Ada".into(),
            subject: "Hello".into(),
            text: "Body".into(),
            message_id: new_message_id("example.test"),
            unsubscribe_url: Some("https://crm.example.org/unsubscribe/abc".into()),
        };
        let wire = String::from_utf8(sender.build(&message).unwrap().formatted()).unwrap();
        assert!(
            wire.contains("List-Unsubscribe: <https://crm.example.org/unsubscribe/abc>"),
            "{wire}"
        );
        assert!(
            wire.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"),
            "{wire}"
        );
    }

    #[test]
    fn a_bad_recipient_is_a_permanent_failure_and_never_reaches_the_wire() {
        let sender = SmtpSender::new(
            "localhost",
            2587,
            Security::None,
            None,
            "a@example.test",
            false,
        )
        .unwrap();
        let message = Message {
            to: "not an address".into(),
            to_name: "X".into(),
            subject: "s".into(),
            text: "t".into(),
            message_id: new_message_id("example.test"),
            unsubscribe_url: None,
        };
        assert!(matches!(
            sender.build(&message),
            Err(SendError::Permanent(_))
        ));
    }

    #[test]
    fn stored_errors_are_short_and_flagged() {
        let long = "x".repeat(500);
        let text = SendError::Permanent(long).describe();
        assert!(text.starts_with("permanent: "));
        assert!(text.len() <= 211);
        assert!(
            SendError::Transient("timeout".into())
                .describe()
                .starts_with("transient: ")
        );
    }

    #[tokio::test]
    async fn the_recorder_records_and_can_be_made_to_fail_then_heal() {
        let recorder = Recorder::default();
        let sender = Sender::Recording(recorder.clone());
        let message = Message {
            to: "a@example.test".into(),
            to_name: "A".into(),
            subject: "s".into(),
            text: "t".into(),
            message_id: new_message_id("example.test"),
            unsubscribe_url: None,
        };
        sender.send(&message).await.unwrap();
        recorder.fail_for("a@example.test");
        assert!(matches!(
            sender.send(&message).await,
            Err(SendError::Transient(_))
        ));
        recorder.heal("a@example.test");
        sender.send(&message).await.unwrap();
        assert_eq!(recorder.sent().len(), 2);
    }

    #[test]
    fn message_ids_are_unique_and_under_the_senders_domain() {
        assert_ne!(new_message_id("a.test"), new_message_id("a.test"));
        assert!(new_message_id("a.test").ends_with("@a.test>"));
        assert_eq!(domain_of("CRM <crm@example.test>"), "example.test");
        assert_eq!(domain_of("crm@example.test"), "example.test");
    }

    /// A scripted SMTP server on a free local port: it answers `rcpt` to `RCPT TO` and keeps the
    /// message text it is given. Plain text, so the sender is configured with `SMTP_TLS=none`.
    async fn smtp_server(rcpt: &'static str) -> (u16, Arc<Mutex<String>>) {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let kept = Arc::new(Mutex::new(String::new()));
        let data = kept.clone();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let data = data.clone();
                tokio::spawn(async move {
                    let (read, mut write) = socket.into_split();
                    let mut lines = BufReader::new(read).lines();
                    write.write_all(b"220 test ESMTP\r\n").await.unwrap();
                    let mut in_data = false;
                    while let Ok(Some(line)) = lines.next_line().await {
                        if in_data {
                            if line == "." {
                                in_data = false;
                                write.write_all(b"250 queued\r\n").await.unwrap();
                            } else {
                                lock(&data).push_str(&line);
                                lock(&data).push('\n');
                            }
                            continue;
                        }
                        let verb = line
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_ascii_uppercase();
                        let answer: &[u8] = match verb.as_str() {
                            "EHLO" | "HELO" => b"250 test\r\n",
                            "RCPT" => rcpt.as_bytes(),
                            "DATA" => {
                                in_data = true;
                                b"354 go on\r\n"
                            }
                            "QUIT" => {
                                write.write_all(b"221 bye\r\n").await.unwrap();
                                break;
                            }
                            _ => b"250 ok\r\n",
                        };
                        write.write_all(answer).await.unwrap();
                    }
                });
            }
        });
        (port, kept)
    }

    fn plain_smtp(port: u16) -> Sender {
        axa::tls::install_provider();
        let port = port.to_string();
        from_settings(
            &settings(&[
                ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE", "smtp"),
                ("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_HOST", "127.0.0.1"),
                ("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_PORT", &port),
                ("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_TLS", "none"),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM",
                    "CRM <crm@example.test>",
                ),
            ]),
            false,
        )
        .unwrap()
    }

    fn a_message(sender: &Sender) -> Message {
        Message {
            to: "ada@example.test".into(),
            to_name: "Ada".into(),
            subject: "Hello".into(),
            text: "A note.".into(),
            unsubscribe_url: Some("https://crm.example.test/u/t".into()),
            message_id: sender.new_message_id(),
        }
    }

    /// A message goes over SMTP with its unsubscribe headers, under the sender's own domain.
    #[tokio::test]
    async fn a_message_is_sent_over_smtp_with_its_unsubscribe_headers() {
        let (port, received) = smtp_server("250 ok\r\n").await;
        let sender = plain_smtp(port);
        assert_eq!(sender.outside_host().as_deref(), Some("127.0.0.1"));
        let message = a_message(&sender);
        assert!(
            message.message_id.ends_with("@example.test>"),
            "{}",
            message.message_id
        );
        assert_eq!(sender.send(&message).await, Ok(None));
        let text = lock(&received).clone();
        assert!(text.contains("Subject: Hello"), "{text}");
        assert!(
            text.contains("List-Unsubscribe: <https://crm.example.test/u/t>"),
            "{text}"
        );
        assert!(
            text.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"),
            "{text}"
        );
        assert!(text.contains("A note."), "{text}");
    }

    /// A refused mailbox is a permanent failure; a server asking to try later is a transient one.
    #[tokio::test]
    async fn a_refused_mailbox_is_permanent_and_a_deferral_transient() {
        let (port, _) = smtp_server("550 no such mailbox\r\n").await;
        let sender = plain_smtp(port);
        assert!(matches!(
            sender.send(&a_message(&sender)).await,
            Err(SendError::Permanent(_))
        ));
        let (port, _) = smtp_server("451 try again later\r\n").await;
        let sender = plain_smtp(port);
        assert!(matches!(
            sender.send(&a_message(&sender)).await,
            Err(SendError::Transient(_))
        ));
    }

    /// A provider's API sender, implicit TLS on its own port, and the settings that are refused.
    #[test]
    fn an_api_sender_implicit_tls_and_bad_smtp_settings() {
        axa::tls::install_provider();
        let api = from_settings(
            &settings(&[
                ("CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_MODE", "Brevo"),
                ("CONTACT_RELATIONSHIP_MANAGEMENT_BREVO_API_KEY", "k"),
                (
                    "CONTACT_RELATIONSHIP_MANAGEMENT_MAIL_FROM",
                    "CRM <crm@example.test>",
                ),
            ]),
            false,
        )
        .unwrap();
        assert_eq!(api.outside_host().as_deref(), Some("api.brevo.com"));
        assert!(api.new_message_id().ends_with("@example.test>"));
        assert_eq!(Sender::Simulated.outside_host(), None);

        let with = |extra: &[(&'static str, &'static str)]| {
            let mut all = SMTP.to_vec();
            all.extend_from_slice(extra);
            from_settings(&settings(&all), false)
        };
        assert!(with(&[("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_TLS", "implicit")]).is_ok());
        for (bad, says) in [
            (
                [("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_TLS", "ssl")],
                "must be starttls, implicit or none",
            ),
            (
                [("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_PORT", "smtp")],
                "is not a port",
            ),
            (
                [("CONTACT_RELATIONSHIP_MANAGEMENT_SMTP_USERNAME", "u")],
                "must be set together",
            ),
        ] {
            let why = with(&bad).unwrap_err();
            assert!(why.contains(says), "{why}");
        }
    }

    /// The two unsubscribe headers read back what they hold.
    #[test]
    fn the_unsubscribe_headers_parse_their_own_text() {
        let header = ListUnsubscribe::parse("<https://x/u>").unwrap();
        assert_eq!(header.0, "<https://x/u>");
        let _ = header.display();
        let post = ListUnsubscribePost::parse("List-Unsubscribe=One-Click").unwrap();
        assert_eq!(post.0, "List-Unsubscribe=One-Click");
        let _ = post.display();
    }
}
