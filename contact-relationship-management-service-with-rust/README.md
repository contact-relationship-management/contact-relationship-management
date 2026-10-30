# Contact Relationship Management — Loco JSON API

A back-end **JSON API** for CRM: contacts and accounts with full
relationship timelines, lead capture and explainable rule-based
scoring, deal pipelines with Kanban ordering and stage-weighted
forecasting, consent-first email campaigns / segments / nurture
sequences, support tickets with SLA tracking, a versioned knowledge
base, and derived dashboards (win rate, pipeline value, campaign
ROI, CLV, SLA health). Implemented in Rust on
[Loco](https://loco.rs) (Axum + SeaORM + PostgreSQL). No built-in
UI — the
[Svelte sibling](../contact-relationship-management-ui-with-svelte/)
provides the sales, marketing, and support client.

> ⚠️ **Demo software.** Not a production CRM; no real email is sent
> (delivery is simulated); synthetic data only. See
> [spec/regulatory](../spec/regulatory.md).

**Status.** The delivery record is
[../spec/tasks.md](../spec/tasks.md) (CRM-T1 to CRM-T162), which says
what each task verified and what it did not. The CRM-G1 and CRM-G2
production gates (activating authentication, and the GDPR/PECR review
of the real send path) are still open, so this is not for exposure
beyond a demo.

## Quick start

```bash
# PostgreSQL and Redis in Podman (disposable; the database is in memory).
podman compose -f compose.test.yaml up -d
export DATABASE_URL=postgres://loco:loco@localhost:5432/contact_relationship_management_development
cargo run -- db create     # the container creates only contact_relationship_management_test
cargo run -- db migrate && cargo run -- task seed && cargo run -- start
curl localhost:5150/api/forecast | jq .
```

The server listens on port 5150 (`config/development.yaml`). Without
`DATABASE_URL`, development uses the database named above. `cargo test`
runs the DB-free tests; `cargo test -- --ignored` runs the request tests
against `contact_relationship_management_test` (`config/test.yaml`),
which the compose file creates.

## What it answers

- *Which leads should I call first?* — score-sorted queue with the
  per-rule breakdown
- *Where is every deal, and what will this quarter close?* — Kanban
  board + stage-weighted forecast (derived, never typed)
- *Did that campaign pay for itself?* — funnel + ROI from
  attributed won revenue
- *Are we keeping our support promises?* — SLA deadlines + breach
  flags
- *What is this account worth?* — CLV from won deals

## Surface

Every route is in the OpenAPI document at `/api-docs/openapi.json`
(Swagger UI at `/swagger-ui`). By area, under `/api` unless noted:

- **Relationships:** contacts, accounts, activities and timelines;
  duplicates, merge and unmerge; tags, custom fields, external links,
  attachments; contact and account imports; search.
- **Sales:** leads and scoring, lead imports, pipelines, deals, boards,
  forecast and snapshots, quotas, commission plans, teams, territories,
  FX rates.
- **Marketing:** consent, segments, campaigns, nurture sequences, email
  templates, attribution; public `/unsubscribe`, `/confirm`,
  `/preferences` and `/forms` pages; provider callbacks at
  `/webhooks/email` and `/webhooks/email/{provider}`.
- **Support:** tickets, SLA policies, business calendars, articles;
  the public knowledge base at `/kb/articles`; email replies at
  `/webhooks/inbound-email`.
- **Analytics:** dashboards, reports, saved views, and sixteen
  `/api/insights/*` derived views (stale-deals, followups,
  pipeline-hygiene, executive, forecast-trends, sla, dpo, cadence,
  engagement, funnel, members, consent-by-account, stakeholders,
  partnerships, memberships, kb-deflection).
- **Engagement:** groups, partnerships.
- **Bridges** to the family's other services (`/api/bridges`, admin
  only, except the signed events route): a won deal opens a delivery idea in the project portfolio
  service; the worker service's departments become teams; the worker
  service's signed hire and leave events pause and resume a contact's
  marketing; and the employment check. See the
  [family bridges tutorial](../docs/tutorial-family-bridges.md).
- **Operations and compliance:** workflows, notifications, outbound
  webhook subscriptions, audits, `/api/events/recent`, privacy (subject
  access, erasure, retention sweep), break-glass, `/mcp`, `/metrics.prom`, `/_health`,
  `/_ping`.

A CRM user (owner, assignee, quota holder, team member) is named
`person:<sign-in subject>` (CRM-D18). `worker:<id>` is still accepted on
input and is stored as `person:<id>`.

## Configuration

- **Auth** enforcement is **off** by default. `CRM_REQUIRE_AUTH` is the
  family activation gate. The optional `keycloak` cargo feature also
  accepts Keycloak access tokens, configured by `CRM_KEYCLOAK_URL`,
  `CRM_KEYCLOAK_REALM`, `CRM_KEYCLOAK_AUDIENCES` and
  `CRM_KEYCLOAK_ROLE_MAP` or `CRM_KEYCLOAK_ROLE_MAP_FILE`.
- **Upstream** lookups and the outbound bridges are in **stub mode**
  unless `CRM_UPSTREAM_MODE=http`.
- **Events** use the in-memory transport unless
  `CRM_EVENT_TRANSPORT=outbox`.
- **Email** is simulated unless `CRM_MAIL_MODE` says otherwise; see
  [INSTALL.md](../INSTALL.md#sending-email).
- **Bridges:** `CRM_PPM_SERVICE_URL`, `CRM_PPM_SERVICE_TOKEN`,
  `CRM_WORKER_SERVICE_URL`, `CRM_WORKER_SERVICE_TOKEN`,
  `CRM_WPM_ORGANIZATION_REF` and `CRM_WPM_WEBHOOK_SECRET`; see
  [INSTALL.md](../INSTALL.md#family-bridges).
