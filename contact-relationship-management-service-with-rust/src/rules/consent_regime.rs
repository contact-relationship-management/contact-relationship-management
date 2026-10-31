//! **Consent and e-marketing rules as jurisdiction profiles** (CRM-D21, CRM-T166; CRM-D19, CRM-R95): the parameter
//! registry, the chain of jurisdiction codes that applies to a recipient, the one pure resolver, and the decision
//! whether a message may be sent on a channel and on which basis. DB-free and clock-free: the rows, the evidence and
//! the moment are supplied.
//!
//! The shape is the workforce planning service's WPM-D144 (`src/rules/jurisdiction.rs` there), so the apps hold
//! jurisdiction rules alike: a code with a layer and a parent, kept per organization; a registry fixing each key's
//! value shape and how layers combine; append-only, dated, sourced rows with a status; one `resolve`. It is a copy
//! adapted to the CRM's keys, not a shared crate (drift between the apps is accepted, as for `money.ts`).
//!
//! **No profile is today's rule.** A parameter has no default value; an unresolved consent regime is
//! [`Regime::OptIn`], which sends only on `granted`, exactly as the CRM did before CRM-D21. A looser regime comes only
//! from rows an administrator loaded, and from an `example` or `unverified` row only once an administrator has
//! acknowledged that code's rows (decision 9); an unacknowledged row can tighten, never loosen.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// The channels consent is held on (decision 7). Only `email` is sent; the others are recorded.
pub const CHANNELS: &[&str] = &["email", "sms", "phone", "post"];

/// The channel every send path uses today.
pub const SEND_CHANNEL: &str = "email";

/// The kinds of evidence an implied-consent regime may list (decision 6).
pub const EVIDENCE_KINDS: &[&str] = &["inquiry", "won_deal"];

/// The most links a chain may have; a longer one is refused as malformed (a cycle is refused before that).
pub const MAX_CHAIN: usize = 8;

/// The longest window or expiry a row may give, in days (ten years).
pub const MAX_DAYS: i64 = 3_660;

/// A code's layer, from the widest to the narrowest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// A region of several countries with shared law, or a deployer-registered code.
    Supranational,
    /// A country (ISO 3166-1 alpha-2), or a deployer-added code ISO 3166-1 lacks.
    National,
    /// A part of a country (ISO 3166-2).
    Subnational,
}

impl Layer {
    /// The stored name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supranational => "supranational",
            Self::National => "national",
            Self::Subnational => "subnational",
        }
    }

    /// Parse a stored name.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "supranational" => Some(Self::Supranational),
            "national" => Some(Self::National),
            "subnational" => Some(Self::Subnational),
            _ => None,
        }
    }
}

/// Whether `code` has the shape of a jurisdiction code: 2 or 3 capital letters or digits, optionally `-` and 1 to 3
/// more (WPM-D144's shape).
#[must_use]
pub fn valid_code(code: &str) -> bool {
    let part = |text: &str, min: usize, max: usize| {
        (min..=max).contains(&text.len())
            && text
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    };
    match code.split_once('-') {
        Some((head, tail)) => part(head, 2, 3) && part(tail, 1, 3),
        None => part(code, 2, 3),
    }
}

/// The layer a code's shape implies when the organization has not recorded it: a code with a subdivision part is
/// subnational, any other national. A supranational code is never inferred.
#[must_use]
pub fn inferred_layer(code: &str) -> Layer {
    if code.contains('-') {
        Layer::Subnational
    } else {
        Layer::National
    }
}

/// Whether `layer` may be recorded for `code`: a code with a subdivision part is only subnational.
#[must_use]
pub fn layer_fits(code: &str, layer: Layer) -> bool {
    code.contains('-') == (layer == Layer::Subnational)
}

/// One code an organization keeps, with its layer and its parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// The jurisdiction code.
    pub code: String,
    /// Its layer.
    pub layer: Layer,
    /// The next wider code, if any.
    pub parent: Option<String>,
}

/// Why a chain could not be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainError {
    /// The start or a parent does not have a code's shape.
    BadCode(String),
    /// A code appears twice on the way up.
    Cycle(String),
    /// A parent is narrower than its child.
    LayerOrder(String),
    /// More than [`MAX_CHAIN`] links.
    TooLong,
}

/// One step of a chain: a code and its layer, from the recipient's code outward.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Step {
    /// The code.
    pub code: String,
    /// Its layer.
    pub layer: Layer,
}

/// The chain under `start`: `start` first, then each parent the organization recorded. A code the organization has
/// not recorded is a chain of itself alone, at the layer its shape implies; its parent is not guessed.
///
/// # Errors
///
/// A malformed code, a cycle, a parent narrower than its child, or a chain longer than [`MAX_CHAIN`].
pub fn chain(start: &str, links: &[Link]) -> Result<Vec<Step>, ChainError> {
    let mut steps: Vec<Step> = Vec::new();
    let mut next = Some(start.to_string());
    while let Some(code) = next {
        if !valid_code(&code) {
            return Err(ChainError::BadCode(code));
        }
        if steps.iter().any(|step| step.code == code) {
            return Err(ChainError::Cycle(code));
        }
        if steps.len() == MAX_CHAIN {
            return Err(ChainError::TooLong);
        }
        let link = links.iter().find(|link| link.code == code);
        let layer = link.map_or_else(|| inferred_layer(&code), |link| link.layer);
        if steps.last().is_some_and(|child| layer > child.layer) {
            return Err(ChainError::LayerOrder(code));
        }
        next = link.and_then(|link| link.parent.clone());
        steps.push(Step { code, layer });
    }
    Ok(steps)
}

/// How the values of several layers meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Combine {
    /// The deepest layer that has a value wins.
    MostSpecific,
    /// For a choice among tokens ordered strictest first: the strictest of all layers (decision 2).
    Strictest,
    /// For a window or an expiry: the shortest of all layers.
    StrictestMax,
}

/// The shape of a parameter's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// One of the tokens, listed strictest first.
    Token(&'static [&'static str]),
    /// A whole number of days, 1 to [`MAX_DAYS`].
    Days,
    /// Text per language: `{"en": "…", "de": "…"}`, 1 to 16 languages, each text 1 to 64 characters.
    Labels,
}

/// One registry entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Parameter {
    /// The key.
    pub key: &'static str,
    /// The value's shape.
    pub shape: Shape,
    /// How layers combine.
    pub combine: Combine,
    /// What the value means.
    pub meaning: &'static str,
}

/// The regimes, strictest first.
pub const REGIMES: &[&str] = &["opt_in", "soft_opt_in", "implied", "opt_out"];

/// A message requirement's values, strictest first.
pub const REQUIREMENT: &[&str] = &["required", "not_required"];

const fn regime(key: &'static str) -> Parameter {
    Parameter {
        key,
        shape: Shape::Token(REGIMES),
        combine: Combine::Strictest,
        meaning: "the consent regime a commercial message on this channel is sent under",
    }
}

const fn window(key: &'static str) -> Parameter {
    Parameter {
        key,
        shape: Shape::Days,
        combine: Combine::StrictestMax,
        meaning: "days since a won deal naming the contact as primary contact during which a soft opt-in applies",
    }
}

const fn expiry(key: &'static str) -> Parameter {
    Parameter {
        key,
        shape: Shape::Days,
        combine: Combine::StrictestMax,
        meaning: "days this kind of evidence gives implied consent for; a kind with no row is not evidence",
    }
}

/// The parameter registry, sorted by key. A key outside it is refused. The spec lists the same keys (CRM-D21).
pub const REGISTRY: &[Parameter] = &[
    expiry("consent.implied.expiry_days.inquiry"),
    expiry("consent.implied.expiry_days.won_deal"),
    regime("consent.regime.email"),
    regime("consent.regime.phone"),
    regime("consent.regime.post"),
    regime("consent.regime.sms"),
    window("consent.soft_opt_in.window_days.email"),
    window("consent.soft_opt_in.window_days.phone"),
    window("consent.soft_opt_in.window_days.post"),
    window("consent.soft_opt_in.window_days.sms"),
    Parameter {
        key: "message.advertising_label",
        shape: Shape::Labels,
        combine: Combine::MostSpecific,
        meaning: "a mark prefixed to the subject of a commercial message, in the template's language",
    },
    Parameter {
        key: "message.postal_address",
        shape: Shape::Token(REQUIREMENT),
        combine: Combine::Strictest,
        meaning: "whether a commercial message must carry the organization's postal address",
    },
    Parameter {
        key: "message.sender_identification",
        shape: Shape::Token(REQUIREMENT),
        combine: Combine::Strictest,
        meaning: "whether a commercial message must carry the organization's sender name and contact link",
    },
];

/// The registry entry for `key`.
#[must_use]
pub fn parameter(key: &str) -> Option<&'static Parameter> {
    REGISTRY.iter().find(|entry| entry.key == key)
}

/// A parameter's value, in the shape its entry fixes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// A number of days.
    Days(i64),
    /// A token.
    Token(String),
    /// Text per language, sorted by language.
    Labels(BTreeMap<String, String>),
}

impl Value {
    /// The token, if this is one.
    #[must_use]
    pub fn as_token(&self) -> Option<&str> {
        match self {
            Self::Token(token) => Some(token),
            _ => None,
        }
    }

    /// The days, if this is a number.
    #[must_use]
    pub const fn as_days(&self) -> Option<i64> {
        match self {
            Self::Days(days) => Some(*days),
            _ => None,
        }
    }
}

fn language_tag(tag: &str) -> bool {
    (2..=35).contains(&tag.len())
        && tag.split('-').all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

/// Check a stored or submitted JSON value against `key`'s entry and return its canonical form.
///
/// # Errors
///
/// An unknown key, or a value of the wrong shape or out of range, with a message naming the key.
pub fn validate(key: &str, raw: &serde_json::Value) -> Result<Value, String> {
    let entry = parameter(key).ok_or_else(|| format!("{key} is not in the parameter registry"))?;
    validate_entry(entry, raw)
}

/// [`validate`] against a given entry.
///
/// # Errors
///
/// A value of the wrong shape or out of range.
pub fn validate_entry(entry: &Parameter, raw: &serde_json::Value) -> Result<Value, String> {
    let key = entry.key;
    match entry.shape {
        Shape::Token(tokens) => raw
            .as_str()
            .filter(|text| tokens.contains(text))
            .map(|text| Value::Token(text.to_string()))
            .ok_or_else(|| format!("{key} must be one of {}", tokens.join(", "))),
        Shape::Days => raw
            .as_i64()
            .filter(|n| (1..=MAX_DAYS).contains(n))
            .map(Value::Days)
            .ok_or_else(|| format!("{key} must be a whole number of days from 1 to {MAX_DAYS}")),
        Shape::Labels => {
            let map = raw
                .as_object()
                .ok_or_else(|| format!("{key} must be an object of language to text"))?;
            if map.is_empty() || map.len() > 16 {
                return Err(format!("{key} must give 1 to 16 languages"));
            }
            let mut out = BTreeMap::new();
            for (tag, text) in map {
                let text = text.as_str().map(str::trim).unwrap_or_default();
                if !language_tag(tag) || !(1..=64).contains(&text.chars().count()) {
                    return Err(format!(
                        "{key} must map a language tag to a text of 1 to 64 characters"
                    ));
                }
                out.insert(tag.clone(), text.to_string());
            }
            Ok(Value::Labels(out))
        }
    }
}

/// How far a row's value has been checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// A worked case in a regime's shape, not a claim about its law.
    Example,
    /// Modelled on a source, or taken from part of it, not checked against the whole text.
    Unverified,
    /// The source was read whole and the value checked against it on `checked_on`.
    Read,
}

impl Status {
    /// The stored name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Example => "example",
            Self::Unverified => "unverified",
            Self::Read => "read",
        }
    }

    /// Parse a stored name.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "example" => Some(Self::Example),
            "unverified" => Some(Self::Unverified),
            "read" => Some(Self::Read),
            _ => None,
        }
    }
}

/// One profile row, as the resolver needs it. `seq` orders rows recorded for the same date (the later wins) and says
/// whether an acknowledgement covers the row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The jurisdiction code.
    pub code: String,
    /// The registry key.
    pub key: String,
    /// The value, already validated.
    pub value: Value,
    /// The first day it is in force.
    pub effective_from: NaiveDate,
    /// The last day it is in force, if it ends.
    pub effective_to: Option<NaiveDate>,
    /// What it rests on.
    pub source: String,
    /// How far it was checked.
    pub status: Status,
    /// Recording order.
    pub seq: i64,
}

/// Where a resolved value came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct Origin {
    /// The code whose row gave it.
    pub code: String,
    /// That row's status.
    #[schemars(with = "String")]
    pub status: Status,
    /// That row's source.
    pub source: String,
    /// That row's first day in force.
    pub effective_from: NaiveDate,
    /// That row's recording order.
    pub seq: i64,
}

/// The resolver's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// A value and the row that gave it.
    Resolved {
        /// The value.
        value: Value,
        /// The row it came from.
        from: Origin,
    },
    /// No row applies. Never a value.
    Unknown,
}

impl Resolution {
    /// The value, if resolved.
    #[must_use]
    pub const fn value(&self) -> Option<&Value> {
        match self {
            Self::Resolved { value, .. } => Some(value),
            Self::Unknown => None,
        }
    }

    /// The origin, if resolved.
    #[must_use]
    pub const fn origin(&self) -> Option<&Origin> {
        match self {
            Self::Resolved { from, .. } => Some(from),
            Self::Unknown => None,
        }
    }
}

fn in_force<'a>(rows: &'a [Row], code: &str, key: &str, as_of: NaiveDate) -> Option<&'a Row> {
    rows.iter()
        .filter(|row| row.code == code && row.key == key)
        .filter(|row| row.effective_from <= as_of && row.effective_to.is_none_or(|to| as_of <= to))
        .max_by_key(|row| (row.effective_from, row.seq))
}

fn origin(row: &Row) -> Origin {
    Origin {
        code: row.code.clone(),
        status: row.status,
        source: row.source.clone(),
        effective_from: row.effective_from,
        seq: row.seq,
    }
}

/// The rank a strict combination compares: a token's place in its list (0 the strictest), or a number of days.
fn rank(entry: &Parameter, value: &Value) -> Option<i64> {
    match (entry.shape, value) {
        (Shape::Token(tokens), Value::Token(token)) => tokens
            .iter()
            .position(|t| t == token)
            .and_then(|n| i64::try_from(n).ok()),
        (Shape::Days, Value::Days(days)) => Some(*days),
        _ => None,
    }
}

/// Resolve `key` over `chain` (`None` when no code applies) as of `as_of`.
///
/// # Errors
///
/// A key outside the registry. A row whose value no longer fits the registry is skipped, as if absent.
pub fn resolve(
    key: &str,
    chain: Option<&[Step]>,
    rows: &[Row],
    as_of: NaiveDate,
) -> Result<Resolution, String> {
    let entry = parameter(key).ok_or_else(|| format!("{key} is not in the parameter registry"))?;
    Ok(resolve_entry(entry, chain, rows, as_of))
}

/// [`resolve`] for a given entry.
#[must_use]
pub fn resolve_entry(
    entry: &Parameter,
    chain: Option<&[Step]>,
    rows: &[Row],
    as_of: NaiveDate,
) -> Resolution {
    let Some(chain) = chain else {
        return Resolution::Unknown;
    };
    let found: Vec<&Row> = chain
        .iter()
        .filter_map(|step| in_force(rows, &step.code, entry.key, as_of))
        .filter(|row| {
            serde_json::to_value(&row.value).is_ok_and(|raw| validate_entry(entry, &raw).is_ok())
        })
        .collect();
    let Some(first) = found.first() else {
        return Resolution::Unknown;
    };
    // `found` is the most specific first; a tie keeps the more specific.
    let picked = match entry.combine {
        Combine::MostSpecific => *first,
        Combine::Strictest | Combine::StrictestMax => {
            let mut best = *first;
            for row in &found[1..] {
                if let (Some(now), Some(new)) = (rank(entry, &best.value), rank(entry, &row.value))
                    && new < now
                {
                    best = row;
                }
            }
            best
        }
    };
    Resolution::Resolved {
        value: picked.value.clone(),
        from: origin(picked),
    }
}

/// A consent regime (CRM-D21, "What a regime allows").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Regime {
    /// Only `granted` sends. Today's rule, and the answer when no profile applies.
    OptIn,
    /// `granted`, or `never`/`pending` with a won deal inside the window, for an eligible purpose.
    SoftOptIn,
    /// `granted`, or `never`/`pending` with evidence of a listed kind younger than its expiry.
    Implied,
    /// Anything but `withdrawn`.
    OptOut,
}

impl Regime {
    /// Parse a stored token.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "opt_in" => Some(Self::OptIn),
            "soft_opt_in" => Some(Self::SoftOptIn),
            "implied" => Some(Self::Implied),
            "opt_out" => Some(Self::OptOut),
            _ => None,
        }
    }

    /// The stored token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OptIn => "opt_in",
            Self::SoftOptIn => "soft_opt_in",
            Self::Implied => "implied",
            Self::OptOut => "opt_out",
        }
    }
}

/// An acknowledgement an administrator gave to a code's `example` and `unverified` rows (decision 9): it covers the
/// code's rows recorded up to and including `through_seq`, so a row added later is not acknowledged by it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Acknowledgement {
    /// The code.
    pub code: String,
    /// The last row it covers.
    pub through_seq: i64,
}

/// Whether a row may loosen a send: it was read, or an acknowledgement covers it.
#[must_use]
pub fn may_loosen(from: &Origin, acks: &[Acknowledgement]) -> bool {
    from.status == Status::Read
        || acks
            .iter()
            .any(|ack| ack.code == from.code && from.seq <= ack.through_seq)
}

/// The rules a send on one channel is under, resolved from a chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct Rules {
    /// The regime in force (after the acknowledgement check).
    pub regime: Regime,
    /// The row the regime came from; `None` when no profile applies (today's opt-in).
    pub regime_from: Option<Origin>,
    /// The regime the rows say when it could not apply because its row is not acknowledged (decision 9).
    pub unacknowledged: Option<Regime>,
    /// The soft opt-in window, in days, when the regime is `soft_opt_in` and a usable row gives one.
    pub window_days: Option<i64>,
    /// The implied-consent expiry, in days, of each evidence kind a usable row lists.
    pub expiry_days: BTreeMap<String, i64>,
}

impl Rules {
    /// The rules when no profile applies: today's opt-in.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            regime: Regime::OptIn,
            regime_from: None,
            unacknowledged: None,
            window_days: None,
            expiry_days: BTreeMap::new(),
        }
    }
}

/// The days a usable row gives for `key`, if any. A row that may not loosen gives none (no window or evidence),
/// which can only make the answer stricter.
fn usable_days(
    key: &str,
    chain: &[Step],
    rows: &[Row],
    acks: &[Acknowledgement],
    as_of: NaiveDate,
) -> Option<i64> {
    match resolve(key, Some(chain), rows, as_of) {
        Ok(Resolution::Resolved { value, from }) if may_loosen(&from, acks) => value.as_days(),
        _ => None,
    }
}

/// The rules for `channel` under `chain` (`None`: no code applies) as of `as_of`.
#[must_use]
pub fn rules_for(
    channel: &str,
    chain: Option<&[Step]>,
    rows: &[Row],
    acks: &[Acknowledgement],
    as_of: NaiveDate,
) -> Rules {
    let Some(chain) = chain else {
        return Rules::none();
    };
    let Ok(Resolution::Resolved { value, from }) = resolve(
        &format!("consent.regime.{channel}"),
        Some(chain),
        rows,
        as_of,
    ) else {
        return Rules::none();
    };
    let stated = value
        .as_token()
        .and_then(Regime::parse)
        .unwrap_or(Regime::OptIn);
    if stated != Regime::OptIn && !may_loosen(&from, acks) {
        return Rules {
            regime: Regime::OptIn,
            regime_from: Some(from),
            unacknowledged: Some(stated),
            window_days: None,
            expiry_days: BTreeMap::new(),
        };
    }
    let window_days = (stated == Regime::SoftOptIn)
        .then(|| {
            usable_days(
                &format!("consent.soft_opt_in.window_days.{channel}"),
                chain,
                rows,
                acks,
                as_of,
            )
        })
        .flatten();
    let expiry_days = if stated == Regime::Implied {
        EVIDENCE_KINDS
            .iter()
            .filter_map(|kind| {
                usable_days(
                    &format!("consent.implied.expiry_days.{kind}"),
                    chain,
                    rows,
                    acks,
                    as_of,
                )
                .map(|days| ((*kind).to_string(), days))
            })
            .collect()
    } else {
        BTreeMap::new()
    };
    Rules {
        regime: stated,
        regime_from: Some(from),
        unacknowledged: None,
        window_days,
        expiry_days,
    }
}

/// What a commercial message must carry under a chain (decision 8), and the subject's label.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct MessageRules {
    /// The sender name and contact link must be set and carried.
    pub sender_identification_required: bool,
    /// The postal address must be set and carried.
    pub postal_address_required: bool,
    /// The advertising label per language, when the profile has one.
    pub advertising_label: Option<BTreeMap<String, String>>,
}

/// The message rules under `chain`. Requirements tighten, so any row counts, acknowledged or not.
#[must_use]
pub fn message_rules(chain: Option<&[Step]>, rows: &[Row], as_of: NaiveDate) -> MessageRules {
    let required = |key: &str| {
        resolve(key, chain, rows, as_of)
            .ok()
            .and_then(|r| r.value().and_then(Value::as_token).map(|t| t == "required"))
            .unwrap_or(false)
    };
    let label = resolve("message.advertising_label", chain, rows, as_of)
        .ok()
        .and_then(|r| match r.value() {
            Some(Value::Labels(map)) => Some(map.clone()),
            _ => None,
        });
    MessageRules {
        sender_identification_required: required("message.sender_identification"),
        postal_address_required: required("message.postal_address"),
        advertising_label: label,
    }
}

/// The label for `language` (a BCP 47 tag): an exact match, else its primary language, else none.
#[must_use]
pub fn label_for<'a>(labels: &'a BTreeMap<String, String>, language: &str) -> Option<&'a str> {
    let lower = language.to_ascii_lowercase();
    let primary = lower.split('-').next().unwrap_or_default().to_string();
    labels
        .iter()
        .find(|(tag, _)| tag.to_ascii_lowercase() == lower)
        .or_else(|| {
            labels
                .iter()
                .find(|(tag, _)| tag.to_ascii_lowercase() == primary)
        })
        .map(|(_, text)| text.as_str())
}

/// One piece of evidence a regime may rest on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// `won_deal` or `inquiry`.
    pub kind: String,
    /// The record it is (a deal's, a form submission's or a ticket's pid).
    pub pid: String,
    /// When it happened.
    pub at: DateTime<Utc>,
    /// The purpose it concerns, when it concerns one (a web-form submission names its purpose); `None` for any.
    pub purpose: Option<String>,
}

/// The basis a message is sent on, recorded on its delivery (CRM-R6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(tag = "basis", rename_all = "snake_case")]
pub enum Basis {
    /// The purpose and channel are `granted`.
    Granted,
    /// A soft opt-in on a won deal.
    SoftOptIn {
        /// The deal's pid.
        evidence_pid: String,
        /// When the window closes.
        expires_at: DateTime<Utc>,
    },
    /// Implied consent on evidence.
    Implied {
        /// The evidence kind.
        kind: String,
        /// The evidence's pid.
        evidence_pid: String,
        /// When it lapses.
        expires_at: DateTime<Utc>,
    },
    /// An opt-out regime and no withdrawal.
    OptOut,
}

impl Basis {
    /// The stored token.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::SoftOptIn { .. } => "soft_opt_in",
            Self::Implied { .. } => "implied",
            Self::OptOut => "opt_out",
        }
    }
}

/// Why no basis exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Blocked {
    /// The purpose and channel are withdrawn: blocks under every regime.
    Withdrawn,
    /// Opt-in and not `granted` (today's only reason).
    NotGranted,
    /// The purpose is not eligible for a soft opt-in.
    PurposeNotEligible,
    /// No evidence of a kind the regime accepts, or all of it is older than its window or expiry.
    NoEvidence,
    /// A state token the CRM does not know: never sends.
    UnknownState,
}

impl Blocked {
    /// The reason recorded on a suppressed delivery.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Withdrawn => "consent withdrawn",
            Self::NotGranted => "consent not granted at send time",
            Self::PurposeNotEligible => "the purpose is not eligible for a soft opt-in",
            Self::NoEvidence => "no evidence in force for the regime",
            Self::UnknownState => "unknown consent state",
        }
    }
}

/// Whether a message for `purpose` may be sent to a contact whose state for that purpose and channel is `state`,
/// under `rules`, with `evidence`, at `now`; and on which basis. `soft_opt_in_eligible` is the purpose's flag.
///
/// # Errors
///
/// [`Blocked`] with the reason.
pub fn basis(
    rules: &Rules,
    state: &str,
    purpose: &str,
    soft_opt_in_eligible: bool,
    evidence: &[Evidence],
    now: DateTime<Utc>,
) -> Result<Basis, Blocked> {
    match state {
        "granted" => return Ok(Basis::Granted),
        "withdrawn" => return Err(Blocked::Withdrawn),
        "never" | "pending" => {}
        _ => return Err(Blocked::UnknownState),
    }
    let concerns = |item: &&Evidence| item.purpose.as_deref().is_none_or(|p| p == purpose);
    match rules.regime {
        Regime::OptIn => Err(Blocked::NotGranted),
        Regime::OptOut => Ok(Basis::OptOut),
        Regime::SoftOptIn => {
            if !soft_opt_in_eligible {
                return Err(Blocked::PurposeNotEligible);
            }
            let window = rules.window_days.ok_or(Blocked::NoEvidence)?;
            evidence
                .iter()
                .filter(|item| item.kind == "won_deal" && item.at <= now)
                .filter(concerns)
                .map(|item| (item, item.at + Duration::days(window)))
                .filter(|(_, expires)| *expires > now)
                .max_by_key(|(_, expires)| *expires)
                .map(|(item, expires_at)| Basis::SoftOptIn {
                    evidence_pid: item.pid.clone(),
                    expires_at,
                })
                .ok_or(Blocked::NoEvidence)
        }
        Regime::Implied => evidence
            .iter()
            .filter(|item| item.at <= now)
            .filter(concerns)
            .filter_map(|item| {
                rules
                    .expiry_days
                    .get(&item.kind)
                    .map(|days| (item, item.at + Duration::days(*days)))
            })
            .filter(|(_, expires)| *expires > now)
            .max_by_key(|(_, expires)| *expires)
            .map(|(item, expires_at)| Basis::Implied {
                kind: item.kind.clone(),
                evidence_pid: item.pid.clone(),
                expires_at,
            })
            .ok_or(Blocked::NoEvidence),
    }
}

/// The code a recipient is under (decision 1): the contact's when set, else the send's, else none.
#[must_use]
pub fn applicable_code<'a>(contact: Option<&'a str>, send: Option<&'a str>) -> Option<&'a str> {
    contact
        .filter(|c| !c.is_empty())
        .or_else(|| send.filter(|c| !c.is_empty()))
}

/// One example profile: its code, layer, parent and `(key, value)` rows.
pub type ExampleProfile = (
    &'static str,
    Layer,
    Option<&'static str>,
    Vec<(&'static str, serde_json::Value)>,
);

/// The synthetic test and demo profiles of CRM-D21 ("The two test profiles"), as `(code, layer, parent, rows)`: codes
/// from the ISO 3166 user-assigned range, so no test names a country. Every row is `example`. The seed loads them
/// (decision 10) and the tests use them.
#[must_use]
pub fn example_profiles() -> Vec<ExampleProfile> {
    use serde_json::json;
    vec![
        (
            "XA",
            Layer::National,
            None,
            vec![
                ("consent.regime.email", json!("soft_opt_in")),
                ("consent.soft_opt_in.window_days.email", json!(730)),
                ("consent.regime.sms", json!("opt_in")),
                ("message.sender_identification", json!("required")),
                ("message.postal_address", json!("required")),
            ],
        ),
        (
            "XB",
            Layer::National,
            None,
            vec![
                ("consent.regime.email", json!("opt_out")),
                ("message.postal_address", json!("required")),
            ],
        ),
        (
            "XB-01",
            Layer::Subnational,
            Some("XB"),
            vec![
                ("consent.regime.email", json!("implied")),
                ("consent.implied.expiry_days.won_deal", json!(730)),
                ("consent.implied.expiry_days.inquiry", json!(180)),
                (
                    "message.advertising_label",
                    json!({"en": "[Advertisement]", "fr": "[Publicité]"}),
                ),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    /// The example profiles as rows and links, every row `example`, recorded on 2026-01-01.
    fn profiles() -> (Vec<Link>, Vec<Row>) {
        let mut links = Vec::new();
        let mut rows = Vec::new();
        let mut seq = 0;
        for (code, layer, parent, entries) in example_profiles() {
            links.push(Link {
                code: code.into(),
                layer,
                parent: parent.map(Into::into),
            });
            for (key, raw) in entries {
                seq += 1;
                rows.push(Row {
                    code: code.into(),
                    key: key.into(),
                    value: validate(key, &raw).unwrap(),
                    effective_from: day(2026, 1, 1),
                    effective_to: None,
                    source: "CRM-D21 example".into(),
                    status: Status::Example,
                    seq,
                });
            }
        }
        (links, rows)
    }

    fn acks_all() -> Vec<Acknowledgement> {
        ["XA", "XB", "XB-01"]
            .iter()
            .map(|code| Acknowledgement {
                code: (*code).into(),
                through_seq: 100,
            })
            .collect()
    }

    fn rules_under(code: Option<&str>, acks: &[Acknowledgement]) -> Rules {
        let (links, rows) = profiles();
        let steps = code.map(|c| chain(c, &links).unwrap());
        rules_for("email", steps.as_deref(), &rows, acks, day(2026, 10, 31))
    }

    fn won(at: DateTime<Utc>) -> Evidence {
        Evidence {
            kind: "won_deal".into(),
            pid: "deal-1".into(),
            at,
            purpose: None,
        }
    }

    fn inquiry(at: DateTime<Utc>) -> Evidence {
        Evidence {
            kind: "inquiry".into(),
            pid: "form-1".into(),
            at,
            purpose: None,
        }
    }

    const STATES: &[&str] = &["granted", "withdrawn", "never", "pending", "bogus", ""];

    #[test]
    fn the_registry_is_sorted_unique_and_covers_every_channel() {
        let keys: Vec<&str> = REGISTRY.iter().map(|p| p.key).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(keys, sorted);
        for channel in CHANNELS {
            assert!(parameter(&format!("consent.regime.{channel}")).is_some());
            assert!(parameter(&format!("consent.soft_opt_in.window_days.{channel}")).is_some());
        }
        for kind in EVIDENCE_KINDS {
            assert!(parameter(&format!("consent.implied.expiry_days.{kind}")).is_some());
        }
        assert!(parameter("consent.regime.fax").is_none());
    }

    #[test]
    fn validation_refuses_what_does_not_fit() {
        use serde_json::json;
        assert!(validate("nope", &json!("opt_in")).is_err());
        assert!(validate("consent.regime.email", &json!("maybe")).is_err());
        assert!(validate("consent.regime.email", &json!(1)).is_err());
        assert_eq!(
            validate("consent.regime.email", &json!("opt_out")).unwrap(),
            Value::Token("opt_out".into())
        );
        assert!(validate("consent.soft_opt_in.window_days.email", &json!(0)).is_err());
        assert!(validate("consent.soft_opt_in.window_days.email", &json!(3_661)).is_err());
        assert!(validate("consent.soft_opt_in.window_days.email", &json!(3_660)).is_ok());
        assert!(validate("consent.soft_opt_in.window_days.email", &json!("30")).is_err());
        assert!(validate("message.advertising_label", &json!({})).is_err());
        assert!(validate("message.advertising_label", &json!({"en": ""})).is_err());
        assert!(validate("message.advertising_label", &json!({"e n": "x"})).is_err());
        assert!(validate("message.advertising_label", &json!(["x"])).is_err());
        assert!(validate("message.advertising_label", &json!({"zh-Hans": "广告"})).is_ok());
        assert!(validate("message.postal_address", &json!("sometimes")).is_err());
    }

    #[test]
    fn codes_and_chains_have_wpms_shape() {
        assert!(valid_code("XA"));
        assert!(valid_code("XB-01"));
        assert!(valid_code("EU"));
        assert!(!valid_code("xa"));
        assert!(!valid_code("X"));
        assert!(!valid_code("XB-0001"));
        let (links, _) = profiles();
        let steps = chain("XB-01", &links).unwrap();
        assert_eq!(
            steps.iter().map(|s| s.code.as_str()).collect::<Vec<_>>(),
            ["XB-01", "XB"]
        );
        // An unrecorded code is a chain of itself; its parent is not guessed from its prefix.
        assert_eq!(chain("XB-02", &links).unwrap().len(), 1);
        assert!(matches!(chain("bad", &links), Err(ChainError::BadCode(_))));
        let cycle = vec![
            Link {
                code: "XC".into(),
                layer: Layer::National,
                parent: Some("XD".into()),
            },
            Link {
                code: "XD".into(),
                layer: Layer::National,
                parent: Some("XC".into()),
            },
        ];
        assert!(matches!(chain("XC", &cycle), Err(ChainError::Cycle(_))));
        let upside_down = vec![Link {
            code: "XE".into(),
            layer: Layer::National,
            parent: Some("XE-1".into()),
        }];
        assert!(matches!(
            chain("XE", &upside_down),
            Err(ChainError::LayerOrder(_))
        ));
        assert!(layer_fits("XB-01", Layer::Subnational));
        assert!(!layer_fits("XB", Layer::Subnational));
        assert_eq!(Layer::parse("national"), Some(Layer::National));
        assert_eq!(
            Status::parse(Status::Unverified.as_str()),
            Some(Status::Unverified)
        );
    }

    #[test]
    fn the_federation_resolves_strictest_regime_parent_address_child_label() {
        let (links, rows) = profiles();
        let steps = chain("XB-01", &links).unwrap();
        let as_of = day(2026, 10, 31);
        let regime = resolve("consent.regime.email", Some(&steps), &rows, as_of).unwrap();
        assert_eq!(regime.value(), Some(&Value::Token("implied".into())));
        assert_eq!(regime.origin().unwrap().code, "XB-01");
        let rules = message_rules(Some(&steps), &rows, as_of);
        assert!(rules.postal_address_required);
        assert!(!rules.sender_identification_required);
        let label = rules.advertising_label.unwrap();
        assert_eq!(label_for(&label, "fr-FR"), Some("[Publicité]"));
        assert_eq!(label_for(&label, "en"), Some("[Advertisement]"));
        assert_eq!(label_for(&label, "de"), None);
        // XB alone is opt-out; XA requires both.
        let xb = chain("XB", &links).unwrap();
        assert_eq!(
            resolve("consent.regime.email", Some(&xb), &rows, as_of)
                .unwrap()
                .value(),
            Some(&Value::Token("opt_out".into()))
        );
        let xa = chain("XA", &links).unwrap();
        let xa_rules = message_rules(Some(&xa), &rows, as_of);
        assert!(xa_rules.sender_identification_required && xa_rules.postal_address_required);
        assert!(xa_rules.advertising_label.is_none());
        assert_eq!(message_rules(None, &rows, as_of), MessageRules::default());
    }

    #[test]
    fn rows_are_dated_and_a_later_row_wins() {
        let (links, mut rows) = profiles();
        rows.push(Row {
            code: "XA".into(),
            key: "consent.regime.email".into(),
            value: Value::Token("opt_in".into()),
            effective_from: day(2027, 1, 1),
            effective_to: None,
            source: "later".into(),
            status: Status::Read,
            seq: 50,
        });
        let steps = chain("XA", &links).unwrap();
        let before = resolve(
            "consent.regime.email",
            Some(&steps),
            &rows,
            day(2026, 12, 31),
        )
        .unwrap();
        assert_eq!(before.value(), Some(&Value::Token("soft_opt_in".into())));
        let after = resolve("consent.regime.email", Some(&steps), &rows, day(2027, 1, 1)).unwrap();
        assert_eq!(after.value(), Some(&Value::Token("opt_in".into())));
        // Not yet in force: unknown.
        assert_eq!(
            resolve("consent.regime.email", Some(&steps), &rows, day(2025, 1, 1)).unwrap(),
            Resolution::Unknown
        );
        assert!(resolve("nope", Some(&steps), &rows, day(2026, 1, 1)).is_err());
    }

    /// The invariant (CRM-D21, "Migration and default behaviour"): with no profile, every answer equals today's
    /// (`granted` sends, nothing else does), whatever the evidence, and nothing blocked today becomes sendable.
    #[test]
    fn no_profile_is_exactly_todays_answer() {
        let evidence = [won(at(2026, 10, 1)), inquiry(at(2026, 10, 30))];
        let (_, rows) = profiles();
        let no_rows: Vec<Row> = Vec::new();
        for rules in [
            Rules::none(),
            rules_for("email", None, &rows, &acks_all(), day(2026, 10, 31)),
            // A code with no rows of its own is no profile either.
            rules_for(
                "email",
                Some(&chain("XZ", &[]).unwrap()),
                &rows,
                &acks_all(),
                day(2026, 10, 31),
            ),
            rules_for(
                "email",
                Some(&chain("XA", &[]).unwrap()),
                &no_rows,
                &acks_all(),
                day(2026, 10, 31),
            ),
        ] {
            assert_eq!(rules.regime, Regime::OptIn);
            for state in STATES {
                for eligible in [true, false] {
                    for purpose in ["marketing", "newsletter"] {
                        let today = *state == "granted";
                        let now = basis(
                            &rules,
                            state,
                            purpose,
                            eligible,
                            &evidence,
                            at(2026, 10, 31),
                        );
                        assert_eq!(now.is_ok(), today, "{state}");
                        if today {
                            assert_eq!(now.unwrap(), Basis::Granted);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_same_contact_gets_three_answers_under_three_profiles() {
        let now = at(2026, 10, 31);
        let evidence = [inquiry(at(2026, 9, 1))];
        let none = basis(
            &rules_under(None, &acks_all()),
            "never",
            "marketing",
            true,
            &evidence,
            now,
        );
        let xa = basis(
            &rules_under(Some("XA"), &acks_all()),
            "never",
            "marketing",
            true,
            &evidence,
            now,
        );
        let xb01 = basis(
            &rules_under(Some("XB-01"), &acks_all()),
            "never",
            "marketing",
            true,
            &evidence,
            now,
        );
        let xb = basis(
            &rules_under(Some("XB"), &acks_all()),
            "never",
            "marketing",
            true,
            &evidence,
            now,
        );
        assert_eq!(none, Err(Blocked::NotGranted));
        // XA's soft opt-in needs a won deal; an inquiry is not one.
        assert_eq!(xa, Err(Blocked::NoEvidence));
        assert!(matches!(xb01, Ok(Basis::Implied { ref kind, .. }) if kind == "inquiry"));
        assert_eq!(xb, Ok(Basis::OptOut));
    }

    #[test]
    fn a_withdrawal_blocks_under_every_profile() {
        let now = at(2026, 10, 31);
        let evidence = [won(at(2026, 10, 1)), inquiry(at(2026, 10, 1))];
        for code in [None, Some("XA"), Some("XB"), Some("XB-01")] {
            assert_eq!(
                basis(
                    &rules_under(code, &acks_all()),
                    "withdrawn",
                    "marketing",
                    true,
                    &evidence,
                    now
                ),
                Err(Blocked::Withdrawn),
                "{code:?}"
            );
            assert_eq!(
                basis(
                    &rules_under(code, &acks_all()),
                    "bogus",
                    "marketing",
                    true,
                    &evidence,
                    now
                ),
                Err(Blocked::UnknownState)
            );
            assert_eq!(
                basis(
                    &rules_under(code, &acks_all()),
                    "granted",
                    "marketing",
                    false,
                    &[],
                    now
                ),
                Ok(Basis::Granted)
            );
        }
    }

    #[test]
    fn soft_opt_in_needs_an_eligible_purpose_and_a_won_deal_inside_the_window() {
        let rules = rules_under(Some("XA"), &acks_all());
        assert_eq!(rules.regime, Regime::SoftOptIn);
        assert_eq!(rules.window_days, Some(730));
        let now = at(2026, 10, 31);
        let recent = [won(at(2025, 1, 1))];
        let got = basis(&rules, "never", "marketing", true, &recent, now).unwrap();
        assert_eq!(
            got,
            Basis::SoftOptIn {
                evidence_pid: "deal-1".into(),
                expires_at: at(2025, 1, 1) + Duration::days(730),
            }
        );
        // Pending is treated as never (decision 11).
        assert!(basis(&rules, "pending", "marketing", true, &recent, now).is_ok());
        assert_eq!(
            basis(&rules, "never", "marketing", false, &recent, now),
            Err(Blocked::PurposeNotEligible)
        );
        let old = [won(at(2024, 10, 30))];
        assert_eq!(
            basis(&rules, "never", "marketing", true, &old, now),
            Err(Blocked::NoEvidence)
        );
        // Evidence in the future (a clock skew) is not evidence.
        assert_eq!(
            basis(
                &rules,
                "never",
                "marketing",
                true,
                &[won(at(2026, 11, 1))],
                now
            ),
            Err(Blocked::NoEvidence)
        );
        // SMS under XA is opt-in.
        let (links, rows) = profiles();
        let steps = chain("XA", &links).unwrap();
        let sms = rules_for("sms", Some(&steps), &rows, &acks_all(), day(2026, 10, 31));
        assert_eq!(sms.regime, Regime::OptIn);
        assert_eq!(
            basis(&sms, "never", "marketing", true, &recent, now),
            Err(Blocked::NotGranted)
        );
        // A soft opt-in with no window row has no evidence that counts.
        let no_window: Vec<Row> = rows
            .iter()
            .filter(|r| !r.key.starts_with("consent.soft_opt_in"))
            .cloned()
            .collect();
        let bare = rules_for(
            "email",
            Some(&steps),
            &no_window,
            &acks_all(),
            day(2026, 10, 31),
        );
        assert_eq!(
            basis(&bare, "never", "marketing", true, &recent, now),
            Err(Blocked::NoEvidence)
        );
    }

    #[test]
    fn implied_consent_lapses_per_kind() {
        let rules = rules_under(Some("XB-01"), &acks_all());
        assert_eq!(rules.regime, Regime::Implied);
        assert_eq!(rules.expiry_days.get("inquiry"), Some(&180));
        assert_eq!(rules.expiry_days.get("won_deal"), Some(&730));
        let now = at(2026, 10, 31);
        // An inquiry 200 days ago has lapsed; a won deal 200 days ago has not.
        let then = now - Duration::days(200);
        assert_eq!(
            basis(&rules, "never", "marketing", false, &[inquiry(then)], now),
            Err(Blocked::NoEvidence)
        );
        let deal = basis(&rules, "pending", "marketing", false, &[won(then)], now).unwrap();
        assert_eq!(deal.as_str(), "implied");
        // The longest-lasting evidence is the basis recorded.
        let both = [
            inquiry(now - Duration::days(10)),
            won(now - Duration::days(700)),
        ];
        match basis(&rules, "never", "marketing", false, &both, now).unwrap() {
            Basis::Implied {
                kind, expires_at, ..
            } => {
                assert_eq!(kind, "inquiry");
                assert_eq!(expires_at, now + Duration::days(170));
            }
            other => panic!("{other:?}"),
        }
        // Evidence for another purpose does not count.
        let other = Evidence {
            purpose: Some("events".into()),
            ..inquiry(now - Duration::days(1))
        };
        assert_eq!(
            basis(
                &rules,
                "never",
                "marketing",
                false,
                std::slice::from_ref(&other),
                now
            ),
            Err(Blocked::NoEvidence)
        );
        assert!(basis(&rules, "never", "events", false, &[other], now).is_ok());
    }

    #[test]
    fn opt_out_sends_anything_but_withdrawn() {
        let rules = rules_under(Some("XB"), &acks_all());
        assert_eq!(rules.regime, Regime::OptOut);
        let now = at(2026, 10, 31);
        for state in ["never", "pending"] {
            assert_eq!(
                basis(&rules, state, "marketing", false, &[], now),
                Ok(Basis::OptOut)
            );
        }
        assert_eq!(
            basis(&rules, "granted", "marketing", false, &[], now),
            Ok(Basis::Granted)
        );
    }

    #[test]
    fn an_unacknowledged_example_profile_cannot_loosen() {
        let now = at(2026, 10, 31);
        let evidence = [won(at(2026, 10, 1)), inquiry(at(2026, 10, 1))];
        for code in ["XA", "XB", "XB-01"] {
            let rules = rules_under(Some(code), &[]);
            assert_eq!(rules.regime, Regime::OptIn, "{code}");
            assert!(rules.unacknowledged.is_some());
            for state in STATES {
                assert_eq!(
                    basis(&rules, state, "marketing", true, &evidence, now).is_ok(),
                    *state == "granted"
                );
            }
        }
        // An acknowledgement covers the rows recorded up to it, not a row added later.
        let (links, mut rows) = profiles();
        rows.push(Row {
            code: "XC".into(),
            key: "consent.regime.email".into(),
            value: Value::Token("opt_out".into()),
            effective_from: day(2026, 1, 1),
            effective_to: None,
            source: "later".into(),
            status: Status::Unverified,
            seq: 200,
        });
        let steps = chain("XC", &links).unwrap();
        let old_ack = [Acknowledgement {
            code: "XC".into(),
            through_seq: 199,
        }];
        let rules = rules_for("email", Some(&steps), &rows, &old_ack, day(2026, 10, 31));
        assert_eq!(rules.regime, Regime::OptIn);
        assert_eq!(rules.unacknowledged, Some(Regime::OptOut));
        let new_ack = [Acknowledgement {
            code: "XC".into(),
            through_seq: 200,
        }];
        let rules = rules_for("email", Some(&steps), &rows, &new_ack, day(2026, 10, 31));
        assert_eq!(rules.regime, Regime::OptOut);
        // A read row needs no acknowledgement.
        rows.last_mut().unwrap().status = Status::Read;
        let rules = rules_for("email", Some(&steps), &rows, &[], day(2026, 10, 31));
        assert_eq!(rules.regime, Regime::OptOut);
    }

    #[test]
    fn an_unacknowledged_layer_can_only_tighten() {
        // XB-01 acknowledged (implied), XB not (opt-out): the stricter implied applies.
        let acks = [Acknowledgement {
            code: "XB-01".into(),
            through_seq: 100,
        }];
        let rules = rules_under(Some("XB-01"), &acks);
        assert_eq!(rules.regime, Regime::Implied);
        // XB acknowledged, XB-01 not: the strictest row (XB-01's implied) is unacknowledged, so opt-in.
        let acks = [Acknowledgement {
            code: "XB".into(),
            through_seq: 100,
        }];
        let rules = rules_under(Some("XB-01"), &acks);
        assert_eq!(rules.regime, Regime::OptIn);
    }

    #[test]
    fn the_contacts_code_wins_over_the_sends() {
        assert_eq!(applicable_code(Some("XA"), Some("XB")), Some("XA"));
        assert_eq!(applicable_code(None, Some("XB")), Some("XB"));
        assert_eq!(applicable_code(Some(""), Some("XB")), Some("XB"));
        assert_eq!(applicable_code(None, None), None);
        assert_eq!(
            Regime::parse(Regime::SoftOptIn.as_str()),
            Some(Regime::SoftOptIn)
        );
        assert_eq!(Regime::parse("x"), None);
        assert_eq!(
            Blocked::NotGranted.describe(),
            "consent not granted at send time"
        );
    }
}
