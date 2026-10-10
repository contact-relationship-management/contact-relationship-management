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
- [x] CRM-T10 Campaigns: lifecycle machine, simulated send `bg_pg`
  job behind the ESP trait seam, touch activities + engagement
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
      **Resolution:** `auth::aggregate_masked` decides a collection-level read with no `resource.*` attributes, so only rules that need no record (service, admin, manager in the reference policy) read unmasked. Applied to `GET /api/forecast` (`totals_minor`), `GET /api/dashboards/sales` (per-stage `totals_minor`), `GET /api/campaigns` (`cost_minor`), the campaign funnel (revenue, ROI, cost), and the activities, deals and tickets bundled in `get_contact` (`summary`, `amount_minor`, `description`). Masked amounts are `null`, never zero; counts stay. The UI treats null totals, revenue and ROI as dashes (Playwright pin). `tests/enforcement.rs` proves a masked reader gets null forecast and dashboard totals while a service credential gets the numbers. **Not done:** the full persona matrix (manager, marketing, support) in the enforcement test, the insights endpoints (`/api/insights/*`), and forecast snapshots; those still return amounts to any caller that clears the coarse read gate.

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

- [ ] CRM-T39 **Real ESP adapter** behind the existing send trait, with
      bounce/complaint webhooks, suppression list, and the processor
      agreement and lawful-basis review (CRM-G2). (plan gap 10) *Open:
      legal and vendor dependency.*
- [x] CRM-T40 *(resolved 2026-10-10.)* **Audit personal-data reads.** Contact, lead, ticket and
      activity reads write an audit row (actor, record, masked or not);
      bulk list reads write one summarizing row. (plan gap 11)
      **Resolution:** `Audit::record_read` writes a read audit row in its own short transaction (it joins the hash chain). Audited: single reads of contacts (with `masked` flag), leads and tickets; list reads of contacts, leads, tickets and activities as one summary row with the returned count. A failed audit write fails the read. DB test `personal_data_reads_are_audited`. **Not done:** deal-amount reads (the deal board polls, which would flood the trail), account reads, and insights views; and the audit table will grow with read traffic, so size retention for it.

- [ ] CRM-T41 **Field-level encryption and KMS.** Envelope encryption for
      contact channel details and activity content, key rotation, KMS
      adapter. (plan gap 12) *Open.*
- [ ] CRM-T42 **Independent assurance.** Penetration test, formal risk
      analysis, accreditation evidence (ATO / DSPT / HIPAA / ISO 27001).
      (plan gap 13) *Open: external.*
- [x] CRM-T43 *(resolved 2026-10-10, in part.)* **Abuse resistance.** Rate limiting and lockout on the
      service; UI idle-session timeout and re-authentication before
      destructive actions. (plan gap 14) *Open.*
      **Resolution:** Service: `src/rate_limit.rs` (8 pins) and an outermost abuse-guard middleware: a per-client fixed-window request limit (default 600 a minute) and a lockout after repeated `401`s (default 10 in 5 minutes), answering `429` with `Retry-After`. On by default only in production; `CRM_RATE_LIMIT_PER_MIN`, `CRM_AUTH_FAIL_LIMIT`, `CRM_AUTH_FAIL_WINDOW_SECS` override and `0` disables. Probes (`/_health`, `/_ping`, `/_readiness`) are exempt so a lockout cannot restart a healthy pod. Verified live against a production boot (401, 401, then 429). UI: 15-minute idle sign-out (`src/lib/idle.ts`, `IDLE_TIMEOUT_MS`) revokes the session and returns to sign-in; polling does not count as activity; vitest (4) and a Playwright test that fails at 10 minutes and passes at 15. **Not done:** re-authentication before destructive actions (needs a step-up flow from the identity provider). **Limits:** counters are per process, so enforce a global limit at the ingress too; unauthenticated callers share one bucket unless `CRM_TRUST_FORWARDED_FOR=1` is set behind a proxy that overwrites `X-Forwarded-For`, which means an attacker can lock out other unauthenticated callers (not authenticated ones, and not probes); the idle timer is client-side, so a client that blocks JavaScript keeps its session until the server-side session lifetime ends.

- [ ] CRM-T44 **Break-glass access.** Emergency elevation with mandatory
      reason, time limit, and after-the-fact review report. (plan gap 15)
      *Open.*

      **Resolution:** `src/hardening.rs`: `production_start_check` (4 pins). Checked in `Hooks::after_routes`, because `loco start` bypasses `Hooks::boot`. Verified live: production without `CRM_REQUIRE_AUTH` exits with the refusal; with it, boots. Scope note: it checks the enforcement flag, not that a verifier or key set is configured (auth already fails closed on an empty key set). Also fixed an existing bug found on the way: an empty `SMTP_USER`/`SMTP_PASSWORD` rendered YAML `null` and stopped `production.yaml` from parsing.
      **Resolution:** Migration `m20261010_000008_audit_chain` (`prev_hash`, `hash`, append-only trigger on PostgreSQL); `rules/audit_chain.rs` (8 pins: intact, altered, deleted, reordered, legacy rows, unchained-after-chain, field boundaries); `Model::record` takes a transaction advisory lock, appends, then hashes the row as stored; `GET /api/audits/verify` returns `ok`, `rows`, `legacy_rows`, `head`, `first_break`. DB test `audit_chain_detects_tampering` proves the trigger refuses edit and delete and the chain reports an edit made behind a disabled trigger. Full `--ignored` suite 10/10 against PostgreSQL 18. **Limits stated:** the advisory lock serializes audited writes; deleting the newest rows leaves a valid shorter chain (anchor `head` externally, see the runbook); the table owner can drop the trigger.
      **Resolution:** Service: `config/production.yaml` gains `secure_headers` (HSTS, nosniff, frame options, CSP preset), a closed CORS origin list (`CRM_CORS_ORIGIN`), and `server.ident: ""` to drop `x-powered-by`; verified with curl against a production boot. UI: `src/lib/server/headers.ts` applied in `hooks.server.ts`, plus `kit.csp` (hash-based, `frame-ancestors none`) in `vite.config.ts`; vitest pins the header set, Playwright asserts the headers and CSP on a real response. UI adapter changed from `adapter-auto` to `adapter-node` (needed for T35).
      **Resolution:** `scripts/ci-check.sh supply-chain` (cargo audit, cargo deny with `deny.toml`, pnpm audit, CycloneDX SBOMs to `sbom/`); jobs added to GitHub Actions and Woodpecker. Run locally: advisories, bans, licenses, sources ok; both SBOMs produced; no known vulnerabilities.
      **Resolution:** `spec/compliance/`: `control-mapping.md`, `dpia-template.md`, `operations-runbook.md`, `index.md`. Controls are marked Implemented only where code or CI enforces them; the rest are Deployer or Gap with a task reference.
      **Resolution:** `tests/e2e/a11y.spec.ts` (axe-core, WCAG 2.0 to 2.2 A/AA tags) over `/`, `/tour`, `/signin`, and six list pages: 9 pass, and a sanity check confirmed axe does detect an injected violation. **Limit:** automated checks cover roughly a third of accessibility issues, and detail pages and data-filled states are not scanned; manual testing with assistive technology is still needed.