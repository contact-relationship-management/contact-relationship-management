# Changelog

All notable changes to this repository are summarized here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Each
subproject keeps a detailed changelog:

- [Rust service](contact-relationship-management-service-with-rust/CHANGELOG.md)
- [Svelte front end](contact-relationship-management-ui-with-svelte/CHANGELOG.md)

## [Unreleased]

### Fixed — the worker service's events are put in order (CRM-T159, CRM-R93)

A hire or leave from WPM that is older (by `occurred_at`, then `effective_on`) than the last one applied for the same person and organization is answered `stale` and changes nothing, so a hire retried after the leave that followed it no longer pauses the contact again. The last kind and time are kept per person and organization while the person is a contact, shown in the subject-access export, deleted on erasure and pruned after a leave.

**Not done or not verified:** not run against a running WPM; an event arriving after the leave's row was pruned (30 days by default, WPM retries for about two) is not ordered; a hire or leave without `occurred_at` is now `422`.

### Added — the worker service's hire and leave events pause marketing (CRM-T158, CRM-R92)

`POST /api/bridges/wpm/events` takes WPM's signed `worker_hired`, `worker_terminated` and `worker_retired` events (`CRM_WPM_WEBHOOK_SECRET`; `404` while unset). A hire marks the contacts with that `person_ref` employees of the organization, which leaves them out of segments, campaigns and nurture steps; a leave lifts it under the consent they already had. Applied once per event id, audited with `via: wpm`, shown on the contact's page. The spec has a "Family bridges" table in integrations.md, CRM-D15, and the DPIA, inventory and record of processing cover the new data.

**Not done or not verified:** no delivery from a running WPM was received (the tests sign requests with the same algorithm, and WPM was adding `data` in parallel); a future `effective_on` takes effect at once rather than when due; events are not ordered (a retried hire after a leave re-pauses); contacts created after their hire are not marked, and the client for WPM's overlap check (WPM-R208) that would find them is not built; the `worker:` versus `person:` reference mismatch in the teams import is unresolved.

### Added — a delay for clicks (CRM-T157, CRM-R91)

`GET /api/attribution` takes `min_click_seconds` (1 to 86,400, with `clicked` counted): clicks reported sooner after sending do not count; echoed in the answer.

### Added — position-model weights (CRM-T156, CRM-R90)

`GET /api/attribution` takes `end_percent` (1 to 49, for `position`): the share each of the first and last touches earns, 40 by default; echoed in the answer.

### Added — attribution settings (CRM-T155, CRM-R89)

`GET /api/attribution` takes `half_life_days` (4 to 90, for `time_decay`) and `min_open_seconds` (opens reported sooner after sending do not count); both are echoed in the answer.

### Added — a reply by e-mail resumes a waiting ticket (CRM-T154, CRM-R88)

`POST /webhooks/inbound-email` (signed like `/webhooks/email`) reads a sender and a subject; a subject carrying a ticket's reference (`[T-0a1b2c3d]`, shown on the ticket) from the ticket's contact puts a waiting ticket back to `open`. Nothing of the message is kept. Not checked against a real mail service.

### Added — a credential for the bridges (CRM-T153, CRM-R87)

`CRM_PPM_SERVICE_TOKEN` and `CRM_WORKER_SERVICE_TOKEN`: sent as a bearer token to PPM and WPM, only over https or to this machine.

### Added — close-day rates in a campaign's ROI (CRM-T152, CRM-R86)

`GET /api/campaigns/{pid}/funnel` takes `rate_basis=close_day` with `convert=true`. The bridge to WPM reads at most 51 pages of workers (WPM refuses a larger offset), and both bridges' contracts were checked against the sibling services' source; neither sends credentials yet.

### Added — close-day rates in attribution (CRM-T151, CRM-R85)

`GET /api/attribution` takes `rate_basis=close_day`: each deal's share to a campaign is converted at the rate of the day the deal closed. Also: `cargo fmt` applied to four files that CI's format check would have refused.

### Added — a starting hazard analysis in the manner of ISO 14971 (CRM-T150, CRM-R84)

`spec/compliance/risk-management.md`: the standard's process against what the software does, thirteen hazards with their controls and status. No conformity is claimed.

### Added — close-day rates in reports (CRM-T149, CRM-R83)

A report of deal amounts can set `rate_basis: close_day` to convert each closing day's sums at that day's rate, for won or lost deals. Fixed on the way: the OpenAPI body test for the bridge routes, and a crash of the reports page when one currency had two rates.

### Added — closing a ticket left resolved (CRM-T148, CRM-R82)

An SLA policy may set `auto_close_days` (1 to 365); the sweep then closes a ticket that has stayed resolved for longer, counted from its last resolution.

### Added — articles suggested for a ticket (CRM-T133, CRM-R66)

`GET /api/tickets/{pid}/suggested-articles`: the published articles that share words with the ticket's title, best first, with the words they share; shown on the tickets page.

### Added — retention for delivery and event logs (CRM-T140, CRM-R81)

The retention sweep also prunes email deliveries and events (`CRM_MAIL_RETENTION_DAYS`, 730 days, never below 400) and the event outbox and webhook deliveries (`CRM_EVENT_RETENTION_DAYS`, 30 days, never below 7), only rows that are finished with.

### Added — ESLint and Prettier as tests (CRM-T147, CRM-R80)

`pnpm test` fails on a lint or formatting fault and tests the project's own lint rule; `pnpm lint` and `pnpm format` now cover tests, scripts and the configuration files.

### Changed — confirmation and preference links work once (CRM-T146, CRM-R79)

They are random tokens kept only as a hash, spent in the transaction of the act they allow, and deleted a week after they expire or are spent. A preference choice hands the page a fresh token. The unsubscribe link is unchanged, on purpose.

### Added — small groups are left out of reports (CRM-T145, CRM-R78)

A report leaves out any group with fewer than `CRM_MIN_CELL_SIZE` records behind it (5 by default, 0 for none) and says how many it left out. Primary suppression only.

### Changed — every page is scanned for accessibility (CRM-T144, CRM-R77)

The axe gate now covers all pages, not 12, and a test fails if a page is added without being listed. Five barriers it found were fixed.

### Changed — no page shows the service's error text (CRM-T143, CRM-R76)

The UI says what kind of failure it was, in the reader's language, and never the service's own words; enforced by lint and a test.

### Added — a translation-review register (CRM-T142, CRM-R75)

Which locales are drafts and which a native speaker has reviewed (none yet), with a test that holds the register, the keys and the `{placeholders}` to the locale files.

### Added — personal-data inventory, retention schedule, DPIA and record of processing (CRM-T141, CRM-R74)

`spec/compliance/` gains an inventory of every table with its retention class, a DPIA filled in as far as the software goes, and an Art. 30 record; a test keeps the inventory complete. Four tables with no deletion path are named as a gap (CRM-T140).

### Added — bridges to PPM and WPM (CRM-T139, CRM-R72, CRM-R73)

`POST /api/bridges/ppm/deals/{pid}` opens one delivery idea in the project portfolio service for a won deal; `POST /api/bridges/wpm/teams-import` sets a team per department from the worker service. Admin only and stub-first: nothing leaves the process unless `CRM_UPSTREAM_MODE=http`; set `CRM_PPM_SERVICE_URL` and `CRM_WORKER_SERVICE_URL`. Checked against stand-ins, not the running services.

### Added — clawing credit back from a worker (CRM-T138, CRM-R71)

`GET`, `PUT` and `DELETE /api/quotas/clawbacks/{pid}` (admin only): an amount taken back from one worker's credit for a won deal, up to what they hold; attainment follows, for that worker alone.

### Added — CLV at the rate of the day each deal closed (CRM-T137, CRM-R70)

`GET /api/accounts/{pid}/clv?convert_to=&rate_basis=close_day` converts each won deal at the rate in force on its closing day, rounded once per deal; deals with no rate then are named, not taken at 1.0. The account page has a checkbox for it.

### Added — resolving or escalating a ticket that waits too long (CRM-T136, CRM-R69)

An SLA policy may set `wait_limit_action` (`flag`, `resolve` or `escalate`, with `escalate_to`) beside `max_wait_minutes`: the sweep resolves the ticket or reassigns it to that worker, once. It resolves rather than closes, since `closed` is final.

### Added — territories of named accounts (CRM-T135, CRM-R68)

`GET`, `PUT` and `DELETE /api/quotas/territories` and `GET /api/quotas/territory-attainment` (admin only): a territory is a named list of accounts, each in at most one, and the revenue won on them is added per currency, never converted. The `/quotas` page keeps territories and shows what was won.

### Added — teams and their quota roll-up (CRM-T134, CRM-R67)

`GET`, `PUT` and `DELETE /api/quotas/teams` and `GET /api/quotas/team-attainment` (admin only): teams of workers, and each team's quotas and credited won revenue added per currency, never converted. The `/quotas` page keeps teams and shows the roll-up.

### Added — splitting a won deal between workers (CRM-T132, CRM-R65)

`GET`, `PUT` and `DELETE /api/quotas/splits/{pid}` (admin only): a deal is credited to several workers in shares of 10000, shared to the minor unit, and quota attainment follows.

### Added — a limit on how long a ticket may wait (CRM-T131, CRM-R64)

An SLA policy may set `max_wait_minutes`; the sweep flags a `pending` ticket that has waited longer, once, and the tickets page marks it.

### Added — a customer reply resumes a waiting ticket (CRM-T130, CRM-R63)

`POST /api/tickets/{pid}/customer-reply` sends a `pending` ticket back to `open` and starts its SLA clock again.

### Added — the CLV margin and look-ahead converted (CRM-T129, CRM-R62)

`GET /api/accounts/{pid}/clv?convert_to=` also adds `margin_converted_minor` and `projection.converted`, at the same rates as the CLV.

### Added — retention taken from an account's own history (CRM-T128, CRM-R61)

`GET /api/accounts/{pid}/clv?retention_from=history&years=` reads each currency's retention from the latest year against the year before it, held at 100%.

### Added — a CLV panel on the account page (CRM-T127, CRM-R60)

The account page shows lifetime value per currency, with the margin and the discounted look-ahead when asked for.

### Added — margin and a discounted look-ahead on CLV (CRM-T126, CRM-R60)

`GET /api/accounts/{pid}/clv` takes `margin_bps`, `retention_bps`, `years` and `discount_bps`, and answers with `margin_minor` and `projection` per currency.

### Added — quotas and commission plans (CRM-T125, CRM-R59)

`/api/commission-plans`, `/api/quotas` and `GET /api/quotas/attainment` (admin only): what each worker is to win in a period, a tiered plan that pays on it, and where each worker stands, with the `/quotas` page.

### Added — opens and clicks as attribution touches (CRM-T124, CRM-R58)

Mail-provider open and click events are recorded once per message, only while the person still consents (the service never turns tracking on), and `GET /api/attribution?touches=sent,opened,clicked` can count them as touches.

### Added — a public help centre and a deflection report (CRM-T123, CRM-R57)

Published knowledge-base articles open to anyone under `/kb/` (off until `CRM_PUBLIC_KB` is set, rate limited, plain text, counted without identifying anyone), the `/help` pages, and `GET /api/insights/kb-deflection` with a Public help centre panel on the Articles page.

### Added — time-decay and position-based attribution (CRM-T122, CRM-R56)

Two more models for `GET /api/attribution` and the Attribution page, exact to the minor unit: `time_decay` (a touch's weight halves every 7 days before the deal closed) and `position` (40% first, 40% last, 20% between).

### Added — SLA clocks that stop while a ticket waits for the customer (CRM-T121, CRM-R55)

A ticket moved to `pending` stops its SLA clock and does not breach while it waits; leaving `pending` moves each deadline by the waiting that counted against it, in working time on a business calendar.

### Added — a sweep for money that skips masking (CRM-T120)

A test asks every `GET` endpoint as a masked reader, after the service has seeded one distinctive amount, and fails if the amount appears in any answer.

### Fixed — CLV is masked (CRM-T119)

`GET /api/accounts/{pid}/clv` returned an account's money to a caller whose collection read is masked. It now returns the count of won deals and `null` for the figures, as the forecast does.

### Added — converted CLV (CRM-T118, CRM-R54)

`GET /api/accounts/{pid}/clv` can show the lifetime value as one total in a chosen currency, naming the rates used and the amounts left out for want of a rate.

### Added — converted forecast (CRM-T117, CRM-R53)

`GET /api/forecast` and the deal board can show the forecast as one total in a chosen currency, naming the rates used and the totals left out for want of a rate.

### Added — converted campaign ROI (CRM-T116, CRM-R52)

The campaign funnel can count revenue won in other currencies at the exchange rate in force, so ROI is not understated, naming the rates used and the revenue left out for want of a rate.

### Added — converted attribution (CRM-T115, CRM-R51)

`GET /api/attribution` and the Attribution page can show every figure in one currency, naming the rates used and the credit left out for want of a rate.

### Added — exchange rates and converted reports (CRM-T114, CRM-R50)

Hand-entered exchange rates (a pair, a day, a source) and a deal report that shows its sums in one currency using the latest rate on or before a date, naming the rates and listing what it could not convert.

### Added — multi-touch attribution (CRM-T113, CRM-R49)

Won-deal revenue shared among the campaigns that touched the buyer, under a first-touch, last-touch or linear model, exact to the minor unit and per currency (`GET /api/attribution`), and an Attribution page.

### Added — Markdown lint (CRM-T94)

`rumdl` runs in the `lint` stage (`.rumdl.toml`), and Woodpecker gains the `lint` step it lacked. Formatting findings fixed in 16 files.

### Changed — dependencies (2026-10-25)

Cargo lockfiles brought to the latest compatible versions (service and matcher); `jsonschema` 0.58 to 0.60; the two GitHub Actions that were behind (`Swatinem/rust-cache` 2.9.3, `taiki-e/install-action` 2.87.30, still pinned by SHA); the Stalwart mail image digest in `compose.email.yaml`. The UI's Lily Design System packages and `vite` are at their latest. Not taken: TypeScript 7 (`svelte-check` does not run on it yet).

### Fixed

- Tests for the limits and the fixed filter values of the new rules (calendar, reports), found by mutation testing; a weekly mutation-testing workflow (CRM-T94).
- The release workflow no longer expands tag names, the actor or step outputs into shell scripts, keeps no token in the checkout, and uses no build cache in the job that signs and publishes; Dependabot waits a week before proposing a new release (found by `zizmor`, CRM-T94). Not yet run: test it with a release-candidate tag.
- On a phone-width screen (360 px) the page no longer scrolls sideways: the header wraps and form fields fit their panel (CRM-R46).

### Added

- A report builder: count records, or add up deal amounts per currency, grouped by one field, with filters and a date range, saved and shared like a view (CRM-R48).
- Tags on contacts, accounts, leads and deals: find records by tag, select a segment by tag, and have tags exported and erased with the person (CRM-R47).
- Business calendars: an SLA policy can count its targets in working time (windows per weekday, a time zone, holidays) instead of round the clock (CRM-R45).
- Lead assignment: a `lead_created` workflow trigger, a lead source condition and an `assign_owner` action that gives each new lead to a worker in turn or by load, with a notification, a run log and a dry run (CRM-R44).
- Import of contacts and accounts from a CSV, with a preview, duplicate advice, lookups in the person and organization services, no consent and no workflows, and undo (CRM-R43).
- File attachments on contacts, accounts, leads and deals: an allow-list of types checked against the file's start, a size limit, an optional malware-scan command, integrity-checked downloads that are always attachments, subject access and erasure (CRM-R42).
- Typed custom fields on contacts, accounts, leads and deals, with a sensitivity that decides masking, sealing, subject access and erasure (CRM-R41).
- Validation errors name the field in the contact and account edit forms; the executive pack and pipeline hygiene read less from the database; the sign-in tests share a process with the others (CRM-T78 to CRM-T80). The matcher is 0.2.0 in the tree, not published (CRM-T87).
- Explainable workflow rules: a trigger, conditions in the segment filter language and a closed list of actions, with a dry run, a loop guard, consent re-checked inside enrolment, and a run log that says why each run happened (CRM-R40).
- `PUT /api/accounts/{pid}` to edit an account after creation (audited by field name), with an edit form on the account page (CRM-R39).
- A contact's optional `given_name`, and `PUT /api/contacts/{pid}` to edit a contact after creation (audited by field name), with an edit form on the contact page (CRM-R38).
- Email templates with merge tokens, a live preview for a made-up contact, and the unsubscribe link required in every body; campaigns and nurture steps can use one (CRM-R36).
- Saved views of the five main lists, per user or shared, in a view bar above each (CRM-R35).
- CSV import of leads with a per-row report, duplicate advice and undo (CRM-R34).
- Server-side search across the main records: accent-blind, typo-tolerant, respecting what the caller may read (CRM-R33).
- Task reminders and a notifications inbox: assigned tasks and SLA breaches notify the assignee once, only for records they may
  read (CRM-R32).
- Web form lead capture (`POST /forms/{slug}`): a sign-up form on a website makes a lead and asks the visitor to confirm
  before anything is mailed; origin check, rate limits and a honeypot protect it (CRM-R31).
- Production-readiness assessment ([spec/plan.md](spec/plan.md)), compliance
  evidence pack ([spec/compliance](spec/compliance/index.md)), tamper-evident
  audit chain, fail-closed production boot, security headers and CSP,
  supply-chain CI stage with SBOMs, hardened Containerfiles, and an axe
  accessibility gate.
- Special files for a public repository: README, LICENSE, CITATION.cff,
  CODEOWNERS, MAINTAINERS, GOVERNANCE, SECURITY, CONTRIBUTING, RFC, INSTALL,
  NEWS, COMPARISONS, BENCHMARKS, AI_STATEMENT.
- Standalone CI for GitHub Actions and Woodpecker.

### Changed

- The repository is self-contained: shared crates come from crates.io or a
  pinned git revision instead of monorepo paths.
- Front-end npm dependencies updated to latest.
