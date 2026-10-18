# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — `authentication-verifier` from crates.io (2026-10-18)

The dependency (and its dev-dependency with `test-idp`) is now the published
0.9.1 instead of a pinned git commit, which crates.io would reject. Tests,
clippy (with `--features keycloak`) and `cargo deny` pass. The migrations crate
is now `contact-relationship-management-migration` (still imported as
`migration`, via `[lib] name` and a renamed dependency) and is publishable; the
service refers to it by version `0.1.0` plus its path. `cargo publish
--workspace --dry-run` passes for both crates. Nothing has been published.
The package ships only `src/` and the README (`include` in `Cargo.toml`), so it
carries no `config/*.yaml`, tests or Containerfile.

### Changed — crate renamed (2026-10-18)

The package is now `contact-relationship-management` (was
`contact-relationship-management-service`), so the library is
`contact_relationship_management`. The binary (`[[bin]]`, `default-run`, the
Containerfile) is now `contact-relationship-management` too, and the default
databases are `contact_relationship_management_development` and
`..._test` (were `contact_relationship_management_service_…`); CI uses
`ci_contact_relationship_management`. **Existing local databases must be
renamed** (`ALTER DATABASE contact_relationship_management_service_development
RENAME TO contact_relationship_management_development;`, same for `_test`), or
point `DATABASE_URL` at the old name. The package description was also reworded.

### Fixed — `updated_at` now moves on every edit (CRM-T57, 2026-10-18)

SeaORM updates write only the columns given, so `updated_at` kept its insert
value: an edited deal said it was last changed when it was created. Migration
`m20261018_000016` adds a `BEFORE UPDATE` trigger on the 18 tables that are
edited in place; it sets `now()` unless the statement sets `updated_at` itself.
Append-only tables (audit log, consent events, forecast snapshots, outbox, queue)
are left alone. A lead rename in the database tests checks it.

### Added — board edits and follow-up reschedule (CRM-T55, 2026-10-17)

`PUT /api/deals/{pid}`, `/leads/{pid}`, `/tickets/{pid}` edit a card in place;
`PUT /api/activities/{pid}` changes a follow-up's due day (audited as
`rescheduled`) or summary; `PUT /api/{deals,leads,tickets}/{pid}/position` stores
the card's place in its column (migration `m20261017_000015` adds
`board_position` to leads and tickets; deals already had `kanban_position`).
Last write wins; there is no version check.

### Added — real email send path (CRM-T39, 2026-10-11)

`mail::Sender` (simulated by default, SMTP with `CRM_MAIL_MODE=smtp`), an
`email_deliveries` table with one row per campaign and contact (atomic claim,
retry, consent re-read before each send), campaign `subject` and `body`, and
`GET /api/campaigns/{pid}/deliveries`. Production refuses `smtp` mode without
`CRM_MAIL_ALLOW_PRODUCTION=1`. Not built: address-level suppression list,
bounce and complaint webhooks, nurture sends.

### Added — SES, Azure, Mailgun and SendGrid adapters (CRM-T39, 2026-10-13)

`CRM_MAIL_MODE=ses|azure|mailgun|sendgrid` (`src/provider_cloud.rs`,
`src/provider_api.rs`) and `POST /webhooks/email/{ses,azure,mailgun,sendgrid}`.
Migration `m20261013_000014` adds `email_deliveries.provider_id` for Azure,
whose events name the message by the id returned at send. Mailgun callbacks are
verified with `CRM_MAILGUN_SIGNING_KEY`. SNS and SendGrid signatures are not
verified. Not run against live accounts.

### Added — Brevo and Mailjet adapters (CRM-T39, 2026-10-12)

`CRM_MAIL_MODE=brevo` or `mailjet` sends through the vendor's HTTP API
(`src/provider.rs`), tagging each message with our id (Brevo `tags`, Mailjet
`CustomID`) so the vendor's events can be matched. `POST
/webhooks/email/{brevo,mailjet}` reads their callbacks (delivered, bounces,
spam) into the generic outcomes, authenticated by `CRM_WEBHOOK_SECRET` as a
bearer token, basic-auth password or `X-CRM-Webhook-Token`. Checked against the
vendors' documentation only; not run against a live account.

### Added — real nurture sends (CRM-T39, 2026-10-12)

Nurture steps carry an optional `subject` and a plain-text `body`
(migration `m20261012_000013_nurture_step_content`). With a real sender,
`POST /api/nurture/advance` mails each due step after the consent and
suppression checks, once per (step, contact), with the unsubscribe link, and
reports `held` for enrolments it will retry. The person record's mailbox is read
from `email` or `emails` (`clients::extract_email`).

### Changed — campaign sends run as a background job (CRM-T39, 2026-10-12)

With a real sender, `run` validates the campaign and enqueues
`CampaignSendWorker`, which sends batch after batch until nothing more can be
sent. It answers `202` until the campaign completes. A pass that sends nothing
stops the job; a later `run` resumes. Simulated mode is unchanged. The worker
is registered in `connect_workers`.

### Added — lifting a suppression (CRM-T39, 2026-10-12)

`GET` and `DELETE /api/contacts/{pid}/email-suppression` read and lift a
mailbox suppression. Both are audited; lifting does not restore consent.

### Added — outcome webhook and suppression list (CRM-T39, 2026-10-11)

`POST /webhooks/email` (HMAC-signed, `CRM_WEBHOOK_SECRET`) records provider
outcomes once per message and event. `delivered` counts on the campaign; a hard
bounce or complaint suppresses the contact (`email_suppressions`, skipped by
the send path), and a complaint withdraws consent. Migration
`m20261011_000012_email_events`.

### Added — unsubscribe link and headers (CRM-T39, 2026-10-11)

Every real message ends with a signed unsubscribe link and carries
`List-Unsubscribe` and `List-Unsubscribe-Post` (RFC 8058). `GET
/unsubscribe/{token}` changes nothing; `POST` withdraws marketing consent
through the ordinary audited consent change and is idempotent. Public (no
bearer token), HMAC-SHA256 signed, no expiry. `smtp` mode needs
`CRM_UNSUBSCRIBE_SECRET` and `CRM_PUBLIC_URL`. A local Stalwart server for
development is in `compose.email.yaml` and `scripts/email-dev.sh`.

### Changed — RFC 9457 error bodies (CRM-T54, 2026-10-11)

Every `4xx`/`5xx` is now `application/problem+json` with `type`, `title`,
`status`, `detail` and `instance`. The earlier `error` and `description`
members are kept, so only the media type changes. A `401` carries
`WWW-Authenticate: Bearer`. The API versioning and deprecation policy is in
`spec/compliance/api-lifecycle.md`.

### Added — response schemas (CRM-T53, 2026-10-11)

The OpenAPI document types the detail reads, forecast, dashboards, insight
views, audit, retention, privacy and break-glass responses; tests validate
real (including masked) responses against them.

### Added — field-level encryption (CRM-T41, 2026-10-10)

AES-256-GCM on `contacts.preferred_channel` and `activities.summary` through
an `EncryptedText` column type, keyring with rotation (`CRM_FIELD_KEYS`),
`rotate_field_keys` task, and `CRM_REQUIRE_FIELD_ENCRYPTION` start-up check.

### Added — break-glass access (CRM-T44, 2026-10-10)

Reasoned, time-limited, audited read elevation (`/api/break-glass`) with a
review queue; reference policy lifts the mask for reads while a grant is live.

### Added — abuse resistance (CRM-T43, 2026-10-10)

Per-client request limit and failed-authentication lockout (`429` with
`Retry-After`), on by default in production; probes exempt. The UI signs out
an idle session after 15 minutes.

### Added — access-model completion (CRM-T32, T38, T40, 2026-10-10)

`mask` now covers the forecast, sales dashboard, campaign cost/funnel and the
rows bundled into a contact read. Audit, event and retention reads are
admin/service tier; `CRM_PROTECT_OPERATIONAL=1` also protects metrics and
OpenAPI. Personal-data reads (contacts, leads, tickets, activities) are
audited into the hash chain.

### Added — production-readiness hardening (CRM-T30, T31, T33, T34, T35, 2026-10-10)

Production boot refuses to start with `CRM_REQUIRE_AUTH` off
(`CRM_ALLOW_INSECURE=1` overrides). `audit_logs` is now a tamper-evident hash
chain with an append-only trigger and `GET /api/audits/verify`
(migration `m20261010_000008_audit_chain`). `config/production.yaml` adds
security headers, a closed CORS list and no `x-powered-by`, and its SMTP
credentials no longer break parsing when empty. `deny.toml`, a hardened
`Containerfile`, and a CI `supply-chain` stage (audit, deny, SBOM).

### Added — optional Keycloak bearer acceptance; PASETO is now a feature (KC-1, 2026-09-29)

`keycloak` cargo feature (off by default): the blanket guard and the
`AuthUser` / `MaybeAuthUser` extractors also accept an
`Authorization: Bearer <Keycloak access token>`, verified by
`authentication-verifier`'s `keycloak` module and mapped onto the same
`Claims`, so ABAC, masking and audit are unchanged. Configured by
`CRM_KEYCLOAK_URL` / `_REALM` / `_AUDIENCES` / `_ROLE_MAP[_FILE]` /
`_REQUIRE_VERIFIED_EMAIL`; unset URL => off; a malformed configuration logs
an error and leaves Keycloak off (fail-closed, service still boots). A token
beginning `v4.` is a PASETO; anything else is offered to Keycloak. An
unmapped Keycloak role grants nothing. New `auth::request_claims` and
`auth::enforce_request` (async) are what the middleware and extractors use;
the sync `enforce` / `bearer_claims` remain for the PASETO tests.

PASETO verification (`Verifier`, the boot key fetch, the rotation refresh)
moved behind the `paseto` feature, **on by default**, so a default build is
unchanged. `--no-default-features --features keycloak` builds a service with
no PASETO code. New `tests/keycloak_guard.rs` (DB-free) pushes real RS256
Keycloak-shaped tokens, served by a local OIDC provider
(`authentication_verifier::test_idp`), through the real guard: editor may
read/write but not delete/merge, admin may, unmapped roles are read-only,
expired / wrong-audience / garbage / PASETO-shaped-junk are 401. See
`agents/share/runbooks/keycloak-sso.md`.

### Fixed — record-level ABAC wired into the contact/deal handlers (CRM-T24)

`auth::deal_resource_attrs`/`auth::contact_resource_attrs` were defined
and unit-tested but `authorize_record` was called only from
`controllers/privacy.rs` (subject-access/erase) — the deal and contact
read/write handlers ran no record-level check at all. `get_contact` and
`list_contacts` (via a new `readable_contacts` helper, omitting a
denied row per invariant 5) now call `authorize_record` and, on a
`mask` obligation, redact `preferred_channel` through a new
`auth::mask_text_json` (a text-placeholder sibling of the existing
amount-nulling `auth::mask_json`); `repoint_contact` gates its write
the same way. `list_deals` (via a new `readable_deals` helper) and
`deal_stage`/`reopen_deal` do the equivalent for deals, redacting
`amount_minor`. New unit test `mask_text_json_redacts_text_keeps_structure`;
`tests/enforcement.rs` gained an owner-vs-non-owner,
masked-vs-unmasked scenario for one deal and one contact endpoint.
`get_contact`'s bundled activities/deals/tickets rows, and the
forecast/ROI fields CRM-T22 also named, are explicitly left out of
this change (documented, not silent). See spec/tasks.md CRM-T24.

### Added — `require_ref` test coverage for Worker and Organization (CRM-T25)

`src/validation.rs`'s `ref_rules` unit test only ever exercised
`EntityType::Person`, even though `controllers/sales.rs`,
`support.rs`, and `relationships.rs` all call the shared `require_ref`
helper with `EntityType::Worker`/`::Organization` too. New
`ref_rules_wrong_type_worker_and_organization` test pins both the
wrong-type rejection and the matching-type acceptance for each. See
`../spec/tasks.md` CRM-T25.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.
  *(Corrected 2026-09-06: `Cargo.toml` now declares `rust-version =
  "1.96"`, matching `ci/msrv.txt` and the repository's current **N-2**
  policy (`spec/rust-msrv-n-minus-2/index.md`) — the policy tightened
  from N-3 to N-2 after this entry was written. No behaviour change;
  `Cargo.toml`, `ci/msrv.txt`, and `scripts/ci-check.sh msrv` already
  agreed on 1.96 before this correction.)*

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. No feature-list changes (default feature set).
- **`ColType::PkAuto` now generates a 64-bit primary key.** Of this
  crate's ~23 tables, exactly one (`audit_logs`) goes through loco's
  schema DSL and moves from `i32` to `i64`; the relationships, sales,
  marketing, support, engagement, and `event_outbox` tables are all
  created with raw SQL and stay `i32` — same split as the sibling
  consumer apps.
- A `useless_conversion` in `src/models/event_outbox.rs` and a
  pre-existing `needless_borrows_for_generic_args` in
  `src/controllers/mod.rs`, both surfaced by the same clippy run.
- No behavioural change; verified with the full DB-gated suite (8
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — engagement / partnerships / confederation (CRM-T20, 2026-07-20)

- Declared stakeholder typing (role + power–interest 1–5 on contacts;
  role on accounts), recorded activity sentiment, the forward-only
  partnership lifecycle, per-account memberships, and working groups
  with derived activity feeds (`controllers/engagement.rs`).
- Nine new derived views (`controllers/insights.rs`): cadence,
  engagement workload, the audit-derived pipeline funnel, member
  health, consent-by-account, the stakeholder register + grid, the
  partnership register, membership renewals; follow-ups gains a
  `kind` filter (renewals convention).

### Added — insight views (CRM-T19, 2026-07-20)

- `controllers/insights.rs`: stale-deal aging (from stage-change
  audits), follow-ups (overdue + 30-day horizon), pipeline-hygiene
  findings, the period executive pack (per-currency won value never
  merged), the stored forecast-trend series (no interpolation), the
  SLA breach register + per-assignee workload, and the DPO view
  (consent coverage + withdrawals + per-source counts +
  duplicate-contact hygiene). All ETag-conditional with `as_of`.

### Added

- 2026-07-18 — CRM-T1–T16 implementation round: full Loco service
  (copy-adapted from the WPM service). 6 migrations / 19 tables,
  pure `rules/` core (lifecycle machines, deterministic lead scoring
  with breakdown, forecast/ROI/CLV/win-rate with per-currency
  honesty, SLA derivation, consent-gated segment evaluation), five
  module controllers (relationships / sales / marketing / support /
  dashboards) + audits/docs/metrics, offline PASETO + ABAC with
  `resource.owner` `$sub` ownership + amount masking,
  ETag-conditional dashboards, idempotent nurture advance + SLA
  sweep, simulated campaign send (consent re-checked at send time),
  seed task. 62 unit + 5 request + 1 enforcement tests green against
  Postgres 18; clippy-pedantic clean; live smoke verified.

- 2026-07-18 — CRM-T0 specification round: cross-cutting spec
  (`../spec/`) with the four-module domain, SDD trio
  (requirements CRM-R1–R17, design CRM-D1–D12, tasks CRM-T*), and
  this edition's doc scaffold. No code yet.
