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
  close/lost-reason/reasoned-reopen. (CRM-D3,
  CRM-D7, CRM-D9; CRM-R4) **Not built, found 2026-10-30:** the
  stalled derivation this task listed; no code in the service or UI
  derives or shows a stalled flag, and there is no setting for its
  days (see CRM-R4).
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
>
> - send-time re-check, ticket SLA journey incl. assignee-only
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
      a real send; *built since by CRM-T124, CRM-R58*), a timer for stalled campaigns and the nurture sweep, a live
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
- [x] CRM-T78 **Typed settings in place of process-global environment reads.** Parse one `Settings` struct at boot and keep it in `AppContext.shared_store`, replacing about 17 `OnceLock`s and 29 `env::var` calls. Done when the enforcement tests no longer need their own process or `set_var`. (Q4, M/L)
      **Done 2026-10-22 (not released), no behaviour change:**
      - **`src/settings.rs`:** one typed `Settings` with 22 fields, parsed by `Settings::from_lookup` from any `name -> value` function (pure; 8 unit tests with no environment). The process holds one snapshot: `settings::current()`, built from the environment on first use, replaced by `install` or `update`.
      - **Moved onto it:** `require_auth`, `protect_operational`, `allow_insecure`, `require_field_encryption`, `trust_forwarded_for`, the upstream mode, the event transport, retention days, the mail batch, the webhook and Mailgun secrets, the dev mail domain, the PASETO key sources, issuer and audience, the ABAC policy sources, the field-key sources and the Redis URL. Four `OnceLock` flags and about 22 `env::var` calls are gone.
      - **Tests:** the enforcement test and the webhook, Mailgun and mail-batch tests in `email_delivery.rs` now call `settings::update` instead of `unsafe set_var` (14 of the 24 `set_var`/`remove_var` calls in tests). 199 unit tests and all 59 DB-backed tests pass, with the enforcement test.
      **Left on purpose, with the reason:**
      - Startup-only readers that already take a lookup function: mail, unsubscribe links, the rate limit. The mail-worker regression test still sets them in the environment, as a deployment would.
      - `CRM_KEYCLOAK_*`: the `authentication-verifier` library reads them (`KeycloakSettings::from_env`), and `keycloak_guard.rs` sets them.
      - The per-service base URLs, whose variable name depends on the record kind.
      **Finished (2026-10-25):** the verifier and the ABAC policy were already swappable holders; what was missing was a way to rebuild them from changed settings. `auth::reload_from_settings()` does that, and `tests/support/scoped_auth.rs` (`scoped(|s| ...)`) changes the settings for one test and puts them back on drop, even by panic. The eight sign-in tests moved from their own binary (`tests/enforcement.rs`) into `tests/requests/enforcement.rs` and share the process with the other request tests (127 request tests pass together, one thread). **Still separate, for other reasons:** `keycloak_guard.rs` (the verifier library reads `CRM_KEYCLOAK_*` itself), `rate_limit.rs`, `outbox.rs`, `smtp_live.rs`.
      **Limits:** the settings are read once per process (a snapshot), as the flags always were; the webhook secret, Mailgun key, retention days and mail batch used to be read on every call, which only a test could notice. A bad value still falls back to its default silently rather than stopping the start.
- [x] CRM-T79 **Typed errors and per-field 422s.** Use `thiserror` enums in place of the 27 `Result<_, String>`. `ensure_valid` emits RFC 9457 `errors: [{pointer, detail}]`. Each error source builds a `Problem` directly; the reshaping middleware stays only as a safety net. Update OpenAPI and the UI's error display. (Q5, M)
      **Done 2026-10-22 (not released):**
      - **Per-field 422s.** A validation `422` now carries `errors: [{ "pointer", "detail" }]` (RFC 9457 §3.2) beside the unchanged `description`/`detail`, which still join the messages with `"; "`. `Problems` records the member each problem is about (`push_field`), `Problems::finish()` builds the response (33 call sites moved from `ensure_valid(&problems.into_vec())`), and `pointer` is a JSON Pointer such as `/display_name`. A problem about the request as a whole has no `pointer`. For a member of a list the check does not know which entry, so `stages[].name` is `/stages/name`.
      - **Body rejections** (`json.rs`): the parser's own path becomes the pointer, so `stages[2].probability_percent` is `/stages/2/probability_percent`, and a missing field is a member of its parent. A message with no path has no pointer.
      - **OpenAPI:** the shared `Error` schema documents `errors`.
      - **Typed errors, where the type earns its keep:** `GrantError` and `ReviewError` (break-glass; the handler no longer compares subject and reviewer to pick `403`, it matches `ReviewError::SelfReview`), `TransitionError` (`lifecycle::check`, used by lead, campaign, ticket and article transitions) and `PartnershipError`. `thiserror` 2 is now a direct dependency (already in the tree). Their `Display` text is unchanged, and tests pin it.
      - **Tests:** 2 request tests (each problem names its member; a whole-request problem has no pointer), the misspelt-member test also checks the pointer, 9 unit tests for the pointer rules and the typed errors (203 unit tests and 61 DB-backed tests pass).
      **Left out, and why:**
      - **The other `Result<_, String>` signatures** (about 12): start-up configuration (mail, provider credentials, unsubscribe, field keys, version, idempotency key) and analytics arithmetic. They are formatted once and logged or returned as a message; no caller branches on them, so an enum would only rename the string.
      - **"Each error source builds a Problem directly; keep the reshaping middleware as a safety net."** Not done. The auth, abuse-guard, version and idempotency layers still answer in their own shapes and `problem_mw` rewrites them; changing four middlewares' output for no client-visible gain was not worth the risk.
      - **UI (2026-10-25):** `fieldErrors(cause)` in `src/lib/errors.ts` turns `errors` into `{ member: detail }`; the contact and account edit forms mark the member (`aria-invalid`) and show the reason beside it, and still show the whole message. 2 unit tests. The other forms still show the message only. The tour's demo answers carry no pointers, so no Playwright test sees it.
      **Closed with the two items above left out on purpose** (the remaining `Result<_, String>` signatures, and the middleware reshaping): no caller branches on them, and changing four middlewares' output has no client-visible gain.
- [x] CRM-T80 **Aggregate insights in SQL.** Replace the 23 unbounded `live!` loads in `controllers/insights/*` with `GROUP BY`/`SUM` queries, and cap or page the related lists in contact detail (`relationships.rs:262-272`). A test with more than one page of seeded rows proves the totals stay exact, as CRM-T51 did for the activity dashboard. (Q6, M)
      **Done 2026-10-22 (not released), same answers, fewer rows read:**
      - **`src/controllers/insights/rollup.rs`:** SQL digests of the two tables that grow without bound. `touches` (latest activity and open dated follow-ups per subject) feeds cadence, members, stakeholders and pipeline hygiene; `engagement_counts` (grouped by recorder, month, kind and sentiment, inside the window) feeds engagement; `consent_events_by_source`, `withdrawals_since` and `withdrawals_by_account` feed the DPO and consent-by-account views. Eight whole-table loads of `activities` and `consent_events` are gone, and so is the decryption of every activity summary on the way.
      - **Pipeline hygiene** was quadratic (it scanned every activity once per deal and lead); it now looks each touch up in a map.
      - **Sales dashboard:** one grouped query replaces loading every open deal and then one stage lookup per deal (an N+1).
      - **Contact page:** activities, deals and tickets are each cut at 200, newest first, and the answer gains `truncated: { activities, deals, tickets }` so a cut is never silent. Activities were already cut at 200 with no sign; deals and tickets were unbounded. OpenAPI documents it.
      - **Measured, not end to end:** the touches query over 200,000 synthetic activities on 5,000 contacts: 17 ms in PostgreSQL, 5,000 rows returned (rolled back; the old path fetched and decrypted all 200,000). The 5,000-row figure is the number of subjects, so the work now grows with the number of contacts, not the number of activities.
      - **Tests:** `tests/requests/insight_rollups.rs` has 3 DB tests (the activity views over a fixture with known touch dates; the consent views; the 200-row cut). The first two were written and passed against the old code before the change, then passed unchanged after it. 203 unit and 64 DB-backed tests pass.
      **Also (2026-10-25):** the executive pack counted resolved tickets and started campaigns by loading every audit row of the window; it now counts in the database. It loaded every deal and filtered closed-in-window in Rust; the database now filters. Pipeline hygiene loaded every lead; it now loads the `new` and `contacted` ones the rule is about. The existing insight tests pass unchanged.
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
- [x] CRM-T83 **OpenAPI from the types.** Use typed response structs with `utoipa` (or `schemars`) derives in place of the 129 `json!` responses, and generate the document from them. Keep the checked-schema tests as the proof. (Q9, L; after T79)
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
      **Responses, dashboards, CLV and audit entries (2026-10-24):** `SalesDashboard`, `SlaDashboard` and `Clv` are typed structs, and the audit-log model derives `AuditEntry`; four hand-written entries are gone. The dashboards keep their `ETag` over the body without `as_of`, as before (the field is added after the tag is taken and documented as required). A masked sales dashboard is the same JSON. Still `json!`: the subject-access export, the event shapes and the insight views.
      **Responses, sales insights (2026-10-24):** `StaleDeals`, `Followups`, `HygieneFindings`, `ExecutivePack` and `ForecastTrends` are typed structs (the follow-up and hygiene rows included; a hygiene finding leaves out the members its rule does not use, as before). A masked amount, won value or forecast total is `null`, as before. Each view keeps `as_of: None` until `conditional` stamps it after the `ETag`. Six hand-written entries and the two money helpers are gone. Still `json!`: the outreach, relationship, support and compliance insights, the subject-access export and the event shapes.
      **Responses, outreach insights (2026-10-24):** `Cadence`, `EngagementView`, `StageFunnel` (its conversions are the shared `Ratio`) and `MembersView` are typed structs; the members rows sort on their typed `days_since_touch` instead of reading it back out of JSON. Four hand-written entries and the unused `boolean()` helper are gone. Still `json!`: the relationship, support and compliance insights, the subject-access export and the event shapes.
      **Responses, relationship, support and compliance insights (2026-10-24):** `StakeholderRegister`, `PartnershipsRegister`, `MembershipsView` (with `MembershipRow`), `SlaRegister`, `DpoView` and `ConsentByAccount` are typed structs. Every insight view is now typed. Eight hand-written entries and the `date`, `counts` and `view` helpers are gone. Still hand-written: `Error`, `Created`, `StatusPayload`, `SequenceStep`, `SequenceView`, `SubjectAccessExport` and `Event`.
      **Responses, the last schemas (2026-10-24):** `Created` is the one shared `PidRef` (four copies became one), `StatusPayload` is one shared struct for the campaign, ticket, article and partnership-stage bodies (four copies became one), `SequenceView`, `SubjectAccessExport` and `Event` are generated. Two documentation errors surfaced and are fixed by generation: the subject-access export omits `external_links` in its old schema though it always returned them, and `Event` described an outbox row shape that `/api/events/recent` never returns (both transports return the same four members). The only hand-written schema left is `Error`, because `problem_mw` builds it from whatever error was produced. The module comment of `openapi_schemas.rs` now says so.
      **The path table (2026-10-24):** the paths and verbs now come from the mounted router (`app::app_routes()`, shared with the server), and each `/api/` operation's tag, one-line summary and outcome kind live in `src/openapi_operations.rs`, one row per route. A route with no row is missing from the document and fails `every_mounted_route_is_documented_and_the_reverse`; a row with no route fails `every_table_entry_is_a_mounted_route`. The five public routes outside `/api/` (unsubscribe, webhooks, metrics) keep their hand-written bodies in `public_operations()`. The document is otherwise the same as before (checked by diffing the two): the only change is that a `200` description is `OK` instead of a type name.
      **Decided:** `Error` stays hand-written, because `problem_mw` shapes it from whatever error was produced; the summaries stay prose in the table, because a router cannot say them.
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
- [x] CRM-T87 **Matcher API stability.** Mark `Field`, `Outcome`, `Verdict`, `MatchConfig` and `FieldParameters` `#[non_exhaustive]`, and make `ConfigError` an enum. Add `cargo semver-checks check-release` to CI. These are breaking changes, so they go out as 0.2.0 with a changelog entry. (Q13, S/M)
      **Done 2026-10-21 (not released; the version is still 0.1.2):**
      - **`ConfigError`** is a `#[non_exhaustive]` enum with five variants; its `Display` text is unchanged. A test (`a_refusal_says_which_setting`) pins each variant.
      - **`#[non_exhaustive]`** on `Field`, `Outcome`, `Verdict`, `MatchResult` and `FieldScore`.
      - **Kept constructible, on purpose** (README, "Changing the API"):
        - `Contact`, `MatchConfig` and `FieldParameters` are built with `..Default::default()`, which `#[non_exhaustive]` forbids.
        - `Ranked` is built by the service to re-sort and re-score candidates. The service's unit tests against the matcher in the repo caught this (E0639) before anything was published.
      - **`scripts/ci-check.sh semver`** runs `cargo semver-checks`. It is not in the workflows yet (CRM-T94).
      - **Verified:** cargo-semver-checks 0.51.0 against the published 0.1.2 reports exactly three failures, all intended (enums marked non-exhaustive, structs marked non-exhaustive, `ConfigError` became an enum) and "requires a new major version"; with `--release-type major` it passes. Matcher clippy and tests pass; the service's 184 unit tests pass against the local matcher.

      **Finished (2026-10-25):** the matcher is bumped to 0.2.0 in the tree (changelog headed "0.2.0 (not published yet)"), and CI has a `semver` job (`taiki-e/install-action` with cargo-semver-checks, then `scripts/ci-check.sh semver`); against the published 0.1.2 it reports a major change and passes. **Not done, and not mine to do unasked:** `cargo publish` of 0.2.0, then moving the service to `0.2.0` and releasing it. The service still depends on 0.1.2.
      *Earlier note:* release as 0.2.0. That means bumping the matcher, publishing it, then moving the service's dependency to `0.2.0` and releasing the service. Not bumped now because the service's CI pins `0.1.2`.
      **Limits:** the cargo-semver-checks installed here (0.47) cannot read Rust 1.99's rustdoc output; 0.51 or later is needed, so CI must install that.
- [x] CRM-T88 **Matcher benchmarks and gates.** Add a criterion bench for `compare_one_to_many` at 1k, 5k and 50k candidates, recorded in `BENCHMARKS.md`. Add pedantic lints, and `cargo audit`/`deny` for the matcher. (Q14, S/M)
      **Resolution (2026-10-21):** `benches/matching.rs` (criterion 0.8, a dev-dependency only) measures one pair and `compare_one_to_many` at 1,000, 5,000 and 50,000 synthetic candidates; results, hardware and limits are in `BENCHMARKS.md`. About 2.7 µs a pair and 1.5 µs a candidate in a batch, linear. Pedantic lints and the matcher's `cargo audit`/`cargo deny` landed earlier (CRM-T75, CRM-T94). **Limits:** one run on one machine; it is not in CI, because timings on shared runners are noisy.
- [x] CRM-T89 **One file per locale, loaded on demand.** Split the `STRINGS` map in `i18n.svelte.ts` into one module per locale loaded with `import()`. Keep the parity test. Record bundle size before and after. List the values identical to English for native reviewers. Reconcile the root `locales/` directories (11) with the UI's 8. (Q15, M)
      **Done 2026-10-24 (not released):**
      - **One module per locale:** `src/lib/locales/<code>.ts`, default-exporting the table; `i18n.svelte.ts` bundles English (the fallback, and the source of the `StringKey` type) and loads the others with `import()` through `loadLocale()`. `translate()` answers in English until a locale has loaded, and the loaded tables are `$state`, so every `t(...)` re-renders when one arrives. A stored locale is fetched at start-up; `i18n.set()` fetches the chosen one.
      - **Parity:** kept as the unit test (now loading every locale), and strengthened: each non-English file is `satisfies Record<StringKey, string>`, so `svelte-check` fails on a missing or extra key.
      - **Bundle size** (`pnpm build`, client JS): before, one chunk of 174,597 bytes (50,524 gzipped) held all eight locales. After, English is 19,369 (7,169 gzipped) and the others are 17,392 to 34,256 (6,961 to 8,244 gzipped) each, loaded one at a time. A visitor downloads English plus at most one more, about 15 KB gzipped, instead of 50 KB. The files on disk total the same (all JS: 761,341 before, 762,248 after).
      - **Identical to English:** `scripts/identical-to-english.mjs` writes `spec/locale-review.md` in the UI folder: ar 1, cy 7, de 19, es 8, fr 16, hi 2, zh 1 of 330 each. Some are right (product names, loan words); a native reviewer decides.
      - **Root `locales/` (11) against the UI (8):** they translate different things (the role briefs and the interface), so they are not made equal. `spec/locale/index.md` now states the relationship, and a unit test reads the `locales/` directory and fails if any of its codes would resolve to English instead of a UI locale.
      - **Verified:** 116 unit tests, `svelte-check` (0 errors), and all 105 Playwright tests pass, including the "follows the language" ones.
- [x] CRM-T90 **Page data through `load`.** Move the `$effect` fetches into `+page.ts` `load` functions (or one resource helper with `AbortController`). Add an `errorMessage(cause)` helper to replace the 36 copies. A test proves a fast parameter change cannot show stale data. (Q16, M)
      **Done (2026-10-24), by the helper route, not `load`:** `src/lib/resource.svelte.ts` fetches when the component starts and again when anything the fetcher reads before its first `await` changes. Only the latest request may write, so a slow answer for an earlier parameter is dropped. A parameter change clears the old record (the page says "loading"); `reload()` after a write keeps it on screen. The `$effect` fetches in 16 routes and in `Dashboard` and `LinksPanel` now use it, and `/contacts/[pid]` has three (the contact, likely duplicates, merge log).
      - **`errorMessage(cause, fallback?)`** in `src/lib/errors.ts` replaces all 39 copies of `cause instanceof Error ? ... : String(cause)`.
      - **Tests:** 6 in `tests/unit/resource.svelte.test.ts`, including a late success and a late failure for an earlier parameter. vitest 122, `svelte-check` 0 errors, Playwright 105.
      - **Behaviour changes:** a failure clears on the next load (before, it stuck); `/articles` searches when the box is committed (it already said `onchange`), not on every keystroke; `/dpo`, `/engagement`, `/partners` and the dashboard fetch their parts in parallel and show them together.
      - **Left:** `/search` keeps its own cancel-on-rerun effect, which already guards against stale answers. `load` functions were not used: the app runs without SSR behind a same-origin proxy, so a `+page.ts` `load` would add nothing the helper does not.
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
- [x] CRM-T93 **UI component tests and a generated API client.** Add `@testing-library/svelte` tests for `DataTable`, `Board` and `LinksPanel`. Generate types from the service's OpenAPI with `openapi-typescript`, and check them against the 911-line `api/crm.ts`. Fix or remove the vitest `deps.inline` regex that matches nothing. (Q19, M; types after T83)
      **Done (2026-10-24):**
      - **Component tests:** `tests/unit/components.test.ts`, 9 tests over `DataTable` (headers and rows, empty text, `onselect`, an `href` column), `Board` (cards in their columns) and `LinksPanel` (list, none, failed read, Add disabled until there is a URL).
      - **`deps.inline`:** the regex matched nothing because the package is `svelte-locale-translations`; it is now `/@lilydesignsystem\/svelte-/`, which is what lets the Lily components load under jsdom.
      - **Generated types:** `openapi-typescript` (dev dependency) writes `src/lib/api/schema.d.ts` from the service's document; `cargo run --example openapi` prints that document, and `OPENAPI=… pnpm api:types` regenerates. `src/lib/api/schema-check.ts` makes `svelte-check` fail when a hand-written type in `crm.ts` is not satisfied by what the service says it sends (22 types checked).
      - **Found by the check, fixed:** the document marked every `Option` member of a response as possibly absent, though the service always sends `null`. Requests are now generated as deserialized and responses as serialized, so response members are required. `Followup.due_on` and the first five members of a break-glass grant are documented non-null (the code guarantees it). `crm.ts` `StaleDeal.amount_minor` is now `number | null` (masked amounts).
      - **Narrowed on purpose:** `ExternalLink.system`, `Duplicate.verdict` and `DuplicateField.outcome` are closed sets in the UI and plain strings in the document; the check skips them, with a comment.
      - **Not done:** `crm.ts` is still hand-written (911 lines); replacing it with the generated types, and running the check in CI, wait for T94.
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
      **Added 2026-10-25 (not yet run by GitHub Actions):**
      - **`semver` job** is now in `ci.yml` (the matcher is 0.2.0 in the tree, so it passes: a major change against the published 0.1.2).
      - **`lint` stage and job:** `typos` (spelling; `_typos.toml` leaves the translations out and lists names and test data that look like misspellings, and two real typos in `spec/locales-x-for-roles` were fixed) and `zizmor --offline .github` (a security audit of the workflows). Both pass locally (typos 1.51.1, zizmor 1.31.1, installed with `cargo install`).
      - **What zizmor found, and was fixed:** 14 template-injection errors (a tag name, the actor, matrix values and step outputs were expanded into shell text in `release.yml`; they now arrive as environment variables), 11 `actions/checkout` steps that left the token in the workspace (`persist-credentials: false`), 2 build caches in the release job that signs and publishes (removed), and 7 Dependabot entries without a cooldown (7 days). The release workflow was edited but **has not run**: the next tag will be its first test, so tag a release candidate first.
      - **Coverage** already has a report-only `coverage` job.
      - **Mutation testing, added 2026-10-25:** `.cargo/mutants.toml` limits `cargo mutants` to `src/rules` and the unit tests (about 780 mutants); `scripts/ci-check.sh mutants` and a weekly `mutants.yml` workflow (also by hand) run it and keep the report. Not yet run by GitHub; the full run was not done here (about three hours), but the two newest rule files were: `calendar.rs` and `reports.rs`, 102 mutants. **It found 21 untested behaviours** (the boundaries of the limits, one match arm per fixed filter set, the default measure, and a test loop that tested nothing when a kind had no fields). Tests were added for each, and a second run left one survivor (two filters each keeping their own expression), which a further test closed (checked on that mutant alone). `tags.rs` (13 mutants) had none missed.
      - **ESLint, added 2026-10-25:** `eslint.config.js` with the recommended rules of ESLint, `typescript-eslint` and `eslint-plugin-svelte` (dev dependencies; `pnpm lint` now runs it after prettier, and the CI `front-end` stage already runs `pnpm lint`). It found 58 problems; clean now. **Fixed:** 8 unused imports, a `let` that never changes, 5 `{#each}` without a key (on the splash and tour pages), a useless `{" "}`. **Switched off, with the reason in the file:** `svelte/no-navigation-without-resolve` (40 links; the app is served from the root, so a plain absolute link is right; turn it on if a base path is ever set), `no-empty-object-type` for SvelteKit's own `app.d.ts`, and one `prefer-svelte-reactivity` line for a map of loads in flight that nothing renders.
      - **Markdown lint and Woodpecker, added 2026-10-25:** `rumdl` 0.3.0 checks every Markdown file (`.rumdl.toml`) in the `lint` stage, which Woodpecker now runs too (it had no lint step). Its first run found 1,852 findings: 1,701 were line length (the prose is one paragraph per line, so that rule is off), 37 inline HTML (used on purpose, off) and 28 links to sibling projects outside this repository (off: this check cannot see them, so **those links are unchecked**). The other 86 were formatting and were fixed. `spec/special-files-for-public-repos` is left out because it is copied as it is. GitHub's `install-action` has no `rumdl`, so that job installs the pinned wheel from PyPI; the Woodpecker step compiles it. Neither job has run `rumdl` yet.
      - **Still not done:** a full run over every rule file (its survivors are the list of rules no test pins), and `actionlint` (not installable here without changing Homebrew's permissions; zizmor covers the security half of what it checks, not the syntax half). Woodpecker has no `lint` step.

      **Not done, and why:**
      - **Coverage** (`cargo llvm-cov`, vitest `--coverage`): the tool is not installed here, and a report-only job is best added when its output can be seen.
      - **Mutation testing:** `cargo-mutants` is installed, but a run over `rules/` needs the database suite and takes long; a scheduled job should be set up and watched once.
      - **ESLint** with `eslint-plugin-svelte`: new dev dependencies and a lockfile change, and its rule set is a team choice.
      - **`typos`, markdown lint, `actionlint`/`zizmor`:** none installed here; each needs its first run reviewed for noise before it can gate.
      - The two new workflow jobs and the Woodpecker steps have not run on their providers; expect a first-run fix.

      **Added 2026-10-24:**
      - **Coverage report:** `front-end` runs `vitest --coverage` (`@vitest/coverage-v8`): statements 68.0%, branches 56.9%, functions 67.9%, lines 70.4% today. A new `coverage` stage runs `cargo llvm-cov --lib --summary-only`: 39% of lines, but unit tests only, since the database suite (which most handlers need) is in `test-db`. Both are reports; no floor is set. A figure for the whole suite needs `llvm-cov` around `test-db`.
      - **`api-types` stage** (GitHub job and Woodpecker step): regenerates `schema.d.ts` from `cargo run --example openapi`, fails if it differs from the committed file, then runs `svelte-check`, so the T93 type check runs against the service as it is.
      - **Still not done:** mutation testing, ESLint, `typos`, markdown lint, `actionlint`/`zizmor` (none installed here; each needs a reviewed first run).

## New capabilities (from the 2026-10-21 research, [plan.md](plan.md) C1 to C10)

Each task first writes its requirement (proposed CRM-R28 onward) and design note, then lands spec, code and tests together.

- [x] CRM-T95 **Outbound webhooks.** A relay job reads `event_outbox` and delivers to subscriptions that admin or service callers manage.
  - **Security:** https only, with SSRF guards that refuse private and link-local addresses. Signed in the Standard Webhooks format (`id.timestamp.body`, several signatures while a secret rotates). Payloads carry ids, never contact values.
  - **Delivery:** retries with backoff, a dead-letter state, a delivery log, and a 410 that disables the subscription.
  - **Audit and erasure:** every subscription change is audited. An erasure cancels queued deliveries for that contact.
  - **Tests:** signing, retry and dead-letter, the SSRF refusals, and erasure.
  - **Sources:** <https://docs.espocrm.com/administration/webhooks/> and <https://www.svix.com/resources/glossary/webhook-signature/>
  - **Limits to state:** at-least-once delivery, so receivers must dedupe on the event id.

  (C1, M)
      **Done (2026-10-24, not released):** requirement CRM-R28 and the design note in `integrations.md` first, then:
      - **Tables** (migration 000020): `webhook_subscriptions`, `webhook_deliveries` (one row per subscription per event, unique), and `event_outbox.webhook_fanned_out_at`. A delivery stores ids only; the body is built from the outbox row when sent.
      - **Pure rules** (`rules/webhooks.rs`, 9 tests): the address guard (`is_public_ip` covers private, loopback, link-local, shared, documentation, benchmarking, multicast, reserved, IPv4-mapped and NAT64), URL checks, event patterns, the Standard Webhooks signature (checked against the specification's own example vector), backoff, and the id-only body.
      - **Relay** (`webhooks.rs`): fan out (marked in the same transaction, only events after the subscription existed), claim (lease, so two runs never send the same delivery), send, record. The network is behind a `Sender` trait; `HttpSender` resolves the name, refuses any non-public answer, connects to the addresses it checked, follows no redirects, times out at 10 s.
      - **Routes** (admin/service only: the prefix derives the destructive action for every method): create (secret shown once), list, remove, rotate secret (old one also signs for 24 h), delivery log (latest 100), and `POST /api/webhook-subscriptions/relay`.
      - **Audit and erasure:** create, rotate, remove and a `410` disable are audited without the address or secret. Erasing a contact cancels its queued deliveries and those of contacts merged into it (the erase audit row counts them).
      - **Tests:** 8 request tests (registration and refusals, signed once with ids only, no backlog, backoff to dead, `410`, rotation, erasure, removal) plus 2 for the real sender's refusals and a policy unit test. Service: 226 unit tests, 77 request tests, clippy clean (both feature sets). OpenAPI and the UI's generated types include the new routes.
      - **Limits (state to receivers):** at least once, so dedupe on `webhook-id`; events exist only under the `outbox` transport; a run sends at most 50 deliveries and fans out at most 500 events, so call it on a schedule; no replay button, per-subscription rate limit or log retention yet; every non-`2xx` except `410` is retried, including other `4xx`.
- [x] CRM-T96 **Consent per purpose and double opt-in.**
  - **Model:** consent is held per purpose (for example newsletter, events, product updates), in place of the single `marketing_consent`. A new `pending` state never allows a send.
  - **Double opt-in:** a confirmation token moves `pending` to `granted`, and the event records its source and date.
  - **Preference page:** a signed-token page lets the subject change each purpose.
  - **Merge and unmerge:** the stricter-side rule (CRM-R24) applies per purpose.
  - **Migration:** existing rows become one default purpose with their history kept.
  - **Tests:** the send gate for each state, the merge cases per purpose, and the migration against populated data (`scripts/migrate-with-data.sh`).
  - **Source:** <https://knowledge.hubspot.com/marketing-email/manage-your-marketing-hub-starter-email-account-settings>

  (C2, M)
      **Done (2026-10-24, not released):** requirement CRM-R29 first (and a line in CRM-R6), then:
      - **Model:** four purposes (`marketing`, `newsletter`, `events`, `product_updates`) and four states (`granted`, `withdrawn`, `never`, `pending`). The default purpose stays in `contacts.marketing_consent`, so the 33 places that read it keep working and the existing rows *are* the default purpose with their history; the others are rows of `contact_consents` (no row is `never`). `consent_events`, `campaigns` and `nurture_sequences` gain a `purpose` (default `marketing`). Migration 000021.
      - **Send gate:** campaign sends, nurture enrolment and the nurture sweep, and segment evaluation check the contact's state *for the campaign's or sequence's purpose*; only `granted` allows a send. Segment preview takes `?purpose=`.
      - **Double opt-in:** `POST /api/contacts/{pid}/consent/request` makes the purpose `pending` and returns a 7-day signed confirmation link. `GET /confirm/{token}` changes nothing (mail scanners); the button `POST`s and grants with the source `double_opt_in`. A withdrawal since the request makes the link grant nothing.
      - **Preference page:** `POST /api/contacts/{pid}/preferences-link` returns a 90-day signed link; `/preferences/{token}` lists each purpose and starts or stops one (source `preference_page`). Tokens carry a kind byte under the HMAC, so a preference token is not a confirmation token and neither is an unsubscribe token.
      - **Unsubscribe** withdraws the purpose of the message's campaign or sequence, not the others; a spam complaint withdraws every purpose.
      - **Merge and unmerge:** the stricter rule runs per purpose (order: withdrawn, never, pending, granted; tested for all 16 pairs), the previews show each purpose's outcome, and the events record `merge:` or `unmerge:` sources. Erasure withdraws every purpose of the contact and of contacts merged into it; the subject-access export lists each purpose's state.
      - **Tests:** 6 request tests (gate per state and per purpose, double opt-in and stale links, preference page, unsubscribe by purpose, merge and unmerge, erasure and export), signed-link unit tests, the merge order. Service: 229 unit tests, 83 request tests, clippy clean for both feature sets. **Migration against populated data:** `scripts/migrate-consent-purposes.sh` (500 contacts in every state, their history, campaigns and sequences; rolls back and forward; all checks pass). The older `scripts/migrate-with-data.sh` is for migration 17.
      - **Behaviour changes:** withdrawing `marketing` now exits only the enrolments in sequences sent under `marketing` (all of them, until sequences name another purpose); a complaint withdraws everything.
      - **Not done:** sending the confirmation and preference messages (the caller does; CRM-T98's form will), translating the public pages (English only, like unsubscribe), a UI for purposes (the contact page still shows `marketing_consent`), and an editable purpose list.
- [x] CRM-T97 **MCP server.**
  - **Design:** a thin MCP layer over the existing endpoints, authenticated with the caller's own token and decided by the same ABAC and masking. Tools are read-only by default.
  - **Destructive tools:** merge, unmerge and erase are offered only as `dry_run` plus a confirmation that a person makes in the UI.
  - **Audit:** every call is audited with an `agent` marker.
  - **Prompt injection:** record text returned to the agent is marked as data, not instructions.
  - **Tests:** a reader cannot write through MCP; a masked caller gets masked values; the audit rows.
  - **Sources:** <https://docs.twenty.com/user-guide/ai/capabilities/mcp> and <https://developers.hubspot.com/changelog/remote-hubspot-mcp-server-is-now-generally-available>
  - **Limits to state:** MCP spec versions move quickly; name the version supported.

  (C3, M)
      **Done (2026-10-24, not released):** requirement CRM-R30 and the design note in `integrations.md`, then:
      - **Version:** MCP `2026-07-28` (current when read on 2026-10-24: per-request version in `_meta` and `MCP-Protocol-Version`, `server/discover`, no sessions, no GET stream), plus `2025-11-25` and `2025-06-18` for clients that open with `initialize`. Unsupported versions get `400` and `-32022` listing the supported ones. Tools only.
      - **Mechanism:** `src/mcp/` (`tools.rs` the catalog, `mod.rs` the protocol). A tool call is the REST request it stands for, sent through the service's own router with the caller's `Authorization`, so ABAC, masking, rate limits and audit apply unchanged. 21 read tools, 2 dry-run tools (`preview_merge_contacts`, `preview_unmerge_contact`: `dry_run` is fixed and not an argument), and `add_activity` only with `CRM_MCP_WRITES`. No tool erases, reaches break-glass or webhooks.
      - **Audit:** an `mcp_tool_called` row with `agent: true`, the tool, kind and target pid, written before the call (no row, no call); the free text an agent passes is not recorded.
      - **Prompt injection:** each description and each result says record text is data; the notice precedes all record text. Honest limit: this lowers the risk and does not remove it; the safeguard is that no tool can do more than the caller already could.
      - **Transport:** `Origin` allow-list (`CRM_MCP_ALLOWED_ORIGINS`, none by default), header/body agreement for the modern version (`-32020`), `404`+`-32601` for an unknown method, `405` for GET/DELETE, batches refused, 256 KB request and 200 KB answer caps. `tower` (util) moved from a dev-dependency to a dependency.
      - **Tests:** tools (9 unit tests, including that every tool is a documented route and nothing destructive is offered), protocol (7 unit), 8 request tests (modern and legacy flows, injection text, previews change nothing, the write tool gated, tool errors, transport rules, audit), and under sign-in (`tests/enforcement.rs`): a stranger's read is masked, a reader's write and preview are refused, no token gets `401`, and the audit rows name the caller. Service: 247 unit tests, 91 request tests, clippy clean for both feature sets.
      - **Not done:** OAuth discovery for MCP clients (they present the same PASETO bearer token), resources, prompts, change notifications, an erase preview (erase has no `dry_run` yet; adding one is a service change), and trying it against a real MCP client.
- [x] CRM-T98 **Web form lead capture.**
  - **Endpoint:** a public `POST` that creates a lead with `source=web` and consent `pending` (needs T96).
  - **Abuse protection:** rate limiting per form and per client, a honeypot field, and an origin check.
  - **Person record:** created or resolved through the person service, since the CRM stores no email.
  - **Optional nurture enrolment:** starts only after opt-in is confirmed.
  - **Tests:** abuse limits, no send before confirmation, and the audit row.
  - **Source:** <https://www.pipedrive.com/features/web-forms>

  (C4, M)
      **Done (2026-10-24, not released):** requirement CRM-R31 first, then:
      - **Endpoint:** `POST /forms/{slug}` (+ `OPTIONS`), public and outside `/api`. Forms come from `CRM_WEB_FORMS` (JSON: slug, origins, optional purpose and nurture sequence); the caller chooses neither purpose nor sequence. Every non-error answer is the same `202`.
      - **Abuse protection:** `Origin` must be on the form's list (missing is refused); 5 submissions per client per 10 minutes and 300 per form per hour (`CRM_WEB_FORM_CLIENT_LIMIT`, `CRM_WEB_FORM_LIMIT`) with `429` and `Retry-After`; a filled `website` honeypot (checked before validation) creates and sends nothing. At most one confirmation mail per contact and purpose per hour, so a stranger's address cannot be flooded.
      - **Person record:** `clients::resolve_person` asks the person service (`POST {CRM_PERSON_SERVICE_URL}/api/persons`, expecting `{pid}`) and keeps only the `person:` reference; `503` and nothing created if it fails. The lead keeps the address's domain only. Stub mode makes a process-local person.
      - **Consent:** the purpose becomes `pending` (never touching a `granted` one) and the one mail sent is the CRM-R29 confirmation, with no unsubscribe link. The nurture enrolment is made inside the confirmation's transaction (`forms::on_confirmed`), from a `form_submissions` row (migration `000022`, no name or address, deleted on erasure).
      - **Refactors:** `capture_lead` is shared with `POST /api/leads`; `enrol_in` with the enrolment endpoint.
      - **Tests:** 4 request tests (the whole journey including no nurture mail before confirmation and the audit row holding no name or address; origin, honeypot, bad input and preflight; client and form limits and the stranger's address; person service down and erasure), 4 rules unit tests, a settings test, and `/forms/` in the public-path test. Service: 250 unit tests, 95 request tests, clippy clean for both feature sets. UI types regenerated.
      - **Not done:** an embeddable script or form builder, CAPTCHA, translating the confirmation mail, and shared (Redis) limit counts. **Assumed, not confirmed:** the person service's `POST /api/persons` contract, which no sibling service in this repository defines.
- [x] CRM-T99 **Tasks, reminders and a notifications inbox.**
  - **Tasks:** task activities gain an assignee (`worker:` URN) and a reminder time.
  - **Notifications:** a `bg_pg` job writes notification rows for due reminders and SLA breaches. The UI gets an unread inbox, with mark-read.
  - **Access:** a notification is created only if the recipient can read the record (ABAC is checked at write time and again at read time).
  - **Tests:** reminders fire once, notifications for unreadable records are suppressed, and the labels exist in 8 locales.
  - **Source:** <https://knowledge.hubspot.com/sequences/complete-your-sequence-tasks>

  (C5, M)
      **Done (2026-10-24, not released):** requirement CRM-R32 first, then:
      - **Tasks:** `assignee_ref` and `remind_at` on task activities (create and `PUT /api/activities/{pid}`; other kinds get `422`). Migration `000023`.
      - **Job:** `NotificationSweepWorker`, queued every minute by the server, plus `POST /api/notifications/sweep` (admin tier) returning `{created, suppressed}`. Loco has no scheduler, so the timing is a server loop; the work is idempotent and a uniqueness rule makes each reminder fire once (a new `remind_at` fires again), so replicas may all queue it. SLA breaches are read from the flags `POST /api/sla/sweep` sets.
      - **Access, twice:** at write time the policy is asked for the assignee as a subject with a `sub` and no attributes (the sweep has no token), and a denied or missing record is stored as `suppressed`; at read time it is asked again with the caller's own token, per record. Marking your own notifications read derives `read`, so a reader may.
      - **API/UI:** `GET /api/notifications` (paged, `?unread=true`), `unread-count`, `PUT {pid}/read`, `read-all`; inbox page `/notifications` with an unread count beside its navigation entry; 8 keys in all 8 locales (the parity test covers them); the tour has two sample notifications. Notifications carry no record text. Erasure deletes them.
      - **Tests:** 2 request tests (once and again; read by the recipient only; gone record suppressed; SLA breach per breach; erasure), 1 under sign-in (suppressed at write for an unreadable record, own inbox only, a record that becomes unreadable drops out), a policy unit test, 1 demo unit test and 1 Playwright test. Service: 251 unit, 97 request, 3 enforcement; UI: 134 unit; clippy clean for both feature sets; types regenerated.
      - **Not done / limits:** a UI form to set an assignee or reminder (the API takes them); delivery by email or push; the write-time check cannot see the recipient's token attributes, so a policy that needs one suppresses their reminders (fails closed); a suppressed notification is not revived if the policy later changes.
- [x] CRM-T100 **Server-side search.**
  - **Endpoint:** `GET /api/search?q=` over contacts, accounts, deals, leads, tickets and articles, using `tsvector` with `unaccent` and `pg_trgm` for fuzzy matches. Results are filtered by ABAC and masking, and the query is audited as a count, not as text.
  - **UI:** the CRM-R22 search uses it when the service is present.
  - **Tests:** accents, typos, a masked caller, and paging.
  - **Source:** <https://www.postgresql.org/docs/current/pgtrgm.html>
  - **Limits to state:** encrypted columns (`activities.summary`, `contacts.preferred_channel`) cannot be indexed.

  (C6, M)
      **Done (2026-10-24, not released):** requirement CRM-R33 first, then:
      - **Endpoint:** `GET /api/search?q=` (2 to 100 characters, at most 8 words, letters and digits only so nothing typed becomes query syntax), paged, best first, over contacts, accounts, leads, deals, tickets, campaigns and articles (campaigns added because the browser search had them). Full text (`simple`, word prefixes) or trigram `word_similarity` >= 0.45, both over `immutable_unaccent(lower(...))`; migration `000024` enables `unaccent` and `pg_trgm`, wraps `unaccent` as IMMUTABLE and makes 14 GIN indexes. The expressions live once in `migration::search_docs` and the service query uses the same constants.
      - **Access and masking:** contacts and deals are checked per row against the caller's token; a hit carries only a name and one plain descriptor, and masked fields are neither indexed nor returned. The audit row holds a count.
      - **UI:** the search page calls the service once and groups the hits instead of loading and filtering seven lists in the browser; the tour answers from its sample data; short queries make no call.
      - **Tests:** 3 rules unit tests; 2 request tests (accents both ways, prefix, typo, every kind, descriptor, lead domain, removed record, bad queries, audit with no text; paging with stable order); 1 under sign-in (a denied contact omitted, no masked field, no token is 401, the masked value is not searchable). Two bugs the tests found are fixed: per-branch limits ignored the offset, and tied scores had no tiebreak inside a branch. Service: 254 unit, 99 request, 4 enforcement; UI: 135 unit and the Playwright suite (107); clippy clean; types regenerated.
      - **Limits:** activity summaries and `preferred_channel` are encrypted and cannot be indexed; the CRM holds no email address, so none is searched (a lead's address domain is); the policy filter runs after the query, so pages can be short; no highlighting, no search inside activities, no ranking by recency.
- [x] CRM-T101 **CSV import of leads.**
  - **Flow:** upload, map columns, then a dry-run report per row (valid, invalid with reason, likely duplicate from the matcher, as advice only).
  - **Commit:** in batches. An import record keeps the ids it created, so an import can be undone.
  - **Consent:** imported only with its source and date, and never inferred.
  - **Scope:** contacts are not imported, because they come from the person service.
  - **Tests:** mapping, a bad row, duplicate advice, undo, and the audit chain.
  - **Source:** <https://docs.twenty.com/user-guide/data-migration/overview>

  (C7, M/L)
      **Done (2026-10-25, not released):** requirement CRM-R34 first, then:
      - **Flow:** `POST /api/lead-imports/preview` (no mapping: columns, row count, suggested mapping; with a mapping: a report per row, valid or invalid with the reason, and likely duplicates), `POST /api/lead-imports` (commit), `POST /api/lead-imports/{pid}/undo`, `GET /api/lead-imports[/{pid}]`. The CSV is JSON text, at most 500 rows and 1 MB, read with the `csv` crate (new dependency; `cargo deny check licenses` passes).
      - **Duplicate advice:** the CRM-R23 matcher on name and company against contacts the caller may read, the newest 2000 leads and earlier rows of the file, narrowed to people sharing a name word of 3+ letters (at most 200 per row); advice only, with `skip_likely_duplicates` to leave such rows out.
      - **Commit and undo:** the import record first, then batches of 100 that append their lead pids in the same transaction; undo removes the leads still `new` with no activity and keeps and counts the rest; twice is `422`, serialised by a row lock.
      - **Consent and email:** leads are `source=import`, no contact, no consent, nothing sent; `source_note` and `obtained_on` are required and recorded on the import and in its audit row. Only the address's domain is kept, as for the web form.
      - **UI:** `/leads/import` (choose file, map, report table, source and date, import, undo), linked from the leads page; 21 keys in all 8 locales; the tour imports nothing but answers the whole flow.
      - **Tests:** 4 rules unit tests; 3 request tests (mapping, bad rows, advice and nothing saved; commit, no email/contact/consent, audit rows and chain, undo with a worked lead kept; skip duplicates and 250 rows in three batches with undo); 1 under sign-in (a reader cannot import; advice never names a contact the caller cannot read); 1 Playwright test. Service: 258 unit, 102 request, 5 enforcement; UI: 135 unit; clippy clean; types regenerated.
      - **Limits:** leads only (contacts come from the person service); 500 rows; comma-separated UTF-8 only; no update of existing leads; no lead source or score column; advice misses a pair with no shared name word; not a background job.
- [x] CRM-T102 **Saved views.**
  - **Model:** per-user and shared views on the contacts, accounts, deals, leads and tickets lists, stored on the server. Each view is a filter in the segment language (CRM-R7) plus sort order and columns.
  - **Sharing:** sharing a view never shares access; rows are still decided per caller.
  - **Tests:** a view written by one user and read by another (shared), and a masked reader.
  - **Source:** <https://attio.com/help/reference/managing-your-data/views/filter-and-sort-views.md>

  (C8, S/M)
      **Done (2026-10-25, not released):** requirement CRM-R35 first, then:
      - **Model:** `saved_views` (migration `000026`): owner, list (`contact`, `account`, `deal`, `lead`, `ticket`), name unique per owner and list (any case), filter, sort, columns, shared; at most 50 per owner. The filter has the segment language's shape (field to values, OR within a field, AND across) over a closed set of fields and values per list, checked in `rules::views`. **Decision:** the segment evaluator itself is not reused, because it structurally keeps only consented contacts, which is right for sending and wrong for looking at a list; the UI mirrors the closed sets in `lib/views.ts` (a test pins the list names).
      - **API:** `GET /api/views?kind=` (own and everyone's shared), `POST`, `PUT` (replace, list cannot change), `DELETE`. Only the owner changes a view: 403 for a shared one, 404 for someone's private one. Making, changing and deleting your own are not data writes, so they derive `read` and a reader may (`auth::policy`). With sign-in off all views belong to one anonymous owner.
      - **Sharing and masking:** the server stores what to ask for; the UI applies the view to the rows the caller's own list read returned (`applyFilter`, `applyShape`), so rows are still decided per caller and a filter on a masked value matches nothing. Contacts' preferred channel is not a filterable field.
      - **UI:** `ViewBar` above the contacts, accounts, leads, tickets and deals (plain list) tables: pick a view, save the current filter, sort and columns, edit, share or delete your own; 17 keys in all 8 locales; the tour has one shared sample view. The tickets table gained a Channel column and the deals list gained amount, currency, status and close-date columns so views have something to show.
      - **Tests:** 2 rules unit tests; 1 request test (save, list, replace, closed sets, unique names, delete, audit with no name); 1 under sign-in (a shared view read by another user, a private one hidden, 403/404 for changes, a reader keeps their own views but cannot write data, and the reader's list stays masked); 5 UI unit tests and 1 Playwright test. Service: 261 unit, 103 request, 6 enforcement; UI: 141 unit and 110 Playwright; clippy clean; types regenerated.
      - **Limits:** the filter is applied in the browser to the list the page loaded, not on the server, so a view cannot make a list shorter to fetch; no views for campaigns, segments or articles; no filters on free text or dates; no default view; the deals board is not filtered by a view, only the plain deals list.
- [x] CRM-T103 **Email templates with merge tokens.**
  - **Tokens:** a closed set (first name, account name, rep name and similar), HTML-escaped, each with a declared fallback.
  - **Preview:** rendered against a synthetic contact.
  - **Unsubscribe:** every marketing template carries the unsubscribe link, and saving one without it is refused.
  - **Use:** nurture steps and campaigns refer to a template.
  - **Tests:** escaping, a missing value, and the missing-link refusal.
  - **Source:** <https://knowledge.hubspot.com/sequences/create-and-edit-sequences>

  (C9, S/M)
      **Done (2026-10-25, not released):** requirement CRM-R36 first, then:
      - **Model:** `email_templates` (migration `000027`) and a nullable `template_pid` on `campaigns` and `nurture_steps`. Rules in `rules::templates` (parse, check, render), DB-free.
      - **Tokens:** a closed set: `given_name` (first word of the name; a heuristic, the CRM keeps one display name), `name`, `account_name`, `rep_name` (the contact's owner, else the account's, resolved best-effort through the person/worker service), `job_title`, `unsubscribe_url`. Unknown or unclosed marks are 422; there is no escape for a literal `{{`.
      - **Fallbacks:** each token but the link has a declared default; a template may declare its own per token; a missing, empty or white-space value uses it, so no message carries a gap or a raw mark.
      - **Escaping:** the mailed text keeps values as they are (the mail is plain text); the HTML form escapes every value and the template's own text, and the link is the only anchor. Control characters in a value become a space, so a value cannot add a header line to a subject. **Decision:** escaping applies to the HTML form, not the text, because escaping plain text would show `&amp;` to the reader.
      - **Unsubscribe:** the body must contain `{{unsubscribe_url}}` (422 otherwise); the subject cannot. A message made from a template does not also get the generic footer.
      - **API:** `GET/POST /api/email-templates`, `GET/PUT/DELETE /api/email-templates/{pid}` (409 while a campaign or step uses it), `GET /api/email-templates/tokens`, `POST /api/email-templates/preview` (a synthetic contact, `blank` to show fallbacks, `problems` for what would block a save; derives `read` so a reader may). Audit `email_template`, no text.
      - **Use:** `template_pid` on `POST /api/campaigns` and on a nurture step, instead of a subject and body (both is 422). The template is read at send time; one gone at that time fails that delivery for good.
      - **UI:** `/templates` (linked from Campaigns): list, editor with token buttons and fallbacks, live preview of the plain text with a switch to show each token as missing; 15 keys in all 8 locales; one sample template in the tour. The campaigns page also has a creation form (name, segment, template or own message; 6 more keys in 8 locales; Playwright test). The token `first_name` was renamed `given_name`; there is no family-name token because the CRM keeps one display name.
      - **Tests:** 9 rules unit tests (escaping, a missing value, header injection, the missing-link refusal, fallback checks); 2 request tests (save, refusals, preview, replace, delete; a campaign and a nurture step really mailed from a template, one link and no footer, edit reaches the next send, 409 while in use); 1 under sign-in (a reader previews and reads but cannot save); Playwright for the page. Service: 272 unit, 107 request, 7 enforcement; UI: 143 unit and 111 Playwright; clippy clean; types regenerated.
      - **Limits:** plain text only (no HTML-bodied mail, attachments or per-language templates); no template versions; a template deleted between the in-use check and a send fails that delivery.
- [x] CRM-T104 **Explainable workflow rules.**
  - **Triggers:** record created or updated, a stage change, or a schedule.
  - **Conditions:** written in the segment filter language.
  - **Actions:** a closed set (create a task, notify, set a field, enrol in nurture, send a webhook after T95), with no arbitrary code or HTTP.
  - **Safety:** a dry run against current data, a loop guard (depth and rate), and consent re-checked inside any action that leads to a send.
  - **Run log:** every run records why it fired and what it did, and is audited.
  - **Tests:** each action, the loop guard, and the consent re-check.
  - **Source:** <https://docs.twenty.com/user-guide/workflows/overview>

  (C10, L; after T95 and T99)
      **Done (2026-10-25, not released):** requirement CRM-R40 first, then:
      - **Model:** `workflows` and `workflow_runs` (migration `000030`). Pure rules in `rules::workflows` (triggers, the closed `Action` enum with `deny_unknown_fields`, `Conditions`, `check`, `guard`); the engine in `workflows.rs`; routes in `controllers/workflows.rs`.
      - **Conditions:** the segment filter clauses without the consent gate (`segment::clauses_match`; `matches` still ANDs consent), as their own type that rejects unknown clauses.
      - **Triggers hooked:** contact create, contact update, and a deal's non-closing stage move; `schedule` runs on the existing minute job.
      - **Safety:** actions in a savepoint (a failing one undoes all, run logged `failed`); depth 3 and 60 runs a minute; consent read inside `enrol_nurture`; dry run over up to 500 contacts with no writes but its own audit row.
      - **API:** `GET/POST /api/workflows`, `GET/PUT/DELETE /api/workflows/{pid}`, `POST /api/workflows/{pid}/dry-run`, `GET /api/workflows/{pid}/runs`, `POST /api/workflows/run-due`. A reader may list but not save or try (derives `write`). Notifications gained the kind `workflow`.
      - **UI:** `/workflows` (nav): list, editor (trigger, condition checkboxes, action editor), dry run and run log; 39 keys in 8 locales; sample workflow and run in the tour; Playwright test.
      - **Tests:** 9 rules unit tests; 8 request tests (a task made only for matching contacts with the run's reason, notify/set_field/send_webhook, the consent re-check, the loop cut and the rate hold, the dry run changing nothing, the schedule interval, a deal stage move, the closed set refused, a failing action undone) and 1 under sign-in. Service: see the changelog; UI: 146 unit and 115 Playwright; clippy clean both feature sets.
      - **Limits:** see CRM-R40 "Not built". A workflow edit restarts a schedule's scan from the first contact. Rollback of a failed run is tested by renaming a table; other failure modes use the same path.
- [x] CRM-T105 **Typed custom fields.** An admin declares a field on contacts, accounts, leads or deals (text, number, date, choice); each field declares its sensitivity so masking, encryption, subject access and erasure treat it like a built-in one. Shown on the record page, usable in saved views and segments. *Source:* `twentyhq/twenty` (`engine/metadata-modules/field-metadata`), `espocrm/espocrm` (`Tools/FieldManager`), `frappe/crm` (`fcrm/doctype/crm_fields_layout`). From the 2026-10-25 sota-scan. (L)
      **Done (2026-10-25, not released):** requirement CRM-R41 first, then:
      - **Model:** `custom_field_defs` and `custom_field_values` (migration `000031`), values keyed by field and record pid with no foreign key. Rules in `rules::custom_fields` (key, kinds, choices, `check_value`), DB-free, 6 tests.
      - **Sensitivity** (`business`, `personal`, `sensitive`): masked reads hide personal values; the subject-access export lists them (including those set on merged-in contacts); erasure deletes personal and sensitive values of the contact, its merged duplicates and the leads it scrubs; sensitive values are sealed with `crypto::seal` and `rotate_field_keys` re-seals only those.
      - **API:** `GET/POST /api/custom-fields`, `PUT/DELETE /api/custom-fields/{pid}`, and `GET/PUT` of `/api/{contacts,accounts,leads,deals}/{pid}/custom-fields`.
      - **UI:** `/custom-fields` and a form component on the four record pages; 23 keys in 8 locales; sample fields in the tour; Playwright test.
      - **Tests:** 4 request tests (kinds and refusals; every record kind and deleting a field; a choice in use; sealing, rotation, export and erasure) and 1 under sign-in (masked read; a reader cannot set or declare).
      - **Not built:** fields in saved views, segments, lists, CSV import, search, webhooks and workflow conditions; multi-choice and lookup kinds; changing kind or sensitivity; erasure of account and deal values. The task asked for use in views and segments; that is the next step.
- [x] CRM-T106 **File attachments.** Attach files to contacts, accounts, deals and tickets, stored outside the database, with a size and type allow-list, a malware-scan hook, subject access and erasure coverage. *Source:* `twentyhq/twenty` (`modules/attachment`), `espocrm/espocrm` (`Entities/Document.php`). The plan lists it under "Later". (L)
      **Done (2026-10-25, not released):** requirement CRM-R42 first, then:
      - **Model:** `attachments` (migration `000032`); bytes in a directory (`src/attachments.rs`: stage in `incoming/`, scan, rename into place; random file names). Rules in `rules::attachments` (allow-list, magic bytes, name cleaning, `Content-Disposition`), DB-free, 5 tests.
      - **API:** upload (raw body, `?name=`), list, content, delete, under all four record kinds; OpenAPI documents the upload as `application/octet-stream`.
      - **Safety:** the type from the allow-list, not the request; digest checked on download; scan exit codes (0 clean, 1 infected, else refused); `nosniff`; a masked reader sees no files; the audit row holds no name.
      - **Privacy:** subject access lists the files; erasure deletes rows in the transaction and files after it.
      - **UI:** `Attachments` panel on the four record pages; 7 keys in 8 locales; the tour keeps a file's name and size, not its bytes; Playwright test.
      - **Tests:** 4 request tests (store, list, download, tamper, refusals and delete; off until configured; infected and unscannable refused with nothing kept; subject access and erasure) and 1 under sign-in (a masked reader).
      - **Not built:** see CRM-R42. Storage is one directory; there is no S3 backend, no encryption of files at rest, and no sweep of files orphaned by a crash between disk and database.
- [x] CRM-T107 **Contact and account CSV import.** Extend the lead import (CRM-R34: column mapping, dry run, duplicate advice, undo) to contacts and accounts, with the person-service lookup for contacts. *Source:* `marmelab/atomic-crm` (`dataImport/`), `krayin/laravel-crm` (`packages/Webkul/DataTransfer`), `espocrm/espocrm` (`Tools/Import`). (M)
      **Done (2026-10-25, not released):** requirement CRM-R43 first, then:
      - **Model:** `record_imports` (migration `000033`). Pure rules in `rules::record_imports` (mappings, suggestions, row judging, account name key), 4 tests. `lead_imports::advise` is split out of `analyse` and shared.
      - **Lookups:** a contact's name and address go to the person service (`resolve_person`, already used by web forms); an account's organization is looked up by name (`resolve_organization`, new, the same contract assumed) unless the file gives its `organization:` reference. A lookup that fails fails that row only.
      - **Safe by default:** no consent, no workflows, and the address kept nowhere. Existing people and organizations are left out and counted.
      - **API:** preview, commit, list, read and undo under `/api/contact-imports` and `/api/account-imports`.
      - **UI:** `RecordImport` component behind `/contacts/import` and `/accounts/import`, linked from the lists; 12 keys in 8 locales; Playwright test.
      - **Tests:** 3 request tests (a contact import end to end, with advice, no consent, no workflow and a partial undo; a person service that cannot answer; an account import with tiers, duplicates and a partial undo).
      - **Not built:** see CRM-R43 (company-to-account linking, owner, updating existing records).
- [x] CRM-T108 *(edge)* **Lead assignment rules.** Round-robin or load-based owner assignment of new leads, as a workflow action (CRM-R40) with a log. *Source:* `frappe/crm` (`crm/api/assignment_rule.py`). (S/M)
      **Done (2026-10-25, not released):** requirement CRM-R44 first, then:
      - **Model:** `leads.owner_ref` and `workflows.assign_cursor` (migration `000034`). Rules in `rules::workflows`: the trigger, the `sources` condition (refused where it cannot apply), `Action::AssignOwner`, and the pure `pick` (round robin by counter, least open by load), with 4 more unit tests.
      - **Engine:** the run's subject is now a contact or a lead (`Target`); `capture_lead` fires `lead_created` (imports excluded) and reloads the lead; `assign_owner` takes the counter under a row lock, reads the loads, sets the owner, audits `assigned` and can notify; the dry run of a lead workflow walks the newest leads and shows each turn without moving the counter.
      - **Also changed:** a lead's owner carries to the contact on conversion; the dry-run sample names its subject (`subject_kind`, `subject_pid`), which replaces `contact_pid` of CRM-R40's unreleased API.
      - **UI:** trigger, source condition and the assign action on `/workflows` (only the actions that fit the trigger are offered); the lead page shows its owner; 9 keys in 8 locales; Playwright test.
      - **Tests:** 4 request tests (in turn with notification and audit; by load; source condition and imports left out; refusals and a dry run that assigns nothing).
      - **Not built:** see CRM-R44 (no hand-assignment, capacity, availability or reassignment; leads from an import are not assigned).
- [x] CRM-T109 *(edge)* **SLA working hours and holidays.** Ticket SLA clocks that pause outside working hours; today they are 24x7 (`rules/sla.rs`). *Source:* `frappe/crm` (`crm_service_level_agreement`: `working_hours`, `holiday_list`). (M)
      **Done (2026-10-25, not released):** requirement CRM-R45 first, then:
      - **Model:** `business_calendars` and `sla_policies.calendar_pid` (migration `000035`). `rules::calendar` (`Calendar::build` checks the zone, the windows and the holidays; `add_working_minutes` walks the days, skipping holidays and time outside the windows) with 6 tests, including a weekend, a holiday, a lunch break and New York's change to summer time; `sla::deadlines_on` with one more. `chrono-tz` is now a direct dependency (it was already in the tree).
      - **Wiring:** `targets_for` returns the policy's calendar with its targets; a new ticket and an audited priority change use it. Deadlines stay stored on the ticket, so breach flags, the sweep and the dashboards needed no change.
      - **API:** `/api/business-calendars` (CRUD, 409 while used) and `PUT /api/sla-policies/{pid}/calendar`.
      - **Tests:** 2 request tests (calendars checked and kept while used; a ticket moved to a calendar priority gets Friday 17:00 and Wednesday 10:00 with a Monday holiday between, and goes back to round-the-clock).
      - **Not built:** a UI for policies and calendars (the SLA settings have no screen today); pausing while waiting for the customer; per-account calendars; re-deriving open tickets when a calendar changes.
- [x] CRM-T110 *(edge)* **Phone-width layout check.** The UI has one media query (`+layout.svelte`); test the main lists, boards and forms at 360 px and fix what breaks. *Source:* `marmelab/atomic-crm` (rated well on mobile in the January 2026 benchmark). (M)
      **Done (2026-10-25, not released):** requirement CRM-R46 first, then:
      - **Test first:** a Playwright test walks 32 screens at 360 by 740 and fails on any page wider than the viewport. It failed on all of them: 14 px on every page (the header's picker buttons), 27 px on the pages with forms, 49 px on the custom-fields page.
      - **Fixes:** below 40 rem the top bar and the header's end wrap (`+layout.svelte`), and a field never exceeds its container (`app.css`, one media rule). A file chooser on the record pages needed the same limit; a second run found it.
      - **Checked by eye:** a screenshot of a contact page at 360 px, header wrapped into three rows.
      - **Not done:** the rest of the task's wording, "fix what breaks", was limited to sideways overflow; I did not check touch-target sizes, the boards on a phone, or text at the larger text sizes.
- [x] CRM-T111 *(edge)* **Tags.** Free labels on contacts, accounts, leads and deals, shown on the record and filterable in lists, saved views and segments; erased and exported with the record like a custom field of sensitivity `personal`. *Source:* `krayin/laravel-crm` (`packages/Webkul/Tag`), `marmelab/atomic-crm` (`src/components/atomic-crm/tags`). From the 2026-10-25 re-scan. (M)
      **Done (2026-10-25, not released):** requirement CRM-R47 first, then:
      - **Model:** `tags` and `taggings` (migration `000036`). `rules::tags` (name tidying, case-insensitive key, a record's set; 3 tests). `Filter` and `ContactFacts` of `rules::segment` gained `tags` (any of), with one more test.
      - **API:** tag CRUD, a record's tags (get and set whole), `?tag=` on four lists, a `tags` clause on segments (tidied when saved; consent still gates).
      - **Privacy:** a masked read gets `403` on a record's tags, and a tag filter drops masked records; the export lists the names, erasure deletes the taggings; the audit row counts and names nothing.
      - **UI:** `Tags` panel on four record pages, `TagFilter` on three lists; 7 keys in 8 locales; tour data; Playwright test.
      - **Tests:** 4 request tests (labels; a record's set and the list filter; a segment selecting by tag; export and erasure) and 1 under sign-in (a masked reader).
      - **Not built:** the task also said saved views. A saved view filters rows already in the browser, and list rows do not carry tags, so a tag in a view is not built; the list's own tag selector does the job, with the service filtering. Also not built: tags in workflow conditions, and bulk tagging.
- [x] CRM-T112 *(edge)* **A report builder.** Pick a record kind, filters (the saved-view and segment language), a grouping and a measure (count, sum of an amount), and see a table or chart that is saved like a view; every number is computed on the service and masked like the lists. *Source:* `twentyhq/twenty` (`modules/dashboard`), `espocrm/espocrm` (reports in the admin guide). (L)
      **Done (2026-10-25, not released):** requirement CRM-R48 first, then:
      - **Model:** `reports` (migration `000037`). `rules::reports` (`Spec`, the per-kind field table, `plan` building the SQL from fixed text with bound values), 6 tests including one that a hostile tag value never reaches the SQL text.
      - **API:** `POST /api/reports/run`, `GET/POST /api/reports`, `PUT/DELETE /api/reports/{pid}`, `GET /api/reports/{pid}/run`; a reader may run and save (they derive `read`); the saved ones follow the saved-view rules (`owner_of` shared with `views.rs`).
      - **Honesty:** sums per currency, never merged; `as_of`; `truncated`; a masked collection read may count but not sum or use tags.
      - **UI:** `/reports` (builder, bars, save, reopen), in the navigation; 46 keys in 8 locales; tour data computed over the sample records; Playwright test.
      - **Tests:** 4 request tests (counts by field, tag and date; per-currency sums; refusals with the member; saved reports) and 1 under sign-in (a masked reader counts but cannot sum or use tags; shared and private reports).
      - **Not built:** see CRM-R48 (no joins across kinds, no second grouping, no average, no custom fields, no export, no schedule). **Not tested:** the 200-group cut, which needs more than 200 groups.
- [x] CRM-T113 *(edge)* **Multi-touch attribution.** First-touch, last-touch and linear models over the touches a campaign has with a won deal's primary contact (e-mails sent, campaign leads, the recorded source campaign), credited exactly to the minor unit and per currency; the roadmap item "multi-touch attribution". (M)
      **Done (2026-10-26, not released):** requirement CRM-R49 first, then:
      - **Model:** no table. `rules::attribution` (`Model`, `Touch`, `credit`, `window_days`): the shares of a deal add up to its amount exactly, the leftover minor units going to the earliest touches; 9 tests, including that the order the touches arrive in changes nothing.
      - **Touches (read in SQL):** e-mails with `state = 'sent'` to the deal's primary contact, campaign leads of that contact, from the window before the deal closed up to its close; the deal's `source_campaign_pid` always, at the deal's creation. **Found while testing:** `email_deliveries` is unique per (campaign, contact), so a campaign sends a contact one e-mail; the requirement first said two e-mails from one campaign are two touches, which cannot happen, and was corrected to say what can (its e-mail, a lead it brought in, being the source).
      - **API:** `GET /api/attribution?model=&window_days=&from=&to=` (derives `read`); `422` naming `/model`, `/window_days` or `/from`; `403` on a masked collection read; audits `attribution_run`. At most 5000 deals (`truncated`).
      - **UI:** `/attribution` (model, window, dates; a row and bar per campaign and currency; the unattributed deals apart), in the navigation; 20 keys in 8 locales (machine drafts); tour data (`TOUCHED` in the demo, with the same arithmetic in TypeScript, so the tour's figures are the service's rule, not a second opinion); Playwright test.
      - **Tests:** 2 request tests (the three models, the window, the recorded source, the untouched deal kept apart, every pound accounted for, the date range, audit; the refusals naming the member) and 1 under sign-in (a masked reader gets `403`, an unmasked one `200`, no token `401`).
      - **Not built:** see CRM-R49 (no time-decay or position model, no per-person open or click touches, no touches by the account's other contacts, no cost per attributed pound, no stored result). **Not tested:** the 5000-deal cut, which needs more than 5000 won deals.
- [x] CRM-T114 *(edge)* **FX normalisation for reports.** A hand-entered table of exchange rates with a day and a source, and a deal report that shows its sums in one currency using the latest rate on or before a chosen date, naming the rates used and what it could not convert; the roadmap item "FX normalization". (M)
      **Done (2026-10-26, not released):** requirement CRM-R50 first, then:
      - **Model:** `fx_rates` (migration `000038`: a pair, `rate_ppb` as an integer number of billionths, the day, the source; unique per pair and day). `rules::fx` (`parse_rate`, `format_rate`, `pick`, `convert`, `convert_rows`) is integer-only and DB-free; 11 tests, including half-to-even rounding on both sides of zero, saturation at the ends of `i64`, and that a date picks the rate in force.
      - **API:** `GET/PUT /api/fx-rates`, `DELETE /api/fx-rates/{pid}` (a write, so a reader may list but not change; each change audited with the pair, rate, day and source). The report spec gains `convert_to` and `rate_date`; the answer gains `rate_date`, `rates` (with the source and whether inverted) and `unconverted`. Each group sum is converted once, so the rounding is at most half a minor unit per group and currency.
      - **UI:** `/fx-rates` (list, set, delete), "Show in" and "Rates as of" on the Reports page with the rates and the left-out sums shown; 18 keys in 8 locales (machine drafts); the tour has three sample rates and the same arithmetic in TypeScript (exact, `BigInt`); two Playwright tests.
      - **Tests:** 4 request tests (set, replace, list, delete and audit; the refusals naming the member; a report converted on three dates, through an inverse rate, with an unconvertible currency left out and counted, and the refusals; a saved report keeping its conversion) and 1 under sign-in (a reader lists but cannot set or delete).
      - **Not built:** see CRM-R50 (no feed of rates, no cross rates, no rate at each deal's close day, no minor-unit exponents other than hundredths, no conversion of the forecast, ROI, CLV or attribution, no bulk import). **Not tested:** a rate near the limits through the API (the arithmetic is tested in `rules::fx`).
- [x] CRM-T115 *(edge)* **Converted attribution.** `GET /api/attribution` shows each campaign's credit and the unattributed deals in one currency, naming the rates used and what had no rate; the first slice of the roadmap item "FX beyond reports". (S)
      **Done (2026-10-26, not released):** requirement CRM-R51 first, then:
      - **Service:** `convert_to` and `rate_date` on the query; the conversion mapping (`RateUsed`, `Unconverted`, `convert_sums`) moved from the reports controller into `controllers::fx_rates` so both use one path. The answer gains `convert_to`, `rate_date`, `rates`, `unconverted`.
      - **UI:** "Show in" and "Rates as of" on `/attribution`, the rates and left-out sums beneath the table (the Reports page's wording, so no new locale keys); the tour converts with the same arithmetic as the Reports page (`convertSums`); one Playwright test.
      - **Tests:** 1 request test (a campaign with pounds and dollars converted at the rate in force, a campaign whose only credit is yen left empty and counted, the unattributed deals converted, the plain answer unchanged, the three refusals naming the member).
      - **Also:** the CRM-T114 request tests had not been run against a database when committed; they were run now (with the attribution, report and enforcement tests, 15 passing).
      - **Not built:** see CRM-R51 (forecast, campaign ROI and CLV stay per currency).
- [x] CRM-T116 *(edge)* **Converted campaign ROI.** The campaign funnel can count revenue won in other currencies in the campaign's own currency at the rate in force, naming the rates and what had none; the second slice of "FX beyond reports". (S)
      **Done (2026-10-26, not released):** requirement CRM-R52 first, then:
      - **Service:** `convert` and `rate_date` on `GET /api/campaigns/{pid}/funnel`, using the shared `fx_rates::convert_sums`; the answer gains `rate_date`, `rates`, `unconverted`.
      - **UI:** a checkbox on the Campaigns page that asks again with `convert=true`, with the rates and left-out amounts shown (the Reports page's wording; one new key, `campaign.convert`, in 8 locales, machine drafts); the client's `campaignFunnel` takes options; the smoke mock now carries the new members; a new smoke test.
      - **Tests:** 1 request test (dollars counted at 0.80 so revenue and ROI change; yen with no rate kept apart and counted; the plain answer unchanged; two refusals naming `/rate_date`).
      - **Not built:** see CRM-R52. **Not tested:** the converted funnel under a masked read (the masking code path is unchanged and returns early values as before).
- [x] CRM-T117 *(edge)* **Converted forecast.** `GET /api/forecast` can add one total in a chosen currency at the rate in force, naming the rates and what had none; the third slice of "FX beyond reports". (S)
      **Done (2026-10-26, not released):** requirement CRM-R53 first, then:
      - **Service:** `convert_to` and `rate_date` on the query, through the shared `fx_rates::convert_sums`; the answer gains `convert_to`, `converted_minor`, `rate_date`, `rates`, `unconverted`; `totals_minor` is untouched.
      - **UI:** a "Show in" box on the deal board's forecast strip (the Reports page's wording, so no new locale keys); `forecast()` takes options; the tour converts with the shared `convertSums`; one Playwright test. The full smoke and tour files pass (116).
      - **Tests:** 1 request test (per-currency totals unchanged; the dollars added at 0.80; yen counted as left out; the plain answer unchanged; three refusals).
      - **Not built:** see CRM-R53. **Not tested:** the converted forecast under a masked read.
- [x] CRM-T118 *(edge)* **Converted CLV.** `GET /api/accounts/{pid}/clv` can add one total in a chosen currency at the rate in force, naming the rates and what had none; the last slice of "FX beyond reports" that the rates can serve. (S)
      **Done (2026-10-26, not released):** requirement CRM-R54 first, then the same `convert_to` and `rate_date` as the forecast, through `fx_rates::convert_sums`; `clv_minor` is untouched. No UI (no page shows CLV). **Tests:** 1 request test (per-currency CLV unchanged; the dollars added at 0.80; yen counted as left out; the plain answer unchanged; three refusals). Full PostgreSQL suite passing.
      - **Not built:** see CRM-R54.
      - **Found:** `GET /api/accounts/{pid}/clv` did not call `aggregate_masked`; fixed in CRM-T119.
- [x] CRM-T119 **CLV is masked.** `GET /api/accounts/{pid}/clv` returned an account's money to a caller whose collection read is masked; every other money read (forecast, funnel, attribution, reports) is masked. (S)
      **Resolution (2026-10-26, not released):** the handler now calls `aggregate_masked`: a masked caller gets `won_deals` and `null` for `clv_minor` and `converted_minor` (the single-record shape the forecast and funnel use, not `403`); no token is still `401`. `clv_minor` is now nullable in the OpenAPI document and the UI's generated types. **Test:** `an_accounts_clv_is_money_so_a_masked_reader_sees_null_and_an_unmasked_one_the_figures` failed first (the masked reader got `clv_minor: {}`), then passed; the full PostgreSQL suite passes (167). **Not done:** a sweep for other money reads that skip masking: only CLV was found by reading, and nothing checks that every money-returning handler calls `aggregate_masked`.
- [x] CRM-T120 **A sweep for money that skips masking.** CRM-T119 found a money read that did not call `aggregate_masked` only by reading one handler; nothing checked the rest. (S)
      **Done (2026-10-26, not released):** `no_read_a_masked_caller_may_make_returns_the_seeded_amount` (in `tests/requests/enforcement.rs`). The service seeds a pipeline, contact, account, segment, campaign (cost), an open and a won deal (both carrying the amount, with the account, contact and source campaign), a lead and a saved deal-sum report, all with one distinctive amount. The test then reads the OpenAPI document and asks **every `GET` path** in it as the service (the amount must show on at least six, so the test cannot pass vacuously) and as a masked reader (it must show on none, and nothing may answer `5xx`); `{pid}` is filled with a record of the right kind. **Checked by breaking it:** with the CLV masking from CRM-T119 removed, the test fails and names `/api/accounts/{pid}/clv`; restored, it passes. **It found nothing else** on the paths it covers; the full PostgreSQL suite passes (168).
      - **Not covered:** `POST` and other writes (a reader cannot write, but a response to a permitted write is not checked); the MCP tools (they call these same paths); paths with a `{pid}` of a kind the test does not seed (tickets, groups, workflows, webhooks, business calendars, attachments, custom fields, tags, views; none carries an amount); `/api/audits*` (privileged, and not seeded with the amount); the public token paths and `/metrics.prom`; amounts that appear only after arithmetic (a weighted forecast is not the seeded number, so a leak of only a derived figure would pass; the forecast, funnel and reports are masked by their own tests). The subject-access export (`/api/contacts/{pid}/subject-access`) returns the contact's deals with amounts to a caller the policy allows; a masked reader is refused there, which is what the sweep saw.
- [x] CRM-T121 *(edge)* **Pause-on-pending SLA clocks.** The SLA clock stops while a ticket waits for the customer and the deadlines move by the waiting when it resumes, in working time on a calendar; the roadmap item "pause-on-pending SLA clocks". (M)
      **Done (2026-10-26, not released):** requirement CRM-R55 first, then:
      - **Model:** migration `000039` adds `paused_at`, `first_response_paused_seconds` and `resolution_paused_seconds` to `tickets`. `Calendar::add_working_seconds` and `working_seconds_between` (the second agrees with the first, tested over a holiday and the zone's clock); `rules::sla`: `deadlines_paused`, `clock_now`, `pause_counted`, `Paused`; 7 SLA and 2 calendar unit tests.
      - **API:** `POST /api/tickets/{pid}/status` stops the clock on `pending` and, on leaving it, adds the waiting and derives the deadlines again; the priority change keeps the waiting; the ticket list, the sweep, the SLA dashboard and the support insights judge a paused ticket at the moment it was paused.
      - **UI:** the tickets page marks a waiting ticket "SLA clock stopped" (`ticket.paused`, 8 locales, machine drafts); the tour's status change sets and clears it; `Ticket.paused_at`; one Playwright test.
      - **Tests:** 1 request test (a deadline that falls while paused is not a breach and a running twin is, and the sweep flags only the twin; resuming gives back 30 minutes on both deadlines; a priority change keeps them; a pause after the deadline gives nothing back). **Checked by breaking it:** with the list reading the present instead of the paused moment, and with the resume not moving the deadlines, the test fails each time.
      - **Not built:** see CRM-R55. **Not tested:** a calendar-backed policy through the API (the working-time arithmetic is tested in `rules::calendar` and `rules::sla`); the SLA dashboard and support insights with a paused ticket (they share `clock_now`, but no request test reads them).
- [x] CRM-T122 *(edge)* **Time-decay and position-based attribution.** Two more models for `GET /api/attribution`, exact to the minor unit; part of the roadmap item "more attribution models". (S)
      **Done (2026-10-26, not released):** requirement CRM-R56 first, then:
      - **Rules:** `rules::attribution::credit` takes the deal's close time and shares by integer weights through one `apportion` (floors, leftovers to the largest remainders, earliest on a tie); `Model::TimeDecay` and `Model::Position`; `decay_weight` is `2^-(age / 7 days)` held in integers. The existing `first`, `last` and `linear` unit tests pass unchanged. 4 new unit tests (4 : 2 : 1 at 0, 7 and 14 days; steady fall between; 40 : 20 : 40 and the middle sharing; leftovers; both shares for a campaign that touched twice, under all five models).
      - **API:** the two names accepted and listed in the refusal; the close time read with the won deals.
      - **UI:** two more options on `/attribution` (2 keys in 8 locales, machine drafts); the tour shares with the same integer arithmetic (`BigInt`); `crm.ts` types the models; one Playwright test, with figures cross-checked against an independent calculation.
      - **Tests:** 1 request test (position 40 / 20 / 40 over three e-mails; time-decay ranks recent above middle above early and adds up to the amount); the refusal test now uses an unknown model name.
      - **Not built:** see CRM-R56. **Not tested:** the half-life through a window where a touch is a year old (the unit tests cover the arithmetic at 300 days).
- [x] CRM-T123 *(edge)* **Public help centre and deflection report.** Published knowledge-base articles open to anyone, off until turned on, counted without identifying anyone, and a report of how many questions they answered beside the tickets raised; the roadmap item "public KB portal + ticket deflection metrics". (M)
      **Done (2026-10-26, not released):** requirement CRM-R57 first, then:
      - **Service:** `CRM_PUBLIC_KB` and `CRM_PUBLIC_KB_CLIENT_LIMIT` settings; `/kb/` public paths; migration `000040` (`kb_daily`: counts per day and article, nothing else); `controllers::kb` (`GET /kb/articles`, `GET /kb/articles/{pid}`, `POST /kb/articles/{pid}/feedback`, and `GET /api/insights/kb-deflection`); the three public paths described in the OpenAPI document by hand, the report in the operations table, five schemas.
      - **UI:** `/help` and `/help/{pid}` (public, linked from the footer and the signed-out menu), a "Public help centre" panel on `/articles`, `kbSearch`, `kbArticle`, `kbFeedback` and `kbDeflection`, the tour's own help centre with counts, 19 keys in 8 locales (machine drafts).
      - **Tests:** 3 request tests (off means `404` on all three; published articles listed and read, a draft, an archived and a missing article `404`, search and its empty count, votes, a refused unknown member, and the report's every figure; the limit answers `429` with `Retry-After`) and 2 UI tests (a signed-out visitor reads an article, the text shown as text and not as markup, and votes; the tour's help centre and the staff panel's counts); a `/kb/articles` case in the public-path unit test and in the settings test. **Checked by breaking it:** with the off switch removed, and with the published-only filter removed, the request tests fail; each was restored.
      - **Not built:** see CRM-R57. **Not tested:** the limit behind a proxy with the forwarded address trusted (the key function is tested in `rate_limit`); the masked sweep (CRM-T120) does not reach `/kb/` (it asks only paths with a seeded kind), and the portal returns no money.
- [x] CRM-T124 *(edge)* **Opens and clicks as attribution touches.** Per-person e-mail opens and clicks, as a mail provider reports them and only while the person consents, usable as touches in `GET /api/attribution`; the rest of the roadmap item "more attribution models". (M)
      **Done (2026-10-26, not released):** requirement CRM-R58 first, then:
      - **Service:** migration `000041` widens the `email_events` check to `opened` and `clicked`; the five vendor parsers (Brevo, Mailjet, Mailgun, SendGrid, SES) read them (Azure's engagement events are not read); the native webhook accepts both; `apply_event` records an open or click only while consent for the campaign's purpose is `granted` and the mailbox is not suppressed (`ignored` otherwise), counts the first on the campaign, and leaves a repeat as `duplicate`; `GET /api/attribution?touches=` (a list of `sent`, `opened`, `clicked`, default `sent`) reads the events in the window, and the answer names `touches`.
      - **UI:** three checkboxes on `/attribution` (5 keys in 8 locales, machine drafts), the run button off with none chosen, the overcounting caveat beside them; the tour has a provider-reported open and click; `attribution()` takes `touches`; one Playwright test.
      - **Tests:** the parsers' old "opens are ignored" assertions now assert the new mapping (and that `request`, `unsub`, `processed` and `Send` stay ignored); 1 request test (an open and a click recorded once, a repeat is a duplicate, a withdrawn contact's open is ignored and not counted, the campaign counters, and attribution by `sent`, `opened`, `clicked` and all three, and a refusal naming `/touches`). **Checked by breaking it:** with the consent gate switched off the request test fails; restored. A first attempt at that check did not apply (the code had been reformatted) and passed, so it was redone before being trusted.
      - **Not built:** see CRM-R58. **Not tested:** a real provider's open and click payloads (the parsers are tested against the documented shapes as the existing ones are); the suppressed-mailbox half of the gate (the consent half is tested).
- [x] CRM-T125 *(edge)* **Quotas and commission plans.** What each worker is to win in a period, a tiered plan that pays on it, and where each worker stands; the roadmap item "quotas & commission views". (M)
      **Done (2026-10-27, not released):** requirement CRM-R59 first, then:
      - **Service:** migration `000042` (`commission_plans`, `quotas`); `rules::quota` (pure marginal-tier arithmetic in integers); `controllers::quotas` (plans, quotas, attainment); `/api/quotas` and `/api/commission-plans` added to the admin-only prefixes; operations and schemas described in the OpenAPI document.
      - **UI:** `/quotas` (standing, quotas, plans), 26 keys in 8 locales (machine drafts), typed calls in `crm.ts`, the tour's plan and two quotas computed from its won deals, regenerated `schema.d.ts`; one Playwright test. Also repaired the demo-data unit test that CRM-T123 left failing on `/kb/articles/x`.
      - **Tests:** 3 request tests (a plan set, replaced, checked and kept while a quota uses it; refusals naming the member; attainment and commission) and 1 enforcement test (admin and service only, reads included).
      - **Not built:** see CRM-R59.
- [x] CRM-T126 *(edge)* **Margin and a discounted look-ahead on CLV.** Two further ways to read `GET /api/accounts/{pid}/clv`; the roadmap item "CLV refinement". (S)
      **Done (2026-10-27, not released):** requirement CRM-R60 first, then:
      - **Rules:** `analytics::margin` and `analytics::projected` (integers, half to even, overflow refused); 1 unit test.
      - **API:** `margin_bps`, `retention_bps`, `discount_bps`, `years` on the CLV read, `margin_minor` and `projection` in the answer, null when masked; operation and schema (`ClvProjection`) described; `schema.d.ts` regenerated.
      - **Tests:** 1 request test (margin per currency, a three-year projection worked by hand, and six refusals).
      - **Not built:** see CRM-R60; no UI.
- [x] CRM-T127 *(edge)* **CLV panel on the account page.** The UI half of CRM-R60: lifetime value, its margin and the discounted look-ahead on `/accounts/{pid}`. (S)
      **Done (2026-10-27, not released):**
      - **UI:** `ClvPanel` (percent inputs sent as basis points, a masked state, `as_of`), `accountClv` and `Clv` in `crm.ts`, 12 keys in 8 locales (machine drafts), the tour's `/accounts/{pid}/clv` (integers, half to even, the same refusals).
      - **Tests:** 1 Playwright test (won £88,000; margin 50% gives £44,000; three years at 80% retention give £85,888 and a total of £129,888; a retention above 100% is refused). Type check, lint, vitest (164) and the whole Playwright suite (136) pass.
      - **Not tested:** the masked state and a discount in the browser (the service tests cover both); the panel against the real service.
- [x] CRM-T128 *(edge)* **Retention from an account's own history.** `retention_from=history` on the CLV read, in place of a retention the caller guesses; the rest of the roadmap item "CLV refinement" apart from conversion. (S)
      **Done (2026-10-27, not released):** requirement CRM-R61 first, then:
      - **Rules:** `analytics::retention_from_history` (integers, half to even, held at 100%, `None` with no year before); 1 unit test.
      - **API:** `retention_from`, `projection.retention_history_bps`, `retention_bps` now nullable; the refusals; operation text; `schema.d.ts` regenerated.
      - **UI:** a checkbox and the retention found per currency in `ClvPanel`, the tour's stub, 2 keys in 8 locales (machine drafts); the Playwright test extended.
      - **Tests:** 1 request test (deals backdated by SQL: 75% found, 7,875 projected, a currency with no year before projects 0 with `null`, three refusals). **Checked by breaking it:** with the year-before window disabled the request test fails; restored.
      - **Not tested:** a retention above 100% through the API (the unit test holds it at 100%); the browser against the real service.
- [x] CRM-T129 *(edge)* **Convert the CLV margin and look-ahead.** `convert_to` on the CLV read also converts the margin and the look-ahead; the last open part of the roadmap item "CLV refinement". (S)
      **Done (2026-10-27, not released):** requirement CRM-R62 first, then:
      - **API:** `margin_converted_minor`, `projection.converted` (`ClvProjectionConverted`), one `convert_sums` call over the margin and look-ahead groups so the CLV's `rates` and `unconverted` are unchanged; operation text; `schema.d.ts` regenerated.
      - **UI:** a "Show in" field and the converted line in `ClvPanel`, the tour's conversion (also giving the CLV's own `converted_minor`), 1 key in 8 locales (machine drafts); the Playwright test extended.
      - **Tests:** 1 request test (GBP, USD at 0.80 and an unconverted JPY: margin 7,000, base 7,000, projected 5,250, total 12,250; per-currency figures kept; nothing converted without `convert_to`; the margin alone).
      - **Not tested:** that margin and base are not swapped (both are 7,000 in the test; projected and total differ); the masked caller (the `convert_to` path is skipped when masked, as for the CLV).
- [x] CRM-T130 *(edge)* **A customer reply resumes a waiting ticket.** `POST /api/tickets/{pid}/customer-reply`; part of the roadmap item "SLA clock refinements". (S)
      **Done (2026-10-27, not released):** requirement CRM-R63 first, then:
      - **API:** the body of the status change became one `transition` shared with the new route, so the clock arithmetic is not duplicated; `via: customer_reply` in the audit snapshot; operation described; `schema.d.ts` regenerated.
      - **UI:** a "Customer replied" button on a waiting ticket, `ticketCustomerReply`, the tour's route, 1 key in 8 locales (machine drafts); the Playwright test for the stopped clock extended.
      - **Tests:** 1 request test (an open ticket is unchanged; after 30 minutes pending the reply opens it, clears `paused_at` and moves the resolution deadline by 30 minutes; the audit names the reply; a resolved ticket is `422`). The existing status and pause tests pass unchanged.
      - **Not tested:** `404` for an unknown ticket; the masked/permission path for the new route (it is a write, like `/status`).
- [x] CRM-T131 *(edge)* **A limit on how long a ticket may wait.** `max_wait_minutes` on an SLA policy and a sweep that flags a ticket left `pending` too long; the last part of the roadmap item "SLA clock refinements" that needs no inbound mail. (M)
      **Done (2026-10-27, not released):** requirement CRM-R64 first, then:
      - **Service:** migration `000043`; `PolicyPayload.max_wait_minutes`; `sweep_waiting` (once per ticket, event `ticket_wait_limit_exceeded`); `new_wait_limits_exceeded` in `SlaSweep`; the flag cleared in `transition`; `schema.d.ts` regenerated.
      - **UI:** "Waiting too long" beside the SLA state, `wait_limit_exceeded_at` on `Ticket`, a flagged ticket in the tour, 1 key in 8 locales (machine drafts); one Playwright test.
      - **Tests:** 1 request test (a limit of 0 is refused; of three waiting tickets only the one past its limit is flagged, a ticket with no limit and one inside it are not; it stays `pending` and paused; a second sweep finds none; a customer reply clears it). **Checked by breaking it:** with the clearing removed the test fails; restored.
      - **Not tested:** the emitted event's payload; a policy changed after tickets were flagged.
- [x] CRM-T132 *(edge)* **Split a won deal between workers.** Shares of a deal for quota attainment; part of the roadmap item "territories; quota refinements". (M)
      **Done (2026-10-27, not released):** requirement CRM-R65 first, then:
      - **Service:** migration `000044` (`deal_splits`); `rules::quota::split` (exact, largest remainders, earliest on a tie); `GET`/`PUT`/`DELETE /api/quotas/splits/{pid}` inside the admin-only `/api/quotas` prefix; `credited_deals` replaces the single sum behind attainment; operations and schemas described; `schema.d.ts` regenerated. Two clippy warnings from CRM-T129/T130 fixed on the way.
      - **UI:** a split form on `/quotas`, `setDealSplit`, `deleteDealSplit`, the tour's splits and attainment, 6 keys in 8 locales (machine drafts); one Playwright test.
      - **Tests:** 1 unit test (sums, tie order, zero), 1 request test (an odd amount halved gives the extra minor unit to the earlier share; a deal handed wholly to another worker leaves the owner with nothing; refusals for empty, short, repeated, non-worker and zero shares; unknown deal `404`; removal restores the owner; a second removal `404`), the admin-only test extended. **Checked by breaking it:** with splits ignored the request test fails; restored.
      - **Not tested:** a split deal whose period straddles two quotas; masked callers (the paths are admin only); eight shares.
- [x] CRM-T133 *(edge)* **Articles suggested for a ticket.** Published articles ranked by word overlap with a ticket's title; part of the roadmap item "help centre refinements". (S)
      **Done (2026-10-27, not released):** requirement CRM-R66 first, then:
      - **Service:** `rules::suggest` (`words`, `suggest`; integers, stable order); `GET /api/tickets/{pid}/suggested-articles`; operation, schema and list schema described; `schema.d.ts` regenerated.
      - **UI:** the suggestions under the selected ticket, `ticketSuggestions`, the tour's version of the ranking, a sixth tour article (so the help centre test counts five), 1 key in 8 locales (machine drafts); one Playwright test.
      - **Tests:** 3 unit tests (words, ranking and weights, the limit of five) and 1 request test (headline match before text match, a draft is never offered, an unrelated article is not, an empty answer, an unknown ticket `404`).
      - **Not tested:** the ranking in a language other than English; a very large knowledge base (all published articles are read per request).
- [x] CRM-T134 *(edge)* **Teams and their quota roll-up.** Teams kept by hand and each team's workers' attainment added per currency; part of the roadmap item "territories; quota refinements". (M)
      **Done (2026-10-28, not released):** requirement CRM-R67 first, then:
      - **Service:** migration `000045` (`teams`); `GET`/`PUT`/`DELETE /api/quotas/teams` and `GET /api/quotas/team-attainment` inside the admin-only `/api/quotas` prefix, reusing `standings`; operations and schemas described.
      - **Tests:** 1 request test, run against Postgres and passing (two workers' quotas and revenue added, a member with no quota listed, a team replaced by name in another case, refusals for no workers, a repeat, a non-worker and no name, removal and a second removal `404`).
      - **UI:** a team form, list and per-currency roll-up on `/quotas`, `listTeams`/`setTeam`/`deleteTeam`/`teamAttainment`, the tour's teams (the attainment rows are now one `attainmentRows` shared with the quota route), 8 keys in 8 locales (machine drafts); one Playwright test; `schema.d.ts` regenerated.
      - **Enforcement:** the admin-only test now covers both read routes (reader and writer `403`, service `200`).
      - **Not tested:** `other_currencies`; a worker on two teams.
- [x] CRM-T135 *(edge)* **Territories of named accounts.** Named accounts in a territory, each in at most one, and the won revenue on them per currency; part of the roadmap item "territories; quota refinements". (M)
      **Done (2026-10-28, not released):** requirement CRM-R68 first, then:
      - **Service:** migration `000046` (`territories`, `territory_accounts` with the organization as primary key, so one territory per account); `GET`/`PUT`/`DELETE /api/quotas/territories` and `GET /api/quotas/territory-attainment` inside the admin-only `/api/quotas` prefix, joining deals to territories through the account's `organization_ref`; operations and schemas described; three clippy lint allowances recorded with their reason.
      - **Tests:** 1 request test (won revenue per currency on two territories, an open deal not counted, a deal with no account listed as unassigned, an account taken by another territory refused naming it, the same name in another case replacing, five refusals and a bad period, removal freeing the account, a second removal `404`); the admin-only test extended to the four routes.
      - **UI:** a territory form, list and per-currency revenue (with the unassigned row) on `/quotas`, `listTerritories`/`setTerritory`/`deleteTerritory`/`territoryAttainment`, the tour's territories (a deal reaches its organization through the contact it is linked to), 7 keys in 8 locales (machine drafts); one Playwright test; `schema.d.ts` regenerated; the changelogs.
      - **Not tested:** a territory with 200 accounts; a total that overflows (it is refused `422`, as written, but not exercised).
- [x] CRM-T136 *(edge)* **Resolve or escalate a ticket that waits too long.** `wait_limit_action` and `escalate_to` on an SLA policy; the rest of the roadmap item "SLA clock refinements" that needs no inbound mail. (M)
      **Done (2026-10-28, not released):** requirement CRM-R69 first, then:
      - **Service:** migration `000047`; `PolicyPayload.wait_limit_action` and `escalate_to` (validated); `sweep_waiting` acts per policy (flag as before, escalate reassigns and audits, resolve goes through the same `transition` as a status change); `new_wait_limit_resolved` and `new_wait_limit_escalated` in `SlaSweep`.
      - **Tests:** 1 request test (a ticket past its limit is resolved with its clock running, another handed to the worker and still pending, a third only flagged; refusals for escalating without a worker, a worker without escalating, an unknown action, a bad worker and an action with no limit; a second sweep does nothing). The CRM-R64 test passes unchanged.
      - **Types and docs:** `schema.d.ts` regenerated, the operation text, the changelogs. No UI change: the client never creates a policy.
      - **Not tested:** the emitted event's payload; a policy changed after tickets were flagged (a policy cannot be edited).
- [x] CRM-T137 *(edge)* **CLV at the rate of the day each deal closed.** `rate_basis=close_day` on the CLV read; the last open part of the roadmap item "CLV refinement". (M)
      **Done (2026-10-28, not released):** requirement CRM-R70 first, then:
      - **Service:** `rules::fx::convert_dated` (each deal at its own day's rate, once per deal, distinct rates and what had none reported); `fx_rates::convert_dated`; `rate_basis` on `ClvQuery` and in `Clv`, validated; operation text; `schema.d.ts` regenerated.
      - **UI:** a checkbox in `ClvPanel` (disabled without a currency or with a margin or look-ahead), `rateBasis` in `accountClv`, the tour converting deal by deal, 1 key in 8 locales (machine drafts); one Playwright test (£88,000 at today's 1.30 is 114,400.00, at the 1.27 of its closing day 111,760.00).
      - **Tests:** 3 unit tests (two rates either side of a change, a deal before any rate left out and counted, a reverse rate inverted with each deal rounded once) and 1 request test (24,000 USD at 0.80 and 0.50 by close day against today's 0.50, a deal before any rate and yen left out and named, two distinct rates, the per-currency figures unchanged, five refusals). The other CLV tests pass unchanged.
      - **Not tested:** a masked caller on this path (the conversion is skipped when masked, as for the CLV); a won deal with no closing day (it falls back to its last update).
- [x] CRM-T138 *(edge)* **Claw credit back from a worker.** An amount taken back from one worker's credit for a won deal; the last open part of the roadmap item "territories; quota refinements". (M)
      **Done (2026-10-28, not released):** requirement CRM-R71 first, then:
      - **Service:** migration `000048` (`deal_clawbacks`, one row per worker and deal); `GET`/`PUT`/`DELETE /api/quotas/clawbacks/{pid}` inside the admin-only `/api/quotas` prefix; `credit_of` (the credit before any clawback, whole or a split share); `credited_deals` takes the clawback off and drops a deal clawed back whole; operations and schemas described.
      - **Tests:** 1 request test (a split deal's odd minor unit, a clawback of 10,000 changing only that worker's attainment, five refusals and a `404`, replacing rather than adding, a share clawed back whole not counting as a deal won, removal restoring it and a second removal `404`); the admin-only test extended.
      - **UI:** a clawback form on `/quotas` (deal, worker, amount in major units, reason; remove), `setClawback`/`deleteClawback`, the tour's clawbacks and attainment (`dealCredit` shared with the quota rows), 5 keys in 8 locales (machine drafts); one Playwright test (£8,000 off a worker's £88,000 deal, refused above what they hold, given back); `schema.d.ts` regenerated; the changelogs.
      - **Not tested:** a clawback on a deal whose split changes afterwards (the clawback is kept and clamps at zero); commission after a clawback beyond what the request test reads.
- [x] CRM-T139 *(edge)* **Bridges to PPM and WPM.** A won deal opens a delivery idea in PPM, and WPM's departments become CRM teams; the roadmap item "cross-app bridges". (L)
      **Done (2026-10-28, not released):** requirements CRM-R72 and CRM-R73 first, then:
      - **Service:** migration `000049` (`deal_deliveries`, one row per deal); `clients::open_delivery_idea` and `clients::active_workers` (reqwest, no redirects, a timeout, nothing but the ids and departments read); `CRM_PPM_SERVICE_URL` and `CRM_WORKER_SERVICE_URL` held in `Settings`; `controllers::bridges` with `POST /api/bridges/ppm/deals/{pid}` and `POST /api/bridges/wpm/teams-import`; `/api/bridges` added to the admin-only prefixes; the team upsert in `teams.rs` became `put_team`, shared with the import; operations and schemas described.
      - **UI:** a button on a won deal's page and one on `/quotas`, `openDelivery` and `importTeams`, the tour answering as the stub does, 7 keys in 8 locales (machine drafts); one Playwright test.
      - **Tests:** 2 request tests against in-process stand-ins for PPM and WPM (the stub sends and keeps nothing; an open deal `422` and an unknown one `404`; an unreachable PPM `502` and nothing kept; one idea sent with only the name, amount and tags, and asking again sending nothing; teams per department over two pages, only active workers asked for, 200 in one department and a case-twin and a bad id skipped with reasons, a team nobody named left alone, a rerun replacing and not adding, an unreachable service `502`); the admin-only test extended to the import.
      - **Not tested:** against a running PPM or WPM (no instance was available; both contracts were read from source); the idea's pid being kept when PPM answers `id` in place of `pid`; a worker list over 100 pages.
- [x] CRM-T140 *(edge)* **Retention for the delivery and event tables.** The tables the inventory marked `gap`: `email_deliveries`, `email_events`, `event_outbox` and `webhook_deliveries` had no deletion path. (M)
      **Done (2026-10-28, not released):** requirement CRM-R81 first, then:
      - **Service:** `rules::privacy` (`event_retention_days`, `mail_retention_days`, `log_rules`: which rows are finished with, per table); `CRM_EVENT_RETENTION_DAYS` and `CRM_MAIL_RETENTION_DAYS` in `Settings`; the retention report and sweep cover them, with the horizons, counts and an audit entry.
      - **Docs:** the four tables are `aged` in the inventory; the DPIA's over-retention row and the record of processing no longer name a gap; the schedule says exactly what is deleted and what is never.
      - **Tests:** 3 unit tests (defaults and floors; the mail floor outlives attribution's 365 days; each rule names its table and never matches pending work) and 1 request test (seven deliveries, two events, four outbox rows and four webhook deliveries of different ages and states: only those finished with and old go, a running campaign's, a retryable, a pending, an unpublished and an unfanned one stay, a second sweep does nothing, and the floors hold); the inventory test now holds `aged` to the sweep's own list.
      - **Not done:** a schedule for the sweep; per-table horizons.
- [x] CRM-T141 *(edge)* **Personal-data inventory, retention schedule, DPIA and record of processing.** Closes the "retention schedules" and DPIA production gates that were only a template. (M)
      **Done (2026-10-28, not released):** requirement CRM-R74 first, then:
      - **Docs:** `spec/compliance/personal-data-inventory.md` (59 tables, five retention classes, what erasure does, what is outside the database), `dpia.md`, `record-of-processing.md`, the compliance index; claims about logs, files and links were checked against the code and two corrected on the way (attachment files are named at random, not by hash; confirmation and preference links expire but the unsubscribe link does not).
      - **Test:** `tests/requests/inventory.rs` (the inventory against `information_schema` and `SOFT_DELETED_TABLES`). **Checked by breaking it:** with the `tags` row removed it fails naming the table; restored.
      - **Gaps recorded, not closed:** CRM-T140 (no deletion path for four tables). Small-group suppression (CRM-T145) and single-use link tokens (CRM-T146), named in the DPIA, were done after.
      - **Not done:** the deployer's decisions; any DPO review.
- [x] CRM-T142 *(edge)* **A translation-review register and a test that holds it to the locales.** (S)
      **Done (2026-10-28, not released):** requirement CRM-R75 first, then:
      - **Docs:** `spec/translation-register.md` (8 locales, 684 strings each, every one but English a draft, none reviewed), `spec/locale-review.md` regenerated (it still said 330 strings), the UI and locale indexes.
      - **Tests:** 5 Vitest tests in `tests/unit/translation-register.test.ts` (a row per locale file, status meanings, counts, the same keys and no empty string, the same `{placeholders}`); the existing key-parity test in `crm.test.ts` stays. No drift was found in the 684 strings. **Checked by breaking it:** changing `{count}` to `{cnt}` in one German string fails naming the locale and key; restored.
      - **Not done:** any native review.
- [x] CRM-T143 *(edge)* **No page shows the service's error text.** (M)
      **Done (2026-10-28, not released):** requirement CRM-R76 first, then:
      - **UI:** `errorMessage` rewritten to choose a fixed message by status and never read the error's text; `PageError` for a page's own localized message (the board's "lane locked"); `fieldErrors` keeps the member and drops the `detail`; the deal page's not-found is an `ApiError(404)`; 9 `error.*` keys in 8 locales (machine drafts); the register's counts moved from 684 to 693.
      - **Rule:** an ESLint block for `src/routes` and `src/lib/components` (`no-restricted-properties` on `message`, `no-restricted-syntax` on `String(error)`). **Checked by breaking it:** a probe page with both patterns fails lint with both messages; removed.
      - **Tests:** `tests/unit/errors.test.ts` (11 statuses never show a service's words, a problem's `title` and `detail` kept out, the page's own fallback and a `PageError` as written, a failed fetch unreachable and anything else generic, every message in every locale, `fieldErrors`, a scan of the pages for the lint's patterns); the resource, `fieldErrors` and five Playwright expectations that had read the service's wording now read the page's.
      - **Not tested:** the message in the browser in each non-English locale (the unit test checks the table, not the rendering).
- [x] CRM-T144 *(edge)* **Every page is scanned by axe.** The accessibility gate covered 12 of the pages; now it covers all of them, and a test keeps it so. (M)
      **Done (2026-10-28, not released):** requirement CRM-R77 first, then:
      - **Gate:** `tests/e2e/a11y-routes.ts` (40 pages, each with a real address and how to reach it), `a11y.spec.ts` rewritten to scan each (signed-in pages in the tour, with data), `tests/unit/a11y-routes.test.ts` (3 tests: the table is the file tree, addresses fit their routes, an exclusion says why). No page is excluded.
      - **Fixes (axe found five on pages it had never reached):** `.error` colour mixed with the page ink (contrast 2.87 to a pass); links in running text underlined and list or cell links given a 24px target; the deals page's pipeline drop-down named; the calendar's day names lead with the visible day number and events, then the full date.
      - **Tests:** 60 Playwright tests over deals, accounts, contacts, help, follow-ups and the board pass again after the style changes; Vitest 193 pass.
      - **Not done:** the manual audit (CRM-T48); other locales and themes; states after an action.
      - **Not tested:** that every locale's calendar label satisfies the rule (the German route is scanned; the others use the same code).
- [x] CRM-T145 *(edge)* **Small-cell suppression for the report builder.** (M)
      **Done (2026-10-28, not released):** requirement CRM-R78 first, then:
      - **Service:** the report SQL returns `n`, the records behind each cell; `rules::reports::suppress` (a pure function: by cell, or by group when converting); `Settings.min_cell_size` from `CRM_MIN_CELL_SIZE` (5, 0 off, at most 1000); `min_cell_size` and `suppressed` in the answer and in the audit entry.
      - **Tests:** 4 unit tests (below the minimum left out and counted; exactly the minimum shown and zero or less off; a sum judged by records, not size; a converted group judged across its currencies) and 1 request test (7 won and 2 open deals: counts, per-currency sums, a filter that narrows to one group, and off). The four existing report tests that check arithmetic on a few records run with suppression off.
      - **UI:** the Reports page says how many groups were left out and why (`rpt.suppressed`, 8 locales, machine drafts); the tour applies the same rule with its own minimum of 2, since its sample data is small and made up; one Playwright test (the one lost deal is left out); `scripts/update-translation-register.mjs` refreshes the register's counts (now 694 strings); `schema.d.ts` regenerated; the DPIA, the record of processing and the env-var list no longer say suppression is missing.
      - **Not done:** secondary suppression; the fixed dashboards and the attribution, funnel and attainment views.
- [x] CRM-T146 *(edge)* **Single-use hashed link tokens.** The confirmation and preference links. (M)
      **Done (2026-10-28, not released):** requirement CRM-R79 first, then:
      - **Service:** migration `000050` (`link_tokens`); `link_tokens` module (`generate`, `hash`, `issue`, `peek`, `consume`, `sweep`); the confirmation request, the web form's confirmation and the preference link issue random tokens; the public pages peek on `GET` and spend on `POST` inside the act's transaction, and a preference choice issues the next token; the signed confirm and preference tokens were removed from `unsubscribe.rs`; erasure deletes a contact's tokens; the hourly job sweeps old rows; the inventory has a `link_tokens` row (`ttl`).
      - **Tests:** 3 unit tests (a token is long, random and URL-safe; what is kept is a SHA-256 that does not contain the token, pinned to a known value; only the right shape reaches the database) and 3 request tests (the confirmation link spent once with a repeat `404` and the database holding the hash and not the token; the preference page spending the link per choice with the next one in the answer, a refused choice spending nothing; two simultaneous uses with exactly one winning, an expired link, the weekly sweep and erasure). **Found by the test:** `make_interval` needs an `int`, so the weekly sweep failed until cast; fixed.
      - **Not done:** the DPIA's other gap (CRM-T140); single-use for the unsubscribe link, by design.
      - **Not tested:** a very large number of simultaneous requests; the tokens against a real mail scanner.
- [x] CRM-T147 *(edge)* **ESLint and Prettier as tests.** (S)
      **Done (2026-10-28, not released):** requirement CRM-R80 first, then:
      - **Tests:** `tests/unit/lint.test.ts` (6 tests: the no-error-text rule refused in a page for `.message` and for `String(error)`, in a script and a component, and leaving `src/lib/errors.ts` and ordinary code alone; the whole project through ESLint; every formattable file through Prettier, 85 of them). **Checked by breaking both:** an unformatted test file and a `src` file with a lint fault each fail the test naming the file; removed.
      - **Scripts:** `pnpm lint` and `pnpm format` now cover `src`, `tests`, `scripts` and the top-level configuration files; `vite.config.ts` was the one file that was not formatted, and is now.
      - **Found on the way:** Vite hands a test the browser build of Prettier, which has no file functions, so the test loads Prettier with Node's own resolver.
      - **Not done:** `.svelte` formatting; Markdown formatting.
- [x] CRM-T148 *(edge)* **Close a ticket left resolved.** (S)
      **Done (2026-10-28, not released):** requirement CRM-R82 first, then:
      - **Service:** migration `m20261028_000051_auto_close_days` (`sla_policies.auto_close_days`, checked 1 to 365); `PolicyPayload.auto_close_days` validated; `sweep_closing` in `support.rs` goes through the same `transition` as a person, with `via: auto_close`; `new_auto_closed` in the sweep's answer.
      - **Tests:** `a_policy_closes_a_ticket_left_resolved_for_its_days_and_no_other` (eight days closed, six not, no days not, reopened-and-resolved-again not, bad values refused, audit says `auto_close`, a second sweep closes nothing); the OpenAPI schema tests still pass.
      - **UI:** only the generated `schema.d.ts`; the UI has no policy form (the wait-limit fields have none either).
      - **Not done:** see CRM-R82's list.
- [x] CRM-T149 *(edge)* **Close-day rates in the report builder.** (M)
      **Done (2026-10-28, not released):** requirement CRM-R83 first, then:
      - **Service:** `rate_basis` on the report spec; `check` refuses it without `convert_to`, with another value, with `rate_date`, or without a `won`/`lost` state filter; the plan groups by day when it is set (cap `MAX_DAY_CELLS`, no group limit that would cut a group's days short); `Cell.day`; `rules::fx::convert_rows_dated` and `fx_rates::convert_rows_dated` read the rates once.
      - **Tests:** unit tests for the pure conversion (own-day rates, group merge, a group with nothing converted has no row, never one-to-one) and for the spec's checks and SQL; `a_report_can_convert_each_deal_at_the_rate_of_the_day_it_closed` against the same deals as the CLV test (14,000 against 13,000 at one date; the deal before any rate and the yen named; five refusals with their pointers); a tour end-to-end test (the choice is disabled until the filter is won or lost; no date is shown beside the rates).
      - **UI:** a checkbox on `/reports` using the existing `clv.closeDay` label (no new strings), enabled only when the filter is `state` of won or lost; it disables the rate date; the tour mirrors the service.
      - **Found on the way:** (1) the OpenAPI unit test `mutating_operations_have_a_request_schema_or_are_bodyless` had been failing since CRM-T139, whose two bridge routes were never listed as bodyless; they now are. (2) The rates list on `/reports` was keyed by currency, so two rates for one currency (which close-day mode produces) crashed the page; it is keyed by currency, day and direction. The other pages that list rates convert at one date and cannot hit it.
      - **Not done:** see CRM-R83's list.
- [x] CRM-T150 *(edge)* **A starting hazard analysis in the manner of ISO 14971.** (S)
      **Done (2026-10-28, not released):** requirement CRM-R84 first, then:
      - **Document:** `compliance/risk-management.md` (the process clause by clause, misuse, thirteen hazards with their controls, the risks the controls create, what an integrator still has to do); entries in the compliance index and `regulatory.md`.
      - **Checked:** the tests it names exist (`consent_withdrawn_between_runs_is_honoured`, `enrolment_rechecks_consent_inside_the_action`, `a_merge_moves_everything_and_is_recorded`, `subject_rights_round_trip`, the wait-limit and auto-close tests); the controls it calls Implemented are ones a test pins, and the ones it cannot say that of are marked Deployer or Gap.
      - **Not done:** review against the standard's text; see CRM-R84's list.
- [x] CRM-T151 *(edge)* **Close-day rates in attribution.** (S)
      **Done (2026-10-28, not released):** requirement CRM-R85 first, then:
      - **Service:** `rate_basis` on the attribution query, validated; each share is kept with the day its deal closed (`Tally.dated`, `Shared`); `convert_credit` converts them with `fx_rates::convert_rows_dated` (from CRM-T149).
      - **Tests:** `attribution_can_convert_each_deals_share_at_the_rate_of_the_day_it_closed` (13,000 against 12,000 at one date; the deal before any rate and the yen named; the untouched deal at its own day's rate; three refusals with their pointers).
      - **UI:** a checkbox on `/attribution` with the existing `clv.closeDay` label; the tour's per-day conversion is now one function shared with the report builder; a tour end-to-end test.
      - **Found on the way:** `cargo fmt --check` had been failing on four files from CRM-T148 and CRM-T149; they are formatted.
      - **Not done:** see CRM-R85's list.
- [x] CRM-T152 *(edge)* **Close-day rates in a campaign's ROI; the bridge contracts checked.** (S)
      **Done (2026-10-28, not released):** requirement CRM-R86 first, then:
      - **Service:** `rate_basis` on the funnel query, validated; each foreign-currency won deal converted with `fx_rates::convert_dated`; `won_revenue_other_currencies` is what had no rate on its day.
      - **Tests:** `a_funnel_can_convert_each_deal_at_the_rate_of_the_day_it_closed` (113,000 against 112,000 at one date, ROI numerator 63,000, the dollars before any rate and the yen kept apart, three refusals with their pointers); the mocked live-UI funnel test now covers the checkbox, the request and the missing date.
      - **UI:** a checkbox on `/campaigns` using the existing `clv.closeDay` label, enabled with the conversion; the rates list keyed by currency, day and direction.
      - **Bridge contracts checked against the sibling source (CRM-R72, CRM-R73):** PPM `create_idea` (`strategy.rs`) and WPM `list_workers` (`hr_core.rs`) match what `clients.rs` sends and reads. Two findings: WPM refuses an offset above 10,000, so the worker import reads at most 51 pages of 200 and stops with a reason of ours; and **neither call sends credentials**, so each reaches its service only while that has authentication off (recorded in both requirements; a service-to-service credential is not built). Neither was run against a live service.
      - **Not done:** see CRM-R86's list.
- [x] CRM-T153 *(edge)* **A credential for the bridges.** Closes the gap CRM-T152 found. (S)
      **Done (2026-10-28, not released):** requirement CRM-R87 first, then:
      - **Service:** `Settings.ppm_service_token` and `worker_service_token`; `clients::with_credential` attaches a bearer token only to `https` or loopback `http`; both calls use it.
      - **Tests:** a unit test of the address rule (four addresses that get the token, four that do not, an error that holds neither token nor address, nothing attached without a token); a wire test against one stand-in (nothing attached without tokens, each service gets its own, a token is not sent to a plain-`http` remote address and the call fails with nothing sent).
      - **Not done:** see CRM-R87's list.
- [x] CRM-T154 *(edge)* **A reply by e-mail resumes the ticket.** The last open part of the roadmap item "SLA clock refinements". (M)
      **Done (2026-10-28, not released):** requirement CRM-R88 first, then:
      - **Service:** `POST /webhooks/inbound-email` (`support::public_routes`, public in `auth::is_public_path`, documented in the public OpenAPI entries); `webhook::signed_request` shared with `/webhooks/email`; `reference` on `TicketDetail`; `recognise_reply` reuses `transition` with `via: inbound_mail`.
      - **Tests:** unit tests of the reference, its recognition in a subject and the sender's address; `a_reply_by_mail_resumes_the_ticket_it_names_when_it_comes_from_the_customer` (no reference, an unknown one, a stranger's address, the customer with a display name and different case, a repeat, a resolved ticket, a ticket about nobody, the audit `via`, unsigned, badly signed and no secret).
      - **UI:** the selected ticket shows its reference (`ticketReference`, the same rule, with a unit test using the service test's pid); one new string in the eight locales (machine drafts, 695 per locale in the register).
      - **Documents:** hazard H14 in `compliance/risk-management.md`; a processing activity in `compliance/record-of-processing.md`.
      - **Not done:** see CRM-R88's list; **not run against a real mail service**.
- [x] CRM-T155 *(edge)* **Attribution settings.** A half-life for time decay and a delay under which an open does not count. (M)
      **Done (2026-10-28, not released):** requirement CRM-R89 first, then:
      - **Service:** `rules::attribution::{half_life_days, credit_with}` (`credit` is `credit_with` at the default); `half_life_days` and `min_open_seconds` on the attribution query, validated and echoed; the open condition is in the touch query (`ev.received_at >= e.sent_at + interval`).
      - **Tests:** unit tests (a chosen half-life, the default unchanged, the other models unmoved, the longest window at the shortest half-life still weighs every touch, the range); `an_open_too_soon_after_sending_is_left_out_and_the_decay_half_life_can_be_chosen` (a thirty-second open and a two-hour open; the setting with sends beside opens; 7 against 90 days; six refusals with their pointers).
      - **UI:** two inputs on `/attribution`, each disabled until it applies; the tour has one open twenty seconds after sending; two strings in the eight locales (machine drafts, 697 per locale); tour tests for both.
      - **Not done:** see CRM-R89's list.
- [x] CRM-T156 *(edge)* **Position-model weights.** The share of each of the first and last touches can be chosen. (S)
      **Done (2026-10-29, not released):** requirement CRM-R90 first, then:
      - **Service:** `rules::attribution::{end_percent, credit_tuned}` (`credit_with` is `credit_tuned` at 40); `end_percent` on the attribution query, validated, echoed and audited.
      - **Tests:** a unit test (30, 40 and 49 percent, the default unchanged, other models unmoved, exact at every value, the range); the position request test now chooses 25 and checks three refusals with their pointers.
      - **UI:** one input on `/attribution`, disabled unless the model is position; one string in the eight locales (machine drafts, 698 per locale); the tour validates and applies it.
      - **Not done:** see CRM-R90's list.
- [x] CRM-T157 *(edge)* **A delay for clicks.** The last open part of the roadmap item "attribution settings" besides weights. (S)
      **Done (2026-10-29, not released):** requirement CRM-R91 first, then:
      - **Service:** `min_click_seconds` on the attribution query, validated, echoed and audited; the click condition is in the touch query beside the open one.
      - **Tests:** `a_click_too_soon_after_sending_is_left_out_and_opens_are_not_touched` (a ten-second and an hour-old click, sends and opens untouched, four refusals with their pointers); passes against PostgreSQL with the other attribution tests.
      - **UI:** one input on `/attribution`, disabled unless clicks are counted; the tour has a click eight seconds after sending; one string in the eight locales (machine drafts, 699 per locale); a tour test.
      - **Not done:** see CRM-R91's list.
- [x] CRM-T158 *(edge)* **The worker service's hire and leave events pause marketing.** The WPM → CRM half of the family bridges (WPM-R207, decided 2026-10-30). (M)
      **Done (2026-10-30, not released):** requirement CRM-R92 and decision CRM-D15 first, then:
      - **Service:** `POST /api/bridges/wpm/events`, outside the session check and signed (`CRM_WPM_WEBHOOK_SECRET`, `404` while unset; `401`, `400`, `413` at 16 KiB, `422`); the pure `rules::wpm_events` (signature in constant time with a five-minute window, parse, decide); `models::employments`; migration 000052 (`wpm_events`, the received ids; `contact_employments`, the pause). The segment facts gain `employee` and the evaluator leaves an employee out as it leaves out a contact without consent; a campaign skips them with a reason; the nurture sweep holds their enrolment. `employments` on the contact detail and in the subject-access export; erasure deletes them; `wpm_events` pruned with the event logs (CRM-R81). Each change audited with `via: wpm`.
      - **Tests:** 8 unit tests of the signature, parse and decision (WPM's algorithm, a stale or future time, a changed body, a repeated `t` or `v1`, the three kinds, no `data`, unknown kinds, wrong references or dates), one on the segment gate, one that only this route is public under `/api/bridges`, the settings floor; 4 request tests (`tests/requests/wpm_events.rs`: no secret is `404`; unsigned, wrong, stale, future and altered are `401` and change nothing, then `413`, `400`, `422`; a hire pauses once, a repeat is `duplicate`, a second hire is `already_paused`, a stranger leaves nothing, a campaign skips and nurture holds, a leave resumes under the old consent and never grants it; erasure deletes the employment). Service: 394 unit, 207 request and the other DB tests pass against PostgreSQL; clippy clean for both feature sets; fmt clean.
      - **UI:** "Employee — marketing paused" with the organization and date on a contact's page; one string in the eight locales (machine drafts, 700 per locale); the demo's contact detail carries `employments: []`; two stubbed Playwright tests; `schema.d.ts` regenerated.
      - **Spec:** a "Family bridges" table and the inbound contract in [integrations.md](integrations.md); the DPIA, inventory and record of processing cover the new data.
      - **Not done / not verified:** no event from a **running WPM** was received; the tests sign requests with the same algorithm, read from WPM's source while WPM was adding `data`. See CRM-R92's "Not built" (a future `effective_on` applied at once, no ordering (since CRM-T159), contacts created after a hire not marked, the overlap-check client (built since: CRM-T160), secret rotation). The `worker:` versus `person:` mismatch in the teams import is unresolved *(resolved since by CRM-T161 and CRM-T162)*. The `/followups` accessibility test fails (a colour-contrast node) at the commit before this one too, and is not touched here.
- [x] CRM-T159 *(edge)* **The worker service's events are put in order.** A hire retried after the leave that followed it no longer pauses the contact again. (S)
      **Done (2026-10-30, not released):** requirement CRM-R93 and an addition to CRM-D15 first, then:
      - **Service:** `rules::wpm_events::{Stamp, is_stale}` and `Handoff::occurred_at` (a hire or leave without an RFC 3339 `occurred_at` is `422`); migration 000053 (`employment_last_events`: per person and organization, the last kind, stamp and event id); `models::employments::{lock_order, last_applied, note_applied, last_events_of, forget_last_events}`; the route answers `stale` and changes nothing for an older event, under a transaction-scoped advisory lock. The subject-access export carries `employment_last_events`; erasure deletes them (audited as `employment_last_events_deleted`); the sweep prunes a leave's row after the event horizon and a hire's once no live contact carries the person.
      - **Tests:** 4 unit tests (`occurred_at` required for a hire or leave and not for the others; older is stale, a rehire is not; within one second the later effective day wins and an equal stamp is applied; a handoff's stamp); the log-rule test names the new table; `a_hire_older_than_the_leave_after_it_changes_nothing` (hire, leave, the retry `duplicate`, the same hire under a new id `stale` and recorded so, a same-second tie both ways, an old leave after a rehire `stale`, a leave that arrives before the older hire it follows, no order kept for a stranger, `422` without `occurred_at`); the erasure test checks the rows and the audit count. Service: 398 unit and 208 request tests pass, the request tests against a throwaway PostgreSQL database; clippy clean for both feature sets; fmt clean; `schema.d.ts` regenerated.
      - **Spec:** CRM-R93; CRM-D15 says why ordering is by person and by `occurred_at`; integrations.md; the inventory, retention schedule, DPIA and record of processing name the new table.
      - **Not done / not verified:** see CRM-R93's "Not built"; **not run against a running WPM** (its `occurred_at` format, whole seconds in UTC, was read from `rules/webhook.rs`). The UI does not show the order; it is in the export only.
- [x] CRM-T160 *(edge)* **The worker service is asked which contacts are its employees.** The CRM client for WPM's overlap check (WPM-R208), so a contact who became one after their hire is marked too. (M)
      **Done (2026-10-30, not released):** requirement CRM-R94 and decision CRM-D16 (one HR token and one configured organization, not the caller's own token) first, then:
      - **Service:** `POST /api/bridges/wpm/employment-check` (admin only, stub-first, bodyless); the pure `rules::employment_check` (batches of at most 500 distinct `person:` references without the erasure tombstone, a strict reading of WPM's answer, and the decision: a no does nothing, a yes records unless the last event was a leave); `clients::overlap_check` (`CRM_WORKER_SERVICE_TOKEN`, CRM-R87's rules); `CRM_WPM_ORGANIZATION_REF` (an `organization:` URN or unset); `models::employments::{last_kind, live_person_refs}`. A yes writes the row a hire event writes (`since` the day of the check, `overlap:<check id>`), audited `employment_recorded` with `via: wpm`; the check is audited once by count.
      - **Tests:** 5 unit tests of the batching (once each, sorted, 500 and 501 and 1,234; only `person:` URNs and never the tombstone), the answer (the workers among those asked; another organization, missing fields, someone not asked about or answered twice refused, and the reason names no one) and the decision; the settings test of the organization; `the_check_marks_the_contacts_wpm_says_are_workers_and_a_no_changes_nothing` against a stand-in WPM that answers only an HR bearer token (524 contacts in two calls of 500 and 24 with references only and no names, the tombstone left out; `502` without an organization, without the token and for an answer for another organization, each recording nothing; a yes marks two contacts, one already marked by a hire event, one skipped for its leave; segments leave the new ones out; the audits; a no then ends nothing; asked again, nothing new; a leave event ends what the check recorded). Service: 404 unit and 209 request tests pass, the request tests against a throwaway PostgreSQL database; clippy clean for both feature sets; fmt clean.
      - **UI:** a button "Check with the worker service which contacts are employees" beside the teams import on `/quotas`, saying how many contacts were marked or, in the tour, how many would be asked about; the demo answers as the stub does; three strings in the eight locales (machine drafts, 703 per locale); a tour test; `schema.d.ts` regenerated. svelte-check clean, 201 vitest, lint clean, 177 of 178 Playwright tests (the `/followups` contrast failure is pre-existing).
      - **Spec:** CRM-R94, CRM-D16; the Family bridges table says the overlap client is built; INSTALL.md and the service spec name `CRM_WPM_ORGANIZATION_REF`; the DPIA (what is now sent to WPM, a risk row), the record of processing and the inventory follow.
      - **Not done / not verified:** **not run against a running WPM**; the token is one deployer-held HR token, not the caller's own as WPM-D135 asks (CRM-D16); see CRM-R94's "Not built" (no schedule, one organization, a no never ends a pause). A yes for a person whose leave's row was pruned (CRM-R93, 30 days) is recorded even if that leave takes effect later.
- [x] CRM-T161 *(edge)* **Imported teams name their members by `person:` reference.** The teams import kept `worker:<pid>` while the family names a person by `person:` URN, so a team member and a contact could not be matched. (S)
      **Done (2026-10-30, not released):** CRM-R73 rewritten and decision CRM-D17 (members by `person_ref`; earlier imports replaced by the next, not migrated) first, then:
      - **Service:** the pure `rules::teams_import::{group, is_person_ref}`; `clients::UpstreamWorker` gains `person_ref` (still only `pid`, `department` and `person_ref` are read from WPM's whole worker record); the import stores members by `person_ref` and answers `without_person_ref`; a worker without a valid `person:` URN joins no team and never becomes `worker:`. WPM's `GET /api/workers` was read again (`hr_core.rs`, `list_workers`, which serialises the whole `workers` model, `person_ref` included, masked only by the `mask` obligation).
      - **Tests:** 5 unit tests of the mapping (members are `person:` references sorted and once each per department; a missing, empty, malformed, `worker:` or `organization:` reference is counted and never a member; a person with no department counted apart; trimming; nothing in, nothing out); `the_worker_service_gives_a_team_per_department_and_leaves_the_rest_alone` now stubs WPM's list with whole records (pay, name, job, `person_ref`) and checks the members are exactly the `person_ref`s, three workers without a valid one counted, no pay, name or job in the answer, a `Support` team stored earlier with a `worker:` member replaced and a hand-set `worker:` team kept. Service: 409 unit and 209 request tests pass, the request tests against a throwaway PostgreSQL database (one run had `errors_are_problem_documents` fail once; it passed alone and in a full rerun); clippy clean for both feature sets; fmt clean.
      - **UI:** `TeamImport.without_person_ref`; the tour's stand-in answers it (it builds no `worker:` members: the tour's import is the stub); `schema.d.ts` regenerated. svelte-check clean, 201 vitest, lint clean, 177 of 178 Playwright tests (the `/followups` contrast failure is pre-existing; one earlier run also failed the informational `/partners` tour check, which passed alone and in the rerun).
      - **Spec:** CRM-R73, CRM-D17; integrations.md marks the mismatch resolved and records WPM's whole-record answer (pay included unless masked, no list audit) as a gap for WPM's owner; the DPIA (sources, minimisation), the inventory's `teams` row and the record of processing follow.
      - **Not done / not verified:** **not run against a running WPM**; WPM's own spec still records the mismatch as open (not edited here). Quotas are owned by `worker:` references, so an imported team's roll-up (CRM-R67) matches no quota; no person-to-worker mapping; `PUT /api/quotas/teams` still takes `worker:` members only, so an imported team cannot be re-saved by hand *(these three resolved by CRM-T162: users are `person:` references, CRM-D18)*; erasing a contact leaves its `person_ref` in an imported team; the `/quotas` page does not show `without_person_ref`.
- [x] CRM-T162 *(edge)* **A CRM user is a `person:` reference.** The CRM wrote its signed-in user as `worker:<sub>` and took `worker:` for every owner, assignee and team member, while the family's sign-in subject is a person id (WPM matches it as `person:<sub>`), so quotas could not meet the `person:` members the teams import stores (CRM-T161). Traces to CRM-R59, CRM-R67, CRM-R73, CRM-D18. (M)
      **Done (2026-10-30, not released):** checked first, in all three apps, that the sign-in subject is a person id (CRM-D18 lists the evidence: WPM `controllers/me.rs`, `models/memberships.rs`, `auth.rs` and its `spec/auth.md`; PPM `discussion.rs`, `subjects.rs`; the CRM's own `$sub` ownership check; the shared design says only "user pid"), then CRM-D18 and CRM-R59, R67, R73 rewritten, then:
      - **Service:** the pure `rules::user_ref::{of_subject, normalise, normalise_opt, normalise_all}`; `owner_of` and the inbox give `person:<sub>`, the anonymous owner is the all-zero person; `Problems::{require_user, user_opt}` check a `person:` URN; every body and `owner_ref` filter that names a user rewrites `worker:<id>` to `person:<id>` first (contacts, accounts, activities, deals, tickets, SLA policies, the board, quotas, splits, clawbacks, territories, teams, notifications, workflow actions via `Action::normalise_user_refs`); the seed makes `person:` owners. Migration 54 rewrites stored `worker:<uuid>` user references in 23 columns, workflow actions and hand-set teams' members, leaves teams last set by the import, history and WPM subjects alone, skips a rewrite that would collide on a unique key, keeps `updated_at`, and rolls back.
      - **Tests:** 4 unit tests of `user_ref`, one of `require_user`, one of the workflow-action rewrite, 5 of the migration's SQL; `an_imported_teams_roll_up_meets_the_quotas_set_for_the_same_people` (an imported team's roll-up counts the quotas and won deals of the same people, set once as `person:` and once in the legacy `worker:` form; the legacy `owner_ref` filter finds the quota; `set_by` is a person; the imported team can be saved again by hand; one person in both forms is named twice); the teams-import test stores a hand-set legacy member as `person:`; the request tests' user helper (`a_user`) is a `person:` URN. `scripts/migrate-person-refs.sh`: 500 deals, contacts and tasks with notifications, quotas, a split, a clawback, a view, a workflow, a hand-set and an imported team, audit history; migrates up with the data in place, 20 checks (every user reference `person:` with the same id, nothing lost, `updated_at` unchanged, a task summary and other workflow values untouched, the imported team's WPM pid kept, the audit log unchanged, the triggers back on, a colliding row skipped, the imported team's members meeting their quotas), down and up again.Service: 415 unit tests (409 before), 5 migration unit tests, 210 request tests against a throwaway PostgreSQL database (209 before), all passing; `ci-check.sh` fmt, clippy (both feature sets, the migration crate, the matcher) and test clean.
      - **UI:** the tour's stand-in reads `worker:` as `person:` on teams, quotas, splits, clawbacks and territories (`userRef`, with a unit test), and its own stand-in user is `person:demo`; the split hint's example is `person:ada:60` in the eight locales; `schema.d.ts` regenerated. svelte-check clean (0 errors, 0 warnings), 203 vitest (201 before), lint clean, 177 of 178 Playwright tests (the `/followups` contrast failure is pre-existing).
      - **Spec:** CRM-D18 (with CRM-D1 and CRM-D17's cost note); CRM-R59, R65, R67, R68, R69, R71, R73 and the older requirement lines that named `worker:`; the Family bridges notes, roadmap, domain model, auth, service support, the DPIA (staff are named like anyone else) and the inventory; the service AGENTS.md.
      - **Not done / not verified:** **not run against a running WPM or with real sign-ins**: that a WPM `person_ref` equals the CRM caller's `sub` rests on WPM's code and spec and the shared design's silence, not on a live end-to-end run after this change. The `worker:` alias has no end date and sends no `Deprecation` header. A team last set by an import before CRM-T161 keeps `worker:<WPM pid>` members until the next import (they match nothing, as before). History (the audit log, the outbox, workflow run logs) still says `worker:`. UI and locale prose still calls sellers "workers". WPM's own spec still records the mismatch as open (not edited here). Erasing a contact still leaves its `person_ref` where it names a member of staff. Migration 54 has not been timed on a large table (operations runbook).
- [x] CRM-T163 *(edge)* **The teams import asks WPM for three fields only.** WPM's list takes `fields` since WPM-R216 (WPM commit `3991007`), so the import no longer receives whole worker records with name, job and pay. Traces to CRM-R73, CRM-D17. (S)
      - **Service:** `clients::active_workers` asks `GET /api/workers?status=active&fields=pid,department,person_ref&limit=200&offset=` (paging and `status=active` unchanged); a `422` from a WPM without `fields` fails the import with `502` naming the status, and it never retries without `fields`.
      - **Tests:** the request tests' stand-in WPM projects `fields` as WPM's source does (`rules::worker_fields`, read 2026-10-30) and refuses an unknown name with `422`; the import test checks every page asked for exactly `pid,department,person_ref`; `a_worker_service_without_the_lean_list_fails_the_import_and_changes_nothing` (a stand-in from before WPM-R216: one call, `502`, the hand-set team kept).
      - **Spec:** CRM-R73, CRM-D17, integrations.md (the WPM gap marked resolved), the DPIA and the record of processing (no name, job or pay received from a WPM that supports `fields`); the bridges tutorial.
      - **Not done / not verified:** not run against a running WPM; an older WPM without `fields` now fails the import (accepted, and said in CRM-R73).

## Worldwide equity (from the 2026-10-30 audit, [worldwide.md](worldwide.md))

Each task traces to CRM-R95 and CRM-D19 (both proposed) and names the rows of the audit it closes ("spec" rows are the spec audit,
"code" rows the code and UI check).

- [x] CRM-T164 **Worldwide equity audit.** (CRM-R95, CRM-D19) (M)
      **Done (2026-10-30):** 64 findings (16 right, 23 partial, 25 gaps) in [worldwide.md](worldwide.md), with file and line; the gaps
      are CRM-T165 to CRM-T176. Wording fixed in [regulatory.md](regulatory.md), [index.md](index.md), the email-provider brief and the
      DPIA and its template (regimes named; other regions *not yet read*); no behaviour changed.
      **Not done / not verified:** no law of any region was read; nothing was run; the identity registries, the role briefs, the
      `@lilydesignsystem/*` packages and the wording of each translation were not audited (worldwide.md, "Not audited").
- [ ] CRM-T165 **Regulatory register and compliance mapping by region.** (CRM-R95, CRM-D19; spec rows 2 to 10, 19; code rows 6, 7)
      [regulatory.md](regulatory.md) gets a register by region (international standards, the EU, the UK, the US, Canada, Latin America,
      Africa, the Middle East and the Gulf, South Asia, East Asia, South-East Asia and Oceania), each *not yet read* until read; the
      control mapping gains a row or column per regime read; the DPIA and record of processing say what a non-GDPR deployer uses
      instead; the `docs/<country-or-body>/` convention is stated beside a generic assurance mapping; CRM-G2's scope is put to the
      product owner (each jurisdiction sent to, not GDPR/PECR only); the retention defaults and the "GDPR subject-access export" text
      name whose value or right they are. (M)
- [ ] CRM-T166 **Consent and e-marketing rules as jurisdiction profiles.** (CRM-R95, CRM-D19; spec row 12; code rows 1, 4) A design
      first: a consent regime per contact or per send (opt-in only as today, soft opt-in for existing customers, implied consent with an
      expiry, opt-out), purposes an administrator can edit, consent per channel; and a sender identification and postal address the
      deployer sets, carried in each commercial message. Tests with two differently shaped profiles. (L)
- [x] CRM-T167 **Money uses the currency's own exponent.** (CRM-R95, CRM-D19; spec row 11; code rows 8 to 11) In the UI, one
      formatter and parser with the behaviour of the sibling apps' shared `src/lib/money.ts` (project portfolio commit `956d9ed`,
      workforce planning WPM-T373: the exponent from `Intl.NumberFormat#resolvedOptions().maximumFractionDigits`, `money`,
      `minorToInput`, `parseAmount`, `toMinor`, a decimal comma accepted); the two hand-made formatters and every `* 100` and `/ 100` on
      money replaced. In the service, `rules::fx::convert` scales by the two currencies' exponents (an ISO 4217 table). CRM-R50's
      "a minor unit is taken as a hundredth" is rewritten. Tests with JPY (0), KWD (3) and CLF (4), and a scan that fails on a fixed
      `* 100` or `/ 100` on a money name. (M)
      **Done (2026-10-30, not released):**
      - **Service:** `rules::currency` (pure): the ISO 4217 minor-unit table (the same shape and entries as the project portfolio service's `src/currency.rs`), `exponent`, `stored_exponent` (2 for a code stored before CRM-T168 that the table lacks) and `check`. `rules::fx::convert` scales by the source and target exponents in the same integer division as the rate, so it rounds half to even once, to a minor unit of the target. CRM-R50's "a minor unit is taken as a hundredth" rewritten, and its "not built" list no longer names other exponents.
      - **UI:** `src/lib/money.ts`, ported from the project portfolio and workforce planning UIs' `money.ts` (header: the three apps share the behaviour and change together) with `money`, `signedMoney`, `minorToInput`, `parseAmount`, `toMinor`, `currencyCode`, and the CRM's own `minorToMajor`, `majorToMinor` and `formMinor`. It takes the exponent from an ISO 4217 table equal to the service's (a unit test compares them), falling back to `Intl`, not from `Intl` first as the siblings do (CLDR differs for 16 codes; see [worldwide.md](worldwide.md)). `money()` left `crm.ts`; the attribution and reports formatters, the deal board (now a text editor in the page's number format), the deal list, the deal page, the quota and clawback forms use it; the tour's stand-in converts by exponent too. Two strings (`money.amountInvalid`, `money.currencyRequired`) in all 8 locales, machine drafts.
      - **Tests:** service: 4 unit tests of `rules::currency`, 3 of `fx::convert` (JPY, EUR, KWD, CLF both ways; half-to-even ties in the target unit; saturation into CLF), 1 request test of a converted report into yen from USD, KWD and CLF; 422 unit tests (415 before), 214 request tests against a throwaway PostgreSQL database (211 before), all passing (the Redis-store tests not run: no Redis here). UI: `tests/unit/money.test.ts` (21 tests, including the ISO table against the service's, every shipped locale's exponents, a decimal comma, and scans that fail on a fixed `* 100` or `/ 100` on a money name, a hand-made currency formatter, a form starting with a currency or a fallback to one); `tests/e2e/money.spec.ts` (a yen deal shown and edited in whole yen, a German decimal comma saved as 123456 minor units and a German refusal, a KWD deal with three places; each page axe-clean). 228 vitest, 181 Playwright, svelte-check clean, lint clean.
      - **Not done / not verified:** the exchange-rate field still takes a decimal point only (`1,27` is refused; the service parses the rate); percentages and basis points (`* 100` on a percent, the split shares, the CLV panel) are CRM-T172's; the seed and the tour stay GBP (CRM-T176); the sibling apps' `Intl`-first exponent was not changed; codes stored before this change are not checked or rewritten and there is no database constraint; not run against a live service with the UI.
- [x] CRM-T168 **No built-in currency; currency codes checked against ISO 4217.** (CRM-R95, CRM-D19; code rows 13 to 15) A campaign
      without a currency is `422`, not GBP; the quota and rate forms start empty; a deal row without a currency shows a dash; deals and
      campaigns check the code as rates do, against the ISO 4217 list; the error text's example varies. (S)
      **Done (2026-10-30, not released):**
      - **Service:** `Problems::currency`; deals (`POST /api/deals` and the deal opened by `POST /api/leads/{pid}/status` to `converted`, refused before the transaction as `/deal/currency`), campaigns, quotas, both sides of a rate and every `convert_to` take only a code in the ISO 4217 table, else `422` naming the member; the text gives JPY, EUR and KWD as examples. A campaign's `currency` is required (the `GBP` default and its OpenAPI `@default` are gone); the OpenAPI examples name EUR, JPY and INR; `schema.d.ts` regenerated. The request tests that made campaigns without a currency now name one (EUR, GBP, INR, NGN, JPY, BRL).
      - **UI:** the quota, rate and campaign forms start empty; the campaign form gained a currency field (`campaign.currency`, 8 locales); a deal with no currency shows a dash; the tour's stand-in refuses what the service refuses.
      - **Tests:** `an_unknown_currency_code_is_refused_wherever_a_currency_is_taken` (ZZZ, XAU, a word and a lower-case code on deals, campaigns, quotas, rates and `convert_to`; a lead conversion refused with the lead unchanged; nothing stored) and `a_missing_currency_is_refused_and_never_assumed`; the UI scans above.
      - **Not done / not verified:** the seed still makes GBP deals (CRM-T176); stored rows in unknown codes are left as they are; the metals and funds without a minor unit are refused, which a deployer holding them would need changed.
- [ ] CRM-T169 **Names and template text in any language.** (CRM-R95, CRM-D19; spec row 15; code rows 16 to 20) The `given_name`
      token without a stored given name uses its fallback, not the first word; token fallbacks are translated or set per template
      language; the matcher's honorifics, nicknames and legal-form words become per-language data with more than English entries;
      import column guesses in the UI locales. (M)
- [ ] CRM-T170 **Phone digits and search in every script.** (CRM-R95, CRM-D19; code rows 24, 25) The matcher reads any Unicode
      decimal digit and drops a trunk prefix only where the calling code's plan has one; search finds part of a Chinese, Japanese or
      Thai name (a segmenter, or n-grams for scripts without spaces). (M)
- [ ] CRM-T171 **Time zones, "today" and reporting periods.** (CRM-R95, CRM-D19; spec row 16; code rows 27 to 29) An organization time
      zone (and a user's) for "today" in the API and the UI and for report months; a fiscal year start and fiscal quarters as report
      groups; the forecast snapshot's day in that zone. (M)
- [x] CRM-T172 **UI dates, numbers, percentages and weekends through the locale.** (CRM-R95, CRM-D19; code rows 31 to 35) One date and
      number helper on `Intl` with `i18n.locale` (as WPM-T375's `locale-format.ts`): no `toLocaleString()` without a locale, no
      hand-made percentages, decimal commas accepted; the calendar's weekend and week start from the locale's region (or an
      organization setting), not `[0, 6]`. (M)
      **Done (2026-10-30, not released):**
      - **UI modules:** `src/lib/locale-format.ts` (ported from WPM's, WPM-T375; header: shared in behaviour, copy a fix across; the project portfolio UI has no such module): `formatNumber`, `formatInteger`, `formatDecimal`, `formatPercent`, `formatPercentValue`, `formatBasisPoints`, `formatSigned`, `formatDate` (a civil date in UTC), `formatDateTime` and `formatTime` (the reader's zone), `formatMonth`, `parseNumber` and `parseScaled` (on `money.ts`'s `parseAmount`), `numberToInput`; WPM's centi-FTE helper left out, and the CRM's own `formatDecimalText` for a decimal the API sends as text (an exchange rate), formatted from the string so no digit is lost. `src/lib/reader-calendar.ts` (WPM-T378, WPM-D149, unchanged but for its header): `todayIso`, `isoDateIn`, `weekInfo` (CLDR via `Intl.Locale#getWeekInfo`, region from the page locale then `navigator.languages`, WPM's fallback table), `calendarLocale` (the `-u-fw-` tag the Lily calendar reads).
      - **UI pages:** every `toLocaleString()` and hand-built `%` replaced (tickets, contact timeline and merges, break-glass, notifications, the CLV panel, attribution, reports, quotas, articles, the two imports, the stage labels in `format.ts` and on both boards, the grid footer total, file sizes as `Intl` units); ISO dates in markup formatted (exchange rates and the dates they were used on, report and funnel rate days, quota periods, employment and membership dates; the partner renewal and executive trend columns are typed `date`); percent and number inputs (CLV margin, retention, discount and years; deal-split shares; commission tiers; the exchange-rate field) read in the page's format and refused in the page's language when they are not a number (one string, `number.invalid`, in all 8 locales, machine drafts); "today" (quotas, follow-ups, the campaign timeline, the two imports, exchange rates) is the reader's date, and a campaign's start day is its creation in the reader's zone; the follow-up calendar takes its first day and weekend from `weekInfo`, not `[0, 6]`.
      - **Tests:** `tests/unit/locale-format.test.ts` (17, WPM's cases in the CRM's locales, and a scan of `src/` for `toFixed`, `toLocale*`, a glued `%`, a sliced timestamp and an ISO date in markup or a template string; one allowed line, with its reason); `tests/unit/reader-calendar.test.ts` (11: WPM's cases plus `de-de` and `zh-cn`; the fallback table checked against the runtime's CLDR; a scan for a UTC today or a fixed `weekendDays`); `tests/e2e/locale-format.spec.ts` (a German page shows "0,86" and "01.01.2026", sends a typed "0,87" as `0.87` and refuses "1,2,3" in German; a reader in Egypt on `ar-001` gets a Saturday-first week with a Friday–Saturday weekend; a reader in Germany Monday and Saturday–Sunday; each page axe-clean). Smoke and tour expectations now read formatted dates. 256 vitest, 184 Playwright, svelte-check and lint clean.
      - **Not done / not verified:** the service's "today" and report months are still UTC and there is no organization time zone or week setting (CRM-T171); the tour's stand-in (`src/lib/server/demo.ts`) still answers with the UTC date, as the service does; an exchange rate typed with more than 15 significant digits is rounded by the float parse before it is sent; the data-grid's own number and date columns are formatted by the Lily grid, not this module; the 57 English grid headers and the English "since" in the engagement list are CRM-T175's; numbers placed into catalogue strings are not yet formatted (CRM-T173 adds the fill); Hindi and Arabic pages show Latin digits because that is CLDR's default for `hi` and `ar-001`, which was not changed; not run against a live service, and only Chromium was run.
- [ ] CRM-T173 **Plural rules.** (CRM-R95, CRM-D19; code row 36) A `tn(key, count)` on `Intl.PluralRules(i18n.locale)` with a CLDR
      category per form (as WPM-T374); the board's `count === 1` and the 10 `{count}` strings first; each locale holds its language's
      categories. (S)
- [ ] CRM-T174 **Right-to-left layout.** (CRM-R95, CRM-D19; code rows 38, 39) Logical CSS (`text-align: start`, `border-inline-start`)
      in `app.css`; direction-neutral or mirrored arrows; an Arabic Playwright check of a table and a back link. (S)
- [ ] CRM-T175 **Locales equal.** (CRM-R95, CRM-D19; spec row 17; code rows 5, 40, 42) The confirmation, preference and unsubscribe
      pages in the contact's language; the 57 English grid headers and other literals into the catalogue; `de-de` and `zh-cn` either
      made world locales (`de-001`, `zh-Hans`) or the regional choice recorded as a limit with a reason; a review record per locale. (M)
- [ ] CRM-T176 **Seed and demo data in more than one shape.** (CRM-R95, CRM-D19; code row 43) The seed and the tour carry deals in at
      least one zero-exponent and one three-exponent currency, rates not all from GBP, and names in more than one order and script; the
      `pounds` helpers take minor units. (S)

## GitHub Pages site ([github-pages-site.md](github-pages-site.md))

- [x] CRM-T177 **A GitHub Pages documentation site, built here and not published.** (CRM-R95 for the locales and
      direction; the family pattern of the workforce planning repository's `spec/monorepo-github-pages/`) (M)
      **Done (2026-10-30, not published):** `contact-relationship-management.github.io/`, adapted from the project
      portfolio site (its GP-1): SvelteKit 3 with `adapter-static` (137 prerendered pages), Svelte 5 runes, Lily
      `ThemePicker`, `TextSizePicker` and, on the briefs, `LocalePicker` at the CRM UI's pinned versions. Pages from
      README, INSTALL, the family-bridges tutorial, the spec index, the locale status and the 121 role briefs (11
      locales); briefs matched across scripts by `.locale-peer-id`; the Arabic pages `dir="rtl"`; the site's own words in
      the page's language by the UI's approach (English, Arabic, Welsh, Hindi, Chinese; machine drafts). Content copied
      by `scripts/sync-content.mjs` (`--check` in CI). `make github-pages` runs `bin/make-github-pages` (checks, then
      `git subtree push --prefix=contact-relationship-management.github.io github-pages main`); it was run once and
      refused, as it should, for want of the remote. `scripts/ci-check.sh site` and the CI job `site`: content check,
      svelte-check, build, 66 Playwright tests with axe (no violation of any impact, light and dark, right to left, the
      locale switch). A Dependabot entry; the root README says what the owner must do to publish.
      **Not done / not verified:** no `github-pages` remote, no exported repository, no push (CRM-T178); the deploy
      workflow has never run and its Pages actions are pinned by tag; GitHub Pages serving the native-script paths;
      the documents other than the briefs are English; the site's words in four languages are unreviewed; two themes;
      axe on 11 of the 121 briefs.
- [ ] CRM-T178 **Publish the site.** *(The owner's.)* Create
      `contact-relationship-management/contact-relationship-management.github.io`, set its Pages source to GitHub Actions,
      add the `github-pages` remote and run `make github-pages`; then check the deployed pages, the native-script paths
      among them. Steps: the root README, "GitHub Pages site". (S)
