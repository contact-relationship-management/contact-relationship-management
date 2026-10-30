# Architecture

```text
 rep / manager / marketing / support browser
        │  (cookie session; no token in JS)
        ▼
 contact-relationship-management-ui-with-svelte  (SvelteKit BFF)
        │  Authorization: Bearer v4.public.…
        ▼
 contact-relationship-management-service-with-rust  (Loco: Axum + SeaORM + PostgreSQL)
        │  EntityRef lookups (read-only, cached, stub-able); the family
        │  bridges (CRM-R72, CRM-R73, CRM-R94), stub-first
        ▼
 person / organization / worker (WPM) / PPM / authentication services
        │  WPM posts signed hire and leave events back
        ▼  (POST /api/bridges/wpm/events, CRM-R92)
```

## Service edition

Loco-idiomatic layout (the patient-flow / WPM shape):

```text
src/
├── app.rs                loco Hooks (+ background workers: campaign
│                         send, the notification sweep; the nurture
│                         and SLA sweeps are POST routes)
├── controllers/          relationships (contacts/accounts/activities),
│                         sales (leads/pipelines/deals/forecast),
│                         marketing (campaigns/segments/nurture/consent),
│                         support (tickets/sla/articles),
│                         dashboards, audits, docs, metrics
├── models/               helpers + _entities/ (SeaORM)
├── clients.rs            stub-first upstream display-name lookups and
│                         the PPM and WPM bridge calls
├── rules/                pure core: lifecycle machines (lead, deal,
│                         campaign, ticket, article), lead scoring
│                         + breakdown, forecast arithmetic, ROI,
│                         CLV, SLA deadline/breach derivation,
│                         segment filter evaluation
├── auth/                 offline PASETO + ABAC + personas + mask
│   ├── mod.rs            the module guide (key source, enforcement, ABAC)
│   ├── verifier.rs       the verifier, key fetch and rotation refresh
│   ├── policy.rs         blanket enforcement, the action, the policy and its reload
│   ├── records.rs        per-record attributes, masking, break-glass
│   ├── extract.rs        the AuthUser and MaybeAuthUser extractors
│   └── tests.rs          the unit tests
├── streaming.rs          envelope + memory/outbox transports
├── validation.rs         caps + URN shapes + money → 422
└── openapi.rs            OpenAPI 3 doc
migration/                sea-orm-migration (crate root)
```

Key decisions (numbered in [design.md](design.md)): normalized
relational schema; every lifecycle and every KPI derivation in the
DB-free pure core; minor-unit money, per-currency reporting;
consent gating in the send path, not the UI; background jobs for
campaign sends and the notification sweep, and `POST` sweep routes for
nurture and SLA (no external broker); ETag-conditional
dashboards; family fixtures (`#![forbid(unsafe_code)]`,
clippy-pedantic, OTLP, `Accepts-version`, Podman). **API versioning is the
`Accepts-version` header on the one `/api` path; never `/api/v1`, `/api/v2`
(CRM-R17).** **All-plural
table names** (the loco `create_table` pluralization lesson);
`404` mapping at `find_by_pid` call sites; enforcement tests
(`tests/requests/enforcement.rs`) that set the settings for their own length.

## Front-end edition

SvelteKit 3 + Svelte 5 runes SPA + same-origin BFF proxy,
eight-locale i18n from the start. Views per module: contact/account
timeline, lead queue (score-sorted with breakdown), deal Kanban
board, forecast table, campaign funnel + ROI, nurture editor,
ticket queue with SLA countdowns, KB editor, and the dashboards.
