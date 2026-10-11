# For the Chief Executive Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## The decision in one paragraph

This is a demo-grade CRM with an unusually strong security and privacy
foundation. It is **not ready** to hold real patient or citizen data, and the
repository says so itself ([plan.md](../../../../spec/plan.md)). Adopting it is a decision to
fund the remaining external work, not to switch it on.

## What exists

- A working CRM across sales, marketing, support and analytics, with a Rust API
  and a web front end, built to a written specification.
- Consent-first marketing, access control that hides sensitive values from
  roles that should not see them, an audit trail that shows tampering, subject
  access and erasure for privacy requests, and emergency ("break-glass")
  access that is reasoned, time-limited and reviewed.
- 48 of 53 tracked work items are done ([tasks.md](../../../../spec/tasks.md)).

## What stands between this and real use

| Blocker | Why it matters | Who can fix it |
|---|---|---|
| No real email delivery (T39) | Marketing is simulated; a real provider needs contracts and a data-processing agreement | You choose the vendor; legal signs |
| No independent security test (T42) | Hospitals and government buyers usually require one; our own tests are not independent | An outside firm |
| No manual accessibility audit (T48) | Public bodies must publish an accessibility statement they can stand behind | People with assistive technology |
| Build signing and API standards review unfinished (T49) | Procurement checklists ask for both | Engineering, with a decision on key custody |
| Idempotency keys missing (T52) | A retried create can duplicate a record | Engineering |

## Decisions that are yours

1. **Which market first.** The NHS Digital Technology Assessment Criteria
   assessment ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   concludes the product probably falls outside DTAC scope and would fail the
   sections that apply for lack of organizational evidence (named clinical
   safety officer, certifications). Pick the route before spending on it.
2. **Whether to fund independent assurance** (T42, T48). Without it the honest
   claim is "built to a high standard, untested by outsiders".
3. **Who owns the risk** once real data is held. See the [CRO](../chief-risk-officer/index.md)
   and [CLO](../chief-legal-officer/index.md) briefs.

## What not to say externally

Do not claim compliance, certification, "HIPAA-compliant" or "NHS approved".
The evidence pack ([compliance](../../../../spec/compliance/index.md)) supports a narrower,
true statement: controls are mapped, implemented ones are tested, gaps are named.
