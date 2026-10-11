# For the Chief People Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Why people matter to this product

Access to personal data is decided by role. The software enforces what a
person's token says; **who gets which role, and when it is removed, is a people
process** that this repository cannot do for you.

## What the software gives your processes

- **Role-based visibility:** some roles see masked values for contact channels,
  deal amounts and money views; managers, admin and service tiers see more
  ([auth](../../../spec/auth.md)).
- **Audited reads:** reading personal data leaves a trail naming the person.
  Staff should be told this before go-live.
- **Emergency access:** a person can take a reasoned read elevation of up to 60
  minutes; a different person must review it afterwards. Train both sides.
- **Idle sign-out** after 15 minutes in the browser.
- **Accessibility:** a public statement and automated checks; the manual audit
  (T48) still needs disabled users and assistive-technology testers.
- **Languages:** the UI is translated into 8 locales; the translations have
  **not** been reviewed by native speakers.

## Your responsibilities

| Area | Action |
|---|---|
| Joiners, movers, leavers | Grant and remove roles through your identity provider; the CRM refuses expired tokens but does not manage accounts |
| Training | Data protection, consent rules, break-glass use and review, accessible practice |
| Policies | Acceptable use, monitoring notice (reads are audited), disciplinary route for misuse of emergency access |
| Reviewer capacity | Name who reviews break-glass grants and the cover when they are away |
| Recruitment | Roles the programme needs: independent tester liaison, clinical safety officer if in NHS scope, data protection lead, accessibility lead |
| Wellbeing | Support staff handling sensitive cases; the CRM is not a case-management system but may hold sensitive relationship data |

## Open items that need people

Manual accessibility audit with assistive-technology users (T48), native-speaker
translation review, and ownership of the review and alerting duties above.
See [tasks.md](../../../spec/tasks.md).
