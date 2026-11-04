# Contact Relationship Management and Microsoft Dynamics 365

Microsoft Dynamics 365 is Microsoft's commercial business applications, including Sales and Customer Service. Contact Relationship Management is open-source demo software, not a production CRM, so this is a fair look at two different things rather than a contest.

## At a glance

| | Microsoft Dynamics 365 | Contact Relationship Management |
| --- | --- | --- |
| What it is | An agentic CRM for sales, alongside other Dynamics 365 apps | Open-source demo software, driven by its specification |
| Licence | Commercial, per user per month | MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only or GPL-3.0-only, at your option |
| Where it runs | Microsoft's service | You run it yourself: a Rust service, a Svelte interface and PostgreSQL |
| Focus | Sales, with AI scoring, forecasting and Copilot; separate Customer Service and Customer Insights apps | Sales, marketing, support and analytics, with consent first |

## Where Microsoft Dynamics 365 is stronger

- It is a supported commercial product, and this project is demo software.
- Copilot and prebuilt sales agents draft emails, summarize opportunities and suggest actions; this project has no equivalent.
- It connects to Outlook and Microsoft 365, which many organizations already use.

## Where this project is different

- Scoring is by rules you can read, so "why this score?" has a plain answer.
- It depends on no vendor's ecosystem: every part is open source and runs on your machine.
- Sales, marketing and support are one application, sharing one record of each contact.

## Which suits you

- Choose Microsoft Dynamics 365 if your organization already runs on Microsoft 365 and needs a supported CRM.
- Try this project if you want a small, readable reference with no vendor ecosystem.

Curious? [Take the tour](../tutorial-first-tour.md), or see [all the comparisons](../../COMPARISONS.md).

## Sources

The statements about Microsoft Dynamics 365 come from these public pages, checked on 2026-11-04. Products change, so check them before you decide. A correction is welcome; see [CONTRIBUTING.md](../../CONTRIBUTING.md).

- [Dynamics 365 Sales](https://www.microsoft.com/en-us/dynamics-365/products/sales)

Microsoft Dynamics 365 is a trademark of its owner. This project is not affiliated with or endorsed by it.
