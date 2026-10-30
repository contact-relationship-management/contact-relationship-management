# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **back-end JSON API**, written in Rust on [Loco](https://loco.rs)
(Axum + SeaORM + PostgreSQL), for contact relationship management:
contacts and accounts with relationship timelines, leads with
rule-based scoring, deal pipelines (Kanban) and stage-weighted
forecasting, email campaigns / segments / nurture sequences with
consent-first enforcement, support tickets with SLA tracking, a
knowledge base, and derived dashboards (win rate, ROI, CLV, SLA
health). There is no built-in UI — the
[Svelte sibling](../contact-relationship-management-ui-with-svelte/)
is the sales / marketing / support client.

**Domain ownership.** CRM **owns relationship state** (its own
tables) but **references identities**: contacts are person-service
records, accounts organization-service, reps/agents the signed-in
users, named by the person id of their sign-in subject (`person:<sub>`,
CRM-D18; `worker:<sub>` is still accepted as input and rewritten,
`src/rules/user_ref.rs`) — always as EntityRef URNs (`person:<uuid>`),
never duplicated demographics. Imported teams name their members by
`person:` too (CRM-R73). Dedup/merge of identities stays upstream. See the
cross-cutting spec's [scope boundary](../spec/scope.md).

> ⚠️ Demo software, not a production CRM; no real email is sent.
> See [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth;
   this subproject's `spec/` adds stack detail only. A behavioural
   change is spec edit + code + tests in one PR. The live task queue
   is [`../spec/tasks.md`](../spec/tasks.md) (CRM-T* ids, traced to
   CRM-D*/CRM-R*).
2. **Family conventions.** Loco-idiomatic layout
   (`src/controllers/`), `#![forbid(unsafe_code)]`, thiserror,
   tracing + OTLP, OpenAPI/Swagger, header API versioning
   (`Accepts-version`), Podman not Docker, PostgreSQL not SQLite,
   `worker` (Postgres-backed) jobs, in-memory loco cache (Redis
   optional, `CRM_REDIS_URL`). The family notes cited in comments as
   `agents/share/*.md` (for example `rust-loco-stack.md`, `jwt.md`,
   `event-bus.md`) live in the `main-x-service` repository, not here.
3. **Pure core.** Every lifecycle machine (lead, deal, campaign,
   ticket, article), the scoring rule table + breakdown, forecast /
   ROI / CLV / win-rate / attribution arithmetic, SLA deadline +
   breach derivation, segment evaluation, and the family-bridge
   decisions (WPM event signature, meaning and ordering in
   `wpm_events.rs`; the employment-check batches and answers in
   `employment_check.rs`; the teams import in `teams_import.rs`) live
   in DB-free `src/rules/` modules with exhaustive unit tests;
   controllers only wire them ([design](../spec/design.md) CRM-D3–D5).
4. **Consent gates sends.** The campaign job and nurture scheduler
   re-check `marketing_consent` at send time; segments AND
   `consent = granted` structurally; ConsentEvent history is
   append-only. Never add a send path that skips this.
5. **Money discipline.** Minor units (`i64`) + ISO-4217; overflow
   refused; mixed currencies report per-currency lines, never a
   silent sum. No floats.
6. **Known family gotchas.** loco `create_table` pluralizes table
   names (use already-plural names / explicit SQL);
   `ModelError::EntityNotFound` is NOT mapped to 404 (return
   `Error::NotFound` at `find_by_pid` call sites); enforcement tests
   need their own test binary (OnceLock caching).

## Running

```bash
cargo run -- db migrate && cargo run -- task seed && cargo run -- start   # :5150
cargo test --features keycloak                     # DB-free unit tests
DATABASE_URL=postgres://… cargo test --features keycloak -- --ignored --test-threads=1
```

The request tests (`--ignored`) need PostgreSQL with the `uuid-ossp`,
`citext` and `unaccent` extensions; the Redis ones also need
`CRM_REDIS_URL`. From the repository root, `scripts/ci-check.sh
<fmt|clippy|test|test-db|…>` runs exactly what CI runs.
