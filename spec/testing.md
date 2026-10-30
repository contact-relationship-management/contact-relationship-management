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
