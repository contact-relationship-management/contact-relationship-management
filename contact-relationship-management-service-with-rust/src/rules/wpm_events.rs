//! The worker service's hire and leave events (CRM-R92, CRM-D15). DB-free and clock-free.
//!
//! WPM (WPM-R151, WPM-R207) posts `{id, kind, occurred_at, subject, data?}` signed with
//! `x-wpm-signature: t=<unix>,v1=<hex HMAC-SHA256(secret, "<t>.<raw body>")>`. For `worker_hired`,
//! `worker_terminated` and `worker_retired`, `data` holds `person_ref`, `organization_ref` and
//! `effective_on` (and `from` and `to` for the two endings, which are not read). This module checks
//! the signature and says what an event means for the CRM; the controller only applies it.
//!
//! WPM may deliver events out of order (a retry of a hire can arrive after the leave that followed
//! it), so a hire or leave also carries a [`Stamp`], and [`is_stale`] says whether it is older than
//! the last one applied for the same person and organization (CRM-R93).

use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use entity_ref::{EntityRef, EntityType};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

/// How far the signature's timestamp may be from now, in seconds (five minutes, as WPM's own check).
pub const TOLERANCE_SECS: i64 = 300;

/// The largest body taken. An event is a few hundred bytes; anything far larger is not one.
pub const MAX_BODY: usize = 16 * 1024;

/// The longest event id kept. WPM's are UUIDs.
pub const MAX_ID: usize = 100;

fn tag(secret: &[u8], timestamp: &str, body: &[u8]) -> Hmac<Sha256> {
    #[allow(clippy::expect_used, reason = "HMAC accepts a key of any length")]
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("any key length");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    mac
}

/// The `x-wpm-signature` value WPM would send for `body` at `timestamp` (used by tests and by
/// anyone sending a test event).
#[must_use]
pub fn sign(secret: &[u8], timestamp: i64, body: &[u8]) -> String {
    use std::fmt::Write as _;
    let bytes = tag(secret, &timestamp.to_string(), body)
        .finalize()
        .into_bytes();
    let hex = bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    });
    format!("t={timestamp},v1={hex}")
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

/// Whether `header` is a fresh, correct signature of `body`: a `t=` within [`TOLERANCE_SECS`] of
/// `now`, and a `v1=` that is the HMAC of `"<t>.<body>"`, compared in constant time. Anything
/// missing, repeated or malformed fails.
#[must_use]
pub fn verify(secret: &[u8], header: &str, body: &[u8], now: i64) -> bool {
    let mut t = None;
    let mut v1 = None;
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", v)) if t.is_none() => t = Some(v),
            Some(("v1", v)) if v1.is_none() => v1 = Some(v),
            Some(("t" | "v1", _)) => return false,
            _ => {}
        }
    }
    let (Some(t), Some(v1)) = (t, v1) else {
        return false;
    };
    let Ok(sent_at) = t.parse::<i64>() else {
        return false;
    };
    if (now - sent_at).abs() > TOLERANCE_SECS {
        return false;
    }
    let Some(given) = unhex(v1) else {
        return false;
    };
    tag(secret, t, body).verify_slice(&given).is_ok()
}

/// The event as WPM sends it. Members not named here are ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    /// The event id: the same on every retry, so the CRM applies an event once.
    pub id: String,
    /// The event kind.
    pub kind: String,
    /// When WPM recorded the change, RFC 3339 to the second. Read for a hire or leave (CRM-R93).
    /// Kept as JSON so that a kind which does not read it is not refused for its shape.
    #[serde(default)]
    pub occurred_at: Option<serde_json::Value>,
    /// `worker:<pid>`; not used.
    #[serde(default)]
    pub subject: Option<String>,
    /// The handoff, for the kinds that carry one.
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// What an event means for the CRM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The person is now employed by the organization: pause their marketing.
    Hired(Handoff),
    /// The person's employment there has ended: lift that pause.
    Left(Handoff),
    /// Received and acknowledged; nothing changes. Carries the reason.
    Ignored(&'static str),
}

/// The three facts a hire or a leave carries, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    /// A `person:` URN.
    pub person_ref: String,
    /// An `organization:` URN.
    pub organization_ref: String,
    /// The first day of employment, or the day it ends.
    pub effective_on: NaiveDate,
    /// When WPM recorded the change.
    pub occurred_at: DateTime<Utc>,
}

/// When a hire or leave happened, for ordering (CRM-R93): first by when WPM recorded it, then, for
/// two recorded in the same second (WPM sends whole seconds), by the day it takes effect. The
/// derived order compares the fields in that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stamp {
    /// When WPM recorded the change.
    pub occurred_at: DateTime<Utc>,
    /// The day it takes effect.
    pub effective_on: NaiveDate,
}

impl Handoff {
    /// The event's place in the order of its person's employment by its organization.
    #[must_use]
    pub const fn stamp(&self) -> Stamp {
        Stamp {
            occurred_at: self.occurred_at,
            effective_on: self.effective_on,
        }
    }
}

/// Whether an event stamped `event` is older than the last one applied for the same person and
/// organization (`last`; `None` when none has been), and so must change nothing (CRM-R93). An
/// event with the same stamp as the last is **not** stale: it is applied, so of two events WPM
/// cannot tell apart the one that arrives last wins, as before ordering existed.
#[must_use]
pub fn is_stale(last: Option<Stamp>, event: Stamp) -> bool {
    last.is_some_and(|last| event < last)
}

/// Read a body as an event. `Err` names the problem; the body is not valid JSON, or has no `id`
/// or `kind`, or an `id` that is empty or longer than [`MAX_ID`].
///
/// # Errors
///
/// When the body is not an event.
pub fn parse(body: &[u8]) -> Result<Event, String> {
    let event: Event =
        serde_json::from_slice(body).map_err(|_| "the body is not an event".to_string())?;
    if event.id.trim().is_empty() || event.id.len() > MAX_ID {
        return Err(format!("id is 1 to {MAX_ID} characters"));
    }
    if event.kind.trim().is_empty() {
        return Err("kind is required".to_string());
    }
    Ok(event)
}

fn urn_of(value: Option<&serde_json::Value>, wanted: EntityType) -> Option<String> {
    let text = value?.as_str()?;
    EntityRef::from_str(text)
        .ok()
        .filter(|r| r.entity_type == wanted)
        .map(|_| text.to_string())
}

/// What an event means (CRM-R92). A hire or leave without `data` (a WPM from before WPM-R207) is
/// ignored, not refused, and so is every other kind. `Err` is a hire or leave whose `data` is
/// there but wrong (a reference of the wrong kind, or a date that is not one), or whose
/// `occurred_at` is missing or not a time (CRM-R93).
///
/// # Errors
///
/// When the handoff is malformed.
pub fn decide(event: &Event) -> Result<Decision, String> {
    let hired = match event.kind.as_str() {
        "worker_hired" => true,
        "worker_terminated" | "worker_retired" => false,
        "worker_activated" | "leave_approved" => return Ok(Decision::Ignored("not_used")),
        _ => return Ok(Decision::Ignored("unknown_kind")),
    };
    let Some(data) = event.data.as_ref().and_then(serde_json::Value::as_object) else {
        return Ok(Decision::Ignored("no_data"));
    };
    let person_ref = urn_of(data.get("person_ref"), EntityType::Person)
        .ok_or("data.person_ref is not a person: reference")?;
    let organization_ref = urn_of(data.get("organization_ref"), EntityType::Organization)
        .ok_or("data.organization_ref is not an organization: reference")?;
    let effective_on = data
        .get("effective_on")
        .and_then(serde_json::Value::as_str)
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .ok_or("data.effective_on is not a date (YYYY-MM-DD)")?;
    let occurred_at = event
        .occurred_at
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc))
        .ok_or("occurred_at is not a time (RFC 3339)")?;
    let handoff = Handoff {
        person_ref,
        organization_ref,
        effective_on,
        occurred_at,
    };
    Ok(if hired {
        Decision::Hired(handoff)
    } else {
        Decision::Left(handoff)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const S: &[u8] = b"wpm-secret-0123456789abcdef-0123456789";
    const P: &str = "person:0c4f1e2a-0000-4000-8000-000000000001";
    const O: &str = "organization:0c4f1e2a-0000-4000-8000-0000000000aa";

    fn event(kind: &str, data: Option<serde_json::Value>) -> Event {
        let mut body = json!({
            "id": "11111111-2222-4333-8444-555555555555",
            "kind": kind,
            "occurred_at": "2026-10-30T12:00:00Z",
            "subject": "worker:11111111-2222-4333-8444-666666666666",
        });
        if let Some(data) = data {
            body["data"] = data;
        }
        parse(body.to_string().as_bytes()).unwrap()
    }

    fn handoff() -> serde_json::Value {
        json!({ "person_ref": P, "organization_ref": O, "effective_on": "2026-11-02" })
    }

    #[test]
    fn a_signature_made_as_wpm_makes_it_verifies() {
        let body = br#"{"id":"e1","kind":"worker_hired"}"#;
        let header = sign(S, 1_800_000_000, body);
        assert!(header.starts_with("t=1800000000,v1="));
        assert_eq!(header.len(), "t=1800000000,v1=".len() + 64);
        assert!(verify(S, &header, body, 1_800_000_010));
        assert!(verify(S, &header, body, 1_800_000_000 + TOLERANCE_SECS));
        assert!(verify(S, &header, body, 1_800_000_000 - TOLERANCE_SECS));
    }

    /// The same vector as WPM's `rules::webhook` would give: HMAC-SHA256 of `"<t>.<body>"`.
    #[test]
    fn the_mac_is_over_the_timestamp_a_dot_and_the_raw_body() {
        // RFC 4231 test case 2: key "Jefe", data "what do ya want for nothing?".
        let header = sign(b"Jefe", 0, b"what do ya want for nothing?");
        let expected = {
            let mut mac = Hmac::<Sha256>::new_from_slice(b"Jefe").unwrap();
            mac.update(b"0.what do ya want for nothing?");
            mac.finalize().into_bytes()
        };
        let hex = expected.iter().fold(String::new(), |mut out, b| {
            use std::fmt::Write as _;
            let _ = write!(out, "{b:02x}");
            out
        });
        assert_eq!(header, format!("t=0,v1={hex}"));
    }

    #[test]
    fn any_change_or_a_stale_time_fails() {
        let body = br#"{"id":"e1","kind":"worker_hired"}"#;
        let header = sign(S, 1_800_000_000, body);
        assert!(!verify(b"another secret", &header, body, 1_800_000_000));
        assert!(!verify(
            S,
            &header,
            br#"{"id":"e2","kind":"worker_hired"}"#,
            1_800_000_000
        ));
        assert!(!verify(
            S,
            &header,
            body,
            1_800_000_000 + TOLERANCE_SECS + 1
        ));
        assert!(!verify(
            S,
            &header,
            body,
            1_800_000_000 - TOLERANCE_SECS - 1
        ));
        // A changed timestamp with the old MAC.
        let moved = header.replace("t=1800000000", "t=1800000001");
        assert!(!verify(S, &moved, body, 1_800_000_000));
    }

    #[test]
    fn a_malformed_header_fails() {
        let body = b"{}";
        let good = sign(S, 1000, body);
        let mac = good.split_once(",v1=").unwrap().1;
        for header in [
            String::new(),
            "t=1000".to_string(),
            format!("v1={mac}"),
            format!("t=x,v1={mac}"),
            "t=1000,v1=zz".to_string(),
            "t=1000,v1=abc".to_string(),
            "t=1000,v1=é".to_string(),
            format!("t=1000,v1={}", &mac[..62]),
            // A second t or v1 is refused, not chosen from.
            format!("t=1000,t=1000,v1={mac}"),
            format!("t=1000,v1=00,v1={mac}"),
        ] {
            assert!(!verify(S, &header, body, 1000), "{header}");
        }
        // Order and spaces do not matter.
        assert!(verify(S, &format!("v1={mac}, t=1000"), body, 1000));
    }

    #[test]
    fn a_body_must_be_an_event_with_an_id_and_a_kind() {
        assert!(parse(b"not json").is_err());
        assert!(parse(br#"{"kind":"worker_hired"}"#).is_err());
        assert!(parse(br#"{"id":"","kind":"worker_hired"}"#).is_err());
        assert!(parse(br#"{"id":"e1","kind":" "}"#).is_err());
        let long = format!(
            r#"{{"id":"{}","kind":"worker_hired"}}"#,
            "x".repeat(MAX_ID + 1)
        );
        assert!(parse(long.as_bytes()).is_err());
        let ok = parse(br#"{"id":"e1","kind":"worker_hired","extra":1}"#).unwrap();
        assert_eq!(ok.id, "e1");
        assert!(ok.data.is_none() && ok.subject.is_none());
    }

    #[test]
    fn a_hire_and_the_two_endings_carry_the_handoff() {
        let want = Handoff {
            person_ref: P.to_string(),
            organization_ref: O.to_string(),
            effective_on: NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
            occurred_at: DateTime::parse_from_rfc3339("2026-10-30T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        assert_eq!(
            decide(&event("worker_hired", Some(handoff()))),
            Ok(Decision::Hired(want.clone()))
        );
        let mut ending = handoff();
        ending["from"] = json!("active");
        ending["to"] = json!("terminated");
        for kind in ["worker_terminated", "worker_retired"] {
            assert_eq!(
                decide(&event(kind, Some(ending.clone()))),
                Ok(Decision::Left(want.clone())),
                "{kind}"
            );
        }
    }

    #[test]
    fn an_event_without_data_or_of_another_kind_is_ignored_not_refused() {
        assert_eq!(
            decide(&event("worker_hired", None)),
            Ok(Decision::Ignored("no_data"))
        );
        assert_eq!(
            decide(&event("worker_retired", Some(json!(null)))),
            Ok(Decision::Ignored("no_data"))
        );
        assert_eq!(
            decide(&event("worker_activated", Some(handoff()))),
            Ok(Decision::Ignored("not_used"))
        );
        assert_eq!(
            decide(&event("leave_approved", None)),
            Ok(Decision::Ignored("not_used"))
        );
        assert_eq!(
            decide(&event("worker_promoted", Some(handoff()))),
            Ok(Decision::Ignored("unknown_kind"))
        );
    }

    #[test]
    fn a_handoff_with_the_wrong_references_or_date_is_refused() {
        let with = |key: &str, value: serde_json::Value| {
            let mut data = handoff();
            data[key] = value;
            decide(&event("worker_hired", Some(data)))
        };
        assert!(
            with("person_ref", json!(O)).is_err(),
            "an organization is not a person"
        );
        assert!(
            with(
                "person_ref",
                json!("worker:0c4f1e2a-0000-4000-8000-000000000001")
            )
            .is_err()
        );
        assert!(with("person_ref", json!(7)).is_err());
        assert!(with("organization_ref", json!(P)).is_err());
        assert!(with("effective_on", json!("2026-13-01")).is_err());
        assert!(with("effective_on", json!("2026-11-02T00:00:00Z")).is_err());
        let mut missing = handoff();
        missing.as_object_mut().unwrap().remove("effective_on");
        assert!(decide(&event("worker_hired", Some(missing))).is_err());
    }

    #[test]
    fn a_hire_or_leave_needs_a_time_it_occurred() {
        let at = |occurred_at: serde_json::Value| {
            let mut body = json!({
                "id": "e1", "kind": "worker_hired", "data": handoff(),
            });
            if !occurred_at.is_null() {
                body["occurred_at"] = occurred_at;
            }
            decide(&parse(body.to_string().as_bytes()).unwrap())
        };
        assert!(at(json!(null)).is_err(), "missing");
        assert!(at(json!("2026-10-30")).is_err(), "a date is not a time");
        assert!(at(json!("yesterday")).is_err());
        assert!(at(json!(1_800_000_000)).is_err());
        // Another offset is the same instant in UTC.
        let Ok(Decision::Hired(h)) = at(json!("2026-10-30T14:00:00+02:00")) else {
            panic!("an offset time is a time");
        };
        assert_eq!(h.occurred_at.to_rfc3339(), "2026-10-30T12:00:00+00:00");
        // Kinds that are ignored do not need one.
        let body = json!({ "id": "e2", "kind": "worker_activated" });
        assert_eq!(
            decide(&parse(body.to_string().as_bytes()).unwrap()),
            Ok(Decision::Ignored("not_used"))
        );
    }

    fn stamp(at: &str, on: (i32, u32, u32)) -> Stamp {
        Stamp {
            occurred_at: DateTime::parse_from_rfc3339(at)
                .unwrap()
                .with_timezone(&Utc),
            effective_on: NaiveDate::from_ymd_opt(on.0, on.1, on.2).unwrap(),
        }
    }

    #[test]
    fn an_event_older_than_the_last_applied_is_stale() {
        let hire = stamp("2026-10-30T09:00:00Z", (2026, 11, 2));
        let leave = stamp("2026-10-30T15:00:00Z", (2026, 11, 30));
        // Nothing applied yet: nothing is stale.
        assert!(!is_stale(None, hire));
        // Hire, then leave: the leave is newer.
        assert!(!is_stale(Some(hire), leave));
        // Hire, leave, then the hire retried: older than the leave, so it changes nothing.
        assert!(is_stale(Some(leave), hire));
        // A rehire recorded after the leave is newer, whatever day it takes effect.
        let rehire = stamp("2026-12-01T09:00:00Z", (2026, 11, 1));
        assert!(!is_stale(Some(leave), rehire));
        // An earlier second is older even if it takes effect later.
        let early = stamp("2026-10-30T14:59:59Z", (2027, 1, 1));
        assert!(is_stale(Some(leave), early));
    }

    #[test]
    fn within_one_second_the_later_effective_day_wins_and_an_equal_stamp_is_applied() {
        let at = "2026-10-30T12:00:00Z";
        let starts = stamp(at, (2026, 11, 2));
        let ends = stamp(at, (2026, 11, 3));
        assert!(!is_stale(Some(starts), ends));
        assert!(is_stale(Some(ends), starts));
        // The same stamp (two events WPM cannot tell apart): applied, the last to arrive wins.
        assert!(!is_stale(Some(starts), starts));
        // The order is by time first, then by day.
        assert!(stamp("2026-10-30T12:00:01Z", (2000, 1, 1)) > ends);
    }

    #[test]
    fn a_handoff_stamps_itself_with_its_time_and_day() {
        let Ok(Decision::Left(h)) = decide(&event("worker_retired", Some(handoff()))) else {
            panic!("a leave");
        };
        assert_eq!(h.stamp(), stamp("2026-10-30T12:00:00Z", (2026, 11, 2)));
    }
}
