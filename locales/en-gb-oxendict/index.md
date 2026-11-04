# Contact Relationship Management

Sales, marketing and support in one friendly place, with consent first.

Welcome! Contact Relationship Management brings the people you work with, and everything you have promised them, together around one record of each contact. It is open-source demo software, so you can explore it freely on fictional data and read exactly how it works.

## Why it is worth a look

- **One picture of each customer.** Sales, marketing, support and analytics share the same record, so nobody starts a conversation from scratch.
- **Consent comes first.** Nothing is sent to a contact without recorded consent, and an email carries a signed unsubscribe link.
- **Less personal data to protect.** The CRM keeps a reference to a person in your identity service, not a copy of their name or address.
- **Decisions you can explain.** Lead scores come from rules you can read, and forecasts from weighted stages, so you can always answer "why this number?"
- **Made for the whole world.** Names, addresses, calendars and writing directions are not assumed to follow one country's shape, and the interface comes in eight languages.
- **Open all the way through.** The specification, the code and the tests are public, and the documents say plainly what is built and what is not.

## What you can do with it

- **Sell.** Score leads with clear rules, move deals across a board, and forecast by weighted stage.
- **Market.** Run campaigns to segments and nurture contacts step by step, with sending simulated until you turn real mail on.
- **Support.** Track tickets with priorities and deadlines, and publish knowledge-base articles.
- **Understand.** See win rate, pipeline by stage, campaign return, service health and customer lifetime value on dashboards.
- **Keep records clean.** Find likely duplicates with a probability you can inspect, merge them, and undo a merge.
- **Stay accountable.** Keep an audit log, control access by role, and answer subject access and erasure requests.

## Try it in four steps

1. **Get the code.** Clone the repository. You need Node.js and pnpm.
2. **Start the app.** In the Svelte folder, run `pnpm install`, then `pnpm dev`.
3. **Sign in.** Use `example@example.com`. No other service is needed.
4. **Take the tour.** Walk through contacts, leads, deals, campaigns, tickets and the executive view on fictional data.

The [tutorial](../../docs/tutorial-first-tour.md) goes through each step, and [install](../../INSTALL.md) covers the full service.

## How it compares

| Project | What it is | How this project differs |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | Open-source CRM | A full product; this is a smaller demo driven by its specification |
| [EspoCRM](../../docs/compare/espocrm.md) | Open-source CRM | Broader features; this project puts explainable rules first |
| [SuiteCRM](../../docs/compare/suitecrm.md) | Open-source CRM | Mature and large; this is a compact reference |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | Commercial SaaS CRM | The practice this project models; no code or data is shared |

This is a demonstration project, not a competitor to those mature products. Read [the full comparison](../../COMPARISONS.md).

## Find your way in

Leading a team? The [role briefs](for/) give each senior role a single page: what the software is, what it gives you, what it does not, and the decisions that are yours.

## An honest word

This is demo software, not a production CRM. Use synthetic data. No real email is sent unless you configure a real mail mode, and legal or compliance wording here is not legal advice.
