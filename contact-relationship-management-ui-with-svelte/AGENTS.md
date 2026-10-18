# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **SvelteKit browser client** for the
[Loco JSON API sibling](../contact-relationship-management-service-with-rust/):
contact/account timelines, the score-sorted lead queue, the deal
Kanban board, forecast tables, campaign funnels and the nurture
editor, the ticket queue with live SLA countdowns, the knowledge
base editor, and the KPI dashboards. The Svelte app owns no data;
every page round-trips through the API.

> ⚠️ Demo software, not a production CRM. See
> [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth —
   especially [auth](../spec/auth.md) (personas + masking) and
   [analytics-reporting](../spec/analytics-reporting.md) (honesty
   rules). Task queue: [`../spec/tasks.md`](../spec/tasks.md)
   (CRM-T17/T18, plus this edition's halves of CRM-T19/T20).
2. **Family front-end conventions.** SvelteKit 3 in
   SPA mode (`ssr = false`) on `adapter-node`, with a same-origin BFF, **Svelte 5 runes
   only** (no legacy stores/`$:`), TypeScript strict. Tables, boards,
   the calendar and the timeline are the Lily Design System
   components (`@lilydesignsystem/svelte-*`, pinned exactly), which
   are headless and ship no English: every label comes from the app's
   i18n, with the generic wording from
   `@lilydesignsystem/svelte-locale-translations`.
   Drift between front-ends is accepted — copy-adapt from a sibling
   (the project-portfolio-management front-end's Kanban and i18n
   are the closest source); no shared package.
3. **BFF auth.** The SvelteKit server holds the cookie session and
   exchanges it for short-lived PASETO tokens; **no token in browser
   JS, no localStorage credentials**. Mutations go through server
   routes with CSRF protection.
4. **Dashboard honesty.** Render `as_of` on every derived view;
   per-currency lines never merge; masked amounts render as a
   first-class masked state, never as errors or zeros.
5. **No client-side KPI math.** Win rate, forecast, ROI, CLV, SLA
   countdowns come from the API; the client formats, it does not
   compute (except ticking a countdown from a served deadline). The
   two display-only exceptions are a table's footer total over the
   rows shown and the campaign timeline's critical path
   (`src/lib/critical-path.ts`); neither is a KPI.
6. **Eight-locale i18n** (`ar-001, cy-001, de-de, en-001, es-001,
   fr-001, hi-001, zh-cn`) with the full-coverage parity test; a
   language in the address (`/de-de/contacts`) sets the language.
   Translations are unreviewed by native speakers.
7. **Tests.** vitest for the API client, `money()`, search, the
   critical path and the locale matcher; Playwright over the tour
   layer and a `page.route`-stubbed API mirroring the endpoint
   contract (unstubbed = 404-loud); `pnpm test:live` drives the
   boards and calendar against a running service on :5160 and reads
   each change back from it.

## Running

```bash
pnpm install
pnpm dev           # expects the Loco sibling (stub mode) on its default port
pnpm test          # vitest
pnpm exec playwright test
pnpm test:live     # against a running service (creates records)
```
