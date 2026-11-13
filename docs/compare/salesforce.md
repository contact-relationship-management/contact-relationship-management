# Contact Relationship Management and Salesforce

Salesforce is a commercial, cloud-based CRM platform. Contact Relationship Management is open-source software that is not ready for production ([path to production](../../locales/en-us/path-to-production/index.md)). The two are not direct alternatives.

## Summary

| | Salesforce | Contact Relationship Management |
| --- | --- | --- |
| Category | AI-powered CRM platform with a unified customer profile | Open-source demonstration software, driven by its specification |
| Licence | Commercial, priced by seats and features | MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only or GPL-3.0-only, at the user's option |
| Hosting | Salesforce's cloud | Self-hosted: a Rust service, a Svelte interface and PostgreSQL |
| Focus | Sales, service, marketing, commerce and AI agents (Agentforce), with Slack connected to CRM data | Sales, marketing, support and analytics, with recorded consent |

## Where Salesforce is stronger

- Commercial platform in established use. This project models the same practice.
- Wider scope, including commerce, AI agents and collaboration through Slack.
- No hosting or maintenance for the customer.

## Where this project differs

- Code, specification and tests are public under a choice of open-source licences.
- Stores a reference to a person, not a copy of the name or address.
- Lead scores are calculated from readable rules.

## Suitability

- Salesforce suits organizations that need a supported production CRM.
- This project suits study of CRM internals or of a consent-first design.

See the [tutorial](../tutorial-first-tour.md) and [all comparisons](../../COMPARISONS.md).

## Sources

Statements about Salesforce are based on the public pages below, checked on 2026-11-04. Product details change. Corrections are welcome; see [CONTRIBUTING.md](../../CONTRIBUTING.md).

- [What is CRM?](https://www.salesforce.com/crm/what-is-crm/)

Salesforce is a trademark of its owner. This project is not affiliated with or endorsed by it.
