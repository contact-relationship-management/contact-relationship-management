# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — every data table is the Lily data grid (2026-10-13)

All 28 data tables now use `@lilydesignsystem/svelte-data-grid` through one
wrapper, `src/lib/components/DataTable.svelte` (sortable columns, a filter box
on lists of 8 or more rows, paging when asked, APG grid keyboard support, and
translated labels in all eight languages). The four SVAR grids (contacts,
accounts, leads, tickets) and their filter bars are gone, and so are the
`@svar-ui/svelte-grid` and `@svar-ui/svelte-filter` dependencies. The Lily grid
shows text only, so tables that had buttons or inputs in their rows (articles,
campaigns, break-glass review) now select a row and act in a panel below it,
as tickets and leads already did. The power–interest matrix on Partners stays a
plain table: it is a two-dimensional heat map, not a data table. The SVAR
Kanban boards and Calendar are unchanged. The contact page's timeline, deals and
tickets tables gained column headers.

### Changed — signed-in headline (2026-10-13)

The signed-in dashboard's headline is now "My Home Page" (translated in all
eight languages). Tests check that a signed-in header offers Sign out and no
Sign in.

### Changed — six dashboard tiles (2026-10-13)

The dashboard now shows six tiles: win rate, open deals, open tickets, breached
tickets (links to Tickets), overdue follow-ups (links to Follow-ups) and the
forecast. The two new ones come from data the dashboard's calls already carry,
plus one read of `/insights/followups`. Also fixed: `main` shrank to its content
width on every page (auto margins in the layout's flex column); it now fills the
page up to 1280 px, which is what lets the tiles sit in one row.

### Added — guided tour with fictitious data (2026-10-13)

Sign in as `example@example.com` (no email is sent) to explore every screen on
invented data: 12 contacts in 6 organisations, 12 deals, 7 leads, 4 campaigns,
8 tickets, 5 articles, the insight views, and break-glass. The BFF proxy
answers a demo session from `src/lib/server/demo.ts` and never forwards it to
the service. Writes (moving a deal, resolving a ticket, recording consent,
erasing a contact with the same refusals as the service) change an in-memory
copy that every demo visitor shares and that signing in to the tour again
resets. A banner marks the session. `CRM_DEMO_TOUR=off` disables it. Tests: a
unit test that every read in `crm.ts` has sample data, and end-to-end tests that
walk all 18 screens.

### Fixed — Sign out returned 403 (2026-10-13)

`Referrer-Policy: no-referrer` made Chrome send `Origin: null` on the plain
Sign out form, which SvelteKit's CSRF check refuses. The policy is now
`same-origin` (nothing is sent to other sites either way).

### Known — accessibility findings on data-filled screens

The tour screens, scanned with data for the first time, report WCAG A/AA
findings (colour contrast on boards and the follow-up list, an unnamed select
and buttons, ARIA attribute values in the grids). The scan in
`tests/e2e/tour.spec.ts` lists them as annotations without failing the build;
they belong to the manual audit (CRM-T48).

### Changed — Lily design system upgrade; pages and search pickers (2026-10-13)

`@lilydesignsystem` packages moved to their latest (picker-bar 0.3.0, themes
0.4.0, the theme, locale, text-size and share pickers 0.2.0, headless 0.4.0).
The header picker bar gains a **link picker** (home icon: the primary
navigation, or Dashboard and Accessibility when signed out) and a **search
picker** (a search for `ada` goes to `/contacts?ada`, which filters the contact
list by name, job title or status; `?q=ada` works too). New strings in all
eight languages (`nav.pages`, `search.*`). The upgraded theme CSS places popups
itself with anchor positioning, so the layout's own popup placement rules were
removed. New end-to-end test for both pickers.

### Added — accessibility statement (CRM-T45, 2026-10-10)

Public `/accessibility` page linked from the footer; it does not claim WCAG
compliance. Break-glass request and review pages, idle sign-out, security
headers and CSP landed earlier in the same release.

### Changed — PickerBar now uses Lily's default themes and text sizes, not app-specific lists

`spec/lily-design-system-svelte-with-picker-bar/index.md` calls for
"all Lily default themes (not any application-specific custom
themes)" and "all Lily default text sizes." Removed the hand-maintained
`THEMES` array (the same 45 slugs as Lily's own `DEFAULT_THEMES`, but
in a different order — `valentine`/`winter`/`wireframe` sat after the
UK/US government group instead of before it) and the `SIZES`/
`SIZE_LABELS` (a custom 4-step `small`/`medium`/`large`/`x-large`
scale — a real narrowing of Lily's 7-step scale) from
`src/routes/+layout.svelte`. `PickerBar` now gets no `themes`/`sizes`
prop at all, so it uses its own `DEFAULT_THEMES` (45, alphabetical,
UK/US government themes grouped at the end) and `DEFAULT_SIZES` (the
7-step `largest`…`smallest` scale) directly. Existing `storageKey`s
(`mxi.crm.theme`, `mxi.crm.text-size`) are unchanged.

`src/app.css`'s `[data-text-size]` rules rewritten for the new 7-step
scale (was 3 rules for `small`/`large`/`x-large`; `medium` unscaled —
now 6 rules for `smallest`/`smaller`/`small`/`large`/`larger`/`largest`;
`normal` unscaled).

### Changed — chrome consolidated onto Lily `PickerBar`; locale picker restored

The top-of-page chrome (theme, text-size, share pickers) previously
wired `ThemePicker`/`TextSizePicker`/`SharePicker` individually in
`+layout.svelte`. Replaced with the single Lily
`lily-design-system-svelte-picker-bar` component, which composes all
four pickers — theme, locale, text-size, share — as one row.

This restores a locale picker to the chrome (there was none before):
`PickerBar` bundles all four unconditionally with no way to omit one.
Wired with `applyDir={false}` and `onChange={(code) => i18n.set(code)}`,
since this app's own `i18n.svelte.ts` store already reflects `lang`/`dir`
onto `<html>`. Existing `storageKey`s (`mxi.crm.theme`,
`mxi.crm.text-size`) are preserved via `themeProps`/`textSizeProps` so
an operator's saved preference survives the change.

`SHARE_TARGETS` gained an `email` (`mailto:`) target and now orders
Email / LinkedIn / Reddit / Bluesky / Mastodon; the Mastodon target
now points at `mastodonshare.com` instead of `mastodon.social/share`
(a generic sharer rather than one specific instance).

New dependencies: `lily-design-system-svelte-locale-picker`,
`lily-design-system-svelte-picker-bar` (both `file:` deps on the
sibling Lily checkout, same pattern as the existing four).

Also added `en_US` ("English (United States)") to `src/lib/i18n.svelte.ts`'s
`LOCALES`/`LOCALE_LABELS`/`STRINGS` (a duplicate of `en`'s copy) and
taught `normaliseLocale` to match a region variant like `en_US`/`en-US`
exactly before falling back to primary-subtag stripping, so it no
longer silently collapses to `en`. `document.documentElement.lang` is
now set from `i18n.locale.replace("_", "-")` so `en_US` renders a valid
BCP 47 `lang="en-US"`, matching what `LocalePicker` itself writes.
`tests/unit/crm.test.ts` gained a test pinning `en_US`/`en-US`
normalisation; the existing locale-parity test already covers the new
`en_US` catalog entry since it loops over `LOCALES`.

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (CRM-T24)

`src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch,
token)` with no `try`/`catch`. A network-level failure (the
authentication service unreachable, timed out, connection reset) makes
`fetch` throw rather than resolve — uncaught, that propagated out of
`load` and SvelteKit rendered its generic 500 error page instead of
this route's own friendly UI. The same bug class was found and fixed
first in `place-front-end-with-svelte` (T-26) and
`thing-front-end-with-svelte` (T-23); ported here: a `try`/`catch`
around the call, a new `"serviceUnavailable"` error variant, and its
message in `+page.svelte`. New `tests/unit/verify.test.ts` unit-tests
the `load` function directly (missing token / service unavailable /
invalid token), verified to fail with the `try`/`catch` reverted and
pass with it restored. See spec §13 CRM-T24.

### Fixed — the follow-ups calendar never showed an upcoming event (CRM-T28)

`@svar-ui/calendar-store` requires an all-day event's `end` to be
strictly after `start`, but `/followups`'s `+page.svelte` passed
`end: day` — the same `Date` object as `start` — so the SVAR Calendar
widget silently dropped every upcoming follow-up it was ever asked to
show. The existing test only asserted the calendar's wrapper was
visible, not that an event actually rendered inside it. The identical
bug was already found and fixed in worker-front-end's/person-front-end's
`/expiry` calendars and ppm-front-end's `/calendar`. Fixed by computing
the following calendar day as `end`; strengthened the test to assert
the calendar renders the event text, with the stubbed due date
computed relative to the actual test-run date. See `spec/tasks.md`
CRM-T28.

### Added — root sign-in gate (CRM-T26)

No `+layout.server.ts` existed anywhere under `src/routes`, so a
visitor with no session reached every page — the deal board, ticket
queue, executive dashboard, all of them — and only discovered they
were signed out once an API call silently failed through the BFF
proxy. Ported workforce-planning-management-front-end's identical
WPM-T38 fix (same underlying architecture): new root
`src/routes/+layout.server.ts` redirects to `/signin` (303) when
`locals.sessionId` is `null`, excluding `/signin`/`/verify`. `tests/e2e/smoke.spec.ts`
gained a `signIn()` helper (injects a fake `__Host-mxi_session` cookie
via Playwright) and a `"sign-in gate (CRM-T26)"` describe; all 12
pre-existing tests moved under a `"signed-in smoke coverage"` describe
whose `beforeEach` now signs in first, verified green (15 Playwright +
5 vitest tests pass). See `../spec/tasks.md` CRM-T26.

### Added — engagement + partners areas (CRM-T20, 2026-07-20)

- `/engagement` (cadence aging, workload with recorded sentiment,
  member health with the silent list) and `/partners` (stakeholder
  register + power–interest grid, partnership register, membership
  renewals). The deal board gains a pipeline selector + the honest
  funnel strip; follow-ups gains a kind filter; DPO gains
  consent-by-account. Nine client functions with path pins; three new
  Playwright specs + extended stubs.

### Added — boards + insight areas (CRM-T19, 2026-07-20)

- `/leads/board` and `/tickets/board` (SVAR Kanban; drag = the
  existing status transitions, SLA/score badges), `/followups`
  (overdue aging + SVAR Calendar), `/executive` (period pack +
  stale deals + hygiene findings + forecast trend), `/dpo` (consent
  coverage + sources + duplicate hygiene). `leadStatus` + seven
  insight client functions with path pins; nav + keys in 13 locales;
  five stubbed Playwright specs.

### Added

- 2026-07-19 — SVAR strong fit: the **/deals** board upgrades from custom CSS columns to the SVAR
  Kanban: columns are the pipeline's stage rows (probability
  labelled), drag = the stage-move API (a lost target carries a
  reason), and the forecast strip still re-reads the derived number
  after every move.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: SVAR index routes: **/contacts** and **/leads** and **/tickets**
  upgrade from plain tables to the SVAR DataGrid with FilterBars
  (the lead score breakdown and ticket status actions move to a
  selection panel under each grid; the breach banner keeps its
  testid), and a new **/accounts** index route (name / tier /
  industry) joins the nav.

- 2026-07-19 — Lily Design System: the Lily Design System lands in the chrome: the hand-rolled
  locale `<select>` is replaced by **LocaleSelect** (wired to the
  i18n store; `applyDir` off), a **ThemeSelect** offers the full
  45-theme catalogue (stylesheets via the `static/assets/themes`
  symlink; choice persisted to `mxi.crm.theme`), and the **Lily
  headless** component library is a dependency.

- 2026-07-18 — CRM-T17/T18 implementation round: SvelteKit 2 +
  Svelte 5 runes SPA with same-origin BFF proxy, 13-locale i18n
  (45 keys, parity-tested, RTL ar/ur), typed API client + honest
  `money()`, and the module views (KPI dashboard, contacts +
  consent + timeline, lead queue + score breakdown, deal board +
  forecast, campaigns + funnel/ROI, tickets + breach flags, KB).
  svelte-check clean; 5 vitest + 4 Playwright tests green.

- 2026-07-18 — CRM-T0 specification round: cross-cutting spec
  (`../spec/`) and this edition's doc scaffold. No code yet; this
  edition is CRM-T17/T18 in the queue.
