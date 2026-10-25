# Plan — production readiness for medical and governmental organizations

Assessed 2026-10-10 against the repository as of commit `1d8909b`.
Companion to [tasks.md](tasks.md) (the CRM-T30–T44 rows) and
[regulatory.md](regulatory.md). This is an engineering assessment, not a
legal or accreditation opinion.

## Verdict

**Not ready.** The software is honest demo software and says so. Its design
is a good base (consent-first send path, ABAC with record-level checks on the
main handlers, audit-in-transaction, subject access/erasure/retention,
minimal PII, DB-free rule core, `forbid(unsafe_code)`), but major medical
(HIPAA, NHS DSPT / DCB0129, GDPR special-category) and governmental
(FedRAMP / NIST SP 800-53, UK Cyber Essentials Plus / GovAssure, ISO 27001,
WCAG 2.2 AA / Section 508 / EN 301 549) buyers would fail it today on the
gaps below. Most gaps are known and stated in [auth.md](auth.md); several
others were found in this assessment.

## What is already strong

| Area | Evidence |
|---|---|
| Authorization model | ABAC personas, record-level `authorize_record` on contact/deal reads and writes, destructive-action classification |
| Authentication | Offline PASETO v4.public, optional Keycloak/OIDC, key rotation, no JWT dependency |
| Privacy by design | URN wrappers, consent history append-only, subject access, erasure, retention sweep |
| Audit | Mutation audit rows commit with the mutation; outbox events |
| Engineering hygiene | No `unsafe`, clippy pedantic, lockfiles, CI on two providers, 100+ tests |

## Gap register

Severity: **B** blocks any non-demo deployment; **H** blocks regulated buyers;
**M** expected by procurement checklists.

| # | Gap | Sev | Task |
|---|---|---|---|
| 1 | `CRM_REQUIRE_AUTH` defaults **off**; the service boots wide open in production with no warning | B | T30 |
| 2 | Audit rows are plain inserts: anyone with DB write access can alter or delete history undetected (HIPAA §164.312(b)/(c), NIST AU-9/AU-10) | H | T31 |
| 3 | `mask` obligation not applied to forecast values, campaign ROI/cost, or the activities/deals/tickets bundled in `get_contact` | H | T32 |
| 4 | No security response headers on the service or the UI; production CORS is `enable: true` with no origin list | H | T33 |
| 5 | No dependency-vulnerability, licence, or SBOM gate in CI | H | T34 |
| 6 | No hardened container image (non-root, read-only rootfs, pinned digests) | M | T35 |
| 7 | No control mapping, DPIA template, or operations runbook (backup/restore, key management, encryption at rest, incident response, logging retention) | H | T36 |
| 8 | No accessibility gate (WCAG 2.2 AA / Section 508) | H | T37 |
| 9 | `/api/audits*`, `/api/events/recent`, `/api/retention` are readable by any caller that clears the coarse read gate; `/metrics.prom` and OpenAPI are unauthenticated | H | T38 |
| 10 | Real send path (ESP adapter), processor agreements, bounce/complaint handling absent | B | T39 |
| 11 | Reads of personal data are not audited (spec claims sensitive reads are; only the consent-history read is) | H | T40 |
| 12 | Field-level encryption now covers two columns with local keys; no KMS integration and other columns are plaintext at the field level | M | T41 |
| 13 | No independent penetration test, risk analysis, or accreditation evidence | B | T42 |
| 14 | No rate limiting or lockout; no UI idle-session timeout or re-authentication for destructive actions | M | T43 |
| 15 | Break-glass / emergency access with mandatory reason and after-the-fact review absent | M | T44 |
| 16 | The service is a demo with synthetic data and no clinical or case content; a medical buyer must also decide whether CRM data about patients is PHI in their context | — | scope note below |

## Phases

1. **Fail safe (T30, T31, T33, T34)** — make the unsafe default impossible,
   make the audit trail tamper-evident, add headers and supply-chain gates.
   All code-side; implemented 2026-10-10.
2. **Evidence (T35, T36, T37)** — container hardening, control mapping and
   runbooks, accessibility gate. Implemented 2026-10-10.
3. **Complete the access model (T32, T38, T40, T43, T44)** — masking,
   admin-only operational reads, and read auditing landed 2026-10-10 with
   stated limits (insights unmasked). Rate
   limits and idle sign-out (T43) landed too, without step-up re-auth.
   Break-glass (T44) landed with stated defaults, API and UI.
4. **Real-world exposure (T39, T41, T42)** — ESP integration and contracts,
   field-level encryption with a KMS, independent test and accreditation.
   Open: external parties and legal work, not code.

## Scope note

This CRM stores relationship data, not clinical records. A hospital or health
department that records patients as contacts, or a government body that
records citizens as contacts, takes on the special-category and public-sector
duties for that data. The deployer decides that classification; the plan above
is written to the stricter reading.

---

## Plan — new capabilities and code quality

Researched 2026-10-21 against commit `331478a`. Companion to the CRM-T75 to
CRM-T104 rows in [tasks.md](tasks.md). Nothing here is built yet. Each
capability needs a new requirement (proposed ids CRM-R28 onward) written with
its code and tests, under the three-part rule.

### How this was researched

- **Code quality:** a read-only pass over the service, matcher and UI.
  Clippy at the default level is clean on both crates, including the
  `--no-default-features --features keycloak` build. A pedantic pass gives no
  pedantic-group warnings. With nursery and restriction lints on, there are 146
  warnings. `cargo machete` and `cargo tree -d` were run. In the UI,
  `svelte-check` (0 errors), vitest (107 of 107) and prettier were run.
  **Not run:** the DB-backed tests, Playwright, `cargo audit`/`deny` and
  `cargo outdated`.
- **Capabilities:** compared with the current documentation of Twenty,
  EspoCRM, SuiteCRM, Krayin, HubSpot, Salesforce, Pipedrive, Attio and Folk.
  Sources are cited in each task. Some are third-party guides, and those are
  marked; check them against the vendor's own docs before quoting them in the
  spec.

### Baseline: already good

- **Rust:** no `unwrap`, `panic!`, `todo!` or TODO comments outside tests.
- **UI:** no legacy Svelte (`$:`, `export let`, stores, `on:`, `<slot>`), no
  `any`, no `@ts-ignore`.
- **i18n:** all 8 locales have the same 330 keys, and every key the code uses
  is defined.
- **Dependabot** covers Cargo, npm, both images and the Actions.

### Code quality findings

| # | Finding | Evidence | Size | Task |
|---|---|---|---|---|
| Q1 | No lint configuration, so the code is pedantic-clean by habit, not by gate | service `Cargo.toml` has no `[lints]`; 17 hand-placed `#[allow]`s | S | T75 |
| Q2 | Write-path boilerplate repeated across controllers | 54 `Audit::record`, 46 `ctx.db.begin()`, 79 `DeletedAt.is_null()` filters, 51 `parse_pid` | M | T76 |
| Q3 | Long handlers mix validation, transaction, audit, outbox and metrics | `idempotency_mw` 185 lines, `unmerge` 165, `lead_status` 129, `merge` 113 | L | T77 |
| Q4 | Configuration read from the environment into about 17 global `OnceLock`s | 29 `env::var` calls; tests use 22 `set_var`; enforcement tests need their own process | M/L | T78 |
| Q5 | Errors are `String`s, and the 422 joins problems with `"; "` | 27 `Result<_, String>`; four error shapes reshaped by one middleware (`src/problem.rs`) | M | T79 |
| Q6 | Insight endpoints load whole tables into memory | `live!` macro: 23 unbounded `.all(db)`; contact detail loads all related rows | M | T80 |
| Q7 | Display-name cache never expires and has no size limit | `src/clients.rs:50-80` | S | T81 |
| Q8 | Middlewares and the largest controllers have no unit tests | `app.rs` (760 lines, 6 middlewares) has no tests; `relationships.rs`, `sales.rs`, `campaign.rs` none | M | T82 |
| Q9 | OpenAPI is hand-written and responses are untyped `json!` | `openapi.rs` 520 lines, `openapi_schemas.rs` 741 lines, 129 `json!` in controllers | L | T83 |
| Q10 | Unused and duplicate dependencies; the service tests the published matcher, not the one in the repo | `tracing-subscriber` unused; `sqlx-sqlite` enabled but never used; reqwest 0.12 and 0.13 both present; service pins matcher 0.1.1 while the repo has 0.1.2 | S | T84 |
| Q11 | Dead and stale configuration and references | `auth.jwt` and `JWT_SECRET` with Loco's JWT feature off; `deny.toml` `allow-git` with no git sources; 12 links to a missing `agents/share/` | S | T85 |
| Q12 | Text copied from another project | UI `package.json` description says "ward whiteboards, bed management"; `clients.rs` and `streaming.rs` describe WPM. ("Main X" itself is the family's name, so it is not a mistake.) | S | T86 |
| Q13 | Matcher's public API is not `#[non_exhaustive]`, and CI has no semver check | 0 `non_exhaustive`; `ConfigError(pub String)`; three published versions in two days | S/M | T87 |
| Q14 | Matcher has no benchmarks, no pedantic lints, and no supply-chain gate | no `benches/`; `BENCHMARKS.md` says none exist | S/M | T88 |
| Q15 | The UI ships all 8 locales in one bundle | `i18n.svelte.ts` is 3,320 lines; the chunk is 52 KB gzipped; 19 de-de values identical to English | M | T89 |
| Q16 | Pages fetch in `$effect` instead of `load`, with no cancellation | 28 `$effect`s; 11 `void (async…)()`; 36 copies of the same error idiom; 0 `AbortController` | M | T90 |
| Q17 | The SvelteKit migration is unfinished | `MIGRATION_TASKS.md` has 10 open boxes; `src/env.ts` falls back to `""` silently; `$lib` and `#lib` imports are mixed | S | T91 |
| Q18 | The demo tour is on by default, and every visitor shares one fixed session id and one copy of the data | `server/config.ts:18`; `demo.ts:30` `DEMO_SESSION` | S | T92 |
| Q19 | No component tests; the API client is hand-written | `@testing-library/svelte` installed but unused; `api/crm.ts` 911 lines; a vitest `deps.inline` regex that matches nothing | M | T93 |
| Q20 | Missing CI gates | no coverage, mutation testing, ESLint, `typos`/markdown/`shellcheck`/`actionlint`, or Playwright in CI; `ci.yml` has no `permissions:`; Woodpecker installs Node 24 while the project needs 26; the matcher is not audited | M | T94 |

**Order:** start with the small gates and cleanups (T75, T84 to T87, T91,
T92, T94) so that later changes cannot regress. Then the service structure
(T76, T78, T79, T82), then the larger rewrites (T77, T80, T83) and the UI
(T89, T90, T93).

### New capability candidates

These are ranked by value and by how much they reuse what already exists.
**Conflict** means the item clashes with [scope.md](scope.md) or with the
boundary between this service and the family's other services.

| # | Capability | Reuses | Size | Value | Task |
|---|---|---|---|---|---|
| C1 | Outbound webhooks, signed (Standard Webhooks), relayed from the event outbox | `event_outbox`, ABAC | M | H | T95 |
| C2 | Consent per purpose, a `pending` state, double opt-in, a preference page | CRM-R6, unsubscribe tokens, merge consent rule | M | H | T96 |
| C3 | MCP server for AI agents, read-only by default, under the caller's own ABAC, audited | OpenAPI, `llms.json`, idempotency, dry runs | M | H | T97 |
| C4 | Public web form that creates a lead, with consent `pending` | C2, rate limiting | M | H | T98 |
| C5 | Tasks with an assignee, reminders, and an in-app notifications inbox | task activities, `bg_pg`, SLA sweep | M | H | T99 |
| C6 | Server-side search (PostgreSQL full-text search plus `pg_trgm`), filtered by ABAC | CRM-R22 | M | H | T100 |
| C7 | CSV import of leads: column mapping, dry run, duplicate advice, undo | matcher, export, merge ledger pattern | M/L | H | T101 |
| C8 | Saved views on the main lists, using the segment filter language | CRM-R7 | S/M | M | T102 |
| C9 | Email templates with a closed set of merge tokens, a preview, and an unsubscribe link that cannot be removed | nurture steps | S/M | M | T103 |
| C10 | Explainable workflow rules (trigger, condition, action) from a closed set of actions, with a run log | C1, C5, nurture scheduler | L | M/H | T104 |

**Later, after the above:** @mentions in notes (needs C5 and a worker
directory); typed custom fields, where each field declares its sensitivity
for masking, encryption, subject access and erasure (L); file attachments,
which need object storage and a malware-scan hook; a live event stream over
server-sent events for the board and inbox; an opt-in AI timeline summary
built from masked input that cites the activity ids it used (rules stay the
basis for next-best-action); A/B tests for campaigns (meaningless while sends
are simulated, so report both arms with no declared winner).

**Out of scope or owned elsewhere:**

- **Conflicts with [scope.md](scope.md):** quotes, products and price books
  (CPQ is out of scope); territories and quotas (already on the
  [roadmap](roadmap.md)).
- **Owned by other services:** SCIM (the authentication and worker
  services); duplicate resolution of people and email or calendar sync (the
  CRM holds no email addresses, and contacts come from the person service).
- **Low value or against the principles:** GraphQL (duplicates the OpenAPI
  REST surface); third-party data enrichment (against consent-first and
  synthetic data); an offline mobile copy (complicates erasure and masking).

**Order:** C1, C2 and C3 first, because they reuse the outbox, ABAC and
consent machinery and none depends on another. C4 needs C2. C10 needs C1 and
C5.

### Limits of this research

The code findings are counts and line references from one read-only pass, not
a review of every function. Sizes are guesses. The capability survey reads
vendors' documentation, not their products, and says nothing about real users
of this CRM. Each task re-checks its finding before work starts.
