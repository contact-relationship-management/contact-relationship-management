# Contact Relationship Management and Microsoft Dynamics 365

Microsoft Dynamics 365 is Microsoft's commercial business application suite, including Sales and Customer Service. Contact Relationship Management is open-source software that is not ready for production ([path to production](../../locales/en-us/path-to-production/index.md)). The two are not direct alternatives.

## Summary

| | Microsoft Dynamics 365 | Contact Relationship Management |
| --- | --- | --- |
| Category | Agentic CRM for sales, alongside other Dynamics 365 applications | Open-source demonstration software, driven by its specification |
| Licence | Commercial, per user per month | MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only or GPL-3.0-only, at the user's option |
| Hosting | Microsoft's service | Self-hosted: a Rust service, a Svelte interface and PostgreSQL |
| Focus | Sales, with AI scoring, forecasting and Copilot. Customer Service and Customer Insights are separate applications. | Sales, marketing, support and analytics, with recorded consent |

## Where Microsoft Dynamics 365 is stronger

- Supported commercial product. This project is demonstration software.
- Copilot and prebuilt sales agents draft emails, summarize opportunities and suggest actions. This project has no equivalent.
- Integration with Outlook and Microsoft 365.

## Where this project differs

- Lead scores are calculated from readable rules.
- No dependency on a vendor ecosystem. Every part is open source and self-hosted.
- Sales, marketing and support are one application with one record for each contact.

## Suitability

- Microsoft Dynamics 365 suits organizations on Microsoft 365 that need a supported CRM.
- This project suits a small, readable reference with no vendor ecosystem.

See the [tutorial](../tutorial-first-tour.md) and [all comparisons](../../COMPARISONS.md).

## Sources

Statements about Microsoft Dynamics 365 are based on the public pages below, checked on 2026-11-04. Product details change. Corrections are welcome; see [CONTRIBUTING.md](../../CONTRIBUTING.md).

- [Dynamics 365 Sales](https://www.microsoft.com/en-us/dynamics-365/products/sales)

Microsoft Dynamics 365 is a trademark of its owner. This project is not affiliated with or endorsed by it.
