# Contact Relationship Management service — edition spec

Stack-specific specification for the Loco edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth** for the domain (model, modules, state
machines, auth posture); this file adds only what is specific to
this edition. It will grow topic files (routes, api-contract,
database, examples) as implementation phases land — each CRM-T* task
adds its spec detail here in the same PR as the code.

## Stack

Per [architecture](../../spec/architecture.md) and the family
[rust-loco-stack](../../../agents/share/rust-loco-stack.md):
Rust 2024, Loco (Axum + SeaORM), PostgreSQL 18, crate-root
`migration/`, loco-idiomatic `src/controllers/` layout, pure
`src/rules/` core, Postgres-queue background jobs (campaign send,
notification sweep), stub-first upstream clients, offline PASETO + ABAC,
OpenAPI/Swagger, `Accepts-version` header versioning, tracing +
OTLP, Podman.

## Edition-specific decisions (so far)

- **Config**: `config/{development,test,production}.yaml`;
  development and test default upstream clients to `stub` and
  `CRM_EVENT_TRANSPORT=memory`.
- **Env vars**: `CRM_REQUIRE_AUTH`, `CRM_PASETO_KEYS[_URL]`,
  `CRM_ABAC_POLICY[_FILE]`, `CRM_EVENT_TRANSPORT`, upstream base
  URLs (`CRM_PERSON_SERVICE_URL`, `CRM_ORGANIZATION_SERVICE_URL`,
  `CRM_WORKER_SERVICE_URL`, `CRM_PPM_SERVICE_URL` for the bridges of
  CRM-R72 and CRM-R73; `CRM_PPM_SERVICE_TOKEN` and `CRM_WORKER_SERVICE_TOKEN`
  are the bearer tokens sent to them, CRM-R87; `CRM_WPM_WEBHOOK_SECRET`
  is the secret WPM signs its hire and leave events with, CRM-R92;
  `CRM_WPM_ORGANIZATION_REF` the organization the employment check asks WPM about, CRM-R94), `CRM_UPSTREAM_MODE` (default `stub`), `CRM_MIN_CELL_SIZE` (smallest group a
  report shows, default 5). These are a selection: every setting is read
  in `src/settings.rs` (or at start-up by the module that uses it) and
  listed in the repository's `INSTALL.md`. There is no setting for a
  stalled-deal threshold or for the lead-scoring weights (checked
  2026-10-30: `rules::scoring::Weights` is only ever used at its
  defaults, and no stalled flag is built; see CRM-R4).
- **Identifiers**: public UUID `pid` on every owned record; EntityRef
  URNs for all upstream references.
- **Table names all-plural** (the loco `create_table` pluralization
  lesson); `event_outbox` as explicit SQL.

## Edition-specific implementation notes (as landed)

- **Layout** (checked 2026-10-30): `src/` holds `app.rs`, `auth/`
  (verifier, policy, per-record attributes, extractors), `clients.rs`
  (the stub-first upstream lookups and the PPM and WPM bridge calls),
  `settings.rs`, `validation.rs`, `middleware/`, `mcp/`, `tasks/`
  (`seed`, `rotate_field_keys`) and the cross-cutting modules (mail,
  webhooks, idempotency, crypto, search, streaming, OpenAPI);
  `src/rules/` is the pure core (lifecycles, scoring, segments, SLA,
  analytics, quotas, attribution, workflows, and since CRM-T158–T162
  `wpm_events`, `employment_check`, `teams_import` and `user_ref`);
  `src/controllers/` has one module per area, `bridges.rs` holding the
  PPM and WPM bridges; `src/models/` has SeaORM entities for 26 tables
  in `_entities/` and helper modules (`employments`, `audit_logs`,
  `event_outbox`, `deliveries`, `records`, `live`, `encrypted`); the
  other tables are reached by explicit SQL. Crate-root `migration/`
  holds **54 migrations** (explicit SQL but for `audit_logs`), the latest
  `m20261030_000054_person_user_refs` (CRM-D18); 000052 and 000053
  added `wpm_events`, `contact_employments` and
  `employment_last_events` (CRM-R92, CRM-R93). The database has 62
  tables besides `seaql_migrations`, each listed in
  [the personal-data inventory](../../spec/compliance/personal-data-inventory.md),
  which a request test keeps true.
- **Users are `person:` references** (CRM-D18): `owner_of` and every
  user field write `person:<sub>`; `rules::user_ref` reads `worker:<id>`
  on input as `person:<id>`.
- **Deal stages are data**, not a token table: `pipeline_stages`
  rows with probabilities + terminal flags; stage moves validate
  pipeline membership on the locked deal row.
- **Consent** is enforced in the segment evaluator (structural
  AND-gate) *and* re-checked at send time by both send paths; the
  `consent_events` table is append-only (no `deleted_at`).
- **Jobs**: `POST /api/nurture/advance` (idempotent per enrolment ×
  step) and `POST /api/sla/sweep` (once per breach) are run by a
  caller, not on a timer. Two background workers are registered in
  `app.rs`: the campaign send (`CampaignSendWorker`) and the
  notification sweep (`NotificationSweepWorker`, queued every minute by
  the server).
- **Dashboards** are ETag-conditional (weak tag over the payload
  minus `as_of`); ratios carry numerator/denominator and go `null`
  on a zero denominator; currencies never merge.
- **Enforcement tests** live in `tests/requests/enforcement.rs`; they
  change the settings for their own length (`tests/support/scoped_auth.rs`)
  and share the request-test process. Request tests need PostgreSQL and
  are `#[ignore]`d — run with `cargo test -- --ignored`.

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md). As of
2026-10-30 every task from CRM-T0 to CRM-T162 is done except those
tasks.md marks open: CRM-T39 (a real email provider: built in code, the
legal and vendor review open), CRM-T42 (independent assurance), CRM-T48
(manual accessibility audit) and CRM-T49 (GDS API review and build
signing) not started; CRM-T92 (opt-in demo tour) and CRM-T94 (CI gates)
in progress. Production gates CRM-G1/CRM-G2 remain partially open — see
tasks.md for what each gate's code side did and did not land. The most
recent service work is the WPM bridges (CRM-T158–T161: hire and leave
events, their order, the employment check, teams by `person_ref`) and
CRM users as `person:` references (CRM-T162); none of it has been run
against a running WPM. Tests per
[../../spec/testing.md](../../spec/testing.md).
