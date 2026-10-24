# Tasks — delivery checklist

Status legend: `[x]` done · `[~]` in progress · `[ ]` not started.
Every task traces to design (CRM-D*) and requirement (CRM-R*) ids.
Three-part rule applies: a behavioural change lands as spec edit +
code + tests in one PR.

## Phase 0 — specification

- [x] CRM-T0 Cross-cutting spec round: topic files + SDD trio, both
  edition doc scaffolds, root AGENTS.md wiring. (all CRM-D*, CRM-R*)
  — landed 2026-07-18. No code.

## Phase 1 — service skeleton & relationship layer (CRM-R1, CRM-R2, CRM-R17)

- [x] CRM-T1 Scaffold `contact-relationship-management-service-with-rust`:
  loco app, config, migration crate, family fixtures (forbid-unsafe,
  tracing/OTLP, `/metrics.prom`, OpenAPI + Swagger, `Accepts-version`
  middleware, health routes). (CRM-D12)
- [x] CRM-T2 Contact + Account + Activity migrations/models/CRUD:
  URN validation, ownership, soft delete, the merged timeline read,
  the manual repoint endpoint, audit + event seam
  (`CRM_EVENT_TRANSPORT=memory`). (CRM-D1, CRM-D2, CRM-D9; CRM-R1,
  CRM-R2, CRM-R16)
- [x] CRM-T3 Upstream client seam: person / organization / worker
  traits + `http` + `stub`, config-selected; display-name cache;
  stub-mode boot test. (CRM-D11)
- [x] CRM-T4 Seed task: synthetic book of business (~50 contacts,
  ~15 accounts, 2 pipelines, ~30 deals, leads, a campaign, ~20
  tickets) — synthetic data only. (CRM-R17)

## Phase 2 — sales automation (CRM-R3–R5)

- [x] CRM-T5 Lead lifecycle + pure-core scoring with per-rule
  breakdown + hot/warm labels + score-sorted queue; conversion
  (contact + optional deal) in one transaction. (CRM-D3, CRM-D5,
  CRM-D9; CRM-R3)
- [x] CRM-T6 Pipeline + PipelineStage + Deal: stage-move validation,
  Kanban ordering with `FOR UPDATE` serialization, terminal
  close/lost-reason/reasoned-reopen, stalled derivation. (CRM-D3,
  CRM-D7, CRM-D9; CRM-R4)
- [x] CRM-T7 Forecast: pure-core stage-weighted arithmetic,
  per-currency grouping, overflow refusal; month-end
  ForecastSnapshot job. (CRM-D4, CRM-D7, CRM-D8; CRM-R5)

## Phase 3 — marketing automation (CRM-R6–R9)

- [x] CRM-T8 Consent: `marketing_consent` + append-only ConsentEvent
  history + unsubscribe cascade (exit nurture, block sends);
  consent-history read audit. (CRM-D6; CRM-R6, CRM-R16)
- [x] CRM-T9 Segments: declarative filter model + pure-core
  evaluation with the structural consent AND-gate + preview
  (count + sample). (CRM-D6; CRM-R7)
- [x] CRM-T10 Campaigns: lifecycle machine, simulated send
  (inline in the request through `mail::Sender`; see CRM-T39), touch activities + engagement
  counters, funnel + ROI derivation (zero-cost `null`). (CRM-D3,
  CRM-D4, CRM-D7, CRM-D8; CRM-R8)
- [x] CRM-T11 Nurture: sequence/step/enrollment models + the
  idempotent scheduler job (advance due steps, complete, exit
  rules). (CRM-D8; CRM-R9)

## Phase 4 — service & support (CRM-R10–R12)

- [x] CRM-T12 Tickets: lifecycle machine, assignment,
  first-response stamping from assignee activities. (CRM-D3;
  CRM-R10)
- [x] CRM-T13 SLA: policy model, pure-core deadline derivation +
  re-derivation on audited priority change, breach computation on
  read + the once-per-breach sweep job. (CRM-D4, CRM-D8; CRM-R11)
- [x] CRM-T14 Knowledge base: article lifecycle + versioning on
  published edits + ILIKE search + ticket-link activity. (CRM-D3;
  CRM-R12)

## Phase 5 — analytics (CRM-R13, CRM-R14)

- [x] CRM-T15 Dashboards: win rate, pipeline by stage, SLA health,
  CLV, activity feed + per-rep counts — pure-core derivations,
  ETag-conditional, `as_of`, per-currency, honest ratios. (CRM-D4,
  CRM-D7, CRM-D12; CRM-R13, CRM-R14)

## Phase 6 — auth activation surface (CRM-R15, CRM-R16)

- [x] CRM-T16 `auth.rs`: offline PASETO verify + blanket
  `CRM_REQUIRE_AUTH` guard (guard-all / deny-unless-public) + ABAC +
  record-level `resource.owner`/`status`/`tier` attrs + `$sub`
  ownership + `mask` obligation on amounts/forecast/ROI/channels;
  sensitive-read audit wiring; persona test matrix in its own
  enforcement binary. (CRM-D10, CRM-D12)

> Phases 1–6 landed 2026-07-18 in one implementation round
> (`contact-relationship-management-service-with-rust`, copy-adapted
> from the WPM service): 6 migrations (17 domain tables + audit +
> outbox), pure `rules/` core (lifecycle tables for lead / campaign /
> ticket / article, deterministic scoring with per-rule breakdown +
> linear recency decay, forecast/ROI/CLV/win-rate arithmetic with
> per-currency honesty + null-on-zero-denominator ratios, SLA
> deadline + breach derivation, segment evaluation with the
> structural consent AND-gate), five module controllers, `auth.rs`
> with `resource.owner` `$sub` ownership + `mask_json` amount
> nulling, ETag-conditional dashboards. 62 DB-free unit tests, 5
> request tests (sales journey incl. forecast-follows-stage +
> closed-immutability + reasoned reopen, 404/pipeline-membership
> pins, consent-gated marketing journey incl. withdraw-exits-nurture
> + send-time re-check, ticket SLA journey incl. assignee-only
> first-response + priority re-derivation + sweep idempotency, KB
> versioning), 1 enforcement binary — all green against Postgres 18;
> clippy-pedantic clean; live smoke verified (seed → forecast → ETag
> 304 → win rate 2/4 → OpenAPI 44 paths).
> Notes: the nurture advance + SLA sweep run via POST endpoints in
> v1 (the `bg_pg` periodic worker is the documented roadmap seam);
> deal stages are pipeline data, not a token table.

## Phase 7 — front-end (all CRM-R*)

- [x] CRM-T17 Scaffold
  `contact-relationship-management-ui-with-svelte`:
  SvelteKit 2 + Svelte 5 runes SPA, BFF proxy + session flow,
  13-locale i18n from the start, typed API client + `money()`.
  (CRM-D12)
- [x] CRM-T18 Views: contact/account timeline, lead queue with
  score breakdown, deal Kanban, forecast table, campaign funnel +
  ROI, nurture editor, ticket queue with SLA countdowns, KB editor,
  dashboards; vitest + `page.route`-stubbed Playwright. (CRM-D10,
  CRM-D12)

> CRM-T17/T18 landed 2026-07-18: SvelteKit 2 + Svelte 5 runes SPA
> (copy-adapted from the WPM front-end: BFF proxy + session seam)
> with a 45-key × 13-locale i18n module (parity-tested, RTL ar/ur),
> typed CRM client + honest `money()`, and views: KPI dashboard
> (win rate with numerator/denominator, forecast per currency),
> contacts + consent actions + timeline, score-sorted lead queue
> with expandable per-rule breakdown, deal board (stage columns +
> forward moves + forecast strip), campaigns with run-simulated +
> funnel/ROI, ticket queue with live breach flags + status moves,
> KB list + publish + search. svelte-check 0 errors; 5 vitest +
> 4 Playwright (page.route-stubbed, unstubbed = 404-loud) green.

## Production gates (before any non-demo exposure)

- [~] CRM-G1 Activate `CRM_REQUIRE_AUTH` + mount a real ABAC
  policy; verify the persona matrix against the deployment's
  attributes. **Code side landed as CRM-T22 (2026-08-28)** — the
  shipped reference policy and the activation runbook (spec
  `auth.md`), including an honest statement of the engine's current
  wiring limits. The remaining act — setting the flag and attributes
  on a real deployment, and extending the enforcement matrix beyond
  the shipped reader/writer pins — is operational by design.
- [~] CRM-G2 GDPR/PECR review of the real send path (ESP adapter,
  lawful basis, unsubscribe in-message), retention schedules,
  subject-access/erasure flows ([regulatory.md](regulatory.md)).
  **Subject-access/erasure/retention code side landed as CRM-T21
  (2026-08-28)**; the remaining items — the ESP-adapter/send-path
  legal review, jurisdiction-correct data residency, and coordination
  of subject rights with the upstream identity services — are
  operational/legal work, not code.

- [x] CRM-T19 (2026-07-20) **Insight views + boards.** (CRM-R18;
  design CRM-D4, CRM-D12 — CRM-R18 was backfilled during the DOC-7
  audit pass, since this task shipped without a requirement id at
  the time). Service: seven
  read-only derived views in `controllers/insights.rs` (`as_of` +
  ETag via the dashboards helpers): `/api/insights/stale-deals`
  (days-in-stage from `deal_stage_changed` audits, derivation
  served), `/insights/followups` (open activities with `due_on`:
  overdue aging + next 30 days; recorder disclosed as recorder),
  `/insights/pipeline-hygiene` (rule-disclosed findings: no amount /
  no expected close / past expected close / no recent activity /
  unworked leads), `/insights/executive` (period pack: won/lost with
  per-currency won value never merged, lost reasons verbatim, leads /
  tickets / activities / campaigns-started / consent withdrawals),
  `/insights/forecast-trends` (stored snapshots only, no
  interpolation), `/insights/sla` (breach register + per-assignee
  workload, 4h at-risk window disclosed), `/insights/dpo` (consent
  coverage verbatim + withdrawals + per-source counts +
  duplicate-contact hygiene over shared `person_ref`; identity dedup
  stays upstream). Front-end: `/leads/board` + `/tickets/board`
  (drag = the existing status transitions; lifecycle machine owns
  legality), `/followups` (overdue table + SVAR Calendar),
  `/executive`, `/dpo`; `leadStatus` client fn; nav + i18n keys ×13.
  **Acceptance:** the seeded insight round-trip (incl. ETag 304)
  green — full `--ignored` suite 7/7 vs Postgres 18; clippy pedantic
  clean; svelte-check 0; vitest 5; Playwright 9.

- [x] CRM-T20 (2026-07-20) **Engagement / partnership / confederation
  round.** (CRM-R19; design CRM-D13 — both backfilled during the
  DOC-7 audit pass, same reason as CRM-T19 above). Migration `m20260720_000007_engagement`: declared
  stakeholder typing (`contacts.stakeholder_role` + power–interest
  1–5, `accounts.stakeholder_role` — all nullable; undeclared stays
  undeclared), recorded `activities.sentiment`
  (positive/neutral/negative; validated, never inferred), and the
  `partnerships` (forward-only scouting→pilot→scaled + retire
  lifecycle in `rules::engagement`), `memberships` (one per account;
  active/lapsed + renewal_on), `working_groups` (+ roster) tables.
  New declared-data endpoints in `controllers/engagement.rs`; nine
  derived views join `controllers/insights.rs`: cadence (untouched
  contacts/accounts + no-next-touch), engagement workload (kinds +
  recorded sentiment), pipeline funnel (entered per stage from
  `to_stage` audits; honest ratios), member health (+ silent list),
  consent-by-account, the stakeholder register + grid (declared
  scores only), the partnership register, membership renewals; the
  follow-ups view gains a `kind` filter (the renewals convention).
  Front-end: `/engagement` + `/partners` pages, deal-board pipeline
  selector + funnel strip, follow-ups kind filter, DPO
  consent-by-account. **Acceptance:** lifecycle/grid pure pins; the
  seeded engagement round-trip green first run — full `--ignored`
  suite 8/8 vs Postgres 18; clippy pedantic clean; svelte-check 0;
  vitest 5; Playwright 12.

- [x] CRM-T21 (2026-08-28) **Subject rights & retention (the code
  side of CRM-G2).** `rules/privacy.rs` (pure): `erasable` (no open
  deal naming the contact primary contact, no open support ticket, no
  active nurture enrolment — deliberately **not** gated on
  `Contact::status`, which no endpoint ever transitions; CRM-D14), the
  floored retention horizon (`CRM_RETENTION_DAYS`, default 365, floor
  30), and the 19-table sweep list (sorted, deduped, pinned; excludes
  the append-only `consent_events` and the plain roster join
  `working_group_members`, neither of which has a `deleted_at`
  column). `controllers/privacy.rs`:
  `GET /api/contacts/{pid}/subject-access` (one audited JSON document
  across every table keyed to the contact — consent history,
  activities, leads, deals as primary contact, tickets, nurture
  enrolments — with exclusions named: campaign counters are aggregate
  simulated data with no per-recipient log, the account is a separate
  subject, upstream identity records are the deployment's coordination
  duty; refused `403` to a masked caller, since a masked export
  contradicts its own purpose); `POST …/erase` (anonymise per CRM-D14:
  identity fields scrubbed + tombstone `person:` URN, `marketing_consent`
  set `withdrawn`, row soft-deleted, linked activity summaries and the
  lead record's name/email scrubbed, working-group roster entries
  removed — deals/tickets/consent history remain, since CRM has no
  monetary field on Contact itself the way WPM's `salary_minor` needed
  clearing; refused `422` on an open deal/ticket/active nurture
  enrolment; audited with counts); `GET /api/retention` +
  `POST /api/retention/sweep` (hard-delete past-horizon soft-deletes
  across the 19-table list; the report additionally discloses, but
  never auto-scrubs, contacts whose consent has stood withdrawn since
  before the horizon — unlike WPM's candidates, a CRM contact always
  carries the `/erase` gate, so the sweep has no ungated bulk-anonymise
  path; audited with counts, including an empty sweep). `/erase` and
  `/sweep` join `DESTRUCTIVE_POST_SUFFIXES` (⇒ `access=admin` or
  `svc=true` under enforcement). **Acceptance:** 3 pure pins (erasable
  matrix, horizon default/floor, sweep-list soundness) + the DB-gated
  `subject_rights_round_trip` (export gathers the footprint + names
  exclusions + audited; erase refused in turn by an open deal, an open
  ticket, and an active nurture enrolment, then anonymises after
  consent withdrawal exits the enrolment, with counts in the audit
  snapshot; report/sweep both audited even at zero) — full `--ignored`
  suite 9/9 vs Postgres 18 (67 unit); clippy pedantic clean; fmt clean.
  Front-end deferred (see CRM-T23 follow-up note below). (CRM-D14;
  CRM-R20, CRM-G2)

- [x] CRM-T22 (2026-08-28) **Auth activation surface (the code side
  of CRM-G1).** Ships `config/abac-policy.reference.json` — the spec
  `auth.md` personas as policy: svc/admin everything;
  `resource.owner = $sub` reads/writes the caller's own record
  unmasked (record-level — reaches only the two CRM-T21 handlers
  today); `manager=true` writes and reads unmasked; `rep=true` writes
  (coarse — CRM has no ownership-enforcing write handler yet, so this
  is a plain grant, not a scoped one); `marketing=true` /
  `support=true` write and read masked; everyone else authenticated
  gets the masked-read fallback — plus the **activation runbook** in
  `auth.md` (mount → keys → flag → verify). Unlike WPM-T31, this task
  states its engine limits are **narrower**, not just "known": the
  `mask` obligation is defined and unit-tested
  (`auth::mask_json`) and is honoured by exactly one consumer
  (`subject_access`'s outright refusal) — no list/get handler yet
  redacts a deal amount, forecast value, campaign ROI, or contact
  channel detail on an ordinary read, so the sensitivity-map masking
  in `auth.md` is a policy contract still being wired, not a control
  already in force everywhere it is described. `DESTRUCTIVE_POST_SUFFIXES`
  extended (`/erase`, `/sweep`) with a `derive_action` pin (incl. the
  SEC-G6 trailing-slash case) for each. **Acceptance:** `cargo test`
  green (67 unit) + the DB-gated `enforcement` suite (1/1) and the
  full `--ignored` suite (9/9) vs Postgres 18; clippy pedantic clean;
  fmt clean. Extending `tests/enforcement.rs` with the full persona
  matrix (manager vs marketing/support masking, `/erase`+`/sweep`
  admin/svc-only, subject-access masked-403) is noted in `auth.md`
  as the next step before relying on this in a real deployment, not
  done here. (CRM-R20, CRM-G1)

> **Follow-up (not done here, CRM-T23):** a front-end "Download my
> data" link + confirm-gated Erase action on the contact detail page,
> matching WPM-T32's shape. Skipped in CRM-T21/T22 to keep this round
> scoped to the service-side code the two gates were actually missing;
> the front-end pages/routes this would touch
> (`contact-relationship-management-ui-with-svelte`) were not
> otherwise touched by this round.

- [x] CRM-T23 **Front-end "Download my data" + Erase action.** *(resolved 2026-10-10: the page and i18n keys already existed; the missing Playwright spec landed — download link, confirm-gated erase, declined confirm, 422 reason.)* The
      follow-up flagged above when CRM-T21/T22 landed: the contact
      detail page has no subject-access download link and no
      confirm-gated erase action, matching WPM-T32's shape (verified:
      `grep -rln "subject-access\|/erase"
      contact-relationship-management-ui-with-svelte/src`
      returns nothing, while the service's
      `GET /api/contacts/{pid}/subject-access` and `POST …/erase`
      have been live since CRM-T21). **Acceptance:** a "Download my
      data" link and an Erase action (confirm-gated, navigating away
      after the record 404s, and surfacing the service's `422` reason
      when an open deal/ticket/active nurture enrolment blocks it) land
      on the contact detail page; a Playwright spec drives both against
      `page.route` stubs; vitest path map extended; svelte-check 0;
      i18n keys ×13. (CRM-D14; CRM-R20, CRM-G2)

- [x] CRM-T24 **Wire record-level ABAC into the contact/deal
      handlers.** *(resolved 2026-09-05.)* `auth::deal_resource_attrs`
      and `auth::contact_resource_attrs` were defined and unit-tested
      in `auth.rs`, but `authorize_record` was called only from
      `controllers/privacy.rs` (the CRM-T21 subject-access/erase
      handlers) — never from `controllers/sales.rs`'s deal read/write
      handlers or `controllers/relationships.rs`'s contact handlers.
  - **Resolved.** There is no single `GET /api/deals/{pid}` endpoint
    (only `list_deals`, `deal_stage`, `reopen_deal` touch one deal) and
    no general contact `PUT` (only `repoint_contact`, which changes
    `person_ref` alone) — the task's literal "GET/PUT" framing didn't
    quite match the routes that exist, so the fix is scoped to the
    handlers that genuinely read or mutate a single record:
    - `relationships.rs::get_contact` now calls `authorize_record`
      (`Read`) and, on a `mask` obligation, redacts `preferred_channel`
      via a new `auth::mask_text_json` (a sibling of the existing
      `auth::mask_json`, which nulls amounts — a redacted **string**
      is replaced with the `auth::MASKED` placeholder instead of
      `null`, since a string has no "looks like zero" honesty problem
      to guard against).
    - `relationships.rs::list_contacts` filters + masks per row via a
      new `readable_contacts` helper — a denied row is **omitted**, the
      same invariant-5 shape landed for care-pathway-service (CP-T2)
      and portfolio-service (SEC-PPM-1) this cycle.
    - `relationships.rs::repoint_contact` calls `authorize_record`
      (`Write`) before mutating.
    - `sales.rs::list_deals` filters + masks per row via a new
      `readable_deals` helper, redacting `amount_minor` (via the
      existing `auth::mask_json`) on a masked row.
    - `sales.rs::deal_stage` and `sales.rs::reopen_deal` call
      `authorize_record` (`Write`) before mutating.
    - **Left explicitly out of scope, not silently:** `get_contact`'s
      bundled `activities`/`deals`/`tickets` rows are not re-authorized
      or masked by this change — each carries its own resource
      attributes a caller might read differently under, and folding
      that in is a separate, larger change than this task's five
      handlers. Forecast value and campaign ROI (also named in CRM-T22)
      are computed in the `forecast`/analytics endpoints, not stored
      fields on `Deal`, and are likewise untouched here.
  - **Acceptance (met).** Two new unit tests in `auth.rs`
    (`mask_json_nulls_amounts_keeps_structure`'s sibling
    `mask_text_json_redacts_text_keeps_structure`, pinning the
    placeholder-vs-null distinction). `tests/enforcement.rs`'s single
    boot gained a third policy rule (`resource.owner: ["$sub"]` reads
    unmasked) and the existing masked-catch-all read rule gained
    `"obligations": ["mask"]`, then a new scenario: a contact and a
    deal are each created with `owner_ref` pointing at a freshly-minted
    "owner" persona's own `sub`; that owner's `GET /api/contacts/{pid}`
    shows the real `preferred_channel` and their
    `GET /api/deals?pipeline=` shows the real `amount_minor`, while the
    existing unrelated `reader` persona (whose `sub` matches no
    `resource.owner`) sees the same two records with
    `preferred_channel` == `auth::MASKED` and `amount_minor` == `null`
    respectively — proving both that ownership changes the outcome and
    that an unowned/other-owned record still discloses nothing extra.
    `cargo test --lib` (69, up from 68) and the DB-gated
    `cargo test --test enforcement -- --ignored` plus the full
    `--ignored` suite all green; `cargo fmt --check` and `cargo clippy
    --all-targets -- -D warnings` clean. (CRM-D10, CRM-D12; CRM-G1)

- [x] CRM-T25 **`require_ref` EntityType coverage.** *(resolved
      2026-09-05.)* Same gap as its
      WPM sibling: `controllers/sales.rs`, `support.rs`, and
      `relationships.rs` pass `EntityType::Worker`,
      `::Organization`, and `::Person` to the shared `require_ref`
      helper, but `validation.rs`'s own `ref_rules` test only ever
      passes `EntityType::Person` (verified: `grep -n "EntityType::"
      src/validation.rs` vs `grep -rn "entity_ref::EntityType::"
      src/controllers/*.rs`). **Acceptance:** `ref_rules` exercises
      the wrong-type branch for at least `Worker` and `Organization`
      too; `cargo test` green; clippy pedantic clean. (CRM-D11)
      - **Resolved.** New `ref_rules_wrong_type_worker_and_organization`
        test in `src/validation.rs` exercises both the wrong-type
        rejection (a `person:` ref where `Worker`/`Organization` is
        expected) and the matching-type acceptance for each, alongside
        the pre-existing `Person`-only `ref_rules` test.

- [x] CRM-T26 **Front-end sign-in gate.** No `+layout.server.ts`
      exists and `hooks.server.ts` only populates `locals.sessionId`
      without redirecting (verified: `find src/routes -iname
      "+layout.server.ts"` returns nothing); every deal/ticket/
      dashboard page is reachable signed-out and only fails silently
      at the API layer — the same page-visit auth-guard gap the repo
      root `tasks.md` WEB-1 finding names. **Acceptance:** a root or
      per-protected-route guard redirects a session-less visitor to
      sign-in (excluding the public sign-in/verify routes); a
      Playwright spec pins it; svelte-check 0. (CRM-D12)
      **Resolution (2026-09-05):** ported WPM-T38's identical fix
      (same task shape, same underlying architecture — same session
      cookie, same SPA `ssr = false` mode, same `hooks.server.ts`
      shape). New root `src/routes/+layout.server.ts` redirects to
      `/signin` (303) when `locals.sessionId` is `null`, excluding
      `/signin`/`/verify`. Root gate chosen over a narrower
      per-mutation-page guard for the same reason as WPM: this app's
      pages (deal board, ticket queue, executive/dpo dashboards) mix
      read content with embedded actions rather than separating reads
      and writes onto dedicated routes. `tests/e2e/smoke.spec.ts`
      gained the same `signIn()` cookie-injection helper and a
      `"sign-in gate (CRM-T26)"` describe proving the redirect; all 12
      pre-existing tests moved under a `"signed-in smoke coverage"`
      describe whose `beforeEach` now signs in first. Verified: `npm
      run check` (svelte-check: 425 files, 0 errors, 0 warnings), `npx
      playwright test` (15 passed), `npx vitest run` (5 passed,
      unchanged).

- [x] CRM-T27 **Front-end test coverage for honesty rendering.** *(resolved 2026-10-10: `src/lib/format.ts` + `tests/unit/format.test.ts`.)*
      `tests/unit/crm.test.ts` is the only vitest file, covering
      `money()`, i18n parity, and the API path map only (verified:
      `find tests src -iname "*.test.ts"` returns exactly one file);
      none of the win-rate/forecast/ROI null-ratio rendering, the deal
      board's stage math, or the SLA countdown/breach-flag logic that
      the deal board, forecast table, and ticket queue views render
      client-side is unit-tested, and `src/lib` has no extracted
      formatting module the way the CMS front-end's `$lib/format`
      provides. **Acceptance:** the null-ratio/masked-amount rendering
      and stage/breach helpers used by the deal board, forecast table,
      and ticket queue are extracted into a testable `$lib` module
      with a vitest suite (zero-denominator, masked, breach-boundary
      cases); svelte-check 0; existing Playwright suite stays green.
      (CRM-D10)

- [x] CRM-T28 **Fix: `/followups`'s calendar never actually showed an
      upcoming follow-up event.** *(resolved 2026-09-06.)*
      `@svar-ui/calendar-store` requires an all-day event's `end` to be
      strictly after `start`, but `+page.svelte` passed `end: day` —
      the same `Date` object as `start` — so the SVAR Calendar widget
      silently dropped every upcoming follow-up it was ever asked to
      show. The existing test only asserted the calendar's wrapper
      `<div>` was visible, not that an event actually rendered inside
      it, so the bug went uncaught. The identical bug and fix already
      landed in worker-front-end's/person-front-end's `/expiry`
      calendars and ppm-front-end's `/calendar`. Fixed by computing
      the following calendar day as `end`; strengthened the test to
      assert the calendar renders the event text, with the stubbed
      follow-up's due date computed relative to the actual test-run
      date (a fixed date would eventually scroll outside the widget's
      default "today's month" view). Verified the strengthened
      assertion actually fails without the fix before confirming it
      passes with it.

- [x] CRM-T29 **KC-1: optional Keycloak bearer acceptance; PASETO as a
      feature (2026-09-29).** `paseto` (default) and `keycloak` (off)
      cargo features; `CRM_KEYCLOAK_*` configuration; `auth::request_claims`
      / `auth::enforce_request`; `tests/keycloak_guard.rs` (DB-free).
      Verifier side: `authentication-verifier` KC-1; reference:
      organization-service.

## Production readiness for medical and governmental organizations

Derived from [plan.md](plan.md) (2026-10-10). Phase 1–2 rows are implemented
in the same round; phase 3–4 rows stay open because they need decisions or
parties outside the repository.

- [x] CRM-T30 *(resolved 2026-10-10.)* **Fail-closed production boot.** In the `production`
      environment the service refuses to start unless `CRM_REQUIRE_AUTH` is
      on and a verifier is configured, unless `CRM_ALLOW_INSECURE=1` is set
      explicitly (which logs an error). **Acceptance:** pure pins for the
      decision function; clippy pedantic clean. (plan gap 1)
- [x] CRM-T31 *(resolved 2026-10-10.)* **Tamper-evident audit trail.** `audit_logs` gains
      `prev_hash` and `hash` (SHA-256 over the previous hash and the row's
      canonical fields); `GET /api/audits/verify` recomputes the chain and
      reports the first break. **Acceptance:** pure chain pins (valid,
      altered row, deleted row, reordered); migration reversible; DB test
      appends and verifies. (plan gap 2)
- [x] CRM-T32 *(resolved 2026-10-10.)* **Finish `mask` coverage.** Apply the obligation to forecast
      values, campaign cost/ROI, and the activities/deals/tickets bundled in
      `get_contact`; extend `tests/enforcement.rs` with the full persona
      matrix. (plan gap 3; auth.md known limits)
      **Resolution:** `auth::aggregate_masked` decides a collection-level read with no `resource.*` attributes, so only rules that need no record (service, admin, manager in the reference policy) read unmasked. Applied to `GET /api/forecast` (`totals_minor`), `GET /api/dashboards/sales` (per-stage `totals_minor`), `GET /api/campaigns` (`cost_minor`), the campaign funnel (revenue, ROI, cost), and the activities, deals and tickets bundled in `get_contact` (`summary`, `amount_minor`, `description`). Masked amounts are `null`, never zero; counts stay. The UI treats null totals, revenue and ROI as dashes (Playwright pin). `tests/enforcement.rs` proves a masked reader gets null forecast and dashboard totals while a service credential gets the numbers. **Follow-up (2026-10-10):** the executive pack, forecast trends, stale-deals (per deal, owner sees own unmasked) and the forecast-snapshot response are masked too, pinned in `tests/enforcement.rs`; `reference_policy_persona_matrix` (unit) evaluates the shipped reference policy for service, admin, manager, rep, marketing, support, plain reader, record owner and break-glass across read, write and destructive. **Still not done:** the other `/api/insights/*` views carry no amounts so are unchanged; an end-to-end persona run through HTTP for manager, marketing and support (the unit matrix covers the policy, not the wiring per persona).

- [x] CRM-T33 *(resolved 2026-10-10.)* **Security headers and CORS.** Service: secure-headers
      middleware and a deny-by-default CORS origin list in production
      config. UI: CSP, `X-Content-Type-Options`, `Referrer-Policy`,
      `Permissions-Policy`, frame denial via `hooks.server.ts`.
      **Acceptance:** vitest pin for the header set; Playwright asserts them
      on a page response. (plan gap 4)
- [x] CRM-T34 *(resolved 2026-10-10.)* **Supply-chain gates.** `scripts/ci-check.sh supply-chain`
      runs `cargo audit`, `cargo deny check`, `pnpm audit --prod`, and
      writes CycloneDX SBOMs; wired into both CI providers. (plan gap 5)
- [x] CRM-T35 *(resolved 2026-10-10.)* **Hardened container images.** Multi-stage Containerfiles for
      the service and UI: non-root user, minimal runtime image, no shell
      tools, healthcheck, read-only-rootfs-compatible. (plan gap 6)
      **Resolution:** `Containerfile`s for the service (distroless `cc`, non-root, release build with `keycloak`) and the UI (distroless Node 24, `adapter-node`). Both built and run with `--read-only --cap-drop ALL`: the UI served the security headers and CSP, the service booted in production mode and served HSTS, both as `nonroot`. Not done: image digests are not pinned (do that in your registry mirror), and no `HEALTHCHECK` is set (distroless has no shell or curl; use the orchestrator probe on `/_health`). The build warns about `ENV CRM_REQUIRE_AUTH`; that is a flag, not a secret.
- [x] CRM-T36 *(resolved 2026-10-10.)* **Compliance evidence pack.** `spec/compliance/`: control
      mapping (NIST SP 800-53 rev 5, HIPAA Security Rule, ISO 27001 Annex A,
      GDPR Art. 32) to implemented controls and gaps; DPIA template;
      operations runbook (backup/restore, encryption at rest, key
      management, incident response, log retention). (plan gap 7)
- [x] CRM-T37 *(resolved 2026-10-10.)* **Accessibility gate.** axe-core scan inside Playwright over
      the signed-in routes with WCAG 2.2 AA tags; known violations either
      fixed or listed with a justification. (plan gap 8)
- [x] CRM-T38 *(resolved 2026-10-10.)* **Admin-only operational reads.** `/api/audits*`,
      `/api/events/recent`, `/api/retention` classified above the plain read
      gate; `/metrics.prom` and OpenAPI optionally authenticated. (plan gap 9)
     
      **Resolution:** `PRIVILEGED_READ_PREFIXES` (`/api/audits`, `/api/events`, `/api/retention`) derive the admin/service tier for `GET`, matched on whole path segments; 2 unit pins; enforcement test shows reader and writer get 403 and a service credential 200 on four endpoints. `CRM_PROTECT_OPERATIONAL=1` additionally puts `/metrics.prom` and the OpenAPI/Swagger paths behind authentication (off by default so an in-network scraper keeps working). **Note:** the tier reuses the engine's `destructive` action, because the shared engine has no privileged-read action; policies must grant `destructive` to whoever may read audits.

- [ ] CRM-T39 **Real ESP adapter** with
      bounce/complaint webhooks, suppression list, unsubscribe link, and the
      processor agreement and lawful-basis review (CRM-G2). (plan gap 10)
      *Open: legal and vendor dependency.*
      **Built (2026-10-11):** the send seam and an SMTP sender. `mail::Sender`
      (simulated by default; real SMTP with `CRM_MAIL_MODE=smtp` and
      `CRM_SMTP_*`; a recording variant for tests). `email_deliveries` holds one
      row per (campaign, contact): an atomic claim means a re-run or a
      concurrent run never mails a contact twice, consent is re-read from the
      live contact immediately before each send, a transient failure is retried
      (5 attempts) and a permanent one is not, and a campaign completes only
      when nothing is left to retry. Sends are batched (`CRM_MAIL_BATCH`, 200); a
      background job walks the batches (below). Campaigns gain `subject` and
      `body`. `GET /api/campaigns/{pid}/deliveries` reports progress; subject
      access lists a contact's deliveries (state, time, message id; no address or
      body). Addresses are not stored: they come from the person service
      (`email` field, `CRM_UPSTREAM_MODE=http`), a primed value in tests, or
      `<id>@$CRM_MAIL_DEV_DOMAIN` against the local Stalwart server
      (`scripts/email-dev.sh`). Production refuses `smtp` mode unless
      `CRM_MAIL_ALLOW_PRODUCTION=1`, and always refuses an unchecked certificate
      or no TLS. Tests: recorder-based request tests (send once, retry without
      double mail, withdrawn consent, no address, body required, disjoint claims;
      one mutation was checked to fail), config unit tests, a live test against
      Stalwart (`scripts/email-dev.sh live`), and a manual end-to-end run of the
      real service sending two messages that arrived with our `Message-ID`s.
      **Unsubscribe (built 2026-10-11):** every real message ends with a signed
      link (HMAC-SHA256 over contact and campaign, no expiry) and carries
      `List-Unsubscribe` and `List-Unsubscribe-Post` (RFC 8058). `GET
      /unsubscribe/{token}` changes nothing; `POST` withdraws consent through
      the ordinary audited consent change and is idempotent; forged links get
      404. `CRM_UNSUBSCRIBE_SECRET` and `CRM_PUBLIC_URL` are required for `smtp`
      mode. Request test covers scanner GET, one-click POST, repeat, forged
      token, and no further mail to the contact.
      **Webhooks and suppression (built 2026-10-11):** `POST /webhooks/email`,
      authenticated by an HMAC-SHA256 signature with a five-minute window
      (`CRM_WEBHOOK_SECRET`), takes a generic outcome (`message_id`, `event` of
      `delivered`, `bounce_hard`, `bounce_soft`, `complaint`). Each
      (message, event) applies once. `delivered` counts on the campaign; a hard
      bounce or complaint puts the contact on `email_suppressions` (the send
      path skips them; audited) and a complaint also withdraws consent. The CRM
      stores no addresses, so suppression is per contact, not per address; a
      changed address in the person service stays suppressed until cleared in
      the database (no clearing endpoint yet). Vendor payloads are read by the
      adapters below.
      `GET` and `DELETE /api/contacts/{pid}/email-suppression` read and lift a
      suppression (both audited; lifting does not restore consent).
      **Background job (built 2026-10-12):** with a real sender `run` enqueues
      `CampaignSendWorker` (PostgreSQL queue; loops batches until nothing more
      can be sent; stops when a pass sends nothing, so a stalled campaign stays
      `running` and a later `run` resumes it) and answers `202` until complete.
      Tested in foreground mode, and by hand with a real `start
      --server-and-worker` process sending three messages through Stalwart.
      There is no timer that retries a stalled campaign on its own.
      **Nurture (built 2026-10-12):** steps gain `subject` and `body`; with a
      real sender `POST /api/nurture/advance` mails each step (consent and
      suppression first, one delivery row per step and contact, same unsubscribe
      link), holds an enrolment on a transient failure or missing address
      (`held`), and exits it on a permanent refusal. The sender is called
      inside the enrolment's transaction because the pool can be one
      connection. Tested with the recorder. The sweep is still a request, not a
      job, and nothing triggers it on a timer.
      **Person-service address:** the accepted shapes are in
      `spec/integrations.md`; still unconfirmed against the real service.
      **Brevo and Mailjet (built 2026-10-12):** `CRM_MAIL_MODE=brevo|mailjet`
      sends over the vendor's HTTP API with our id attached (Brevo tag, Mailjet
      `CustomID`) so events match; `POST /webhooks/email/{brevo,mailjet}` reads
      their callbacks (bearer, basic-auth or `X-CRM-Webhook-Token` carrying
      `CRM_WEBHOOK_SECRET`; no timestamp, replays harmless). Built from the
      vendors' documentation; request bodies, answer classification and event
      mapping are unit-tested and the callbacks have a database test, but
      **nothing has run against a live account**, and whether the vendors keep
      our `List-Unsubscribe` custom headers is unconfirmed.
      **SES, Azure, Mailgun, SendGrid (built 2026-10-13):** same pattern
      (`CRM_MAIL_MODE=ses|azure|mailgun|sendgrid`). SES: Signature V4 signer
      matched to AWS's test vector, tag-carried id, events via SNS (subscription
      auto-confirmed for AWS hosts; SNS signature not verified). Azure:
      access-key HMAC, operation id kept in `email_deliveries.provider_id` and
      used to match Event Grid reports. Mailgun: webhook HMAC verified.
      SendGrid: ECDSA event signature not verified. Database test covers all
      four callbacks. **No live account was used.**
      **Still open:** `opened` and `clicked` events (so those counters stay 0 on
      a real send), a timer for stalled campaigns and the nurture sweep, a live
      trial with each vendor, the address contract with the person service (the
      `email` field is assumed and unverified), and the vendor, agreement and
      legal review.
      The brief for a vendor and legal is `spec/compliance/email-provider-brief.md`;
      the local server is `scripts/email-dev.sh` (`INSTALL.md`).
- [x] CRM-T40 *(resolved 2026-10-10.)* **Audit personal-data reads.** Contact, lead, ticket and
      activity reads write an audit row (actor, record, masked or not);
      bulk list reads write one summarizing row. (plan gap 11)
      **Resolution:** `Audit::record_read` writes a read audit row in its own short transaction (it joins the hash chain). Audited: single reads of contacts (with `masked` flag), leads and tickets; list reads of contacts, leads, tickets and activities as one summary row with the returned count. A failed audit write fails the read. DB test `personal_data_reads_are_audited`. **Later:** account reads (T47), deal list reads and all insights views (`deal_list_read`, `insight_read` via a middleware that fails closed; DB test `deal_and_insight_reads_are_audited`) are now audited too. The bundled UI does not poll, but any client that does will grow the audit table quickly; size retention for it.

- [x] CRM-T41 *(resolved 2026-10-10, local keys.)* **Field-level encryption and KMS.** Envelope encryption for
      contact channel details and activity content, key rotation, KMS
      adapter. (plan gap 12) *Open.*
- [ ] CRM-T42 **Independent assurance.** Penetration test, formal risk
      analysis, accreditation evidence (ATO / DSPT / HIPAA / ISO 27001).
      (plan gap 13) *Open: external.* **Brief written 2026-10-18:** `compliance/penetration-test-scope.md` is what to hand a tester. No test has been done.
- [x] CRM-T43 *(resolved 2026-10-10, in part.)* **Abuse resistance.** Rate limiting and lockout on the
      service; UI idle-session timeout and re-authentication before
      destructive actions. (plan gap 14) *Open.*
      **Resolution:** Service: `src/rate_limit.rs` (8 pins) and an outermost abuse-guard middleware: a per-client fixed-window request limit (default 600 a minute) and a lockout after repeated `401`s (default 10 in 5 minutes), answering `429` with `Retry-After`. On by default only in production; `CRM_RATE_LIMIT_PER_MIN`, `CRM_AUTH_FAIL_LIMIT`, `CRM_AUTH_FAIL_WINDOW_SECS` override and `0` disables. Probes (`/_health`, `/_ping`, `/_readiness`) are exempt so a lockout cannot restart a healthy pod. Verified live against a production boot (401, 401, then 429). UI: 15-minute idle sign-out (`src/lib/idle.ts`, `IDLE_TIMEOUT_MS`) revokes the session and returns to sign-in; polling does not count as activity; vitest (4) and a Playwright test that fails at 10 minutes and passes at 15. **Not done:** re-authentication before destructive actions (needs a step-up flow from the identity provider). **Limits:** counters are per process, so enforce a global limit at the ingress too; unauthenticated callers share one bucket unless `CRM_TRUST_FORWARDED_FOR=1` is set behind a proxy that overwrites `X-Forwarded-For`, which means an attacker can lock out other unauthenticated callers (not authenticated ones, and not probes); the idle timer is client-side, so a client that blocks JavaScript keeps its session until the server-side session lifetime ends.

- [x] CRM-T44 *(resolved 2026-10-10.)* **Break-glass access.** Emergency elevation with mandatory
      reason, time limit, and after-the-fact review report. (plan gap 15)
      *Open.*

      **Resolution:** `src/hardening.rs`: `production_start_check` (4 pins). Checked in `Hooks::after_routes`, because `loco start` bypasses `Hooks::boot`. Verified live: production without `CRM_REQUIRE_AUTH` exits with the refusal; with it, boots. Scope note: it checks the enforcement flag, not that a verifier or key set is configured (auth already fails closed on an empty key set). Also fixed an existing bug found on the way: an empty `SMTP_USER`/`SMTP_PASSWORD` rendered YAML `null` and stopped `production.yaml` from parsing.
      **Resolution:** Migration `m20261010_000008_audit_chain` (`prev_hash`, `hash`, append-only trigger on PostgreSQL); `rules/audit_chain.rs` (8 pins: intact, altered, deleted, reordered, legacy rows, unchained-after-chain, field boundaries); `Model::record` takes a transaction advisory lock, appends, then hashes the row as stored; `GET /api/audits/verify` returns `ok`, `rows`, `legacy_rows`, `head`, `first_break`. DB test `audit_chain_detects_tampering` proves the trigger refuses edit and delete and the chain reports an edit made behind a disabled trigger. Full `--ignored` suite 10/10 against PostgreSQL 18. **Limits stated:** the advisory lock serializes audited writes; deleting the newest rows leaves a valid shorter chain (anchor `head` externally, see the runbook); the table owner can drop the trigger.
      **Resolution:** Service: `config/production.yaml` gains `secure_headers` (HSTS, nosniff, frame options, CSP preset), a closed CORS origin list (`CRM_CORS_ORIGIN`), and `server.ident: ""` to drop `x-powered-by`; verified with curl against a production boot. UI: `src/lib/server/headers.ts` applied in `hooks.server.ts`, plus `kit.csp` (hash-based, `frame-ancestors none`) in `vite.config.ts`; vitest pins the header set, Playwright asserts the headers and CSP on a real response. UI adapter changed from `adapter-auto` to `adapter-node` (needed for T35).
      **Resolution:** `scripts/ci-check.sh supply-chain` (cargo audit, cargo deny with `deny.toml`, pnpm audit, CycloneDX SBOMs to `sbom/`); jobs added to GitHub Actions and Woodpecker. Run locally: advisories, bans, licenses, sources ok; both SBOMs produced; no known vulnerabilities.
      **Resolution:** `spec/compliance/`: `control-mapping.md`, `dpia-template.md`, `operations-runbook.md`, `index.md`. Controls are marked Implemented only where code or CI enforces them; the rest are Deployer or Gap with a task reference.
      **Resolution:** `tests/e2e/a11y.spec.ts` (axe-core, WCAG 2.0 to 2.2 A/AA tags) over `/`, `/tour`, `/signin`, and six list pages: 9 pass, and a sanity check confirmed axe does detect an injected violation. **Limit:** automated checks cover roughly a third of accessibility issues, and detail pages and data-filled states are not scanned; manual testing with assistive technology is still needed.
      **Resolution (defaults chosen, change in `src/break_glass.rs`):** any authenticated caller may `POST /api/break-glass` with a written reason of 20 to 500 characters and 1 to 60 minutes (default 30). While the grant is live the caller's authorization context carries `env.break_glass=true`; the reference policy lets that read unmasked, and it never elevates writes or destructive actions. Every grant and review is audited into the hash chain; contact reads made under a grant record `break_glass: true`. `GET /api/break-glass` lists grants with the unreviewed count; `POST /api/break-glass/{pid}/review` needs a note of 10 or more characters, is refused 403 for the grantee, and 422 if already reviewed (both admin/service tier). Migration `m20261010_000009_break_glass`; 5 unit pins; the enforcement test walks the whole flow (thin reason and over-long grant refused, reader elevated and unmasked, still cannot write, review surface closed to the reader, self-review refused, second review refused). **Limits:** the live set is cached per process and refreshed from the database every 10 seconds, so another replica honors a new grant within that interval, and a revoked or expired grant lapses on the same schedule; there is no revoke endpoint and no automatic alert on a grant (alert on `break_glass_granted` audit rows); UI: `/break-glass` (request form) and `/break-glass/review` (grants list with per-row review note), i18n keys ×8, Playwright and axe coverage; the elevation applies to the record-level and aggregate masking paths, not to the admin-tier audit reads.
      **Resolution:** `src/crypto.rs` (AES-256-GCM, keyring with a first-listed active key, `enc:v1:<kid>:<nonce+ciphertext>` format, key id authenticated, 11 unit pins incl. tamper, wrong key, rotation, retired key, bad specs) and `models/encrypted.rs`, an `EncryptedText` column type that seals on write and opens on read, so no handler can forget it. Applied to `contacts.preferred_channel` and `activities.summary`. Keys: `CRM_FIELD_KEYS` or `CRM_FIELD_KEYS_FILE`; `CRM_REQUIRE_FIELD_ENCRYPTION=1` makes the service refuse to start without them; `rotate_field_keys` task re-seals everything (and legacy plaintext), idempotent. DB test `fields_are_encrypted_at_rest_and_plaintext_in_the_api` proves ciphertext at rest, plaintext through the API and the subject-access export, legacy plaintext reading, rotation, idempotence, and a retired key failing with a server error instead of garbage. **Limits, stated:** keys come from the environment or a mounted file, not a KMS (the `Keyring` is the seam to add one; a KMS adapter needs a vendor decision); only these two columns are covered (lead names and emails, ticket text, account names, deal names are not); the authenticated data names the format and key id but not the column, so a ciphertext copied between the two columns would decrypt; encrypted columns cannot be searched or filtered in SQL (nothing does today); raw SQL, such as the erase path, bypasses the type and writes plaintext, which reads back and is sealed by the next rotation; the audit-log snapshots and the event outbox can hold copies of values and are not encrypted.

## DTAC follow-ups (from the NHS DTAC v2.0 assessment)

Source: [the DTAC assessment](../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md).

- [x] CRM-T45 *(resolved 2026-10-10.)* **Accessibility statement.** Public
      `/accessibility` page linked from every page's footer (i18n key ×8, axe
      scan, Playwright test). It states the WCAG 2.2 AA target, that compliance
      is **not** claimed, what was and was not tested, and known risks (drag and
      drop boards, unaudited third-party components, unreviewed translations).
      The axe gate caught a real violation on the new page (links in running
      text relied on colour) which is fixed. **Still needed:** the manual audit
      the statement says is missing; update the statement when it is done.
- [x] CRM-T46 *(resolved 2026-10-10.)* **DTAC supporting statements.**
      [spec/compliance/dtac-supporting.md](compliance/dtac-supporting.md): the
      C1.2.1 scope justification, the MFA plan (C3.5), the logging statement with
      exclusions (C3.6), a threat-model summary, and the C3.4 and C4.1.2 gaps.
      Drafts for the manufacturer to adopt; the threat model is a summary, not a
      structured exercise.
- [x] CRM-T47 *(resolved 2026-10-10.)* **Close the account-detail masking leak
      and audit account reads.** `GET /api/accounts/{pid}` bundled the account's
      contacts and deals with no record-level decision, so a masked caller saw
      real channel detail and deal amounts. Contacts now go through
      `readable_contacts` and deals through the per-deal decision; account single
      and list reads are audited. The enforcement test fails without the fix.
- [ ] CRM-T48 **Manual accessibility audit** (keyboard, NVDA/JAWS/VoiceOver,
      400% zoom) of data-bearing pages and the boards, fixes, and an updated
      statement. *Open: needs people and assistive technology.* **Brief written 2026-10-18:** `compliance/accessibility-audit-brief.md` is the test plan. No audit has been done.
- [ ] CRM-T49 **GDS API checklist review and build signing.** Review the API
      against the GDS standards, and sign release builds and pin image digests
      (Software Security Code of Practice). *Open.*
      **Partly done (2026-10-10):** the four base images in both Containerfiles are now pinned by digest (the UI image builds with the pins; the service image was not rebuilt). Digests do not update themselves: refresh them on a schedule (for example Renovate or Dependabot) or they will pin known-vulnerable layers. **API review (2026-10-10):** a self-review is in `spec/compliance/gds-api-review.md`; it fixed the OpenAPI document (22 missing paths, security scheme, shared error responses) and opened T51 (pagination) and T52 (idempotency keys). It is a draft that has not been checked against the standards' published text. **Build signing and digest refresh (2026-10-11):** `.github/dependabot.yml` opens weekly pull requests for Cargo, npm, both Containerfiles' digest pins, the compose file and the Actions, so the pins do not freeze. `.github/workflows/release.yml` runs on a `v*` tag: it builds and pushes both images to GitHub Container Registry, signs each by digest with keyless cosign (workflow OIDC identity, no stored key), attaches a build-provenance attestation, and signs and publishes the CycloneDX SBOMs to the release. The runbook has the `cosign verify` and `gh attestation verify` commands and says to deploy by digest. The service image was rebuilt from the pinned bases: it builds, runs as `nonroot`, and exits with a clear error without `DATABASE_URL`. **Not verified:** the workflow has not run (it needs a pushed tag and a repository with Packages and attestations enabled), and the actions are pinned to the commit hashes their version tags pointed at on 2026-10-11 (read with `git ls-remote`; the action code itself was not audited), with Dependabot proposing bumps. **Still open:** a reviewer's check of the GDS draft and a first real tagged release to prove the signing and verification commands. The dry-run steps are in `spec/compliance/first-release-checklist.md`.
      **Dry run 2026-10-18:** three pre-release tags. `v0.0.1-rc.1` failed: the service Containerfile still built with Rust 1.96 while `Cargo.toml` requires 1.99 (fixed; base image now `rust:1.99-bookworm` by digest). `v0.0.1-rc.2` failed: the SBOM job installed only `cargo-cyclonedx`, but `ci-check.sh supply-chain` also runs `cargo audit` and `cargo deny` (fixed). `v0.0.1-rc.3` passed: both images built, were pushed to GitHub Container Registry and signed (keyless), the provenance step ran, and the release got both SBOMs with Sigstore bundles. **Verified by hand:** both SBOM bundles verify with `cosign verify-blob` against the workflow identity, and a one-byte change to an SBOM makes verification fail. **Checked by hand without registry access (2026-10-18):** GitHub holds a SLSA provenance attestation for each image digest (service `sha256:8aea27c8…`, ui `sha256:0e253fa3…`) naming `release.yml` at `refs/tags/v0.0.1-rc.3` in this repository, and the two image signatures are in the public Sigstore log (indexes 3314161007 and 3314079321) with certificates whose identity is that workflow and run 39147091785. **Not yet verified:** `cosign verify` and `gh attestation verify` against the registry itself (the local `gh` token lacks `read:packages`, the device login timed out, and the packages are probably private), so the certificate chain and the signature over each image were not checked, and the signatures' presence in the registry was not seen; and running the image by digest. rc.1 and rc.2 left images and signatures behind in the registry and in the public Sigstore log, which cannot be deleted; the rc tags were deleted, rc.3 and its release were kept for the remaining checks. The release was not marked a pre-release (fixed in the workflow, for the next tag). The Dependabot Rust image bump to 1.98 would break the build (the code needs 1.99).
      **First release 2026-10-19:** `v0.1.0` was tagged and its UI image failed to build (Node 26 has no `corepack`, from my Node 26 bump; fixed in the Containerfile and in the Woodpecker front-end step); its service image was built, pushed and signed, and that tag was left in place with no release. `v0.1.1` (commit `3e0fee8`) then passed: both images, both SBOMs, and a release that is not a pre-release. **Verified from here (2026-10-19):** for rc.5 and v0.1.1, `cosign verify` (keyless, workflow identity, Sigstore log) and `cosign verify-attestation --type slsaprovenance1` pass for both images against the registry (service `sha256:b66e98c7…`, ui `sha256:60989f83…` for v0.1.1); `gh attestation verify` failed on registry credentials, so cosign was used instead. One scripted provenance check reported a failure that did not recur on rerun. **Run by digest (2026-10-19):** the UI image (v0.1.1) serves `GET /` with 200 as `nonroot`; the service image (amd64, emulated on arm64) starts as a Loco app and exits with an error naming the missing `DATABASE_URL` (the error also prints the production config template, which holds no secrets). The service was not run against a database from the image. **Dry run `v0.1.2-rc.1` (2026-10-20):** cosign 3 and attest 4 ran; both images and the SBOM job succeeded; cosign 3.1.3 verifies both signatures and the provenance (service `sha256:a8f14795…`, ui `sha256:b893f2a8…`); cosign 2.5.3 cannot verify the new signatures ("no signatures found"). The earlier one-off provenance failure was the verify script running `gh` outside a git repository. Matcher 0.1.1 and service 0.1.2 are on crates.io; **Release `v0.1.2` (2026-10-20, commit `385a26c`):** the workflow succeeded (both images, SBOMs); cosign 3.1.3 verifies both signatures and the provenance (service `sha256:fa8b6adf…`, ui `sha256:b5d11d34…`); the GitHub release is not a pre-release and has 4 assets. The images were verified, not run. **Image labels and version check (2026-10-19):** both images now carry `org.opencontainers.image.{title,source,licenses,version,revision}` from build arguments; `scripts/release-version.sh` makes the release workflow refuse a tag that disagrees with `Cargo.toml` / `package.json`, and a second step checks the built image's own label. Tested: the script on good and bad tags, and the UI image's labels in a local build. **Not tested:** the service image's labels (not built locally) and the workflow steps (the next tag runs them). Under this rule `v0.1.1` would be refused, since `Cargo.toml` says 0.1.0: the next release must bump both manifests first. The UI manifest says `MIT OR Apache-2.0` and the service's the five-licence list, so the two labels differ. **Cleanup and v0.1.2 preparation (2026-10-19):** the rc.3 package versions, the `v0.1.0` tag and every image of v0.1.0, rc.1, rc.2 and rc.4 were deleted from the registry (only v0.1.1 and rc.5 remain; the deleted images' signatures stay in the public Sigstore log, which cannot be edited). Dependabot #22, #23 and #24 (attest-build-provenance 4.2.2, cosign-installer 4.1.2 which installs cosign 3, install-action) were merged into main and both manifests set to 0.1.2, so `v0.1.2` can be tagged; **the release workflow with cosign 3 and attest 4 has not yet run**: tag `v0.1.2-rc.1` from main first, and check that the signatures and provenance verify, and whether a cosign 2 client verifies them (the runbook says to use cosign 3). `cargo publish --dry-run` passes at 0.1.2; nothing was published. **Not yet done:** running the service image with PostgreSQL and Redis; deleting rc.3's package versions (the release and tag are gone); the `v0.1.1` tag does not include later commits (the matcher, the duplicates endpoint, Redis in CI). Still open: the reviewer's check of the GDS draft.
      **Checked against the published text (2026-10-21):** `spec/compliance/gds-api-review.md` now has a row-by-row check against the GDS guidance page. It found the header-based versioning contradicts the published advice (version in the URI), plus four small gaps: unknown request members are ignored, no `Cache-Control`, no rate-limit headers, no stated OAuth 2.0 position. The page has no pagination or idempotency requirement. The check is mine, not an independent reviewer's, and The code gaps were then fixed (2026-10-21): unknown request members are `422`, every `/api/*` response is `no-store`, `RateLimit-*` headers are sent. The `422` names the offending member (own `Json` extractor, `src/json.rs`). Service tests: 184 unit, 59 DB-backed; UI live tests 14/14. One existing test had been sending a stray `stage_pid` on deal create, which the strictness caught. Not released. Versioning decided: the path stays `/api`, never `/api/v1`; the header scheme is kept and the deviation recorded in the review and `api-lifecycle.md`. The OAuth 2.0 position is written in the runbook (the service is a resource server; the earlier "bearer JWTs" wording was wrong: tokens are PASETO or Keycloak).

- [x] CRM-T50 **Redis cache.** Loco's cache is now Redis (`cache_redis`;
      `config/production.yaml` requires `CRM_REDIS_URL`, use `rediss://` for TLS;
      development defaults to a local Redis; `config/test.yaml` stays in-process
      so the suites need no Redis).
      **Resolution:** (1) `GET /api/insights/*` responses are cached for 30 s,
      inside auth and the read audit, so every request is still authenticated
      and audited. The key is a hash of the caller's bearer token, their
      break-glass state, the path and query, and a generation that every
      successful API write bumps, so masking never crosses callers and a write
      is visible at once; ETag and other headers are kept; requests with
      `If-None-Match` bypass the cache. (2) The rate limit and failed-auth
      lockout count in Redis (`src/redis_store.rs`) when `CRM_REDIS_URL` is
      set, so they hold across replicas. Tests: `insight_cache_hits_then_write_invalidates`
      (also run once against a real Redis), `redis_store` counter test
      (`compose.test.yaml` now starts Redis). **Limits:** a policy reload or
      revoked token shows up only after the 30 s TTL (a revoked token is still
      refused, since auth runs first); a Redis outage makes cached insights
      recompute and counters fall back to per replica; Redis holds insight
      bodies, which include personal data, so protect it as you do the
      database (TLS, auth, no public exposure, eviction policy).
      **Break-glass on Redis (later):** a new grant is also published to
      `crm:bg:<subject>` (expiring with the grant, never shortened), and each
      replica polls Redis every second, so a grant reaches the others in about
      a second instead of 10. The database stays the record: it is re-read every
      10 s, the live set is the union of both, and a Redis error leaves the
      database path working. Without `CRM_REDIS_URL` nothing changes (10 s).
      Tests: Redis grant publish/never-shorten/expired pin; the break-glass
      enforcement flow was also run against a real Redis. **Limit:** there is
      still no revoke endpoint, so a grant ends only by expiry.

- [x] CRM-T51 **Pagination for list endpoints.** List reads cap results
      silently (`LIMIT` 500 to 5000, 11 sites) with no page parameters, total
      or next link, so a consumer cannot tell rows were dropped. Add `limit`
      plus a cursor and a next link, keep the caps as maximums, and test a
      list larger than one page. Found by the API review
      (`spec/compliance/gds-api-review.md`).
      **Resolution:** the seven lists that cut silently (contacts, accounts,
      activities, leads, deals, tickets, articles) take `?limit=` (1 to the old
      cap, default the old cap) and an opaque `?cursor=`, and answer
      `Link: <...>; rel="next"` when more rows exist (`src/controllers/paging.rs`).
      The body stays a bare array, so existing clients are unchanged until they
      pass the cap, and then they are told. Orders now end in `id`, so pages are
      stable; bad `limit` or `cursor` is 422. Article search moved into SQL: it
      used to filter after the 500-row cut, which hid matches past the cut.
      The activity dashboard now counts in SQL (`GROUP BY kind`) over every
      activity, so its totals are exact (test seeds 5003 rows). OpenAPI documents the parameters and header. UI:
      `apiAll` follows the links (50-page safety stop that throws rather than
      return a partial list), the six list calls use it, and the BFF proxy
      forwards and re-points `Link`. Tests: 4 paging unit pins, 3 DB tests
      (follow `next` visits every row once; bad input is 422; search past the
      page and a literal `%`), 5 vitest, existing Playwright. **Limits:** the
      cursor is an offset, so a row added or removed between page fetches can
      make a later page skip or repeat a row; there is deliberately no total
      count (it would include rows a record-level policy hides); the contact
      timeline (latest 200), group feed (50) and the nurture sweep (500 per
      run) are bounded by design and not paginated; insights are not paginated.
- [x] CRM-T52 **Idempotency keys for creates.** A retried `POST` after a
      timeout makes a duplicate. Accept an `Idempotency-Key` header on create
      endpoints and replay the first result (store it in Redis or the
      database, with an expiry).
      **Resolution:** any authenticated `POST` under `/api/` that carries
      `Idempotency-Key: <1-255 visible ASCII>` runs at most once per
      `(caller, key)` (`src/idempotency.rs`, a middleware inside auth, table
      `idempotency_keys` from migration `m20261011_000010`). The claim is one
      `INSERT ... ON CONFLICT` on the primary key, so it holds across replicas.
      Same key and same request after a `2xx`: the stored response is replayed
      with `Idempotent-Replayed: true`. Same key, different request: 422. Same
      key while the first attempt runs: 409 with `Retry-After: 1`. A first
      attempt that is not `2xx` is forgotten, so the retry runs again. Keys
      expire after 24 hours (hourly cleanup); a row stuck `in_progress` for 2
      minutes (a crash) is taken over. Fails closed: if the store is down the
      request is refused 503 rather than run unprotected. Without the header
      nothing changes. OpenAPI documents the header, 409 and the replay header;
      production CORS allows it. Tests: 3 unit pins, 5 DB tests (replay returns
      the same record and creates one; no key means two records; reused key and
      bad keys are 422; a failed attempt does not stick; two simultaneous
      identical requests create one). **Limits:** opt-in, so a client that sends
      no key is not protected (the bundled UI now does send one, see below); only
      successes are replayed (an error is re-run on retry); bodies over 1 MiB
      are refused 413 when a key is sent; a crash after the work but before the
      result is recorded leaves the key `in_progress` for 2 minutes, then a
      retry repeats the work; the key is scoped to the caller's subject, so a
      different caller reusing a key gets their own execution.
      **UI (later):** `api()` in `src/lib/api/client.ts` sends an
      `Idempotency-Key` on every `POST` (a fresh random key per call, with a
      fallback where `crypto.randomUUID` is unavailable) and accepts an explicit
      `idempotencyKey` so a caller can keep one key across retries of one
      action. Two identical `POST`s issued while the first is in flight share
      one request, so a double click sends one. Tests: 5 vitest and a
      Playwright assertion on the break-glass request. **Stable keys per form (later):**
      `src/lib/intent.ts` holds one key per user action. The break-glass
      request form and each row of the review list keep the same key across
      retries while what they submit is unchanged, start a new key when the
      content changes (the service refuses a key reused for a different
      request), and start a new key after a success. Tests: 4 vitest and 3
      Playwright (failed attempt then retry sends the same key; editing the
      reason starts a new one; a review retry reuses its key; a new request
      after success gets a new key). The retry test fails without the page
      change. **Limits:** only these two forms exist in the UI that submit
      `POST`s, so they are the only ones covered; the other `POST` helpers in
      `crm.ts` have no page yet and fall back to a fresh key per call plus
      in-flight coalescing; `apiAll` is GET-only and unaffected.

- [x] CRM-T53 **OpenAPI schemas and route coverage.** The API review found the
      document had no request or response schemas and nothing checked that it
      matched the routes.
      **Resolution:** `src/openapi_schemas.rs` adds `components.schemas`: a
      request schema for every operation that takes a body (required fields
      follow the controllers' payload structs, including serde defaults), a
      resource schema for each of the 11 lists, `Created`, and a shared `Error`
      body attached to every error status. Tests: (1)
      `every_mounted_route_is_documented_and_the_reverse` builds Loco's real
      route table and fails on a mounted route or verb missing from the
      document, or a documented one that is not mounted (probes and the docs
      endpoints are the only exemptions, listed in the test; it found two
      undocumented `stakeholder` routes on its first run, and fails when a
      path is removed); (2) a mutating operation must have a request schema or
      be on an explicit bodyless list; (3) every `$ref` resolves; (4) the
      DB-gated `responses_match_the_documented_schemas` validates 10 real
      request bodies and the rows of all 11 lists against the schemas with a
      JSON Schema validator. That test caught a wrong schema on its first run
      (`/api/pipelines` returns `{pipeline, stages}`), and my first draft had
      marked five defaulted fields as required (`kind`, `currency`, `tier`,
      `preferred_channel`, `channel`), which the existing tests exposed.
      Also fixed a bug from the idempotency work: the document-building
      helpers indexed missing keys on `serde_json` values, which inserts `null`
      operations on every path; the test now fails on a null operation.
      **Response schemas for the rest (follow-up):** the detail reads (contact,
      account, lead, ticket, group, account CLV), forecast and snapshot, the
      three dashboards, the 15 insight views, audit entries and chain verdict,
      events, the subject-access export, retention report and sweep, the
      sweeps, break-glass grants, and the campaign funnel and segment preview
      now have schemas. Fields a role can mask (amounts, cost, ROI, totals)
      are nullable, since a masked value is absent, never zero. Two tests
      check them: `read_responses_match_the_documented_schemas` seeds one of
      everything and validates 30 reads and the typed writes; the enforcement
      test validates the masked and unmasked forecast, sales dashboard,
      executive pack and account detail, and the break-glass bodies, with real
      tokens. The first run caught a campaign `cost_minor` that is null when
      masked.
      **Limits:** hand-written, so they can still drift on fields no test
      reaches (for example an empty section of a view); operations whose
      response is the updated record (transitions, `PUT` upserts) are untyped;
      enumerations (statuses, kinds, channels) are plain strings, not
      enumerated; a mutating route added with no schema must be listed as
      bodyless to pass, which is a prompt, not a proof.
- [x] CRM-T54 **RFC 9457 errors and a deprecation policy.** The API review
      found error bodies were Loco's `{error, description}`, produced in
      several places with different shapes (plain text from the auth and
      rate-limit layers, JSON from controllers, empty bodies from the router),
      and no written rule for changing or retiring the API.
      **Resolution:** `src/problem.rs` is one outermost middleware that turns
      every `4xx` and `5xx` into `application/problem+json` with `type`,
      `title`, `status`, `detail` and `instance`. `error` and `description`
      stay as members so existing clients (including the bundled UI) keep
      working; only the media type changed. Extension members such as
      `supported` pass through. A `401` now carries `WWW-Authenticate:
      Bearer`. The OpenAPI `Error` schema and the error content type match.
      `spec/compliance/api-lifecycle.md` records the policy: what is breaking,
      the deprecation steps (changelog, OpenAPI `deprecated`, `Deprecation`
      and `Sunset` headers, a 180-day notice period, version overlap).
      Tests: five unit tests on the conversion; a DB-gated test that checks a
      422, an unknown record, a refused API version, an unknown route and a
      wrong method against the `Error` schema and the media type; and the
      enforcement test's token-less `401`.
      **Update 2026-10-12:** the payload-size limit's `413` is converted too
      (tested; its reason-phrase label is no longer taken for a machine code).
      **Limits:** the request timeout and the panic handler are not tested for
      conversion. Nothing is deprecated, so the `Deprecation` and `Sunset`
      headers are specified but not built. The 180-day notice period is the
      maintainer's choice, not a standard's. Not checked by a reviewer against
      the RFCs' published text.

- [x] CRM-T55 **Board edits and reschedule.** The boards and the follow-up
      calendar can change records, not only read them.
      **Resolution:** `PUT /api/deals/{pid}` (name, amount, expected close; open
      deals only), `PUT /api/leads/{pid}` (display name), `PUT /api/tickets/{pid}`
      (title), `PUT /api/activities/{pid}` (due day and summary; a due-day change
      is audited as `rescheduled` with both days, and a finished activity or one
      with no due day is refused), and `PUT /api/{deals,leads,tickets}/{pid}/position`
      (the 0-based place within the column; the column is renumbered). Deals
      reuse `kanban_position`; leads and tickets get `board_position`
      (migration `m20261017_000015`, default 0, so the old order holds until a
      column is reordered). Audit entries name the fields changed, not their
      values. Tests: 4 unit, 3 database. **Limits:** deal edits go through the
      record-level policy; the lead, ticket and activity edits rely on the route
      guard only, as their other writes do. An edit overwrites without a version
      check, so two people editing the same card last-write-wins.
      **Update 2026-10-18:** run by hand with `curl` against the rebuilt dev
      service on a migrated database: all seven endpoints answered as specified
      (200 for edits and positions, 404 for an unknown record, 422 for an empty
      title and a malformed date), and a rescheduled follow-up showed on its new
      day in `/api/insights/followups`. Not yet driven from the browser through
      the UI; the audit entries were not read back.
      **Update 2026-10-18:** driven through the UI (`pnpm test:live`, 4 tests):
      ticket rename, deal amount edit, within-column reorder by Alt+arrow and
      follow-up reschedule by Alt+arrow, each read back from the service. A
      mouse drag of a ticket card within its column is covered too (5 tests).
      Not covered: lead edits, a drag between columns or lanes, the audit
      entries.

- [x] CRM-T56 **Export to CSV, TSV and Excel.** Traces to CRM-R21. Every
      `DataTable`, the boards and the follow-up calendar offer all three
      formats, plus Copy, through the Lily components' own export.
      **Resolution:** TSV and Copy labels added in all eight locales; an e2e
      test downloads `.csv`, `.tsv` and `.xlsx` from the contacts table.
      **Limits:** export holds what the browser already has, so a table paged by
      the service exports only the loaded rows; the file contents (column order,
      typed cells) are not asserted by a test; translations are unreviewed.
      **Update 2026-10-18:** the campaign timeline exports too (e2e test).

- [x] CRM-T57 **`updated_at` moves on every edit.** Found while checking CRM-T55
      by hand: an edited deal's `updated_at` equalled its `created_at`, and the
      campaign timeline uses `updated_at` as a bar's end.
      **Resolution:** migration `m20261018_000016` (trigger on 18 tables that are
      edited in place; respects an explicit `updated_at`). One database test
      assertion. **Limits:** only the lead rename is asserted, not every table;
      existing rows keep their old `updated_at` until next edited; the Rust 1.99
      clippy lints (`assert_is_empty`, one redundant borrow) were fixed
      separately.

- [x] CRM-T58 **Search every list, and a language in any address.** Traces to CRM-R22. The header
      search found only contacts; the language-in-address routes existed only
      for the kanban page.
      **Resolution:** `/search` (seven lists, filtered in the browser; 5 unit
      tests on the matcher and an e2e test across contacts and tickets) and a
      `reroute` hook that lets every page open under `/<locale>/…` (2 unit
      tests on the splitter, 2 e2e tests, including the sign-in gate).
      **Limits:** search reads each whole list, so it is slow on large data and
      sees only what the caller may read; the service has no search endpoint;
      account, lead, ticket and campaign hits link to their list, not a record
      page (only contacts have one); search does not look inside notes,
      activities or custom fields; the language picker's address rewrite is not
      tested; the app is a single-page app (no server rendering), so a prefixed
      address shows the page in the chosen language only after it loads.

- [x] CRM-T59 **Publish to crates.io.** The service and its migrations crate
      were not publishable: a git dependency and a path dependency whose name
      belongs to someone else.
      **Resolution:** `authentication-verifier` 0.9.1 from crates.io; the
      migrations crate renamed `contact-relationship-management-migration`
      (imported as `migration`). `contact-relationship-management` 0.1.0 and
      `contact-relationship-management-migration` 0.1.0 were published on
      2026-10-18 after a clean dry run, tests, clippy and `cargo deny`.
      **Limits:** the package ships only `src/` and the README, so it has no
      `config/*.yaml`, tests or Containerfile and a user must supply a Loco
      config; the container image remains the supported way to run it; a
      published version is permanent (yank only); no `v*` git tag was pushed
      for this (that would start the image release workflow).

- [x] CRM-T60 **Contact matcher crate.** Traces to CRM-R23. Nothing could say
      whether two contacts were the same person.
      **Resolution:** `contact-relationship-management-matcher-with-rust/`, a
      library crate (`contact-relationship-management-matcher` 0.1.0,
      published to crates.io on 2026-10-19) using Fellegi–Sunter record linkage: each field adds log-odds
      to a prior, the sum is a probability, and every field's contribution is
      returned. 8 unit and 17 behaviour tests and a doctest, including that the
      result is symmetric and that more agreement never lowers the probability;
      checked by `scripts/ci-check.sh` (fmt, clippy, test), CI and Dependabot.
      **Limits:** the default rates are reasoned guesses, not fitted to data,
      and field independence is assumed, so probabilities are over-confident
      when fields are correlated; names are English-centred (nicknames, honorifics)
      and Jaro–Winkler suits Latin scripts best; phones are compared as digits
      without a country database; there is no date of birth or address field; it
      is called by the service only (CRM-T61), not by the UI; nothing merges
      contacts. Validate the thresholds against reviewed pairs before any
      automatic action.

- [x] CRM-T61 **Duplicate-contact endpoint.** Traces to CRM-R23. The matcher
      (CRM-T60) was not reachable from the service.
      **Resolution:** `GET /api/contacts/{pid}/duplicates?limit=` (default 20,
      at most 100), documented in OpenAPI with a checked response schema.
      Candidates are the other active contacts the caller may read; they are
      scored on name, job title, account name and `person_ref`, then the best
      20 by that score have their email addresses fetched and are rescored, so a
      shared address can bring back a changed surname. 3 database tests (order,
      reasons, limit, the certain `person_ref` case, the email rescue, 404 and
      the audit entry; the audit chain still verifies). The matcher was published
      first so the service depends on it by version, which keeps the image build
      and the service's own publish working.
      **Limits:** at most 5,000 contacts are compared (`truncated` says when
      more exist), and a contact whose names say "no" can only be rescued by an
      email address if it is among the 20 best on the local fields, so a changed
      surname is missed on a large table; the email look costs one call to the
      person service per candidate, up to 21 a request, and does nothing when
      the target has no address; no phone, since the CRM holds none either; the
      defaults are untuned (see CRM-T60); advice only, with no merge or delete;
      the UI shows it since CRM-T62.

- [x] CRM-T62 **Show likely duplicates on the contact page.** Traces to CRM-R23.
      **Resolution:** a "Possible duplicates" section on `/contacts/{pid}`: each
      candidate links to its record and shows a verdict, the probability (rounded
      down, so only a shared person record reads 100%) and which fields agree and
      differ, never their values. 13 new labels in all 8 locales, written by me
      and **not reviewed by native speakers**. The tour has one fictitious pair
      (Imogen Hartley and Imogen Hartly, a 13th contact, `con-13`); its figures
      are illustrative, not computed. 1 e2e test (English, a contact with none,
      the link-through, German labels); 104 unit and 91 Playwright tests pass.
      A live test (`tests/live/duplicates.spec.ts`) drives the page against the
      real service and passes on three runs in a row (it ignores records left by
      earlier runs, which resemble the new ones).
      The live deal-amount test now creates its own pipeline and deal, so
      the live suite passes (6 tests) on an empty database.
      **Limits:** a failed request shows nothing, not an error; no
      action beyond the link, so merging stays a manual job in the person
      service; the candidate link drops a language prefix; phone is never shown.

- [x] CRM-T63 **Tune the matcher.** Traces to CRM-R23. The defaults in
      `contact-relationship-management-matcher` (the prior, each field's agreement
      and coincidence rates, the two thresholds) are reasoned guesses; nothing has
      measured them. Build a labelled set of synthetic contact pairs, measure,
      and set the defaults from the result.
      **Scope:** a fixture of pairs, each labelled same person or different
      people, covering the cases that decide it: nicknames and initials, typos,
      accents and transliteration, swapped and compound names, a changed surname
      with a shared mailbox, one mailbox shared by two people (a family, a team
      alias), the same name at different employers, a shared employer with
      different people, a job change, sparse records, and a shared `person_ref`.
      Report precision and recall at several thresholds, find where the review
      band should begin and end, and check how far the independence assumption
      over-states the probabilities (a calibration table: stated probability
      against the observed share that really were the same person).
      **Done when:** the fixture and the measuring test are in the matcher crate;
      the defaults change only where the numbers justify it, each change with its
      reason in the README; a test pins the measured precision and recall as
      floors, so a later change cannot quietly make it worse; the limits below are
      in the README.
      **Limits:** synthetic pairs show how the matcher treats the cases someone
      thought of, not how it performs on real contacts; real tuning needs pairs a
      person has reviewed, drawn from the deployment's own data, under the same
      data-protection rules as that data. A fixture built by the people who wrote
      the matcher will flatter it, so keep the hard cases (the look-alikes) at
      least as numerous as the easy ones. Changing the defaults is a behaviour
      change: record it in the changelog and the service's CRM-T61 note.
      **Result (2026-10-19):** 105 pairs, `tests/fixtures/pairs.json`, measured
      by `tests/tuning.rs` with floors pinned. `match_threshold` 0.9 to 0.99
      (different pairs called `Match` 17 to 5, one true match lost); nothing
      else justified a change. The probabilities are over-confident (0.90 to
      0.99 was right 1 time in 13). Matcher 0.1.1 was published to crates.io on
      2026-10-20 and the service (0.1.2) now depends on it, so the 0.99 default
      is live in the service. The CRM-T61 note records the earlier 0.9 figures.

- [x] CRM-T64 **Merge two duplicate contacts.** *Done 2026-10-19: service (endpoint, ledger migration, 4 DB tests, enforcement and schema checks) and UI (Merge on a duplicate, dry-run preview, required reason, 8 locales unreviewed by native speakers, 2 e2e tests, 1 live test). Migration run against a populated database 2026-10-20 (`scripts/migrate-with-data.sh`: 300 contacts, 600 activities, 150 tickets, 75 deals, 200 consent events; rolled back to migration 16, migrated forward in about 13 ms; every table's rows identical apart from the new column, nothing marked merged; then 150 merges, ledger and invariants and audit chain all hold; one run, small data, not a production-sized table). Limits: no unmerge (the ledger makes one possible); the duplicates hint in all 8 languages was reworded because it said nothing is ever merged.* Traces to CRM-R23 and CRM-R24
      (agreed 2026-10-19: merge ledger, consent order `withdrawn` > `never` > `granted`, no second approver), written with it (the three-part rule: spec,
      code and tests together). This is the first feature built on the matcher
      that changes data, so it needs the most care. Today the duplicates
      endpoint is advice only (CRM-T61) and the manual repoint endpoint (CRM-T2)
      only changes which person record a contact wraps.
      **Scope:** `POST /api/contacts/{pid}/merge` with `{ "into_pid", "reason" }`,
      done in one transaction (CRM-D9). The survivor keeps its identity; the other
      contact is soft-deleted with a pointer to the survivor. The activities,
      deals (as primary contact), tickets, nurture enrolments and segment
      memberships of the merged contact move to the survivor.
      **Consent is the dangerous part.** The merged result must never be more
      permissive than either record was: a withdrawal on either side wins; a
      grant is never inferred from the other record; both consent histories are
      kept, each entry with its original source and date. Send-path rules (CRM-D6)
      then apply to the survivor as they stand.
      **Also decide and write down:** which `person_ref` survives, and what happens
      to the other (repoint in the person service is a separate system); that both
      contacts need record-level write access (CRM-T24), and masking applies to
      what the response shows; that an erased contact (CRM-R20) cannot be merged
      in either direction; that a subject-access export and an erasure of the
      survivor cover the merged-in data; that the audit chain records one entry
      naming both contacts, the reason and the counts moved, never the values; an
      event for the seam; idempotency keys (CRM-T52); and whether a merge can be
      undone (the safe default is that it cannot be undone automatically; counts in the
      audit entry are not enough to repair by hand, so CRM-R24 proposes a merge
      ledger of moved row ids; an unmerge is its own task).
      **UI:** a Merge action on a duplicate in the contact page's list, behind a
      confirmation that states what moves and that consent follows the stricter
      side; the labels in all 8 locales (unreviewed, as before).
      **Done when:** database tests cover the moves, the consent cases (granted
      with withdrawn, withdrawn with granted, never with granted), refusal of an
      erased contact, of the same contact twice, and of a contact the caller
      cannot write; the audit chain still verifies; a failure part-way leaves both
      contacts unchanged; OpenAPI and the checked response schema are updated; an
      e2e test and a live test drive it through the UI.
      **Limits to state in the task's resolution:** a match probability is not a
      decision, and a person must choose the merge (it is never automatic); a
      wrong merge of two real people is a data-protection incident, so the
      confirmation and the audit entry matter more than speed; the person
      service's own records are untouched.

- [x] CRM-T65 **Chaos test.** Traces to CRM-R25. `scripts/chaos.sh` starts its own
      PostgreSQL 18 and Redis (ports 15439 and 16379), migrates, starts the service,
      and injects: Redis stopped and frozen, PostgreSQL stopped and frozen, eight
      merges racing for one contact, and SIGKILL in the middle of a burst of 150
      merges. After the merge faults it checks four invariants in SQL and the audit
      chain. 27 checks pass.
      **Found and fixed (2026-10-20):** with Redis down or frozen a *successful write*
      hung for the Redis pool's own timeout (over 20 seconds), because the insight
      cache's generation bump after every write had no time limit. Every cache call
      is now bounded to 750 ms (`src/app.rs`); a write with Redis down answers in
      0.76 s.
      **Limits:** it covers the faults above, not network partitions, slow
      (rather than stopped) dependencies, disk-full, clock jumps, or a crash
      between the database commit and the HTTP response (a client that retries
      then relies on `Idempotency-Key`). The kill lands at a fixed delay, so the
      point of interruption varies run to run; the script says when it missed the
      burst. It uses the debug binary and a single service instance, so it does not
      test failover between replicas. It is not in `ci-check.sh` (it needs podman
      and takes about two minutes); run it before a release.

- [x] CRM-T66 **Undo a merge.** *Done 2026-10-20, service side only: migration 18 (`before`, `unmerged_at`, `unmerged_actor`, and a partial unique index so a contact can be merged again), the merge now records what it needs, `POST /api/contacts/{pid}/unmerge`, 4 DB tests, an enforcement check, OpenAPI. Limits: no UI; merges made before migration 18 cannot be undone (none exist outside tests); the survivor keeps a consent tightened by the merge; enrolments exited by a withdrawal stay exited; two migrations back is the v0.1.1 schema, so `scripts/migrate-with-data.sh` rolls back two.* Traces to CRM-R26
      **Found while building it (2026-10-20):** erasing a survivor (`/erase`) left the
      merged-away duplicate's name and job title in its soft-deleted row until the
      retention sweep, and the ledger kept its `person_ref`. Erasure now scrubs both.

- [x] CRM-T67 **Links to external systems.** *Done 2026-10-20 (service and UI; the UI is the contact page only, because accounts, deals and leads have list pages and no detail page): migration 19 (`external_links`), `rules/external_links.rs` (6 systems, https-only, per-system domains, 4 unit tests), `GET/POST /api/{contacts,accounts,deals,leads}/{pid}/external-links` and `DELETE /api/external-links/{pid}`, merge/unmerge move contact links, subject access lists them and erasure deletes them, OpenAPI, 4 DB tests; the contact page's Linked records panel (add with system, address and label; remove; opens in a new tab with `noopener`), the tour layer, 11 strings in 8 locales (translations mine, unreviewed), 3 e2e tests and 1 live test. *Update 2026-10-20: accounts, deals and leads now have record pages with the same panel (CRM-T70).* Limits: links only, no sync or import; a masked caller can still read the addresses; two contacts both linked to the same record keep both links after a merge (no dedupe on move).* Traces to CRM-R27

- [x] CRM-T68 **Undo a merge from the UI.** *Done 2026-10-20: `GET /api/contacts/{pid}/merges` (what can be undone, newest first), a "Recently merged" list and an Undo panel on the contact page (preview, reason, confirm), the tour layer, 13 strings in 8 locales (unreviewed), 2 e2e tests; the service test covers the list.* Traces to CRM-R26. Limits: no live (real-service) UI test for undo yet; only the survivor's page offers it, because a merged contact has no page.

- [x] CRM-T69 **Release v0.1.3.** *Done 2026-10-20 (commit `edb2d9b`): service 0.1.3 on crates.io (the matcher stays at 0.1.1; its pending `const fn` change is unreleased); `v0.1.3` tag pushed; the release workflow succeeded (both images, SBOMs); cosign 3 verifies both signatures and the provenance (service `sha256:01dc7452…`, ui `sha256:ed81123f…`); the GitHub release is not a pre-release and has 4 assets. Carries merge undo (CRM-R26), links to external systems (CRM-R27), the erasure fix, the UI for both, and the module splits. Migrations 18 and 19 are new, and `auto_migrate` is off in the image, so run `db migrate` before starting 0.1.3 (runbook section 6). The images were verified, not run.*
- [x] CRM-T70 **Link panels on accounts, deals and leads; live test for undo.** *Done 2026-10-20: `LinksPanel` component shared with the contact page; record pages `/accounts/{pid}`, `/deals/{pid}` (read from the deal list, since there is no single-deal read), `/leads/{pid}`; the tour layer gained `GET /accounts/{pid}` and links for all four kinds; 3 e2e tests and 3 live tests for the pages, and a live test that merges and then undoes through the UI against the real service. Two older live tests that assumed data were fixed. Limits: the deal page cannot edit the deal; the account page lists contacts and deals but the tour account has no deals.* Traces to CRM-R27, CRM-R26
- [x] CRM-T71 **More chaos cases; matcher 0.1.2 prepared.** *Done 2026-10-20: `scripts/chaos-proxy.py` (drop, reset, delay, and kill-after-COMMIT) sits between the service and PostgreSQL; `scripts/chaos.sh` gained network partitions (dropped and reset), a slow database, a service killed just after COMMIT (merge and idempotent create), and a primary-to-replica failover; 71 checks pass. The run uses `CRM_EVENT_TRANSPORT=outbox`, since only that transport writes the event in the mutation's transaction. Not covered: full disk, clock jumps, long replica lag, real failover tooling, TLS, several service replicas; the two-minute idempotency takeover window is described, not waited out. Matcher 0.1.2 (the `const fn` change) was published to crates.io on 2026-10-20 at the maintainer's word (no git tag; the service does not need it).* Traces to CRM-R25
- [x] CRM-T72 **Run the released image; fix what that found.** *Done 2026-10-20: `scripts/image-smoke.sh` and `examples/mint_token.rs`; the v0.1.3 image runs hardened in production mode with signed tokens (no token and a forged token 401, reader 403 on write, rep 403 on merge and unmerge, admin merge and unmerge and link work, audit chain verifies, a `--worker` container stays up). Phase 2 found a real bug: a `--worker`-only process never ran `mail::init`, so it sent campaigns with the simulated sender and marked them delivered. Fixed by `init_sending` called from `connect_workers` as well as `after_routes`; the script passes 27 of 27 on a local build with the fix; the released v0.1.3 image passes 24 of 27. Released as v0.1.4 (2026-10-21 image smoke: 27 of 27), with a DB-backed regression test (`a_worker_only_process_sets_up_the_real_sender`) that fails without the fix. Limits: the release image is amd64 only; phase 2 runs in development mode because production refuses unverified SMTP certificates.* Traces to CRM-T39
- [x] CRM-T73 **Edit and reopen a deal on its own page.** *Done 2026-10-21: `/deals/{pid}` has an Edit form (name, amount, expected close) that sends only what changed; a masked amount is disabled and a closed deal shows why it cannot be edited. A closed deal can be reopened from the same page with a reason (`POST /deals/{pid}/reopen`). Tests: three demo-data Playwright, two live; nine new strings in eight locales.*
- [x] CRM-T74 **Multi-arch release images.** *Done 2026-10-21, proven by the pre-release `v0.1.5-rc.1`: `.github/workflows/release.yml` builds each image on a native runner per architecture (`ubuntu-latest`, `ubuntu-24.04-arm`), pushes `-amd64` and `-arm64` tags, and a `manifest` job joins them into one image under the version tag and signs and attests the index digest. Both published images are OCI indexes with linux/amd64 and linux/arm64; the service's arm64 image passes `scripts/image-smoke.sh` 27 of 27 natively, and `gh attestation verify` on the tag exits 0. Then verified for the stable `v0.1.5` (2026-10-21): `cosign verify` (cosign 3.1.3, keyless, the release workflow identity) passes for both images at the index digests (service `sha256:8579252e…`, ui `sha256:6d20ba20…`), `gh attestation verify` exits 0, and the smoke script passes 27 of 27 on the arm64 half. The UI image now has `scripts/ui-image-smoke.sh` (23 checks: hardened flags, non-root, version label, every security header, a static file, a 404, the tour off and on, a demo edit that sticks); it passes on the pulled v0.1.5 UI image (arm64). Not checked: the amd64 half of either pulled image. Limits: the `-amd64` / `-arm64` tags stay visible in the registry; the UI smoke script does not run the UI against the service.*


## Code quality (from the 2026-10-21 research, [plan.md](plan.md) Q1 to Q20)

- [x] CRM-T75 **Lint gates in the manifests.** Add `[lints.rust] unsafe_code = "forbid"`, and add `[lints.clippy] pedantic = "warn"`, `unwrap_used = "warn"` and `expect_used = "warn"` to the service. Each existing justified `expect` gets an `#[allow]` with its reason; `dashboards.rs:74` gets a typed struct instead. Fix or allow the 146 nursery and restriction warnings. Done when CI fails on a new pedantic warning. (Q1, S)
      **Resolution (2026-10-21, not released):**
      - **Service:** `unsafe_code`, pedantic, and missing docs were already enforced by crate attributes in `src/lib.rs` and `src/bin/main.rs` (the audit looked only at `Cargo.toml`). Added `clippy::unwrap_used` and `clippy::expect_used` as warnings (errors in CI under `-D warnings`), allowed in tests.
      - **The 13 existing `expect`s:**
        - 11 are impossible failures (HMAC key length, AES-GCM on in-memory data, Prometheus registration, serde on a `Value`); each now carries `#[allow(…, reason = "…")]`.
        - `dashboards.rs` pipeline-by-stage: the `.as_object_mut().expect("object")` on a JSON value is gone. The totals are held in typed maps and turned into JSON at the end, with the same output.
        - `break_glass.rs`: two `ensure_valid(&[m]).unwrap_err()` calls became a new `invalid(&str)` helper in `controllers/mod.rs`.
      - **Matcher:** `[lints.clippy]` gains pedantic, `unwrap_used` and `expect_used`; a `clippy.toml` allows both in tests. The library code needed no change.
      - **Tests:** clippy `-D warnings` clean on the matcher, and on the service for the default + keycloak and the keycloak-only builds. 184 service unit tests, 59 DB-backed tests and the Redis tests pass; the matcher's 4 test targets pass.
      - **Not done:** the 146 nursery and restriction warnings (indexing may panic, doc first-paragraph length, `const fn`); those are opinionated lints and are left off on purpose.
- [x] CRM-T76 **Remove write-path boilerplate.** One `audited_tx` helper (begin, mutate, audit, emit, commit); `Entity::find_live()` or a `SoftDelete` trait for the 79 `DeletedAt.is_null()` filters; a `Path<Pid>` extractor in place of the 51 `parse_pid` calls. No behaviour change, and the DB suite and the audit chain stay green. (Q2, M)
      **Resolution (2026-10-22, not released), two of the three helpers; behaviour unchanged:**
      - **`find_live()`** (`src/models/live.rs`, a `FindLive` trait over the 19 entities with a `deleted_at`): replaces `Entity::find().filter(Column::DeletedAt.is_null())` at 34 sites and in the `live!` insight macro. 2 tests check the SQL.
      - **`PidPath`** (`controllers/mod.rs`): the `{pid}` path parameter parsed as a UUID, `404` otherwise, exactly as `Path<String>` plus `parse_pid` did. 40 handlers use it; 3 `parse_pid` calls are left in handlers and one inside `PidPath`; the 3 take the id from a body or a query. 1 test (a UUID, and three non-UUIDs that answer 404).
      - **Checks:** clippy clean on both feature sets, 190 unit tests, all 59 DB-backed tests. One DB run had 3 email-delivery failures while the machine was busy; they pass alone and the full run passed again.
      **Not done, on purpose:**
      - **`audited_tx`.** A wrapper taking the mutation as a closure does not fit: the 46 transactions do different things between `begin` and `commit` (read, then lock, then write, then audit with different data, then emit or not), and a closure borrowing the transaction across an `await` adds lifetime noise without removing the real duplication. The duplication that is cheap to remove is the finder and the id parse, done above.
      - **The other 45 `DeletedAt.is_null()` filters:** most come after other filters in the chain, or on `update_many`/`delete_many` (erasure), where `find_live()` does not apply. They are left as they are rather than reordered.
- [x] CRM-T77 **A service layer for domain operations.** Move lead conversion, deal opening, merge and unmerge out of their handlers (`sales.rs:255`, `merge.rs:347`, `unmerge.rs:248`) into functions that take `&impl ConnectionTrait`. Split `idempotency_mw` (185 lines) and `insight_cache_mw`. Handlers only extract, call and respond. (Q3, L; after T76)
      **Resolution (2026-10-22, not released), no behaviour change:**
      - **`merge`** 113 to 63 lines and **`unmerge`** 165 to 55: split into named phases in the same files (`lock_rows`/`pick_pair`, `authorize_destructive` shared by both, `record_merge`; `open_ledger`, `lock_for_unmerge`, `check_unmergeable`, `bring_back`, `record_unmerge`). The order of checks is unchanged.
      - **`lead_status`** 129 to about 60: `contact_for_conversion` and `open_deal_for_conversion`.
      - **`idempotency_mw`** 185 to about 80: the SQL moved to `src/idempotency_store.rs` (`claim`, `lookup`, `record`, `forget`), and `answer_for_existing` builds the replay or conflict response.
      - **`insight_cache_mw`**: `insight_key` (pure, now unit-tested for what the key depends on) and `bump_cache_generation` extracted.
      - Four `#[allow(clippy::too_many_lines)]` removed (merge, unmerge, lead_status, idempotency).
      - **Tests:** clippy clean on both feature sets, 191 unit tests, all 59 DB-backed tests (they cover merge, unmerge, idempotency and the cache), and `scripts/chaos.sh` 71 of 71.
      **Not done:** the helpers take `&DatabaseTransaction`, not a generic `&impl ConnectionTrait` (they all run inside the caller's transaction, so nothing needs the generality); the other long functions (`cadence` 101 lines, `executive` 100, `deliver_one` 99) are insight or delivery walks, not operations, and keep their allows or sit just under the limit; there is no separate service module: the phases stay beside their handler, which keeps each task's change in one file.
- [~] CRM-T78 **Typed settings in place of process-global environment reads.** Parse one `Settings` struct at boot and keep it in `AppContext.shared_store`, replacing about 17 `OnceLock`s and 29 `env::var` calls. Done when the enforcement tests no longer need their own process or `set_var`. (Q4, M/L)
      **Done 2026-10-22 (not released), no behaviour change:**
      - **`src/settings.rs`:** one typed `Settings` with 22 fields, parsed by `Settings::from_lookup` from any `name -> value` function (pure; 8 unit tests with no environment). The process holds one snapshot: `settings::current()`, built from the environment on first use, replaced by `install` or `update`.
      - **Moved onto it:** `require_auth`, `protect_operational`, `allow_insecure`, `require_field_encryption`, `trust_forwarded_for`, the upstream mode, the event transport, retention days, the mail batch, the webhook and Mailgun secrets, the dev mail domain, the PASETO key sources, issuer and audience, the ABAC policy sources, the field-key sources and the Redis URL. Four `OnceLock` flags and about 22 `env::var` calls are gone.
      - **Tests:** the enforcement test and the webhook, Mailgun and mail-batch tests in `email_delivery.rs` now call `settings::update` instead of `unsafe set_var` (14 of the 24 `set_var`/`remove_var` calls in tests). 199 unit tests and all 59 DB-backed tests pass, with the enforcement test.
      **Left on purpose, with the reason:**
      - Startup-only readers that already take a lookup function: mail, unsubscribe links, the rate limit. The mail-worker regression test still sets them in the environment, as a deployment would.
      - `CRM_KEYCLOAK_*`: the `authentication-verifier` library reads them (`KeycloakSettings::from_env`), and `keycloak_guard.rs` sets them.
      - The per-service base URLs, whose variable name depends on the record kind.
      **Not met, and what it would take:** "the enforcement tests no longer need their own process". The verifier and the ABAC policy are still `OnceLock`s built on first use, so a second test with different keys in the same process would see the first one's. Making them replaceable (the way `settings` and the mail sender already are) is a separate change to `auth/`; until then the enforcement suite stays its own test binary.
      **Limits:** the settings are read once per process (a snapshot), as the flags always were; the webhook secret, Mailgun key, retention days and mail batch used to be read on every call, which only a test could notice. A bad value still falls back to its default silently rather than stopping the start.
- [~] CRM-T79 **Typed errors and per-field 422s.** Use `thiserror` enums in place of the 27 `Result<_, String>`. `ensure_valid` emits RFC 9457 `errors: [{pointer, detail}]`. Each error source builds a `Problem` directly; the reshaping middleware stays only as a safety net. Update OpenAPI and the UI's error display. (Q5, M)
      **Done 2026-10-22 (not released):**
      - **Per-field 422s.** A validation `422` now carries `errors: [{ "pointer", "detail" }]` (RFC 9457 §3.2) beside the unchanged `description`/`detail`, which still join the messages with `"; "`. `Problems` records the member each problem is about (`push_field`), `Problems::finish()` builds the response (33 call sites moved from `ensure_valid(&problems.into_vec())`), and `pointer` is a JSON Pointer such as `/display_name`. A problem about the request as a whole has no `pointer`. For a member of a list the check does not know which entry, so `stages[].name` is `/stages/name`.
      - **Body rejections** (`json.rs`): the parser's own path becomes the pointer, so `stages[2].probability_percent` is `/stages/2/probability_percent`, and a missing field is a member of its parent. A message with no path has no pointer.
      - **OpenAPI:** the shared `Error` schema documents `errors`.
      - **Typed errors, where the type earns its keep:** `GrantError` and `ReviewError` (break-glass; the handler no longer compares subject and reviewer to pick `403`, it matches `ReviewError::SelfReview`), `TransitionError` (`lifecycle::check`, used by lead, campaign, ticket and article transitions) and `PartnershipError`. `thiserror` 2 is now a direct dependency (already in the tree). Their `Display` text is unchanged, and tests pin it.
      - **Tests:** 2 request tests (each problem names its member; a whole-request problem has no pointer), the misspelt-member test also checks the pointer, 9 unit tests for the pointer rules and the typed errors (203 unit tests and 61 DB-backed tests pass).
      **Left out, and why:**
      - **The other `Result<_, String>` signatures** (about 12): start-up configuration (mail, provider credentials, unsubscribe, field keys, version, idempotency key) and analytics arithmetic. They are formatted once and logged or returned as a message; no caller branches on them, so an enum would only rename the string.
      - **"Each error source builds a Problem directly; keep the reshaping middleware as a safety net."** Not done. The auth, abuse-guard, version and idempotency layers still answer in their own shapes and `problem_mw` rewrites them; changing four middlewares' output for no client-visible gain was not worth the risk.
      - **UI:** the UI still shows `detail`. It could mark the field named in `errors`; that is a separate change.
- [~] CRM-T80 **Aggregate insights in SQL.** Replace the 23 unbounded `live!` loads in `controllers/insights/*` with `GROUP BY`/`SUM` queries, and cap or page the related lists in contact detail (`relationships.rs:262-272`). A test with more than one page of seeded rows proves the totals stay exact, as CRM-T51 did for the activity dashboard. (Q6, M)
      **Done 2026-10-22 (not released), same answers, fewer rows read:**
      - **`src/controllers/insights/rollup.rs`:** SQL digests of the two tables that grow without bound. `touches` (latest activity and open dated follow-ups per subject) feeds cadence, members, stakeholders and pipeline hygiene; `engagement_counts` (grouped by recorder, month, kind and sentiment, inside the window) feeds engagement; `consent_events_by_source`, `withdrawals_since` and `withdrawals_by_account` feed the DPO and consent-by-account views. Eight whole-table loads of `activities` and `consent_events` are gone, and so is the decryption of every activity summary on the way.
      - **Pipeline hygiene** was quadratic (it scanned every activity once per deal and lead); it now looks each touch up in a map.
      - **Sales dashboard:** one grouped query replaces loading every open deal and then one stage lookup per deal (an N+1).
      - **Contact page:** activities, deals and tickets are each cut at 200, newest first, and the answer gains `truncated: { activities, deals, tickets }` so a cut is never silent. Activities were already cut at 200 with no sign; deals and tickets were unbounded. OpenAPI documents it.
      - **Measured, not end to end:** the touches query over 200,000 synthetic activities on 5,000 contacts: 17 ms in PostgreSQL, 5,000 rows returned (rolled back; the old path fetched and decrypted all 200,000). The 5,000-row figure is the number of subjects, so the work now grows with the number of contacts, not the number of activities.
      - **Tests:** `tests/requests/insight_rollups.rs` has 3 DB tests (the activity views over a fixture with known touch dates; the consent views; the 200-row cut). The first two were written and passed against the old code before the change, then passed unchanged after it. 203 unit and 64 DB-backed tests pass.
      **Left as loads, on purpose:** contacts and accounts (the views list a row for each, so the work is the output; paging would change the API), pipeline stages, memberships and partnerships (small registers), and the executive pack's deals (windowed in Rust; its remaining cost is bounded by deals, not activities).
      **Limits:** a deal whose stage was soft-deleted is now counted in the sales dashboard (the old lookup refused it); the per-subject digest still reads the whole `activities` table inside the database (a sequential scan, parallel), which an index on `(deleted_at, subject_kind)` would not help with at this width.
- [x] CRM-T81 **Bounded display-name cache.** Add a time limit and a size cap (`moka`, or the Redis cache) to `src/clients.rs:50-80`, and state the retention in the privacy notes. (Q7, S)
      **Resolution (2026-10-21, not released):** `NameCache` in `src/clients.rs`: each name is kept for 10 minutes (`NAME_TTL`) and at most 10,000 are held (`NAME_CAPACITY`). When full, expired names go first, then the oldest. No new dependency (`moka` was not needed for this size). The clock is passed in, so the 3 new tests (expiry, eviction order, replacing a name in a full cache) do not sleep. Retention is written in `spec/integrations.md`. 187 unit tests and the 59 DB-backed tests pass.
      **Limits:** per process, so replicas each keep their own copy; the 10 minutes and 10,000 are constants, not settings; an erasure upstream shows here within 10 minutes, and nothing evicts a name sooner; eviction scans the map (10,000 entries, only when full).
- [x] CRM-T82 **Tests for the middlewares and the largest controllers.** Move the six middlewares out of `app.rs` into their own modules with `tower::ServiceExt::oneshot` unit tests. Add request tests for `relationships`, `break_glass`, `streaming` and `rate_limit`. (Q8, M)
      **Done 2026-10-23 (not released), no behaviour change:**
      - **`src/middleware/`:** the six middlewares (`require_auth`, `insight_audit`, `idempotency`, `abuse_guard`, `no_store`, `insight_cache`) moved out of `app.rs`, which is now only the `Hooks` wiring.
      - **Tests:** 12 `tower::ServiceExt::oneshot` unit tests (no database): the 429 and its `RateLimit-*` and `Retry-After` headers, probes never throttled, the failed-auth lockout, a limit of 0 switching the guard off, `no-store` and a handler's own policy kept, and the four idempotency answers (replay, different request, still running, vanished).
      - **Request tests (Postgres):** `tests/requests/relationships.rs` (4: create/list/fetch, 422 with a pointer, 404 and malformed ids, accounts, task-only `done`), `tests/requests/streaming.rs` (an event per committed write, newest first, none for a refused write), `tests/outbox.rs` (the event is an outbox row; own binary) and `tests/rate_limit.rs` (budget headers, 429, probes spared; own binary, since the limit is read at boot).
      - **`break_glass`:** already covered end to end by `tests/enforcement.rs` (grant, list, self-review refused, one review), so no new file.
- [~] CRM-T83 **OpenAPI from the types.** Use typed response structs with `utoipa` (or `schemars`) derives in place of the 129 `json!` responses, and generate the document from them. Keep the checked-schema tests as the proof. (Q9, L; after T79)
      **Done 2026-10-23 (not released), request schemas only:**
      - **`schemars` 1** derives `JsonSchema` on 36 request payload structs (`relationships`, `sales`, `board`, `support`, `engagement`, `merge`, `unmerge`, `external_links`, `break_glass` and the marketing controllers). `openapi_schemas::generated()` builds their `components.schemas` entries, so a request member and its documentation cannot drift; the 36 hand-written entries are gone. Doc comments become descriptions, `deny_unknown_fields` becomes `additionalProperties: false`, and a defaulted member is optional.
      - **Renames kept:** `sales::StagePayload` is `StageInput` and `external_links::LinkPayload` is `ExternalLinkPayload`, as the paths already referred to them. The segment `filter` is documented as a plain object.
      - **Tests:** a unit test pins `ContactPayload` (required members, unknown members refused, defaults), another proves every `$ref` in the document resolves; the checked-schema request tests pass unchanged (69 request tests).
      **Resources (2026-10-23):** the SeaORM models for `Contact`, `Account`, `Activity`, `Lead`, `Deal`, `Ticket`, `Article`, `Campaign`, `Segment`, `SlaPolicy` and `EmailDelivery` derive `JsonSchema` (renamed to the schema names the paths use), and their hand-written entries are gone. Masking replaces a value (text becomes `•••`, an amount becomes `null`) and never removes the member, so the model's shape stays true; the two masked amounts (`deals.amount_minor`, `campaigns.cost_minor`) are documented as nullable, and `EncryptedText` is a plain string. This replaced the plan of a view struct per resource: no view structs are needed while masking only substitutes values.
      **Responses, `relationships` (2026-10-23):** `ContactDetail`, `AccountDetail` and `ContactDuplicates` (with its entries, fields and `truncated`) are typed structs that the handlers build and that generate their schemas; rows that masking rewrites stay JSON values with a `#[schemars(with = …)]` naming their model. The remaining wrapper responses (merge, unmerge, external links, sales, support, marketing, engagement, dashboards, insights) are still `json!` and hand-written.
      **Responses, merge, unmerge and external links (2026-10-24):** `ContactMerge`, `ContactUnmerge`, `ContactMerges` and `ExternalLinks` are typed structs built by their handlers, and the `ExternalLink` model derives its own schema; five hand-written entries are gone. Still `json!`: `sales`, `support`, `marketing`, `engagement`, dashboards and insights.
      **Responses, `sales` (2026-10-24):** `LeadDetail`, the pipeline list (`Pipeline`) and `Forecast` are typed structs; the score breakdown, the pipeline and stage models and the forecast snapshot model derive their own schemas (a masked forecast or snapshot total is `null`, documented as nullable). Seven hand-written entries are gone. Still `json!`: `support`, `marketing`, `engagement`, dashboards and insights.
      **Responses, `support` and `engagement` (2026-10-24):** `TicketDetail`, `SlaSweep` and the group read (`Group`, with its roster) are typed structs, and the working-group model derives its schema (named `WorkingGroup`); three hand-written entries are gone. Still `json!`: `marketing`, dashboards and insights.
      **Responses, `marketing` and the activity dashboard (2026-10-24):** `Preview`, `NurtureSweep` (its schema had left out `held`, which the handler always returned), `Deliveries`, `CampaignFunnel` and `ActivityDashboard` are typed structs, and `Ratio` derives its schema. A masked funnel is the same JSON as before (revenue and ROI `null`, campaign cost `null`). Six hand-written entries are gone. Still `json!`: the sales and SLA dashboards, the insight views, privacy, retention, break-glass and audit responses.
      **Responses, privacy, retention, break-glass and audit verify (2026-10-24):** `Erased`, `RetentionReport`, `RetentionSweep`, `Grant`, `Grants` (with `GrantRecord`), `Reviewed` and `ChainVerdict` are typed structs; eight hand-written entries are gone. The grant list keeps its members nullable, as the handler reads each column with `.ok()`. Still `json!`: the sales and SLA dashboards, the subject-access export, the insight views and the event and audit-entry shapes.
      **Left to do:** the `StatusPayload` shared by four different status bodies (campaign, ticket, article, partnership stage); the response side (the 129 `json!` responses and the hand-written `view_schemas`, `base_schemas` resources and the path table in `openapi.rs`). Responses are masked as `serde_json::Value` maps, so they need typed structs first; that is the larger part and is done one controller at a time.
- [x] CRM-T84 **Dependency cleanup.** Drop `tracing-subscriber` and the `sqlx-sqlite` feature. Move to reqwest 0.13 to end the duplicate. Make the service depend on the matcher by path plus version, so service CI tests the matcher in the repo. Check the duplicate count in `cargo tree -d` before and after. (Q10, S)
      **Resolution (2026-10-21, not released):**
      - **Removed:** `tracing-subscriber` (no references, so `tracing-log` left the lockfile too).
      - **SQLite:** `sqlx-sqlite` is now a dev-dependency feature. loco's `tests_cfg::app::get_app_context` (used by the OpenAPI route test) opens an in-memory SQLite connection, so tests still need it; the release binary no longer links the driver.
      - **reqwest 0.12 to 0.13:** 0.13 puts `.form()` behind a `form` feature, so that feature is now enabled. 0.13's `rustls` feature verifies certificates against the operating system's trust store, where 0.12's `rustls-tls` used bundled webpki roots. That affects the email-provider and upstream calls; the verifier's key fetch already worked this way in the image. The distroless base image has CA certificates.
      - **Matcher:** the service now depends on matcher 0.1.2 from crates.io, not a path dependency, because the image builds from the service directory alone. `scripts/ci-check.sh test` also runs the service's unit tests with a `--config patch.crates-io…path` override against the matcher in this repository, and puts `Cargo.lock` back afterwards (the patch rewrites it).
      - **Duplicates:** `cargo tree -d -e normal` printed 65 lines before and 63 after.

      **Tests:** clippy `-D warnings` is clean on the default + keycloak and the keycloak-only builds. 184 unit tests pass, also against the local matcher. All 59 DB-backed tests and the Redis tests pass, run against PostgreSQL 18 and Redis from `compose.test.yaml`.

      **Limits:** reqwest 0.12 stays in builds with the keycloak feature, because `axum-keycloak-auth` (pulled in by `authentication-verifier`) still depends on it. The image was not rebuilt or smoke-tested.
- [x] CRM-T85 **Remove dead configuration.** Delete the `auth.jwt` blocks and `JWT_SECRET` (Loco's JWT feature is off), and the `deny.toml` `allow-git` entry. Fix or remove the 12 links to the missing `agents/share/*.md`. Move the literal secrets in `development.yaml` and `test.yaml` behind clearly-fake markers. (Q11, S)
      **Resolution (2026-10-22, not released):**
      - **JWT config:** the `auth.jwt` blocks are gone from `production.yaml`, `development.yaml` and `test.yaml`, which removes the two committed literal secrets and the production `JWT_SECRET` requirement. Checked: the production config loads with `JWT_SECRET` unset (`doctor --config`), and the 187 unit tests and 59 DB-backed tests pass without the blocks.
      - **`deny.toml`:** `allow-git` is empty; `cargo deny check sources` passes.
      - **Already-released images** (up to v0.1.5) still demand `JWT_SECRET` because their config template has the block; the runbook and `scripts/image-smoke.sh` say so, and the smoke script keeps passing the variable.
      - **`agents/share/*.md` links:** left in place. They are the family's shared documents in the `main-x-service` repository, which exists beside this one on the maintainer's machine; `spec/index.md` now says so. Deleting 12 references to real documents would lose information.
      **Limits:** the variable can be dropped from deployments only once an image built from this change is released.
- [x] CRM-T86 **Fix text copied from another project.** Correct the UI `package.json` description ("ward whiteboards, bed management") and decide the UI licence against the crates' list. Replace `brand.name` "Main X · CRM" (×8 locales and two page titles), the `mxi.crm.locale` storage key (migrate the old key once), and "WPM" in `clients.rs`. (Q12, S)
      **Done 2026-10-21:**
      - **UI description:** the `package.json` description now names sales, marketing, support and analytics, in place of the hospital text.
      - **Comments:** `clients.rs` names CRM-D11 and the CRM, not WPM. The event-kind comment in `streaming.rs` lists the CRM's own entities and kinds, not WPM's employee and payroll ones.

      **Kept on purpose:** "Main X" is the product family's name, not a copying mistake. The token audience is `main-x-service`, and the `mxi.crm.*` storage keys were kept deliberately in earlier changes. So `brand.name` and the storage keys stay, as do the comments that compare with WPM on purpose (`rules/privacy.rs`).

      **Licence (maintainer chose the five-licence expression, 2026-10-21):** the UI `package.json`, the UI image label and `LICENSE.md` now carry the same expression as the crates. Only the files changed; no release carries it yet.
- [~] CRM-T87 **Matcher API stability.** Mark `Field`, `Outcome`, `Verdict`, `MatchConfig` and `FieldParameters` `#[non_exhaustive]`, and make `ConfigError` an enum. Add `cargo semver-checks check-release` to CI. These are breaking changes, so they go out as 0.2.0 with a changelog entry. (Q13, S/M)
      **Done 2026-10-21 (not released; the version is still 0.1.2):**
      - **`ConfigError`** is a `#[non_exhaustive]` enum with five variants; its `Display` text is unchanged. A test (`a_refusal_says_which_setting`) pins each variant.
      - **`#[non_exhaustive]`** on `Field`, `Outcome`, `Verdict`, `MatchResult` and `FieldScore`.
      - **Kept constructible, on purpose** (README, "Changing the API"):
        - `Contact`, `MatchConfig` and `FieldParameters` are built with `..Default::default()`, which `#[non_exhaustive]` forbids.
        - `Ranked` is built by the service to re-sort and re-score candidates. The service's unit tests against the matcher in the repo caught this (E0639) before anything was published.
      - **`scripts/ci-check.sh semver`** runs `cargo semver-checks`. It is not in the workflows yet (CRM-T94).
      - **Verified:** cargo-semver-checks 0.51.0 against the published 0.1.2 reports exactly three failures, all intended (enums marked non-exhaustive, structs marked non-exhaustive, `ConfigError` became an enum) and "requires a new major version"; with `--release-type major` it passes. Matcher clippy and tests pass; the service's 184 unit tests pass against the local matcher.

      **Needs a decision to finish:** release as 0.2.0. That means bumping the matcher, publishing it, then moving the service's dependency to `0.2.0` and releasing the service. Not bumped now because the service's CI pins `0.1.2`.
      **Limits:** the cargo-semver-checks installed here (0.47) cannot read Rust 1.99's rustdoc output; 0.51 or later is needed, so CI must install that.
- [x] CRM-T88 **Matcher benchmarks and gates.** Add a criterion bench for `compare_one_to_many` at 1k, 5k and 50k candidates, recorded in `BENCHMARKS.md`. Add pedantic lints, and `cargo audit`/`deny` for the matcher. (Q14, S/M)
      **Resolution (2026-10-21):** `benches/matching.rs` (criterion 0.8, a dev-dependency only) measures one pair and `compare_one_to_many` at 1,000, 5,000 and 50,000 synthetic candidates; results, hardware and limits are in `BENCHMARKS.md`. About 2.7 µs a pair and 1.5 µs a candidate in a batch, linear. Pedantic lints and the matcher's `cargo audit`/`cargo deny` landed earlier (CRM-T75, CRM-T94). **Limits:** one run on one machine; it is not in CI, because timings on shared runners are noisy.
- [ ] CRM-T89 **One file per locale, loaded on demand.** Split the `STRINGS` map in `i18n.svelte.ts` into one module per locale loaded with `import()`. Keep the parity test. Record bundle size before and after. List the values identical to English for native reviewers. Reconcile the root `locales/` directories (11) with the UI's 8. (Q15, M)
- [ ] CRM-T90 **Page data through `load`.** Move the `$effect` fetches into `+page.ts` `load` functions (or one resource helper with `AbortController`). Add an `errorMessage(cause)` helper to replace the 36 copies. A test proves a fast parameter change cannot show stale data. (Q16, M)
- [x] CRM-T91 **Finish the SvelteKit migration.** Close or strike the 10 boxes in `MIGRATION_TASKS.md`. Make required variables in `src/env.ts` throw instead of falling back to `""`. Move every import to `#lib` and drop the deprecated `config.alias`. Delete the file when done. (Q17, S)
      **Resolution (2026-10-21):** every listed finding turned out not to apply:
      - `+layout.svelte` only reads `page.url`; it never mutates it.
      - Both `goto` calls go to internal routes (`/contacts`, `/contacts/{pid}`).
      - Nothing calls `invalidateAll`.
      - Nothing depends on CORS for static assets.

      **Imports:** every import already used `#lib`, so `alias: { $lib: "src/lib" }` was removed and `svelte-check` no longer warns.

      **Environment:** the variables in `src/env.ts` keep the `""` fallback on purpose. `server/config.ts` turns `""` into the localhost defaults, and the tour needs the UI to start with nothing set. The `@migration-task` comment now says so. Making them throw would break that.

      **Done:** `MIGRATION_TASKS.md` deleted. Checks: `svelte-check` 0 errors and 0 warnings, vitest 114, build, prettier, Playwright 105.
- [~] CRM-T92 **Make the demo tour opt-in and per visitor.** `CRM_DEMO_TOUR=on` turns it on (it is on today unless set to `off`). Use a random demo session per sign-in in place of the constant `DEMO_SESSION`, and cap the demo state. Update the image smoke test (it checks the tour off and on) and the runbook. This is a behaviour change, so record it in the changelog. (Q18, S)
      **Per visitor, done 2026-10-21 (not released):**
      - **Sessions:** each tour sign-in gets `demo-tour-<random uuid>` and its own copy of the data (`startDemoSession`, `demoHandleFor` in `src/lib/server/demo.ts`). At most `MAX_DEMO_SESSIONS` (200) copies are kept; the least recently used goes first. Sign-out forgets the copy.
      - **Tests:** 4 vitest tests (ids differ, one visitor's write does not reach another, the cap, sign-out).
      - **Docs:** the UI changelog and INSTALL.

      **Found on the way and fixed:** with the authentication service unreachable, `requestMagicLink` threw and the sign-in page went blank instead of saying it failed; `signout` threw the same way. Both now fail softly (3 vitest tests in `tests/unit/auth.test.ts`). The existing e2e test `another address is not the tour` caught this locally; it had not been caught before because Playwright does not run in CI (CRM-T94). vitest 114 of 114; Playwright 105 of 105.

      **Not done, left to the maintainer:** making the tour opt-in. The code says on purpose that it is on unless `CRM_DEMO_TOUR=off`, so changing the default is a product decision. A fixed session id was not a security hole: anyone can get a demo session by signing in as `example@example.com`, and demo sessions never reach the service.

      **Limits:** copies live in one process's memory, so several UI replicas each keep their own and a visitor moved to another replica starts again; a visitor can create copies up to the cap by signing in repeatedly, which evicts other visitors' copies.
- [ ] CRM-T93 **UI component tests and a generated API client.** Add `@testing-library/svelte` tests for `DataTable`, `Board` and `LinksPanel`. Generate types from the service's OpenAPI with `openapi-typescript`, and check them against the 911-line `api/crm.ts`. Fix or remove the vitest `deps.inline` regex that matches nothing. (Q19, M; types after T83)
- [~] CRM-T94 **CI gates and hygiene.**
  - Add coverage (`cargo llvm-cov`, vitest `--coverage`) as a report first, and set floors later.
  - Add a scheduled `cargo mutants` run on `rules/`.
  - Add ESLint with `eslint-plugin-svelte`, plus `typos`, markdown lint, `shellcheck`, and `actionlint` or `zizmor`.
  - Run the Playwright e2e and a11y suites in CI.
  - Compile the `--no-default-features --features keycloak` build.
  - Add a top-level `permissions: contents: read` to `ci.yml`.
  - Use Node 26 in Woodpecker's supply-chain step.
  - Audit the matcher.

  (Q20, M)

      **Done 2026-10-21 (not yet run by GitHub Actions or Woodpecker):**
      - **Playwright in CI:** a new `e2e` stage in `scripts/ci-check.sh`, a GitHub job and a Woodpecker step. Passes locally, 105 of 105.
      - **Keycloak-only build:** `--no-default-features --features keycloak` is now in the `clippy` stage (clean locally).
      - **shellcheck:** a `scripts` stage (warnings and errors only). Fixed what it found: four `cd` without `|| exit`, an unused `C3` and an unused `START`.
      - **Matcher supply chain:** `cargo audit` and `cargo deny` run on the matcher too, with its own `deny.toml` (the licences it uses today). Both pass.
      - **`permissions: contents: read`** at the top of `ci.yml`.
      - **Woodpecker supply-chain step:** Node 26 (was 24) and pnpm from npm, since Node 25+ has no corepack. The nodesource `setup_26.x` URL answers 200; the step itself has not run.
      - **`semver` stage** from CRM-T87 exists in the script but is in neither workflow: it fails by design until the matcher version is bumped.

      **Not done, and why:**
      - **Coverage** (`cargo llvm-cov`, vitest `--coverage`): the tool is not installed here, and a report-only job is best added when its output can be seen.
      - **Mutation testing:** `cargo-mutants` is installed, but a run over `rules/` needs the database suite and takes long; a scheduled job should be set up and watched once.
      - **ESLint** with `eslint-plugin-svelte`: new dev dependencies and a lockfile change, and its rule set is a team choice.
      - **`typos`, markdown lint, `actionlint`/`zizmor`:** none installed here; each needs its first run reviewed for noise before it can gate.
      - The two new workflow jobs and the Woodpecker steps have not run on their providers; expect a first-run fix.

## New capabilities (from the 2026-10-21 research, [plan.md](plan.md) C1 to C10)

Each task first writes its requirement (proposed CRM-R28 onward) and design note, then lands spec, code and tests together.

- [ ] CRM-T95 **Outbound webhooks.** A relay job reads `event_outbox` and delivers to subscriptions that admin or service callers manage.
  - **Security:** https only, with SSRF guards that refuse private and link-local addresses. Signed in the Standard Webhooks format (`id.timestamp.body`, several signatures while a secret rotates). Payloads carry ids, never contact values.
  - **Delivery:** retries with backoff, a dead-letter state, a delivery log, and a 410 that disables the subscription.
  - **Audit and erasure:** every subscription change is audited. An erasure cancels queued deliveries for that contact.
  - **Tests:** signing, retry and dead-letter, the SSRF refusals, and erasure.
  - **Sources:** https://docs.espocrm.com/administration/webhooks/ and https://www.svix.com/resources/glossary/webhook-signature/
  - **Limits to state:** at-least-once delivery, so receivers must dedupe on the event id.

  (C1, M)
- [ ] CRM-T96 **Consent per purpose and double opt-in.**
  - **Model:** consent is held per purpose (for example newsletter, events, product updates), in place of the single `marketing_consent`. A new `pending` state never allows a send.
  - **Double opt-in:** a confirmation token moves `pending` to `granted`, and the event records its source and date.
  - **Preference page:** a signed-token page lets the subject change each purpose.
  - **Merge and unmerge:** the stricter-side rule (CRM-R24) applies per purpose.
  - **Migration:** existing rows become one default purpose with their history kept.
  - **Tests:** the send gate for each state, the merge cases per purpose, and the migration against populated data (`scripts/migrate-with-data.sh`).
  - **Source:** https://knowledge.hubspot.com/marketing-email/manage-your-marketing-hub-starter-email-account-settings

  (C2, M)
- [ ] CRM-T97 **MCP server.**
  - **Design:** a thin MCP layer over the existing endpoints, authenticated with the caller's own token and decided by the same ABAC and masking. Tools are read-only by default.
  - **Destructive tools:** merge, unmerge and erase are offered only as `dry_run` plus a confirmation that a person makes in the UI.
  - **Audit:** every call is audited with an `agent` marker.
  - **Prompt injection:** record text returned to the agent is marked as data, not instructions.
  - **Tests:** a reader cannot write through MCP; a masked caller gets masked values; the audit rows.
  - **Sources:** https://docs.twenty.com/user-guide/ai/capabilities/mcp and https://developers.hubspot.com/changelog/remote-hubspot-mcp-server-is-now-generally-available
  - **Limits to state:** MCP spec versions move quickly; name the version supported.

  (C3, M)
- [ ] CRM-T98 **Web form lead capture.**
  - **Endpoint:** a public `POST` that creates a lead with `source=web` and consent `pending` (needs T96).
  - **Abuse protection:** rate limiting per form and per client, a honeypot field, and an origin check.
  - **Person record:** created or resolved through the person service, since the CRM stores no email.
  - **Optional nurture enrolment:** starts only after opt-in is confirmed.
  - **Tests:** abuse limits, no send before confirmation, and the audit row.
  - **Source:** https://www.pipedrive.com/features/web-forms

  (C4, M)
- [ ] CRM-T99 **Tasks, reminders and a notifications inbox.**
  - **Tasks:** task activities gain an assignee (`worker:` URN) and a reminder time.
  - **Notifications:** a `bg_pg` job writes notification rows for due reminders and SLA breaches. The UI gets an unread inbox, with mark-read.
  - **Access:** a notification is created only if the recipient can read the record (ABAC is checked at write time and again at read time).
  - **Tests:** reminders fire once, notifications for unreadable records are suppressed, and the labels exist in 8 locales.
  - **Source:** https://knowledge.hubspot.com/sequences/complete-your-sequence-tasks

  (C5, M)
- [ ] CRM-T100 **Server-side search.**
  - **Endpoint:** `GET /api/search?q=` over contacts, accounts, deals, leads, tickets and articles, using `tsvector` with `unaccent` and `pg_trgm` for fuzzy matches. Results are filtered by ABAC and masking, and the query is audited as a count, not as text.
  - **UI:** the CRM-R22 search uses it when the service is present.
  - **Tests:** accents, typos, a masked caller, and paging.
  - **Source:** https://www.postgresql.org/docs/current/pgtrgm.html
  - **Limits to state:** encrypted columns (`activities.summary`, `contacts.preferred_channel`) cannot be indexed.

  (C6, M)
- [ ] CRM-T101 **CSV import of leads.**
  - **Flow:** upload, map columns, then a dry-run report per row (valid, invalid with reason, likely duplicate from the matcher, as advice only).
  - **Commit:** in batches. An import record keeps the ids it created, so an import can be undone.
  - **Consent:** imported only with its source and date, and never inferred.
  - **Scope:** contacts are not imported, because they come from the person service.
  - **Tests:** mapping, a bad row, duplicate advice, undo, and the audit chain.
  - **Source:** https://docs.twenty.com/user-guide/data-migration/overview

  (C7, M/L)
- [ ] CRM-T102 **Saved views.**
  - **Model:** per-user and shared views on the contacts, accounts, deals, leads and tickets lists, stored on the server. Each view is a filter in the segment language (CRM-R7) plus sort order and columns.
  - **Sharing:** sharing a view never shares access; rows are still decided per caller.
  - **Tests:** a view written by one user and read by another (shared), and a masked reader.
  - **Source:** https://attio.com/help/reference/managing-your-data/views/filter-and-sort-views.md

  (C8, S/M)
- [ ] CRM-T103 **Email templates with merge tokens.**
  - **Tokens:** a closed set (first name, account name, rep name and similar), HTML-escaped, each with a declared fallback.
  - **Preview:** rendered against a synthetic contact.
  - **Unsubscribe:** every marketing template carries the unsubscribe link, and saving one without it is refused.
  - **Use:** nurture steps and campaigns refer to a template.
  - **Tests:** escaping, a missing value, and the missing-link refusal.
  - **Source:** https://knowledge.hubspot.com/sequences/create-and-edit-sequences

  (C9, S/M)
- [ ] CRM-T104 **Explainable workflow rules.**
  - **Triggers:** record created or updated, a stage change, or a schedule.
  - **Conditions:** written in the segment filter language.
  - **Actions:** a closed set (create a task, notify, set a field, enrol in nurture, send a webhook after T95), with no arbitrary code or HTTP.
  - **Safety:** a dry run against current data, a loop guard (depth and rate), and consent re-checked inside any action that leads to a send.
  - **Run log:** every run records why it fired and what it did, and is audited.
  - **Tests:** each action, the loop guard, and the consent re-check.
  - **Source:** https://docs.twenty.com/user-guide/workflows/overview

  (C10, L; after T95 and T99)
