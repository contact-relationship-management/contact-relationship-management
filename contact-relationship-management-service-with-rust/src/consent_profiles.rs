//! Consent profiles in the database (CRM-T166, CRM-D21): loading the codes, rows and acknowledgements, the evidence
//! a looser regime rests on, the purposes, and the one decision every send path makes. The rules themselves are the
//! pure `rules::consent_regime`; this module only reads what they need.
//!
//! **No profile is today's rule.** When no row applies to a recipient the decision is opt-in, and no evidence is
//! read: only `granted` sends.
//!
//! On `sqlx` since CRM-T215 (CRM-D24).

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::models::_entities::contacts;
use crate::rules::consent_regime::{
    self as regime, Acknowledgement, Basis, Blocked, Layer, Link, MessageRules, Regime, Row, Rules,
    Status, Step,
};

/// Everything the resolver needs, read once per send or preview.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    /// The codes the organization keeps.
    pub links: Vec<Link>,
    /// Every profile row.
    pub rows: Vec<Row>,
    /// Every acknowledgement.
    pub acks: Vec<Acknowledgement>,
}

/// Read the codes, rows and acknowledgements. A stored row whose value no longer fits the registry is skipped.
///
/// # Errors
///
/// A database error.
pub async fn load(db: &mut PgConnection) -> Result<Loaded, sqlx::Error> {
    let mut loaded = Loaded::default();
    for row in sqlx::query!("SELECT code, layer, parent FROM consent_jurisdictions ORDER BY code")
        .fetch_all(&mut *db)
        .await?
    {
        loaded.links.push(Link {
            code: row.code,
            layer: Layer::parse(&row.layer).unwrap_or(Layer::National),
            parent: row.parent,
        });
    }
    for row in sqlx::query!(
        "SELECT id, code, key, value, effective_from, effective_to, source, status \
         FROM consent_profile_rows ORDER BY id"
    )
    .fetch_all(&mut *db)
    .await?
    {
        let (Ok(value), Some(status)) = (
            regime::validate(&row.key, &row.value),
            Status::parse(&row.status),
        ) else {
            continue;
        };
        loaded.rows.push(Row {
            code: row.code,
            key: row.key,
            value,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            source: row.source,
            status,
            seq: row.id,
        });
    }
    for row in sqlx::query!(
        "SELECT code, max(through_id) AS \"through_id!\" FROM consent_profile_acknowledgements \
         GROUP BY code"
    )
    .fetch_all(&mut *db)
    .await?
    {
        loaded.acks.push(Acknowledgement {
            code: row.code,
            through_seq: row.through_id,
        });
    }
    Ok(loaded)
}

impl Loaded {
    /// The chain under `code`. A chain the stored links cannot form (a cycle, a layer out of order) is the code
    /// alone, so its own rows still apply and no parent is guessed.
    #[must_use]
    pub fn chain_for(&self, code: Option<&str>) -> Option<Vec<Step>> {
        let code = code?;
        Some(regime::chain(code, &self.links).unwrap_or_else(|_| {
            vec![Step {
                code: code.to_string(),
                layer: regime::inferred_layer(code),
            }]
        }))
    }

    /// The rules for `channel` under `code` on `as_of`.
    #[must_use]
    pub fn rules(&self, channel: &str, code: Option<&str>, as_of: NaiveDate) -> Rules {
        let chain = self.chain_for(code);
        regime::rules_for(channel, chain.as_deref(), &self.rows, &self.acks, as_of)
    }

    /// The message rules under `code` on `as_of`.
    #[must_use]
    pub fn message_rules(&self, code: Option<&str>, as_of: NaiveDate) -> MessageRules {
        let chain = self.chain_for(code);
        regime::message_rules(chain.as_deref(), &self.rows, as_of)
    }
}

/// The evidence a looser regime may rest on for `contact` (decisions 5 and 6): won deals naming the contact as
/// primary contact (`won_deal`, at the close), web-form submissions (`inquiry`, for the form's purpose) and tickets
/// the contact is on (`inquiry`, any purpose, at the opening).
///
/// # Errors
///
/// A database error.
pub async fn evidence<'e>(
    db: impl PgExecutor<'e>,
    contact: Uuid,
) -> Result<Vec<regime::Evidence>, sqlx::Error> {
    Ok(sqlx::query!(
        "SELECT 'won_deal' AS \"kind!\", pid AS \"pid!\", closed_at AS \"at!\", \
           NULL::varchar AS purpose FROM deals \
           WHERE primary_contact_pid = $1 AND won AND closed_at IS NOT NULL AND deleted_at IS NULL \
         UNION ALL \
         SELECT 'inquiry', pid, created_at, purpose FROM form_submissions WHERE contact_pid = $1 \
         UNION ALL \
         SELECT 'inquiry', pid, opened_at, NULL FROM tickets \
           WHERE contact_pid = $1 AND deleted_at IS NULL",
        contact,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| regime::Evidence {
        kind: row.kind,
        pid: row.pid.to_string(),
        at: row.at,
        purpose: row.purpose,
    })
    .collect())
}

/// A purpose, as an administrator keeps it (CRM-D21, "Purposes an administrator edits").
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "ConsentPurposeDefinition")]
pub struct Purpose {
    /// The key campaigns, sequences and consent name it by.
    pub key: String,
    /// Its place in the list, which a signed link names it by; never reused.
    pub position: i32,
    /// Its name per locale.
    pub names: BTreeMap<String, String>,
    /// Its description per locale.
    pub descriptions: BTreeMap<String, String>,
    /// Whether a new campaign, sequence or preference-page row may name it.
    pub active: bool,
    /// Whether a soft opt-in may apply to it (decision 5).
    pub soft_opt_in_eligible: bool,
}

/// Every purpose, in position order.
///
/// # Errors
///
/// A database error.
pub async fn purposes<'e>(db: impl PgExecutor<'e>) -> Result<Vec<Purpose>, sqlx::Error> {
    Ok(sqlx::query!(
        "SELECT key, position, names, descriptions, active, soft_opt_in_eligible \
         FROM consent_purposes ORDER BY position"
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| Purpose {
        key: row.key,
        position: row.position,
        names: serde_json::from_value(row.names).unwrap_or_default(),
        descriptions: serde_json::from_value(row.descriptions).unwrap_or_default(),
        active: row.active,
        soft_opt_in_eligible: row.soft_opt_in_eligible,
    })
    .collect())
}

/// The purpose named `key`, if it exists.
///
/// # Errors
///
/// A database error.
pub async fn purpose<'e>(
    db: impl PgExecutor<'e>,
    key: &str,
) -> Result<Option<Purpose>, sqlx::Error> {
    Ok(purposes(db).await?.into_iter().find(|p| p.key == key))
}

/// The purpose at `position` (a signed link's), if it exists.
///
/// # Errors
///
/// A database error.
pub async fn purpose_at<'e>(
    db: impl PgExecutor<'e>,
    position: i32,
) -> Result<Option<Purpose>, sqlx::Error> {
    Ok(purposes(db)
        .await?
        .into_iter()
        .find(|p| p.position == position))
}

/// One send decision: the code and rules it was taken under and its outcome.
#[derive(Debug, Clone)]
pub struct Decision {
    /// The code the recipient is under, if any.
    pub code: Option<String>,
    /// The rules in force.
    pub rules: Rules,
    /// The basis, or why there is none.
    pub outcome: Result<Basis, Blocked>,
}

impl Decision {
    /// Whether the message may be sent.
    #[must_use]
    pub const fn sends(&self) -> bool {
        self.outcome.is_ok()
    }

    /// What a delivery records (CRM-R6): the basis, the code, the regime and the row it came from (with its status
    /// and source), and a regime that could not apply for want of an acknowledgement.
    #[must_use]
    pub fn record(&self) -> serde_json::Value {
        let mut out = match &self.outcome {
            Ok(basis) => serde_json::to_value(basis).unwrap_or_default(),
            Err(blocked) => serde_json::json!({ "blocked": blocked }),
        };
        if let Some(map) = out.as_object_mut() {
            map.insert("consent_jurisdiction".into(), self.code.clone().into());
            map.insert("regime".into(), self.rules.regime.as_str().into());
            map.insert(
                "profile".into(),
                serde_json::to_value(&self.rules.regime_from).unwrap_or_default(),
            );
            if let Some(stated) = self.rules.unacknowledged {
                map.insert("unacknowledged_regime".into(), stated.as_str().into());
            }
        }
        out
    }
}

/// The contact's state for `purpose` on `channel`. `email` is what [`crate::consents::state_of`] reads; another
/// channel is a `contact_consents` row, none meaning `never`.
///
/// # Errors
///
/// A database error.
pub async fn state_on<'e>(
    db: impl PgExecutor<'e>,
    contact: &contacts::Model,
    purpose: &str,
    channel: &str,
) -> Result<String, sqlx::Error> {
    if channel == regime::SEND_CHANNEL {
        return crate::consents::state_of(db, contact, purpose).await;
    }
    Ok(sqlx::query_scalar!(
        "SELECT state FROM contact_consents WHERE contact_pid = $1 AND purpose = $2 \
         AND channel = $3",
        contact.pid,
        purpose,
        channel,
    )
    .fetch_optional(db)
    .await?
    .unwrap_or_else(|| "never".to_string()))
}

/// Decide whether a message for `purpose` on `channel` may go to `contact`, sent under `send_code` (the campaign's
/// or sequence's), at `now` (`as_of` its day in the organization's zone). Evidence is read only when the regime is
/// looser than opt-in and the state is one evidence can help.
///
/// # Errors
///
/// A database error.
#[allow(clippy::too_many_arguments)] // the decision's inputs, each named in the design
pub async fn decide(
    db: &mut PgConnection,
    loaded: &Loaded,
    contact: &contacts::Model,
    purpose: &str,
    channel: &str,
    send_code: Option<&str>,
    now: DateTime<Utc>,
    as_of: NaiveDate,
) -> Result<Decision, sqlx::Error> {
    let state = state_on(&mut *db, contact, purpose, channel).await?;
    let code = regime::applicable_code(contact.consent_jurisdiction.as_deref(), send_code)
        .map(ToString::to_string);
    let rules = loaded.rules(channel, code.as_deref(), as_of);
    let needs_evidence = matches!(rules.regime, Regime::SoftOptIn | Regime::Implied)
        && matches!(state.as_str(), "never" | "pending");
    let (eligible, evidence) = if needs_evidence {
        (
            crate::consent_profiles::purpose(&mut *db, purpose)
                .await?
                .is_some_and(|p| p.soft_opt_in_eligible),
            evidence(&mut *db, contact.pid).await?,
        )
    } else {
        (false, Vec::new())
    };
    let outcome = regime::basis(&rules, &state, purpose, eligible, &evidence, now);
    Ok(Decision {
        code,
        rules,
        outcome,
    })
}
