# Testing

- **Pure-core unit tests** (DB-free, exhaustive): every lifecycle's
  legal/illegal transition matrix (lead, deal incl. terminal
  immutability + reasoned reopen, campaign, ticket incl. reopen,
  article); the scoring rule table + clamp + breakdown sums;
  forecast arithmetic (per-currency separation, empty pipelines);
  ROI incl. zero-cost `null`; CLV; SLA deadline derivation +
  breach flip + priority-change re-derivation; segment filter
  evaluation incl. the consent AND-gate; overflow refusal on all
  money sums.
- **Request tests** (Postgres, `#[ignore]`d): the sales journey
  (capture lead → score → convert → deal through stages → won →
  forecast reflects it); the marketing journey (consent → segment
  preview → campaign run (simulated) → attribution → unsubscribe
  exits nurture and blocks the next send); the support journey
  (open → first response stamps → resolve; priority change
  re-derives; breach sweep emits once); Kanban reorder race
  (`FOR UPDATE`, one winner); nurture scheduler idempotency (rerun
  ⇒ no double-send); unknown-pid 404s (family lesson, pinned from
  day one).
- **One integration-test binary** (CRM-T263): `tests/main.rs` declares every suite as a module
  (`autotests = false`, one `[[test]]` target), so the service is linked into one test binary, not
  seven. A test that changes the settings or the environment does it for its own length through
  `tests/support/scoped_auth.rs` or `scoped_env.rs`, which put them back, including by panic, and
  is `#[serial]`. Run one suite with `cargo test --test main <module>`.
- **Enforcement tests** (`tests/requests/enforcement.rs`; they once
  ran in their own process for the OnceLock lesson, and now set the
  settings for their own length through `tests/support/scoped_auth.rs`
  and share the request-test process): the persona matrix (rep `$sub` ownership vs other-rep refusal,
  manager team scope, marketing pipeline read-only, support scope,
  `mask` obligation on amounts/forecast).
- **Bridges** (CRM-R72, CRM-R73, CRM-R92–R94): request tests against
  stand-in PPM and WPM servers and requests signed as WPM signs
  (`tests/requests/bridges.rs`, `wpm_events.rs`,
  `employment_check.rs`); none is run against a running PPM or WPM.
- **Migrations with data**: `scripts/migrate-with-data.sh` (migration
  17), `scripts/migrate-consent-purposes.sh` (migration 21) and
  `scripts/migrate-person-refs.sh` (migration 54, CRM-D18) migrate a
  populated database forward and check the rows; the last two also roll
  it back and forward again.
- **Front-end**: vitest for the API client path map, `money()`,
  score-breakdown + SLA-countdown components; Playwright over a
  `page.route`-stubbed API (contract-mirroring, unstubbed =
  404-loud).
- Seed task: a synthetic book of business (~50 contacts, ~15
  accounts, leads in every status, 2 pipelines, ~30 deals, a
  campaign + nurture sequence, ~20 tickets against 2 SLA policies)
  — synthetic data only.

## Mutation testing (CRM-T94, CRM-T259)

`cargo mutants` changes each rule a little and checks that a unit test notices; a mutant that
survives is a rule no test pins. It covers the domain crate (`contact-relationship-management-domain`:
`rules/` and `time.rs`, CRM-T260) and the matcher, which have no database or network, and leaves the
controllers to the request tests.

- **Weekly, everything:** `.github/workflows/mutants.yml`, seven shards, the matcher in shard 0;
  `scripts/nightly.sh` runs a seventh a night locally.
- **On every push and pull request, what changed:** `scripts/ci-check.sh mutants-diff` tests only the
  mutants in lines changed since the push's previous commit or the pull request's base, so a new rule
  that no test pins fails the change that adds it.
- **Builds:** the `mutants` profile (no debug information), and the domain crate's own unit tests,
  which build without axum, axa or sqlx: about 5 seconds a mutant.
- **Equivalent mutants** (no test can tell them from the code) are listed with a reason in each
  crate's `.cargo/mutants.toml`; where the code had a redundant check, the check went instead.

## Coverage (CRM-T266)

`scripts/ci-check.sh coverage` measures the service's and the domain crate's unit tests with
`cargo-tarpaulin`, on its LLVM engine, so a Mac and a Linux runner count lines alike. It is a
report, not a gate: the database suite (`test-db`) is not in it, so the figure understates what the
whole suite reaches, and no floor is set until the figures have been watched. It replaced
`cargo-llvm-cov`.

## Performance (CRM-T242)

Three kinds of measure, each with synthetic data only and each recorded in
[`BENCHMARKS.md`](../BENCHMARKS.md) with its commit, machine, data size and command:

- **The rules' CPU cost** (criterion): the matcher (`benches/matching.rs`), and the service's
  pure rules a request runs (the domain crate's `benches/rules.rs`): lead scoring, segment evaluation, the
  weighted forecast, the audit-chain check, a lead import at its 500-row limit, attribution
  under each model, report currency conversion, report planning and small-cell suppression,
  search-query building, and (CRM-T262) business-calendar deadlines and sums, template rendering,
  webhook signing and address checks, and reading and writing times. No database, network or clock: a request's rule step, not its
  latency. `scripts/ci-check.sh bench` runs both (CRM-T266); `CRM_BENCH_SAVE=<name>` keeps the run as
  a criterion baseline and `CRM_BENCH_BASELINE=<name>` reports each case's change against one, so a
  change to a rule can be read against the run before it on the same machine.
- **The API under load** (k6, [`tests/load/`](../tests/load/README.md)): one module of real
  calls with real request bodies, a weighted mixed workload, and smoke, basic, stress, spike,
  soak and breakpoint runs; a seed that makes a volume of rows first; and an endpoint
  benchmark that runs each group of routes on its own at a fixed arrival rate, with a p95 limit
  per group. A load test writes synthetic records and deletes none, so it runs only against a
  throwaway database, never one holding real personal data. It sends nothing to anyone: no
  campaign run, nurture, webhook, MCP call or web form.
- **The UI's browser code** (`contact-relationship-management-ui-with-svelte/scripts/bundle-size.mjs`):
  the built JavaScript and CSS, raw and gzipped, and the part every page loads.
- **Page load in a browser** (`pnpm perf:pages`, `tests/perf/page-load.spec.ts`, CRM-T262): eleven tour
  pages, cold and warm, plain and throttled to a mid-range phone, with time to first byte, DOM
  ready, load, largest contentful paint, requests and JavaScript bytes.

None of these is a CI gate: a figure from a shared runner, or from a laptop, is a reading, not a
limit. The k6 limits fail a run, and the endpoint limits are the ones a change should not
cross on the same machine.
