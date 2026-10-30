# Contact Relationship Management — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../contact-relationship-management-service-with-rust/):
sales, marketing, and support views over the relationship
lifecycle — lead queue, deal Kanban, forecasts, campaign funnels,
nurture sequences, ticket queue with SLA countdowns, knowledge
base, and KPI dashboards.

> ⚠️ **Demo software.** Not a production CRM; synthetic data only.
> See [spec/regulatory](../spec/regulatory.md).

**Status: implemented.** The delivery record is
[../spec/tasks.md](../spec/tasks.md) and [CHANGELOG.md](CHANGELOG.md).
The boards, calendar, timeline and tables are Lily Design System
components (`@lilydesignsystem/svelte-*`), with labels from
`@lilydesignsystem/svelte-locale-translations`. The header search is
`/search`, and any page opens under `/<locale>/…` in one of eight
locales (`src/lib/locales/`).

## Quick start

```bash
pnpm install
pnpm dev
```

Sign in as `example@example.com` for the guided tour: fictitious data
held in the UI server, with no service needed (`CRM_DEMO_TOUR=off`
removes it). To use the Loco sibling instead, set
`CRM_API_URL=http://localhost:5150/api` and point `AUTH_API_URL` at the
family authentication service, which provides the sign-in routes
(`/api/auth/magic-link`, `/api/auth/token`). The CRM service does not.

## Tests

| Command | What it runs |
|---|---|
| `pnpm check` | `svelte-kit sync` and `svelte-check` |
| `pnpm test` | vitest (`tests/unit`) |
| `pnpm lint` | Prettier and ESLint |
| `pnpm exec playwright test` | Playwright (`tests/e2e`: smoke, the tour, axe accessibility) on a built preview at :4373; the tour data and `page.route` stubs stand in for the service |
| `pnpm test:live` | Playwright (`tests/live`: boards, calendar, duplicates, links, records) against a running service on :5160 (or `CRM_API_URL`), with its authentication off |

## Environment variables

Read by `src/lib/server/config.ts` and the routes (declared in
`src/env.ts`); see [`.env.example`](.env.example). Only the `PUBLIC_`
variable reaches the browser.

| Variable | Default | Purpose |
|---|---|---|
| `CRM_API_URL` | `http://localhost:5150` | The service's API base. The BFF proxy sends `/api/proxy/<path>` to `<CRM_API_URL>/<path>`, so set it to the service's `/api` path, for example `http://localhost:5150/api` (the default, without `/api`, misses the service's routes) |
| `AUTH_API_URL` | `http://localhost:5150` | The authentication service (session → PASETO exchange, magic link) |
| `CRM_DEMO_TOUR` | on | `off` removes the guided tour |
| `PUBLIC_OIDC_SIGNIN_ENABLED` | unset | Public. Set `true` to show "Sign in with SSO" (`/signin/sso`); needs OIDC configured on the auth service. Unset, `/signin/sso` is `404` |

## Stack

SvelteKit 3 · Svelte 5 runes · TypeScript strict · SPA mode
(`ssr = false`) with a same-origin BFF proxy (session cookie →
short-lived PASETO; no token in browser JS) · eight-locale i18n · Lily
Design System · vitest + Playwright.

## Views

| Area | Routes |
|---|---|
| Relationships | `/contacts` (+ `/contacts/import`, a contact's timeline and consent), `/accounts` (+ `/accounts/import`), `/search` |
| Sales | `/leads` (+ `/leads/board`, `/leads/import`), `/deals` (Kanban board; on a won deal's page, open a delivery idea in PPM), `/quotas` (quotas, teams, the WPM teams import and the employment check), `/fx-rates` |
| Marketing | `/campaigns` (with segments), `/templates`, `/attribution` |
| Support | `/tickets` (+ `/tickets/board`, with SLA countdowns), `/articles`, `/help` |
| Analytics | `/` (dashboard), `/reports`, `/followups`, `/executive`, `/dpo` |
| Engagement | `/engagement`, `/partners` |
| Operations | `/workflows`, `/notifications`, `/custom-fields`, `/break-glass`, `/accessibility`, `/tour` |
