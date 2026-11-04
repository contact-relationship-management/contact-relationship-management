# Contact Relationship Management

Open-source CRM software for sales, marketing and customer support.

Contact Relationship Management tracks customer and prospect relationships across sales, marketing and support. It holds one record for each contact and requires recorded consent before a message is sent. The software is a demonstration: it runs on synthetic data and is not intended for production use.

## Benefits

- **Single contact record.** Sales, marketing, support and analytics use the same record.
- **Recorded consent.** Nothing is sent to a contact without recorded consent. Each email includes a signed unsubscribe link.
- **Reduced personal data.** The CRM stores a reference to a person in an identity service, not a copy of the name or address.
- **Explainable results.** Lead scores are calculated from readable rules. Forecasts are calculated from weighted pipeline stages.
- **International design.** Names, addresses, calendars and writing directions are not tied to one country's format. The interface is available in eight languages.
- **Public specification.** The specification, code and tests are public. The documentation states what is built and what is not.

## Features

- **Sales.** Rule-based lead scoring, deal pipelines on a board, and stage-weighted forecasts.
- **Marketing.** Segmented campaigns and nurture sequences. Sending is simulated unless a mail mode is configured.
- **Support.** Tickets with priorities and service-level deadlines, and knowledge-base articles.
- **Analytics.** Dashboards for win rate, pipeline by stage, campaign return, service-level health and customer lifetime value.
- **Data quality.** Duplicate detection by probability, with merge and undo.
- **Governance.** Audit log, role-based access, and support for subject access and erasure requests.

## Getting started

1. **Get the code.** Clone the repository. Node.js and pnpm are required.
2. **Start the application.** In the Svelte directory, run `pnpm install`, then `pnpm dev`.
3. **Sign in.** Use `example@example.com`. No other service is required.
4. **Review the tour.** The tour covers contacts, leads, deals, campaigns, tickets and the executive view, using fictional data.

The [tutorial](../../docs/tutorial-first-tour.md) describes each step. [Install and use](../../INSTALL.md) covers the full service.

## Comparison

| Project | Category | Difference |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | Open-source CRM | Full product. This project is a smaller, specification-driven demonstration. |
| [EspoCRM](../../docs/compare/espocrm.md) | Open-source CRM | Broader features. This project emphasizes explainable rules. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | Open-source CRM | Mature and large. This project is a compact reference. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | Commercial SaaS CRM | This project models the same practice. No code or data is shared. |

This project is a demonstration and does not compete with these products. See [all comparisons](../../COMPARISONS.md).

## Role briefs

The [role briefs](for/) provide one page for each senior role: what the software does, what it does not do, and which decisions belong to that role.

## Status

This is demonstration software, not a production CRM. Use synthetic data only. No real email is sent unless a real mail mode is configured. Nothing on this site is legal advice.
