# Authentication & authorization

The family stack unchanged
([authentication-sessions](../../agents/share/authentication-sessions.md),
[authorization-attributes](../../agents/share/authorization-attributes.md)):
cookie sessions + BFF for humans, offline PASETO v4.public for
services, blanket guard `CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH` (default **off** — the
family activation gate; activate before any real exposure), shared
ABAC engine.

## The UI's calls to the authentication service

The UI's server makes four calls to the authentication service at `AUTH_API_URL`: request a magic link, verify one,
exchange the session for a token (on every proxied API call), and sign out. They are server-to-server, so they send no
`Origin`: they use the platform's `fetch`, not SvelteKit's, which sets `Origin` to the UI on a request to another origin.
A service on axa 0.13 refuses a write whose `Origin` is not its own host when there is no `Sec-Fetch-Site`, so that
header would turn the token exchange and sign-out into `403`s (CRM-T240; the same fault in the API proxy is CRM-T237).

## Personas as policy (not code)

One API; four typical policy personas expressed over `attrs`:

| Persona | Typical rules |
|---|---|
| **rep** | own contacts/leads/deals/activities (`resource.owner = $sub` ownership template); read team accounts |
| **sales manager** | team-wide pipeline + forecast + reassign, scoped by `resource.owner` / team attrs |
| **marketing** | campaigns, segments, nurture, consent views; read-only pipeline |
| **support** | tickets, KB, SLA views; read contacts/accounts |

## Record-level attributes

Handlers derive these for the second ABAC pass (`src/auth/records.rs`). `resource.owner` is the bare id of the
owner's `person:` URN, the form a sign-in subject takes, so a `$sub` rule scopes a record to its owner (CRM-D18);
`resource.owner_ref` is the URN as stored. An unowned record has neither.

| Record | Attributes | What `mask` hides |
|---|---|---|
| contact | `owner`, `owner_ref`, `status` | channel details |
| deal | `owner`, `owner_ref`, `status` (`open`, `won`, `lost`) | amounts |
| ticket (CRM-T321) | `owner` and `owner_ref` from the assignee, `status`, `priority`, `channel` | title, activity summaries |

Other records (accounts, leads, campaigns, articles) are decided on the entity alone; no record-level attribute,
`resource.tier` included, is derived for them. The `mask` obligation also redacts forecast values and campaign
costs/ROI, while leaving structure (stages, counts, statuses) visible.

## Sensitivity map

| Data | Tier |
|---|---|
| consent history, unsubscribe records | high — compliance evidence, append-only, reads audited |
| deal amounts / forecasts / campaign ROI | high — commercial; masked under `mask` |
| contact channel details & activity content | medium — personal data; masked under `mask` |
| pipelines, stages, SLA policies, published KB | low |

## Subject rights & retention (CRM-R20, the code side of CRM-G2)

`GET /api/contacts/{pid}/subject-access` (one audited export across
every table keyed to the contact, exclusions named in the payload),
`POST /api/contacts/{pid}/erase` (anonymise; refused `422` while the
contact holds an open deal, an open ticket, or an active nurture
enrolment — CRM-D14), and `GET /api/retention` /
`POST /api/retention/sweep` (the floored-horizon report and sweep,
`CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS`, default 365, floor 30). `/erase` and `/sweep`
join [`DESTRUCTIVE_POST_SUFFIXES`](../contact-relationship-management-service-with-rust/src/auth/policy.rs)
(⇒ `access=admin` or `svc=true` under enforcement — no persona below
admin/svc can erase or sweep, including a manager or the record's own
owner). Subject access is refused (`403`) to any caller whose decision
carries the `mask` obligation: a masked export would contradict its
own purpose.

## Activation runbook (CRM-G1)

The shipped default is **wide open** (family posture,
`agents/share/security.md` §4). Activation is a release gate, not a
config tweak:

1. **Mount a policy.** Start from the shipped reference,
   [`config/abac-policy.reference.json`](../contact-relationship-management-service-with-rust/config/abac-policy.reference.json)
   — svc/admin do everything; `resource.owner = $sub` reads and writes
   the caller's own record unmasked (record-level — see the engine
   limit below); `manager=true` writes and reads unmasked (team-wide
   pipeline/forecast oversight); `rep=true` writes (coarse: CRM has no
   ownership-enforcing write handler yet, so this is a plain write
   grant, not a scoped one); `marketing=true` / `support=true` write
   and read **masked**; every other authenticated caller gets the
   masked-read fallback. Point `CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY_FILE` at your copy (it
   hot-reloads on change) or inline it via `CONTACT_RELATIONSHIP_MANAGEMENT_ABAC_POLICY`.
2. **Point at the keys.** `CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS_URL` (boot-fetched +
   refreshed) or `CONTACT_RELATIONSHIP_MANAGEMENT_PASETO_KEYS`; set `CONTACT_RELATIONSHIP_MANAGEMENT_TOKEN_ISSUER` /
   `CONTACT_RELATIONSHIP_MANAGEMENT_TOKEN_AUDIENCE` if they differ from the defaults.
3. **Flip the flag.** `CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH=1` (read once at boot —
   restart to change).
4. **Verify.** `cargo test --test main enforcement -- --ignored` runs the
   activation matrix against the reference policy shape: public paths
   open; 401 without a token; a reader can GET but not POST; a writer
   (`access=write`) completes a real consent-gated flow end to end.
   Extend it with the persona matrix above (manager unmasked vs
   marketing/support masked, `/erase` and `/sweep` admin/svc-only)
   before relying on it in a real deployment — the shipped suite
   proves the mechanism, not every persona row.

**Known engine limits a deployment must plan around** (stated, not
hidden — CRM's wiring is younger than WPM's, so this list is shorter
in the wrong direction: less is actually enforced today):

- **`resource.owner = $sub` ownership currently reaches only two
  handlers**: `GET /api/contacts/{pid}/subject-access` and
  `POST /api/contacts/{pid}/erase` (both new, CRM-T21). Every other
  handler in this crate — contacts, accounts, deals, leads, campaigns,
  tickets, articles — is gated **only** by the coarse blanket guard
  (method + destructive-suffix ⇒ action; no record loaded, no
  `resource.*` match). `auth::deal_resource_attrs` /
  `auth::contact_resource_attrs` exist and are unit-tested, but until
  more handlers call `auth::authorize_record` with them, a rep's
  "own contacts/leads/deals" scoping (spec `auth.md` persona table) is
  a **policy aspiration**, not yet an enforced boundary — any caller
  who clears the coarse write/read gate can act on any record. Plan
  deployments accordingly until that wiring lands.
- **Update (CRM-T32, 2026-10-10): `mask` now covers** contact and deal
  reads, the forecast, the sales dashboard, campaign cost, funnel revenue
  and ROI, and the rows bundled into a contact read. The insights money views and
  forecast snapshots are covered too (follow-up). The next bullet describes
  the earlier state and is kept for history.
- **The `mask` obligation is not yet applied by any read handler.**
  `auth::mask_json` exists and is unit-tested (nulls named keys,
  never fakes a zero), and `subject_access` treats *any* `mask`
  obligation as an outright refusal — but no list/get handler in this
  crate currently calls `mask_json` to redact deal amounts, forecast
  values, campaign ROI, or contact channel details on an ordinary
  read. A `marketing=true` or `support=true` caller today receives the
  **unredacted** response from every endpoint except subject-access.
  The obligation is real infrastructure with one real consumer so
  far; treat the sensitivity-map masking above as a policy contract to
  finish wiring, not a control already in force.
- **`rep=true` is a coarse write grant**, not a scoped one (see the
  first bullet) — a deployment relying on true per-rep ownership needs
  the record-level wiring above before this persona means what the
  spec table implies.

## Production start-up check (CRM-T30)

In the `production` environment the service refuses to start unless
`CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH` is on. `CONTACT_RELATIONSHIP_MANAGEMENT_ALLOW_INSECURE=1` overrides it and logs an
error. The default stays off in development and test. The full readiness
assessment, with the remaining access-model gaps (mask coverage, admin-only
audit reads, read auditing, break-glass), is in [plan.md](plan.md).

## Privileged reads (CRM-T38)

`GET` under `/api/audits`, `/api/events`, and `/api/retention` derives the
`destructive` action (the admin/service tier), so an ordinary reader,
manager, marketing, or support caller is refused. A policy must grant
`destructive` to whoever may review audits. `CONTACT_RELATIONSHIP_MANAGEMENT_PROTECT_OPERATIONAL=1`
also requires authentication on `/metrics.prom` and the OpenAPI paths.

## Break-glass (CRM-T44)

`POST /api/break-glass` (reason of 20+ characters, 1 to 60 minutes) gives the
caller `env.break_glass=true` for the grant's life. A policy matches it with
`"when": {"env.break_glass": ["true"]}`; the reference policy lifts the mask
for reads and nothing else. Grants are audited, listed at
`GET /api/break-glass`, and reviewed by someone other than the grantee at
`POST /api/break-glass/{pid}/review`. Other replicas honor a grant within 10
seconds.
