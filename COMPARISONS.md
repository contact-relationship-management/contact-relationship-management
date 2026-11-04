# Comparisons

How this project relates to other CRM software. This is a demonstration
project, not a competitor to mature products.

| Project                                      | Kind                         | Relationship                                                       |
| -------------------------------------------- | ---------------------------- | ------------------------------------------------------------------ |
| [Twenty](https://twenty.com)                 | Open-source CRM              | Full product; this project is a smaller, spec-driven demo          |
| [EspoCRM](https://www.espocrm.com)           | Open-source CRM (PHP)        | Broader features; this project emphasizes explainable rules        |
| [SuiteCRM](https://suitecrm.com)             | Open-source CRM (PHP)        | Mature, large; this project is a compact reference                 |
| Salesforce, HubSpot                          | Commercial SaaS CRM          | The practice this project models; no code or data is shared        |

## Side by side

Each page lists where the other product is stronger, where this project differs, and the sources:

- Commercial: [Salesforce](docs/compare/salesforce.md), [HubSpot](docs/compare/hubspot.md),
  [Microsoft Dynamics 365](docs/compare/microsoft-dynamics-365.md), [Zoho CRM](docs/compare/zoho-crm.md),
  [Pipedrive](docs/compare/pipedrive.md)
- Open source: [Twenty](docs/compare/twenty.md), [EspoCRM](docs/compare/espocrm.md),
  [SuiteCRM](docs/compare/suitecrm.md)

## What is different here

- Specification first: [spec](spec/index.md) is the single source of truth for
  both the Rust and Svelte editions.
- Consent-first marketing, explainable rule-based lead scoring, and
  stage-weighted forecasting.
- Contacts are consumed from a separate identity service rather than
  registered here: the CRM holds `person:` and `organization:` references, not
  the records.
- It works with sibling services by reference: won deals open delivery ideas
  in a project portfolio service, and a worker service's hire and leave events
  pause marketing to the organization's own employees.
- Synthetic data only; no real email is sent unless a real mail mode is
  configured.

Corrections and additional comparisons are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).
