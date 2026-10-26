# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — attribution page (CRM-T113)

`/attribution`: choose the model, the window and the dates; each campaign's credit per currency with a bar, and the deals no campaign touched apart. 20 keys in 8 locales (machine drafts), tour data and a Playwright test.

### Changed — Lily Design System and tooling

`@lilydesignsystem/svelte-headless` 0.6.1, `svelte-locale-picker` 0.3.1, `svelte-locale-translations` 0.3.0, `themes` 0.6.0 and `vite` 8.3.5. The new themes draw the sort glyph (⇅ ▲ ▼) after a sortable header as generated content, which put it into the header's accessible name ("Due ⇅"); `app.css` now gives it empty alternative text, so it still shows and is no longer read out. TypeScript stays on 6 (7.0.2 breaks `svelte-check`).

### Added — ESLint (CRM-T94)

`eslint.config.js` (recommended rules for JavaScript, TypeScript and Svelte); `pnpm lint` runs it after prettier. Fixed what it found: unused imports, a `let` that never changes, and `{#each}` blocks without a key on the splash and tour pages.

### Added — reports (CRM-R48)

`/reports`: build, run, save and reopen reports; a bar for each row.

### Added — tags (CRM-R47)

A tags panel on the contact, account, lead and deal pages, and a tag selector on the contact, account and lead lists.

### Fixed — the screens fit a phone (CRM-T110, CRM-R46)

At 360 px every screen scrolled sideways: the header's seven buttons ran past the edge (14 px on every page), and forms were wider than their panel (up to 49 px). The header now wraps below 40 rem, and a field takes at most its container's width. A Playwright test checks 32 screens at 360 by 740.

### Added — lead assignment (CRM-R44)

The workflows page offers the `lead_created` trigger, the lead source condition and the assign action; the lead page shows its owner.

### Added — contact and account import (CRM-R43)

`/contacts/import` and `/accounts/import`, linked from the lists.

### Added — attachments (CRM-R42)

A files panel on the contact, account, lead and deal pages (upload, download, delete). The BFF proxy now forwards `Content-Disposition` and `X-Content-Type-Options`.

### Added — custom fields (CRM-R41)

`/custom-fields` to declare them; a form of the fields on the contact, account, lead and deal pages.

### Added — workflow rules page, contact and account editing (CRM-R38 to CRM-R40)

`/workflows`: list, editor, dry run and run log. The contact and account pages have an edit form (names, job title, tier, industry).

### Added — component tests, and the API types are checked against the service (CRM-T93)

Tests for `DataTable`, `Board` and `LinksPanel`. The vitest `deps.inline` pattern never matched the Lily package it
was meant for; it now does. `src/lib/api/schema.d.ts` is generated from the service's OpenAPI document
(`pnpm api:types`), and `svelte-check` fails when a hand-written type in `crm.ts` expects something the document does
not promise. It found one: a stale deal's amount can be `null` when masked, which the type did not say.

### Changed — page data goes through one helper, and a slow answer cannot replace a newer one (CRM-T90)

Pages fetched in `$effect` blocks that each kept their own error handling, and a slow answer for a previous
record could overwrite the current one. `resource()` (`src/lib/resource.svelte.ts`) now loads each page's data,
drops any answer but the latest, and clears the previous record when the parameters change. `errorMessage()`
replaces 39 copies of the same error-to-text line. Visible changes: a failed load clears on the next try; the
articles search runs when the box is committed, not on each keystroke; the dashboard, DPO, engagement and partners
pages fetch their parts in parallel.

### Changed — one language is downloaded, not eight (CRM-T89)

The interface strings were one 3,300-line table (175 KB, 50 KB compressed) that every visitor downloaded for all
eight languages. Each language is now its own file under `src/lib/locales/`, and only English (19 KB, 7 KB
compressed) is bundled; the language a visitor chooses, or chose last time, is fetched when it is needed (17 to 34 KB,
7 to 8 KB compressed). While it loads the page shows English, then switches without a reload. Each locale file
is typed against the English keys, so a missing or extra key is a type error as well as a test failure.
`scripts/identical-to-english.mjs` lists the strings that match English, for native reviewers.

### Changed — each tour visitor has their own copy of the data (CRM-T92)

Signing in as `example@example.com` now starts a new demo session with a random id and its own copy of the
fictitious data, so one visitor no longer sees what another typed or moved. Before, every visitor shared one copy
under one fixed session id, and each new sign-in reset it for everyone. At most 200 copies are kept in memory; the
least recently used goes first, and that visitor starts again from the original data. Sign-out forgets the copy.
The tour is still on unless `CRM_DEMO_TOUR=off`.

### Fixed — sign-in when the authentication service is down

A magic-link request to an unreachable authentication service threw, so the sign-in page rendered blank instead
of saying the request failed; it now says it failed. Sign-out is best-effort as documented: an unreachable service
no longer stops it. Found because the e2e test `another address is not the tour` failed locally (Playwright does
not run in CI; see CRM-T94).

### Changed — the tour's account pages list deals

In the demo data an account's deals are its contacts' deals, so `/accounts/{pid}` shows them as links to
the deal page (before, the list was always empty). Demo data only; the real service already returned them.

## [0.1.5] — 2026-10-21

### Changed — the UI image is built for amd64 and arm64 and published as one image (CRM-T74)

### Added — a deal can be edited and reopened on its own page (2026-10-21)

`/deals/{pid}` has an Edit button: name, amount and expected close, saved with `PUT /deals/{pid}`.
Only what changed is sent. A masked amount stays disabled, and a closed deal says it cannot be
edited instead of showing the form. Five new strings (`deal.edit`, `deal.save`, `deal.cancel`,
`deal.saved`, `deal.closedLocked`) in all eight locales.
A closed deal offers Reopen: a reason is required, it calls `POST /deals/{pid}/reopen` (back to the
pipeline's last open stage), then re-reads the deal and shows the edit form. Four more strings
(`deal.reopen`, `deal.reopenReason`, `deal.reopenConfirm`, `deal.reopened`). The live follow-up test now picks a random
day, because the calendar shows three events a day and earlier runs left tasks on the same one.

## [0.1.4] — 2026-10-21

### Added — links to external systems on account, deal and lead pages (CRM-R27, 2026-10-20)

The links panel is now a shared component (`LinksPanel`) used by the contact page and by new
record pages at `/accounts/{pid}`, `/deals/{pid}` and `/leads/{pid}`. The account and lead
tables link names to those pages; the deals page lists its deals under the board for the
same purpose. No new strings.

### Fixed — live tests no longer assume data

The follow-up test makes its own contact, and the deal-amount test selects its own pipeline.
Both failed on a database that was empty or held other pipelines.

## [0.1.3] — 2026-10-20

### Added — undo a merge from the survivor's page (CRM-R26, 2026-10-20)

A "Recently merged" list on a contact page shows the contacts merged into it, each with
Undo. Undo previews first (what goes back, what stays because it changed since, what is
restored, which fields are cleared, and the returning contact's consent), then needs a
reason. Thirteen strings in all eight languages (unreviewed by native speakers).

### Changed — Playwright's preview server moves to port 4373

Port 4173 is Vite's default, and another project's preview server on it made the first run
fail with every test timing out (Playwright reuses a server it finds there). The port is
now `4373`.

### Added — links to other systems on the contact page (CRM-R27, 2026-10-20)

A "Linked records" panel lists a contact's links to HubSpot, Salesforce, Attio, Pipedrive,
Microsoft Dynamics or elsewhere, each opening in a new tab (`noopener noreferrer`), with a
form to add one (system, address, optional label) and a Remove button. The service refuses
an address that is not `https` or is on the wrong domain, and the page shows its reason.
Merge previews now count moved links. Eleven strings in all eight languages (unreviewed by
native speakers); the tour has a sample HubSpot link. Contacts only: accounts, deals and
leads have no detail page.

### Changed — Lily Design System packages (2026-10-20)

`svelte-calendar` 0.6.0 to 0.7.0, `svelte-data-grid` 0.10.0 to 0.11.0,
`svelte-gantt-chart` 0.7.0 to 0.8.0, `svelte-headless` 0.5.0 to 0.5.1. Type check,
104 unit tests and 93 end-to-end tests pass unchanged. TypeScript 7 is available
and not adopted (a major version; svelte-check support not checked).

### Added — merge a duplicate into this contact (CRM-R24, 2026-10-19)

Each likely duplicate on a contact page has a Merge action. It asks the service
for a dry run, shows what will move and what marketing consent becomes (the
stricter of the two), requires a reason, and merges only on confirmation. The
duplicates hint now says nothing changes until you choose Merge. Labels in all 8
locales (not reviewed by native speakers). Needs a service with `POST
/api/contacts/{pid}/merge`.

### Changed — kanban board 0.8.1 (2026-10-18)

A card's title now edits in place on the lead, ticket and deal boards (double-click
it, or F2; Enter saves, Escape cancels), replacing the "Edit" pseudo-field. A
drop into another swimlane is refused outright with the board's own "can't drop
here" cue (`allowLaneChange={false}`), and the search field has a visible
label. Not adopted: the compact field layout (needs CSS).

### Changed — Lily label packs (2026-10-18)

Adds `@lilydesignsystem/svelte-locale-translations` 0.1.0 (it was first added as `locale-translations`). Tables, boards, the
follow-up calendar and the campaign timeline now take their generic wording
(export, copy, search, undo and the announcements) from the pack for the
current language, through `src/lib/lily-labels.ts`; the app's own labels still
win where it sets one. The 11 strings I had written by hand for copy, undo and
search were removed from `i18n.svelte.ts`. `de-de` uses the `de-001` pack and
`zh-cn` uses `zh-001`. The packs other than English are machine drafts that no
native speaker has read. Vitest bundles the package (`vite.config.ts`),
because its `dist` imports without file extensions, which Node cannot load.

### Added — site-wide search, a language in any address (2026-10-18)

The header search now goes to `/search`, which looks in contacts, accounts,
leads, deals, tickets, campaigns and the knowledge base (every word must
appear, any order, ignoring case and accents) and lists hits by group. It
filters in the browser from the same reads the screens use, so masking and
access rules apply; a list that fails to load is reported and the rest still
show. Any page now opens under a language in the address: `/de-de/contacts`
shows the contacts in German, the page links keep the prefix, and choosing
another language rewrites the address (`src/hooks.ts` removes the prefix before
routing; `/<locale>/kanban` stays its own route). The prefix does not change
which pages need sign-in. The search box text and placeholder changed in all
eight locales (unreviewed by native speakers).

### Added — campaign timeline export, live tests (2026-10-18)

The campaign timeline exports CSV, TSV and Excel and has a Copy button. `pnpm
test:live` (`playwright.live.config.ts`, `tests/live/`) drives the boards and
the follow-up calendar through the UI against a running service on port 5160
and reads each change back from the service: a ticket rename, a deal amount, a
card moved within its column with Alt+arrow, and a follow-up rescheduled with
Alt+arrow. It creates records and leaves them, so use a development database.
A mouse drag of a ticket card above another in its column is covered too (the card's title is the drag handle, so the drag must start there; a drag from elsewhere on the card does nothing). Five live tests.

### Changed — Lily upgrade, TSV and copy (2026-10-18)

Calendar 0.6.0, data grid 0.10.0, gantt chart 0.7.0 and kanban board 0.7.0.
Tables, the boards and the follow-up calendar now export TSV as well as CSV and
Excel (CRM-R21, CRM-T56), and have a Copy button that puts the same rows on the
clipboard for pasting into a spreadsheet. New labels in all eight locales
(unreviewed by native speakers). On the boards, Ctrl/Cmd+Z takes back
a move or an in-place edit (Ctrl+Shift+Z redoes it) and an Undo button shows;
the follow-up calendar has a search field and tints weekends; the boards and
tables get Lily's type-ahead and result announcements. Not switched on: the
campaign timeline's weekends (its columns are weeks), holiday ranges (the app
holds no holiday data) and undo in the tables (they are not editable).

### Added — edit in place, stored order, swimlanes, reschedule, critical path (2026-10-17)

Needs the service's new endpoints (CRM-T55). On the deal board an amount and an
expected-close date edit in place (click, or F2); on the lead and ticket boards a
card can be renamed the same way. Cards keep the place they are dropped in, also
after a reload (Alt+Up/Down moves one from the keyboard). Each board has swimlanes
(deals by owner, leads by source, tickets by priority; lanes only group, so
dropping a card into another lane is refused), and Ctrl/Cmd+click or `x` selects
several cards to move together. On `/followups`, dragging an event to another day
(or Alt+arrow on it) reschedules the follow-up. Data tables have typed number and
date cells, optional "shown" totals, a pinned first column, and columns you can
drag or move from the keyboard (remembered per table). The campaign timeline now
works out the critical path (`src/lib/critical-path.ts`, 8 unit tests): the
campaign that ends last, with every other campaign's slack in days; the code also
handles dependencies of all four kinds with lags, though campaigns have none.
The tour's end-to-end tests now run one after another, because they share one
in-memory copy of the data. Also fixed: two demo leads had a status ("working")
that no board column showed.

### Changed — every board on the Lily kanban; a campaign timeline (2026-10-17)

The deal board, the lead board, the ticket board and `/<locale>/kanban` now share
one `Board` component on `@lilydesignsystem/svelte-kanban-board` 0.6.0, and the
SVAR Kanban and Gantt packages are removed. Cards move by drag or by the
keyboard-reachable menu; the move is saved, shown at once, and put back with the
reason if the service refuses it. The lead and ticket boards offer only the moves
the service's lifecycle allows, and a finished lead or ticket is locked. New:
search, collapsible columns, an empty-column message, and CSV and Excel export.
`/campaigns` gains a read-only timeline on `@lilydesignsystem/svelte-gantt-chart`:
a bar runs from creation to the last status change (to today while the campaign is
open). The service stores no planned send dates, so it shows what happened, not a
plan. The follow-ups calendar gains day and agenda views, week numbers, "+n more"
on crowded days, day numbers in the cells, a today outline that keeps events
readable, and CSV and Excel export; every data table gets CSV and Excel export.
The unused SVAR file-manager package is removed. New keys `kanban.*`, `timeline.*`, `cal.*` and `grid.export*` in all eight languages (unreviewed by
native speakers).

### Changed — Lily upgrade (2026-10-14)

`svelte-calendar` 0.3.0, `svelte-data-grid` 0.5.0, `svelte-headless` 0.5.0 and
`svelte-kanban-board` 0.3.0; all unit and end-to-end tests pass unchanged. The
kanban board now has subtitles, badges, flags, links, locked cards, async moves,
`canMove`, WIP limits, search and add-card, which covers what the SVAR boards
are used for; moving the three SVAR boards over is not done yet.

### Changed — follow-ups calendar on `@lilydesignsystem/svelte-calendar` 0.2.0 (2026-10-14)

`/followups` now uses the Lily `CalendarView` (week, four-week and month views,
keyboard navigation, a live announcement of each new period) in place of the
SVAR Calendar, which is removed from the dependencies. Weekday and day names
come from `Intl` in the UI language; the toolbar labels are new `cal.*` keys in
all eight languages (unreviewed by native speakers). The package ships no CSS,
so the layout is in `app.css`. The SVAR kanban boards and the deal board's
pipeline selector remain the open accessibility findings.

### Changed — better data tables, on data-grid 0.4.0 (2026-10-13)

`@lilydesignsystem/svelte-data-grid` is now 0.4.0. The shared `DataTable`
wrapper uses what it added:

- **Rows open their record.** Click a row, or press Enter on it, instead of
  ticking a radio button; the selection column is gone and the selected row is
  highlighted. Contact names are real links too.
- **Rich cells.** Status, priority and consent show as chips, a breach is a
  flagged label, numbers and amounts are right-aligned with tabular figures.
- **Better sorting.** Text sorts with the UI language's collation, and digit
  runs compare as numbers ("10" after "9"). Sort and column widths are
  remembered per table.
- **Row headers.** The first column is the row header for assistive technology.
- **Empty tables keep their headers** and say why, inside the grid.
- **Long lists page** at 25 rows, with the page announced to screen readers.
- The grid scrolls sideways on its own region on narrow screens.

Also fixed: the "Board" links on Tickets and Leads had invisible text (the Lily
theme's `.button` colours matched the background), and the breach marker no
longer relies on a pink that failed contrast. Every page that has a data table
now scans clean for WCAG A/AA; what remains in the tour scan is the SVAR
kanban boards, the follow-ups calendar and the deal board's pipeline selector.

### Added — `/<locale>/kanban` (2026-10-13)

`/en-001/kanban` (or any of the eight UI languages) shows the signed-in user's
deal board as a Lily kanban board (`@lilydesignsystem/svelte-kanban-board`): a
column per pipeline stage, a card per deal. A card moves by pointer drag or the
keyboard-reachable "Move to column" menu; the new stage is saved with the same
call the deal board uses, and a refused move is undone and explained. The
language in the address wins (`/de-de/kanban` is German and is remembered);
`/xx/kanban` is a 404; no session redirects to sign-in. It is in the main
navigation and the pages menu. Needed SvelteKit 3's route-matcher module
(`src/params.ts`, `files.params` in `vite.config.ts`). Only the kanban route is
locale-prefixed; the other routes are unchanged. The older SVAR boards (deals,
leads, tickets) are untouched.

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
