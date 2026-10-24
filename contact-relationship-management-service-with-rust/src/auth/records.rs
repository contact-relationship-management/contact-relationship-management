//! Per-record decisions: resource attributes, masking, request attributes, break-glass.

use super::{
    Action, BTreeMap, Claims, ENTITY, MaybeAuthUser, StatusCode, contacts, deals, policy,
    require_auth,
};

/// Derive the **record-level resource attributes** of a stored deal
/// for the ABAC decision (`authorization-attributes.md` §9). Keys a
/// policy matches with `resource.<key>`:
///
/// | Resource key | From | Example tokens |
/// |---|---|---|
/// | `resource.owner` | `Deal::owner_ref` | the bare worker uuid (enables `$sub` ownership) |
/// | `resource.owner_ref` | `Deal::owner_ref` | the full `worker:` URN |
/// | `resource.status` | `won`/`closed_at` | `open`, `won`, `lost` |
///
/// A deployment can then write e.g. "allow write only when
/// `resource.owner = $sub`" (rep ownership, CRM-R15), or "allow a
/// **masked** read otherwise" (an `allow` rule with the `mask`
/// obligation) — entirely as policy, no code change.
#[must_use]
pub fn deal_resource_attrs(deal: &deals::Model) -> BTreeMap<String, Vec<String>> {
    let mut attrs = BTreeMap::new();
    if let Some(owner_ref) = &deal.owner_ref {
        let owner = owner_ref
            .split_once(':')
            .map_or(owner_ref.as_str(), |(_, id)| id);
        attrs.insert("owner".to_string(), vec![owner.to_string()]);
        attrs.insert("owner_ref".to_string(), vec![owner_ref.clone()]);
    }
    let status = if deal.closed_at.is_none() {
        "open"
    } else if deal.won {
        "won"
    } else {
        "lost"
    };
    attrs.insert("status".to_string(), vec![status.to_string()]);
    attrs
}

/// Record-level resource attributes of a **contact**: `resource.owner`
/// (bare worker uuid + full URN, when owned) and `resource.status`.
#[must_use]
pub fn contact_resource_attrs(contact: &contacts::Model) -> BTreeMap<String, Vec<String>> {
    let mut attrs = BTreeMap::new();
    if let Some(owner_ref) = &contact.owner_ref {
        let owner = owner_ref
            .split_once(':')
            .map_or(owner_ref.as_str(), |(_, id)| id);
        attrs.insert("owner".to_string(), vec![owner.to_string()]);
        attrs.insert("owner_ref".to_string(), vec![owner_ref.clone()]);
    }
    attrs.insert("status".to_string(), vec![contact.status.clone()]);
    attrs
}

/// The placeholder shown for a redacted text field under the `mask`
/// obligation (spec `auth.md`).
pub const MASKED: &str = "\u{2022}\u{2022}\u{2022}";

/// Apply the `mask` obligation to a JSON value: set each named
/// top-level key to `null` (CRM-R15 masks **amounts** — deal values,
/// forecast totals, ROI — while structure stays visible). Setting to
/// `null` rather than `0` keeps the family's honesty rule: a masked
/// amount is *absent*, never a fake zero.
#[must_use]
pub fn mask_json(mut value: serde_json::Value, keys: &[&str]) -> serde_json::Value {
    if let Some(object) = value.as_object_mut() {
        for key in keys {
            if object.contains_key(*key) {
                object.insert((*key).to_string(), serde_json::Value::Null);
            }
        }
    }
    value
}

/// Apply the `mask` obligation to a **text** field (CRM-R15 masks
/// contact channel detail): set each named top-level key to
/// [`MASKED`] rather than `null`. Unlike an amount, a redacted string
/// has no "looks like a real value" failure mode `null` is guarding
/// against for [`mask_json`] — a visible, obviously-redacted
/// placeholder keeps the field's type (a string, not an absence) while
/// still hiding the content.
#[must_use]
pub fn mask_text_json(mut value: serde_json::Value, keys: &[&str]) -> serde_json::Value {
    if let Some(object) = value.as_object_mut() {
        for key in keys {
            if object.contains_key(*key) {
                object.insert(
                    (*key).to_string(),
                    serde_json::Value::String(MASKED.to_string()),
                );
            }
        }
    }
    value
}

/// Environment attributes for the current request, for the `env.*`
/// policy namespace (`authorization-attributes.md` §10). Derived here —
/// not in the pure engine — so the clock stays at the service edge and
/// the engine stays deterministic:
///
/// | Env key | Meaning |
/// |---|---|
/// | `env.hour` | Current UTC hour, `0`–`23`. |
/// | `env.after_hours` | `true` outside 08:00–17:59 UTC, else `false`. |
///
/// A deployment can then write e.g. "deny write when
/// `env.after_hours=true` unless `access=admin`". With no `env.*` rule
/// these attributes are inert.
#[must_use]
pub fn request_env_attrs() -> BTreeMap<String, Vec<String>> {
    use chrono::Timelike;
    env_attrs_at(chrono::Utc::now().hour())
}

/// Pure derivation of the environment attributes for a given UTC `hour`
/// (`0`–`23`), split out of [`request_env_attrs`] so it is unit-testable
/// without a clock.
#[must_use]
pub(super) fn env_attrs_at(hour: u32) -> BTreeMap<String, Vec<String>> {
    let after_hours = !(8..18).contains(&hour);
    let mut env = BTreeMap::new();
    env.insert("hour".to_string(), vec![hour.to_string()]);
    env.insert("after_hours".to_string(), vec![after_hours.to_string()]);
    env
}

/// **Record-level** authorization for a handler that has loaded the
/// target record: evaluate the policy with the record's resource
/// attributes ([`stay_resource_attrs`] / [`ward_resource_attrs`]) and
/// the request's environment attributes
/// ([`request_env_attrs`]). A finer, second pass on top of the coarse
/// blanket guard — the guard already required a valid token and a
/// coarse (entity-level) allow before the handler ran; this refines the
/// decision with attributes of the *specific* record and the request
/// context.
///
/// Gated on the same `CRM_REQUIRE_AUTH` flag as the blanket guard:
/// when enforcement is **off** this is a no-op (behaviour-neutral, no
/// authn/authz); when **on**, the blanket guard guarantees a token, so
/// absent claims here are a `401` fail-safe.
///
/// # Errors
///
/// `401` if enforcement is on but no verified claims are present (should
/// not happen behind the blanket guard — fail safe). `403` if the policy
/// denies the action given the record's / request's attributes (the
/// message names the deciding rule).
/// On an **allow**, returns the decision's **obligations** — advisory
/// instructions the handler must honour, e.g. `["mask"]` (return the
/// masked view). When enforcement is off the vec is empty (no authz).
pub fn authorize_record(
    caller: &MaybeAuthUser,
    action: Action,
    resource: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, (StatusCode, String)> {
    if !require_auth() {
        return Ok(Vec::new());
    }
    let claims = caller
        .claims()
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    let mut env = request_env_attrs();
    if crate::break_glass::registry().active(&claims.sub, chrono::Utc::now()) {
        env.insert("break_glass".to_string(), vec!["true".to_string()]);
    }
    let decision = policy()
        .current()
        .evaluate_with_context(claims, action, ENTITY, resource, &env);
    if decision.allowed {
        Ok(decision.obligations)
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

/// Whether the policy lets `claims` read a record with these `resource` attributes. Always true
/// when enforcement is off. The same decision [`authorize_record`] makes, as a yes or no, for a
/// list that leaves out what the caller may not see (CRM-R32).
#[must_use]
pub fn may_read(claims: &Claims, resource: &BTreeMap<String, Vec<String>>) -> bool {
    if !require_auth() {
        return true;
    }
    let mut env = request_env_attrs();
    if crate::break_glass::registry().active(&claims.sub, chrono::Utc::now()) {
        env.insert("break_glass".to_string(), vec!["true".to_string()]);
    }
    policy()
        .current()
        .evaluate_with_context(claims, Action::Read, ENTITY, resource, &env)
        .allowed
}

/// What the sweep knows of a worker who is not making a request: who they are and nothing else
/// (a token's attributes exist only on a request). `worker_ref` is the `worker:` URN.
#[must_use]
pub fn claims_for_worker(worker_ref: &str) -> Claims {
    Claims {
        sub: worker_ref
            .split_once(':')
            .map_or(worker_ref, |(_, id)| id)
            .to_string(),
        email: String::new(),
        name: String::new(),
        iss: String::new(),
        aud: String::new(),
        exp: i64::MAX,
        iat: 0,
        nbf: None,
        sid: String::new(),
        scope: Vec::new(),
        roles: Vec::new(),
        attrs: BTreeMap::new(),
    }
}

/// Whether the caller currently holds a live break-glass grant (CRM-T44).
#[must_use]
pub fn break_glass_active(caller: &MaybeAuthUser) -> bool {
    caller
        .actor()
        .is_some_and(|sub| crate::break_glass::registry().active(sub, chrono::Utc::now()))
}

/// Whether the caller's **collection-level** read carries the `mask`
/// obligation (CRM-T32). Aggregate views (forecast, pipeline value,
/// campaign cost and ROI) have no single record to match, so the decision
/// is made with no `resource.*` attributes: a rule that depends on
/// `resource.owner` does not apply, and a caller is unmasked only by a
/// rule that does not need one (service, admin, manager in the reference
/// policy). With enforcement off this is always `false`.
///
/// # Errors
///
/// `401` without a token and `403` when the policy denies the read.
pub fn aggregate_masked(caller: &MaybeAuthUser) -> Result<bool, (StatusCode, String)> {
    authorize_record(caller, Action::Read, &BTreeMap::new())
        .map(|obligations| obligations.iter().any(|o| o == "mask"))
}
