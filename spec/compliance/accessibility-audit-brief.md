# Manual accessibility audit: brief for the auditors (CRM-T48)

What to hand a person who will test this software with assistive technology.
It is a test plan, not a result. **No manual audit has been done.** The only
checks so far are automated (axe scans on every tour screen, run by
`tests/e2e/tour.spec.ts`, and keyboard paths the end-to-end tests exercise),
and automated scans find only some problems. The public statement is
`/accessibility`.

## Setup

- Start the UI (`pnpm dev` in `contact-relationship-management-ui-with-svelte`),
  open `/signin`, sign in as `example@example.com`. Every screen then shows
  fictitious data and sends no email. Sign in again to reset the data.
- Test in the app's eight languages where the auditor reads them
  (`en-001, de-de, fr-001, es-001, cy-001, ar-001, hi-001, zh-cn`); at least
  `en-001` and `ar-001` (right to left).
- The labels were translated without native-speaker review. Report wrong or
  unclear text as findings.

## Combinations to cover

| Screen reader | Browser | Platform |
|---|---|---|
| NVDA | Firefox, Chrome | Windows |
| JAWS | Chrome, Edge | Windows |
| VoiceOver | Safari | macOS, and iOS for the narrow layout |

Also: keyboard only (no pointer), 200% and **400% zoom** at 1280 px wide (the
page must reflow with no two-dimensional scroll, except data tables and the
calendar, which may scroll on one axis), text spacing override, Windows high
contrast / forced colours, `prefers-reduced-motion`, and every theme in the
theme picker (contrast of text, focus ring, today marker, critical-path bars).

## Screens and what to try

Data pages: `/contacts`, `/contacts/con-1`, `/accounts`, `/leads`, `/campaigns`,
`/tickets`, `/articles`, `/executive`, `/dpo`, `/engagement`, `/partners`,
`/break-glass`, `/break-glass/review`.

1. **Data tables** (Lily data grid). Arrow keys move between cells; PageUp and
   PageDown move a page; typing letters jumps to a row. Sort a column (is the
   direction announced?), search, change page (is the pager a landmark and the
   new page announced?), move a column with Alt+Arrow, export CSV, TSV and Excel
   and Copy (is each announced? does the file open?). Is the totals row read
   with its label?
2. **Boards** (`/leads/board`, `/tickets/board`, `/deals`, `/en-001/kanban`).
   Move a card with the "Move to column" menu and with Alt+Arrow (between
   columns and within one). Does the screen reader say where the card went? Try
   a move the workflow refuses (is the refusal announced?). Select several cards
   (Ctrl/Cmd+click or `x`) and move them together. Edit an amount or a name in
   place (F2), enter an invalid value, undo with Ctrl/Cmd+Z and redo with
   Ctrl/Cmd+Shift+Z. Search the board. Collapse a column and a swimlane. Is the
   swimlane (owner, source, priority) conveyed to a screen reader?
3. **Follow-up calendar** (`/followups`). Day, agenda, week and month views;
   week numbers; "N more" overflow; move an event with Alt+Arrow and by
   dragging; search; weekend cells (are they marked other than by tint?); the
   announcement of the shown period. Does the agenda view work well on a phone?
4. **Campaign timeline** (`/campaigns`). Read the bars as text: does each row
   say its dates and "critical path" or "N days slack"? Zoom in and out. Is
   the chart usable without sight (the bars are visual; the text is the
   description)?
5. **Everywhere.** Landmarks (and whether a skip link is needed), heading order, focus order and focus
   visibility, error messages (`role="alert"`), the idle sign-out (is it announced?), the
   language and text-size pickers, and the guided tour banner.

## Known weak spots to look at first

- Mouse drag has no screen-reader path of its own; the menu and Alt+Arrow are
  the accessible paths. Check that they give the same result.
- The timeline's weekly columns are labelled by their start date only.
- Lily components ship no English text; every label is ours. A missing label
  means a control with no name, so look for unnamed controls.
- Colour is never meant to carry meaning alone (flags, accents, weekends), but
  that has been checked only by the axe contrast rules.
- The rename field on ticket and lead cards uses the label "Edit" for every
  card; check the card's name is part of the button's accessible name.

## What to send back

For each finding: the screen, the language, the screen reader and browser, what
was done, what was heard or seen, what was expected, and the WCAG 2.2 success
criterion. Rate it blocker / serious / minor. Fixes, a re-test and an updated
`/accessibility` statement close CRM-T48.
