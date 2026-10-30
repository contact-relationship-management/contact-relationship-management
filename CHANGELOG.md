# Changelog

All notable changes to this repository are summarized here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Each
subproject keeps a detailed changelog:

- [Rust service](contact-relationship-management-service-with-rust/CHANGELOG.md)
- [Svelte front end](contact-relationship-management-ui-with-svelte/CHANGELOG.md)

## [Unreleased]

### Added — an organization time zone and fiscal year; "today" and report periods follow them (CRM-T171, 2026-10-30)

"Today" in the service and report months were UTC's, and there was no fiscal year. An administrator now sets the organization's IANA time zone and the month its fiscal year starts (`/api/settings/organization`, migration `000057`, starting at UTC and January); a caller's own zone, sent by the UI in `CRM-Time-Zone`, decides its "today" (quota, team and territory attainment, import dates), and the organization's dates the forecast snapshot. Reports group by month, fiscal quarter or fiscal year in the organization's calendar, with `from`/`to` in its days. The reports page shows and sets the calendar. **Not done or verified:** UTC remains the starting value; some "today"s still read UTC (the KB window, attribution's rate date, the WPM bridge, campaign and workflow due days); no week groups and no non-Gregorian calendar; not run against a live service with the UI. Details: the [service](contact-relationship-management-service-with-rust/CHANGELOG.md) and [UI](contact-relationship-management-ui-with-svelte/CHANGELOG.md) changelogs.

### Changed — locales equal: public consent pages translated, UI literals in the catalogue, a review record per locale (CRM-T175, 2026-10-30)

The confirmation, preference and unsubscribe pages a data subject reads were English only; they now follow the reader's browser language (or the language chosen from links on each page) in the UI's eight languages, right to left in Arabic, and their error pages are no longer turned into JSON. The 58 English grid headers and the English text of the executive, partners, engagement, DPO and follow-ups pages are catalogue strings in all 8 locales. `de-de` and `zh-cn` stay regional codes, recorded as a limit with the reason; each locale has a review record in `contact-relationship-management-ui-with-svelte/spec/reviews/`, none reviewed. **Not done or verified:** all translations are unreviewed machine drafts; the consent mails are still English; English literals elsewhere in the UI were not scanned for. Details: the [service](contact-relationship-management-service-with-rust/CHANGELOG.md) and [UI](contact-relationship-management-ui-with-svelte/CHANGELOG.md) changelogs.

### Changed — names and template text in any language (CRM-T169, 2026-10-30)

The `given_name` merge token took the first word of the display name when no given name was stored, so "Wang Fang" got "Dear Wang"; it now takes its fallback. A template names its language (a BCP 47 tag; migration `000056`), and its built-in fallbacks are in that language for the UI's eight (machine drafts but English); a template in another language must give its own for each token it uses, so no English word lands in it; a template with no language keeps English, as before. The matcher's honorifics, name suffixes, nicknames and legal-form words are per-language data in several scripts, with joined forms such as `株式会社` and `女士` stripped; the imports guess columns named in any of the UI's eight languages. The templates page has a language field. **Not done or verified:** every non-English word list and fallback is an unreviewed machine draft; names without spaces are still not split into parts; a campaign cannot choose a template by each recipient's language; the service keeps the published matcher 0.1.2 until 0.2.0 is published. Details: the [service](contact-relationship-management-service-with-rust/CHANGELOG.md) and [UI](contact-relationship-management-ui-with-svelte/CHANGELOG.md) changelogs and the matcher's.

### Fixed — the deal list fits a phone again with three forecast currencies (CRM-T176 follow-up, 2026-10-30)

CRM-T176 gave the tour three forecast currencies; their totals were joined by non-breaking spaces, so `/deals` scrolled sideways at 360px in CI (Linux Chromium). They now wrap. Details: the [UI changelog](contact-relationship-management-ui-with-svelte/CHANGELOG.md).

### Fixed — phone numbers and search in every script (CRM-T170, 2026-10-30)

The matcher read only ASCII digits, so a phone typed in Arabic-Indic or Devanagari digits was no number, and with a default calling code it dropped a leading `0` everywhere, which is wrong where the `0` is part of the number (Italy). It now reads any of 31 Unicode decimal digit sets and full width (the workforce planning service's table, WPM-T382) and drops a trunk prefix only by a table of 90 calling codes' plans (`0`, `8`, `06` or none); an unknown code drops nothing. Search cut Devanagari and Thai words at each vowel sign and treated a Chinese, Japanese or Thai name as one word, so part of it was not found; words now keep their marks, and scripts written without spaces are indexed and queried as character pairs (migration `000055`, `search_ngrams`), so `芳` finds `王芳`. Why n-grams and not a segmenter is in CRM-R33. **Not done or verified:** the trunk table is from public summaries, not each regulator; the service keeps the published matcher 0.1.2 until 0.2.0 is published, so the duplicate check does not yet use the new phone rules; the n-gram index was not measured for size or speed; no transliteration across scripts. Details: the [service](contact-relationship-management-service-with-rust/CHANGELOG.md) and [UI](contact-relationship-management-ui-with-svelte/CHANGELOG.md) changelogs and the matcher's.

### Changed — the seed and the tour carry more than one currency, rate base and name shape (CRM-T176, 2026-10-30)

The service's seed made every deal in GBP with English placeholder names, and the tour's stand-in typed amounts as pounds times a hundred with every rate from GBP, so no exponent or name-order bug could show in a demo. The seed's deals are now in GBP, JPY (no decimals), KWD (three), EUR, INR and BRL, it sets six rates from five bases, and its contacts are named in seven scripts, given name first and family name first, some with the parts stored and some without. The tour adds a yen and a dinar deal, three rates from other bases and five contacts in Han, Devanagari, Vietnamese, Arabic and Japanese; its helpers take minor units, and its forecast totals each currency apart, as the service does. **Not done or verified:** the names were not checked by a speaker of each language; the won deals (attribution, executive) stay GBP; the request tests do not run the seed (it was run once on a throwaway database). Details: the [service](contact-relationship-management-service-with-rust/CHANGELOG.md) and [UI](contact-relationship-management-ui-with-svelte/CHANGELOG.md) changelogs.

### Fixed — the UI mirrors on the Arabic page (CRM-T174, 2026-10-30)

Logical CSS properties replace the physical left sides, and arrows and date ranges follow the page's writing direction, as the workforce planning UI's do; a scan test and an Arabic Playwright check with axe keep it so. **Not done:** the Lily components' own styles were not audited. Details: the [UI changelog](contact-relationship-management-ui-with-svelte/CHANGELOG.md).

### Fixed — the UI's plural forms follow each language's rules (CRM-T173, 2026-10-30)

Strings that fill a count choose their form with `Intl.PluralRules`, as the workforce planning UI's do, and each locale holds exactly its language's CLDR forms (six in Arabic and Welsh, one in Chinese), held there by a parity test. **Not done:** native review of the forms. Details: the [UI changelog](contact-relationship-management-ui-with-svelte/CHANGELOG.md).

### Fixed — the UI's dates, numbers, percentages and week follow the reader's locale (CRM-T172, 2026-10-30)

One formatter (`locale-format.ts`) and one calendar module (`reader-calendar.ts`) in the UI, shared in behaviour with the workforce planning UI's: numbers, percentages and dates in the page's locale, typed numbers with the locale's decimal mark, today in the reader's time zone, and the week's first day and weekend from CLDR for the reader's region. A scan test fails on hand-rolled formatting. **Not done:** the service's "today" and an organization time zone (CRM-T171). Details: the [UI changelog](contact-relationship-management-ui-with-svelte/CHANGELOG.md).

### Changed — the Pages site's deploy actions pinned by commit; TypeScript 7 held back there too (maintenance, 2026-10-30)

The site's `deploy.yml` now pins `actions/configure-pages` v6.0.0, `actions/upload-pages-artifact` v5.0.0 and `actions/deploy-pages` v5.0.1 to their commit SHAs with the tag in a comment, the workforce planning site's pins and the latest releases (published 2026-03-25, 2026-04-10 and 2026-09-01, so past the week's cooldown); before they were the tags v5, v3 and v4. The upload action's v4 drops dotfiles from the artifact; the build has none. The site's README and spec say so. Dependabot's first site PR (#32) was TypeScript 6.0.3 to 7.0.2 only, and `pnpm outdated` lists nothing else: with 7.0.2 in a scratch copy, `svelte-kit sync` (SvelteKit 3.1) fails with "Cannot read properties of undefined (reading 'readFile')" in `load_tsconfig`, so check and build both fail. The PR is left open with that reason and the lockfile is unchanged. **Gates run locally:** the content check, svelte-check, build, the 66 Playwright tests with axe, and `scripts/ci-check.sh lint`. **Not done or verified:** the deploy workflow has still never run; Dependabot does not watch it (the `github-actions` entry covers the root `.github` only); zizmor still flags its workflow-level `pages: write` and `id-token: write` (as in the workforce planning site); no Dependabot ignore rule was added for TypeScript 7.

### Added — a GitHub Pages documentation site, built here and not published (CRM-T177, 2026-10-30)

New subproject [contact-relationship-management.github.io/](contact-relationship-management.github.io/README.md), the family pattern of the workforce planning repository's `spec/monorepo-github-pages/`, adapted from the project portfolio site: SvelteKit 3 with `adapter-static` (137 prerendered pages), Svelte 5 runes, Lily `ThemePicker`, `TextSizePicker` and `LocalePicker`. Pages from this README, INSTALL, the family-bridges tutorial, the spec index, the locale status and the 121 role briefs in 11 locales, matched across scripts by `.locale-peer-id`; the Arabic pages are right to left; the site's own words follow the page's language in the CRM UI's way (machine drafts for Arabic, Welsh, Hindi and Chinese). `make github-pages` runs `bin/make-github-pages` (`git subtree push --prefix=contact-relationship-management.github.io github-pages main` after checks); `scripts/ci-check.sh site` and the CI job `site` run the content check, svelte-check, the build and 66 Playwright tests with axe. Spec [github-pages-site.md](spec/github-pages-site.md); the README says what the owner must do to publish (CRM-T178). **Not done or verified:** no `github-pages` remote, no exported repository, no push; the deploy workflow has never run; GitHub Pages serving the native-script brief paths; the documents other than the briefs are English; axe runs on 11 of the 121 briefs.

### Fixed — money in each currency's own minor unit; no built-in currency (CRM-T167, CRM-T168, CRM-R50, 2026-10-30)

Exchange-rate conversion assumed every currency had two decimals, so USD 1.00 at 150 became ¥15,000, not ¥150, in every converted report, forecast, attribution, ROI and CLV figure; the UI showed and typed `minor / 100` and `x * 100`, so a yen deal typed as 1500 was stored as 150,000. The service now holds the ISO 4217 minor-unit table (`rules::currency`, the same shape as the project portfolio service's) and `rules::fx::convert` scales by both exponents with one rounding; the UI formats and reads money through `src/lib/money.ts`, the behaviour the project portfolio and workforce planning UIs share, with the exponent from the same ISO 4217 table and a decimal comma accepted. Every currency code the API takes (deals, lead conversions, campaigns, quotas, rates, `convert_to`) must be in the table (`422` otherwise), a campaign without a currency is `422` rather than GBP, and the UI's forms start empty. CRM-R50 rewritten; [worldwide.md](spec/worldwide.md) rows 8 to 11 and 13 to 15 closed. **Found:** `Intl` (CLDR) and ISO 4217 disagree on 16 currencies' decimals; the two sibling UIs take `Intl`'s, so their forint or rupiah amounts disagree with their services; recorded in worldwide.md, the siblings not changed. **Not done or verified:** the rate field still needs a decimal point; percentages are CRM-T172's; the seed and tour are GBP only (CRM-T176); stored codes outside the table are not checked or rewritten and there is no database constraint; metals and funds without a minor unit are refused; not run with the UI against a live service.

### Added — the worldwide equity audit (CRM-T164, CRM-R95, CRM-D19, 2026-10-30)

[spec/worldwide.md](spec/worldwide.md) states the worldwide requirement CRM-R95 and the decision CRM-D19 (jurisdiction is data, a standard is a profile), both **proposed for the product owner**, and audits the spec (19 rows) and the code and UI (45 rows) against them, with file and line: 64 findings, 16 right, 23 partial, 25 gaps, filed as CRM-T165 to CRM-T176. The worst: money assumes two decimals in the UI and in exchange-rate conversion (CRM-T167, to follow the sibling apps' `money.ts`), the regulatory register and compliance mapping are UK and EU only (CRM-T165), consent has one shape and messages carry no postal address (CRM-T166), a built-in GBP default (CRM-T168), and UTC "today" with no fiscal periods (CRM-T171). Wording only: `regulatory.md`, `index.md`, the email-provider brief, the DPIA and its template now name their regimes and say the rest are not yet read; `llms.txt` and `llms.json` list the new file. **Not done or verified:** no behaviour changed; no law of any region was read; nothing was run; the identity registries, the role briefs, the design-system packages and the wording of each translation were not audited.

### Fixed — `compose.test.yaml` names commands that exist (maintenance, 2026-10-30)

The file and `scripts/ci-check.sh` pointed at a `scripts/test-db.sh` that was never added. They now give `podman compose -f compose.test.yaml up -d --wait`, `scripts/ci-check.sh test-db` and `down`, and say that a local PostgreSQL without the `loco` role works with `PG_USER=postgres` (the stage already reads `PG_*` and makes its own `ci_*` database). No script was added: only these comments named it, the README and INSTALL.md already use `podman compose`, and the stage already does what the script would. Checked: `up -d --wait` on ports 5433 and 6380, then `PG_PORT=5433 REDIS_PORT=6380 scripts/ci-check.sh test-db` (208 passed, 3 webhook tests failed; those 8 webhook tests pass against a local PostgreSQL, so likely the podman VM's clock, not investigated further), and `PG_USER=postgres REDIS_PORT=6380 scripts/ci-check.sh test-db` on a local PostgreSQL 18.4 (all 218 passed). **Not done:** the stage leaves its `ci_*` database behind, as before; nothing was run on Linux or with Docker.

### Fixed — the UI's default `CRM_API_URL` reaches the service (maintenance, 2026-10-30)

The default and `.env.example` are now `http://localhost:5150/api`, the base the BFF proxy needs, so a default local setup reaches the service; a trailing slash is dropped and no `/api` is added to a value without it. A unit test pins the default and the upstream URL; INSTALL.md and the UI README say so. **Not verified:** the proxy against a running service.

### Changed — llms.txt and llms.json list every current document (maintenance, 2026-10-30)

Regenerated with `scripts/build-llms.py`, whose table now adds the bridges tutorial, the matcher, the service and UI index and stack-spec pages, the translation register, the DPIA, inventory, record of processing, risk management, GDS review and DTAC pages, NEWS, RFC and MAINTAINERS, and gives the current id ranges (CRM-R1 to R94, CRM-T0 to T163, CRM-D1 to D18). Left out on purpose: the per-locale role briefs (reached through `spec/locale/index.md`), the DTAC folder's README, and the AI-statement template under `spec/special-files-for-public-repos/`. There is no `*-skill` folder and no `*.github.io` folder for this repository, and none was made.

### Changed — the teams import asks the worker service for three fields only (CRM-T163, CRM-R73, CRM-D17)

WPM's worker list takes `fields` since WPM-R216, so `POST /api/bridges/wpm/teams-import` now asks for `fields=pid,department,person_ref` (paging and `status=active` unchanged) and no longer receives workers' names, jobs or pay. A WPM from before WPM-R216 answers `422`; the import then fails with `502` and changes no team, and it never falls back to the whole record. The DPIA, record of processing, integrations table and tutorial say so.

**Not done or not verified:** not run against a running WPM (the stand-in projects `fields` as WPM's source does).

### Added — a tutorial for the family bridges; the docs checked against the code (maintenance, 2026-10-30)

`docs/tutorial-family-bridges.md`: the four bridges, their environment variables, a signed WPM hire and leave sent with `openssl` and `curl`, the ordering of events, and the employment check. INSTALL.md has a "Family bridges" section (the two WPM settings were in the email table), the service README's route list is rebuilt from `app.rs` and the controllers and its quick start creates the database first, and the UI README's views, environment and test tables match `src/routes`, the config and the scripts; stale test counts were removed rather than recounted.

**Not done or not verified:** the tutorial's `curl` calls were not run, and nothing was run against a running WPM or PPM (the `openssl` signature was checked against the RFC 4231 vector the unit tests use). Found and not fixed: the UI's `CRM_API_URL` default (`http://localhost:5150`, also in `.env.example`) lacks the `/api` the proxy needs, so the docs say to set it; `compose.test.yaml` mentions a `scripts/test-db.sh` that does not exist.

### Fixed — the agent guides match the code (maintenance, 2026-10-30)

The service's `AGENTS.md` linked `../../agents/share/rust-loco-stack.md`, which is not in this repository (the family notes are in `main-x-service`); it now says where they are. It names the bridge rule modules, the `worker:` alias and how to run the database tests; the UI's names its test folders and ports and drops the T17 to T20 wording. Sizes: service 4,165 bytes, UI 3,749 bytes. There is no root `AGENTS.md` or `CLAUDE.md`, and none was added. The code comments that cite `agents/share/*.md` were not changed.

### Fixed — the specification matches the code after CRM-T158 to CRM-T162 (maintenance, 2026-10-30)

Checked against the code with grep and reads, not by running anything: reps and agents are `person:<sub>` users, not worker-service records (index, integrations, scope, the inventory and record of processing); the WPM bridges, their audit rows and the `contact_employments`, `employment_last_events` and `wpm_events` tables are in the architecture, audit and domain model; the service spec's layout (54 migrations, the rules modules, the two background workers) and the UI spec's delivery state are current; SvelteKit 3 and 8 locales; CRM-R73's page limit is 51, not 100. Every CRM-R, CRM-D and CRM-T id is defined once (R1 to R94, D1 to D18, T0 to T162).

**Found not built, and now said so:** the stalled-deal flag of CRM-R4 (CRM-T6 was marked done with it) and the config-tunable scoring weights of CRM-R3 (only the defaults are used). Whether to build them or drop the claims is not decided. Not audited: `plan.md` (a dated 2026-10-10 assessment); requirement titles that still call sellers "workers" were left as they are.

### Changed — dependencies updated (maintenance, 2026-10-30)

Rust: `cargo update` in the service (aws-lc-rs 1.19, jsonschema 0.60.2, wasm-bindgen 0.2.130 and other patch releases; the matcher's lockfile was already current) and the service's minimum versions raised to what is locked (tokio 1.54, uuid 1.29, entity-ref 0.2.1, redis 1.8.0). loco-rs stays 1.2.0 (the latest), so sea-orm stays on 2.0. UI: `pnpm update` (svelte 5.57.3, typescript-eslint 8.72.1, @types/node 26.6.6) and two exact-pinned Lily packages moved up (svelte-locale-translations 0.4.0, svelte-picker-bar 0.5.1). Actions: every pin was already the newest release at least a week old (the Dependabot cooldown); the rust-cache and install-action comments now name the exact release (v2.9.3, v2.87.30) instead of `v2`.

**Held back:** TypeScript 7.0 (`svelte-kit sync` in SvelteKit 3.1 fails to load the tsconfig with it, and openapi-typescript 7.13 asks for TypeScript 5); install-action v2.87.31 and later (out less than a week); axum-test 22 (release candidate only); the matcher dependency stays 0.1.2 (the latest on crates.io; the 0.2.0 in this repository is unpublished). The Rust toolchain (1.99.1), the pnpm used by CI and the images (11.0.8) and the Woodpecker images were not changed. **Gates run locally:** fmt, clippy, unit tests, the database and Redis request tests on a throwaway database, supply-chain (audit, deny, SBOMs written to the ignored `sbom/`), UI check, vitest, build, lint and Playwright. Not run: the image builds.

### Fixed — a CRM user is a `person:` reference, so imported teams meet their quotas (CRM-T162, CRM-D18, CRM-R59, CRM-R67, CRM-R73)

The family's sign-in subject is a person id: WPM matches it as `person:<sub>` and PPM addresses it so (checked in both apps' code and specs; CRM-D18 lists where). The CRM wrote it as `worker:<sub>`, so after CRM-T161 an imported team's `person:` members met no quota. Every field that names a CRM user (owners, assignees, the actor, notification recipients, quota, split and clawback holders, team members, workflow assignees and pools, `set_by`, a view's or report's owner) is now `person:<id>`, and the signed-in caller is `person:<sub>`. **`worker:<id>` is still accepted on input** and stored and answered as `person:<id>` (the same id), for a deprecation period with no end date yet. **Migration 54** rewrites stored `worker:<uuid>` user references, leaving alone teams last set by the WPM import (their members were WPM worker ids), history and WPM's event subjects; `updated_at` is kept. The `{{rep_name}}` template value now comes from the person service.

**Not done or not verified:** not run against a running WPM or with real sign-ins (the request tests use a stand-in WPM with sign-in off; the migration was run against a populated throwaway database by `scripts/migrate-person-refs.sh`, not a production-sized one). No `Deprecation` header is sent for `worker:` input, and when the alias goes is not decided. A team last set by an import before CRM-T161 keeps its `worker:` members until the next import. History (audit log, outbox, workflow run logs) still says `worker:`. UI and locale prose still calls sellers "workers". WPM's spec still records the mismatch as open.

### Fixed — imported teams name their members by `person:` reference (CRM-T161, CRM-R73, CRM-D17)

`POST /api/bridges/wpm/teams-import` now makes each team's members the workers' `person_ref` from WPM's list (validated as a `person:` URN), the reference contacts, employments and WPM's events use, instead of `worker:<pid>`. A worker without a valid one joins no team and is counted in the new `without_person_ref`; it never falls back to `worker:`. Teams an earlier import set with `worker:` members are replaced by the next import that names them; nothing is migrated. The import still keeps only `department` and `person_ref` (and reads `pid` to recognise a row).

**Not done or not verified:** not run against a running WPM (tested against a stand-in; WPM's `person_ref` was read from its source on 2026-10-30). Quotas are owned by `worker:` references, so an imported team's roll-up (CRM-R67) now matches no quota and lists its members in `without_quota`; no person-to-worker mapping is built, and `PUT /api/quotas/teams` still takes `worker:` members only, so an imported team cannot be re-saved by hand. Erasing a contact does not remove its `person_ref` from an imported team. WPM answers the import with whole worker records (pay included unless its policy masks it) and does not audit that list read: recorded for WPM's owner, WPM not changed, and WPM's own spec still calls the mismatch open. The `/quotas` page does not show `without_person_ref`.

### Added — the worker service is asked which contacts are its employees (CRM-T160, CRM-R94)

`POST /api/bridges/wpm/employment-check` (admin only, stub-first) sends WPM's overlap check (WPM-R208) the `person:` references of the live contacts, at most 500 a call and never a name, for `CRM_WPM_ORGANIZATION_REF` under `CRM_WORKER_SERVICE_TOKEN`; each yes marks the contacts employees as a hire event would (audited with `via: wpm`), unless the last event applied was a leave; a no changes nothing. A button on `/quotas`. CRM-D16 records why one HR token and one configured organization.

**Not done or not verified:** not run against a running WPM (tested against a stand-in); the token is not the caller's own, as WPM-D135 asks; no schedule; one organization; a no never ends a pause, by design.

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
