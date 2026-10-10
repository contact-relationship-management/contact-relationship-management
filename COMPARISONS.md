# Comparisons

How this project relates to other CRM software. This is a demonstration
project, not a competitor to mature products.

| Project                                      | Kind                         | Relationship                                                       |
| -------------------------------------------- | ---------------------------- | ------------------------------------------------------------------ |
| [Twenty](https://twenty.com)                 | Open-source CRM              | Full product; this project is a smaller, spec-driven demo          |
| [EspoCRM](https://www.espocrm.com)           | Open-source CRM (PHP)        | Broader features; this project emphasizes explainable rules        |
| [SuiteCRM](https://suitecrm.com)             | Open-source CRM (PHP)        | Mature, large; this project is a compact reference                 |
| Salesforce, HubSpot                          | Commercial SaaS CRM          | The practice this project models; no code or data is shared        |

## What is different here

- Specification first: [spec](spec/index.md) is the single source of truth for
  both the Rust and Svelte editions.
- Consent-first marketing, explainable rule-based lead scoring, and
  stage-weighted forecasting.
- Contacts are consumed from a separate identity service rather than
  registered here.
- Synthetic data only; no real email is sent.

Corrections and additional comparisons are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).
