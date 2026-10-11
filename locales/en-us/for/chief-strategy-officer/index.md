# For the Chief Strategy Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## The strategic question

What is this for? Today it is a **specification-first reference CRM** with an
unusually complete security, privacy and audit layer. It is not a product with
customers, and it does not compete with mature CRMs
([COMPARISONS.md](../../../../COMPARISONS.md)). The choice in front of you is
whether to keep it a reference, or to invest in making it deployable in
regulated settings.

## What is genuinely different

| Strength | Why it could matter | How solid it is |
|---|---|---|
| Specification first: one spec drives both the Rust service and the Svelte UI ([spec](../../../../spec/index.md)) | Cheaper to audit, port and keep consistent | Real; tests cite the spec |
| Consent-first marketing, enforced in the send path | A hard requirement for health and public-sector buyers | Real, but no real sender exists yet (T39) |
| Explainable, rule-based lead scoring and forecasting | Buyers who must justify decisions about people | Real; deliberately no machine learning |
| Contacts consumed from an identity service, not duplicated | Less personal data held, smaller breach surface | Real; depends on that service existing |
| Tamper-evident audit, masking by role, reasoned emergency access | The controls regulated buyers ask about first | Implemented and tested; not independently verified |

## What is not different

Feature breadth. Quoting, contracts, telephony, machine-learning scoring and
payments are out of scope ([scope](../../../../spec/scope.md)). Anyone choosing a
CRM on features will choose another one.

## Options

| Option | What it means | Cost and risk | Evidence needed |
|---|---|---|---|
| A. Stay a reference | Keep it as a teaching and pattern library | Lowest; value is reputational | None beyond today |
| B. Regulated-sector niche | Finish the open items and sell or support it to health and public bodies | Highest; needs a vendor, legal terms, outside testing, ongoing assurance | Independent test (T42), accessibility audit (T48), real email (T39) |
| C. Components, not product | Offer the audit chain, masking and break-glass patterns as reusable pieces | Moderate; needs packaging and documentation | Extraction work; no new certification |

**Recommendation: do A now, and fund the cheapest test of B before committing.**
The NHS assessment concluded the product probably falls outside DTAC scope
([assessment](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
so the route to market is not settled. Spend first on finding out which buyer
needs this and what they would require, not on certifications nobody has asked for.

## Roadmap, in the order the evidence suggests

1. Independent security test and manual accessibility audit (T42, T48): they
   decide whether B is possible at all.
2. Real email provider behind the existing send trait (T39), once a vendor and
   data-processing terms are chosen ([roadmap](../../../../spec/roadmap.md)).
3. Build signing (T49): table stakes for procurement.
4. Only then the product features on the roadmap (attribution, territories,
   business-hours SLA clocks, public knowledge-base portal).

## Strategic risks

- **Claiming more than is true.** The fastest way to lose a regulated buyer is a
  compliance claim the evidence does not support. See the [GRC brief](../governance-risk-compliance/index.md).
- **Maintenance.** Security work does not finish: dependency updates, digest
  refreshes, key rotation, re-testing. Option B commits you to it.
- **Dependency on an identity service** that is outside this repository.
- **One maintainer.** The project has a single maintainer and says that review
  depth is one person's ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). Option B needs more than one person
  who can answer for the security work.

## Decisions for you

1. Which of A, B or C, and the review date for that choice.
2. Who the first target buyer is, found by conversation before spending.
3. Whether to fund independent assurance now (see the [CEO brief](../chief-executive-officer/index.md)).
