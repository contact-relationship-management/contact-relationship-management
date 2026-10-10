# DTAC v2.0 assessment — Contact Relationship Management

Self-assessment of this repository against the NHS Digital Technology Assessment
Criteria form v2.0, using the [checklist](checklist.md). Assessed 2026-10-10
against the repository after the field-level encryption work (CRM-T41). It is an engineering assessment by the maintainer's tooling, not a
submission, an attestation, or legal advice, and nobody has audited it.

## Verdict

**This product cannot pass DTAC today, and it is doubtful that DTAC is the right
route for it.**

1. **Applicability is the first question.** DTAC is for digital health
   technologies. This is a general-purpose contact relationship management system
   (sales, marketing, support, analytics). It has no clinical purpose, and its
   own documentation says it is demo software with synthetic data and no real
   email sending ([regulatory.md](../../../../../spec/regulatory.md)). An NHS
   organization buying a CRM for non-clinical use (for example stakeholder,
   partner, supplier, or membership management) would normally assess it under
   its own information governance and procurement checks, with DTAC asked for
   only if the buyer decides it applies.
2. **If DTAC is required anyway**, the assessed sections fail on missing
   evidence rather than on missing design: no named Clinical Safety Officer (if
   C1 applies), no Cyber Essentials certificate, no external penetration test,
   no Data Security and Protection Toolkit return, no completed DPIA, no ICO
   registration evidence, no published accessibility statement.
3. **The engineering is a reasonable base** for the technical parts of C2 and C3
   and for the accessibility part of D1; see the question-by-question table.

Counts by assessed section (questions that need an answer, after the
skip-logic in the form):

| Section | Result today |
|---|---|
| C1 Clinical safety | Not applicable if the product is not used for direct care, but that justification must be written and a commissioner can challenge it |
| C2 Data protection | Fails: no DSPT, no ICO evidence, no completed DPIA, no privacy notice for a hosted service |
| C3 Technical security | Fails: no Cyber Essentials, no pen test; MFA and logging partly met |
| C4 Interoperability | Passes by skip-logic if the product is declared not to exchange patient-identifiable data with national systems; otherwise fails (no NHS number, PDS, or NHS login) |
| D1 (scored) | Partial: automated WCAG 2.2 AA checks pass, nothing else evidenced |

## How to read the table

- **Met** — a control exists in this repository and is pinned by a test or CI check.
- **Partial** — some of the control exists; the gap is stated.
- **Not met** — nothing exists today.
- **Deployer** — depends on who operates the service. This project ships
  software; it does not host it or hold anyone's data. Where the form asks about
  "your product", the answer changes if the maintainer or a vendor hosts it.
- **N/A** — the form's own skip-logic removes the question, given the stated answer.
- Evidence paths are relative to the repository root.

## A. Company information (non-assessed)

| Code | Status | Note |
|---|---|---|
| A1 | Info needed | The legal entity that would be the manufacturer. The repository names an individual maintainer ([MAINTAINERS.md](../../../../../MAINTAINERS.md)), not a company. |
| A2 | Met | Contact Relationship Management. |
| A3 | Info needed | Pin the version under assessment; the service is `0.1.0`, unreleased. |
| A4 | Met | Software as a Service if hosted by a vendor; standalone software if self-hosted. Decide, because it changes C2 and C3. |
| A5–A7 | Info needed | Key contact (the maintainer's email is in [MAINTAINERS.md](../../../../../MAINTAINERS.md)); phone not held. |
| A8–A10 | Info needed | No registered company, charity, or Companies House number is recorded. |
| A11–A12 | N/A | No CQC registration applies to a software supplier. |

## B. Value proposition (non-assessed)

| Code | Status | Note |
|---|---|---|
| B1 | Met | "Other". It is not patient care, diagnostics, or clinical support; "Workforce Support or Management" is the nearest listed option if a buyer prefers it. |
| B2 | Met | [spec/purpose.md](../../../../../spec/purpose.md) and [spec/index.md](../../../../../spec/index.md). |
| B3 | Partial | Intended users are described (sales, marketing, support, analytics roles). No benefits have been validated, no evaluation, no trial. |
| B4 | Partial | Data flows are in [spec/architecture.md](../../../../../spec/architecture.md) and pre-filled in the [DPIA template](../../../../../spec/compliance/dpia-template.md); no user journey map exists. |

## C1. Clinical safety (assessed)

| Code | Status | Assessment |
|---|---|---|
| C1.1.1 | Met (answer: No) | Nothing in the product has a medical purpose, so it is not Software as a Medical Device under the UK Medical Devices Regulations 2002. No PAQ is needed. A deployer that adds a medical purpose (for example triage, or a patient-facing feature) would change this answer. |
| C1.1.2 | N/A | Follows from C1.1.1. |
| C1.2 | Met (answer: No), but challengeable | The product is not designed to influence, support, or manage real-time or near-real-time direct care. Customer-support tickets and SLA timers are about service requests, not patients. |
| C1.2.1 | **Not met** | The justification must be written and submitted. Draft: "A CRM for managing relationships with customers, partners, and stakeholders; it holds no clinical information and no workflow depends on it for direct care." State plainly that an organization recording **patients as contacts** or using tickets for care requests would put it in scope and must do its own DCB0129/DCB0160 work. Not written yet. |
| C1.2.2–C1.2.5 | Not met, and required only if C1.2 is answered Yes | No clinical risk management system, safety case, hazard log, or named Clinical Safety Officer exists. If a buyer rules C1 applicable, this is a substantial piece of new work that needs a clinician, not engineering. |

## C2. Data protection (assessed)

| Code | Status | Assessment |
|---|---|---|
| C2.1 | **Not met** / Deployer | No DSPT return exists for the maintainer. If the software is self-hosted by the buyer and the manufacturer has no remote access, the honest answer is "No access to patient data or national NHS System". A hosted service, or any remote support access, needs a DSPT return at Standards Met or Exceeded. |
| C2.2 | Deployer | Per the form's note, "No" is available only when the manufacturer has no role in operating or hosting the product and no means of accessing its data. Self-hosted: No. Vendor-hosted: Yes, and everything below applies. |
| C2.2.1 | **Not met** | No ICO registration evidence. Required if the manufacturer is a controller or processor in a hosted service. |
| C2.2.2 | Partial | A pre-filled [DPIA template](../../../../../spec/compliance/dpia-template.md) exists, but no completed DPIA. The template covers the form's required headings, but every risk and residual rating is blank. Supporting facts that are true today: URN wrappers so identity data stays upstream; subject access, erasure, and retention endpoints ([auth.md](../../../../../spec/auth.md)); an audit trail with a tamper-evident hash chain; security headers and a CSP; rate limiting and lockout. Field-level AES-256-GCM encryption of the contact channel and activity text exists (CRM-T41) with key rotation, but keys come from the environment or a file rather than a key-management service, and other columns are not covered; whole-database encryption at rest is still the deployer's. |
| C2.2.3 | **Not met** | No privacy notice or transparency information. A self-hosting buyer writes their own; a hosting vendor must publish one. |
| C2.2.4 | Partial | A licence exists ([LICENSE.md](../../../../../LICENSE.md)) but no terms on use of user data or end-user licence agreement. "Not applicable" is defensible for self-hosted software, with a stated reason. |
| C2.2.5 | Deployer | Data location is chosen by the deployer. The software makes no cross-border transfers itself; the optional Keycloak and key-fetch URLs are configured by the deployer. |
| C2.2.6 | Deployer | Only if hosted outside the UK. |

## C3. Technical security (assessed)

| Code | Status | Assessment |
|---|---|---|
| C3.1 | **Not met** | No Cyber Essentials certificate. It is organization-level, so it is the manufacturer's to obtain; it cannot be derived from the code. |
| C3.2 | **Not met** | The Cyber Security Charter for Suppliers to the NHS has not been signed. Signing it skips C3.3 to C3.6 on the form, so it is the cheapest route to a pass here. |
| C3.3 | **Not met** | No external penetration test (CRM-T42, open). The CI gates (`cargo audit`, `cargo deny`, `pnpm audit`, SBOM) and the hardening in [plan.md](../../../../../spec/plan.md) are not a substitute: the form requires a third-party OWASP Top 10 test from the last 12 months with nothing at CVSS 7.0 or above. |
| C3.4 | Partial | Mapped to the four principles of the Software Security Code of Practice below. Evidence exists for most, but no one has reviewed it against the code. |
| C3.5 | Partial / Deployer | The product delegates authentication to an identity provider (PASETO tokens or Keycloak/OIDC); MFA is therefore the identity provider's to enforce, which fits the form's "preferably through identity federation". The service refuses to start in production without authentication enabled. The plan itself is not written down; it should be stated in the runbook. |
| C3.5.1 | N/A / Deployer | Not applicable when the supplier has no access to the product. A hosting vendor needs MFA on all privileged and remote access. |
| C3.6 | Partial | Logging requirements are defined ([audit.md](../../../../../spec/audit.md)) and an audit trail exists, but "audit trails of all access" is not fully true: contacts, leads, tickets, and activities are read-audited; deals, accounts, and insights views are not (CRM-T40 limits). |

### C3.4 mapped to the Software Security Code of Practice

| Principle | Evidence today | Gap |
|---|---|---|
| Secure design and development | ABAC with record-level checks, masking, input validation, `forbid(unsafe_code)`, clippy pedantic, fail-closed production start | No threat model document; masking not complete for every view |
| Secure build environment | Lockfiles, pinned git revision, CI on two providers, `cargo deny` source policy, SBOMs | Builds are not signed or attested; CI runners are the provider's |
| Secure deployment and maintenance | Distroless non-root images, read-only compatible, security headers, rate limiting, runbook | No image digest pinning or signing; no stated patch SLA beyond the runbook's targets |
| Communication with customers | [SECURITY.md](../../../../../SECURITY.md) with a private reporting route and a 7-day acknowledgement | Single maintainer; no advisory publication process or coordinated-disclosure timeline |

## C4. Interoperability (assessed)

| Code | Status | Assessment |
|---|---|---|
| C4.1 | Met (answer: Yes) | A REST/JSON API with an OpenAPI 3 document served at `/api-docs/openapi.json`. |
| C4.1.1 | Partial | Standards used: HTTP, JSON, OpenAPI 3, OAuth/OIDC (Keycloak), PASETO for service tokens. **No health-care standard (FHIR, HL7) is implemented**; there is nothing to justify as appropriate for health data because it does not handle any. |
| C4.1.2 | Partial | The API is openly documented and the code is under open licences ([LICENSE.md](../../../../../LICENSE.md)). It has not been checked against GDS API guidance (for example versioning, error format, and caching are present via `Accepts-version` and ETags; other points are unverified). Confirm after a review. |
| C4.1.3 | N/A | Only if C4.1.2 cannot be confirmed. |
| C4.2 | Met (answer: No) | The product is not intended to exchange data with national or local care systems where patient identity matters. Contacts are `person:` URN references to an upstream registry. Answering No skips the rest of C4.2. |
| C4.2.1–C4.2.6 | N/A, with a caveat | If a buyer uses it with patient records: no NHS number support, no PDS integration, and no NHS login. All of these would be Not met. |

## D1. Usability and accessibility (scored, not pass/fail)

| Code | Status | Assessment |
|---|---|---|
| D1.1 | Partial | A guided tour exists in the UI (`/tour`) and the spec describes the roles; no instructions for use or pathway document. |
| D1.2 | Not met | No usability testing with intended users. |
| D1.3 | Not met | The Accessible Information Standard has not been reviewed or considered in design. It concerns communicating with disabled patients, so its relevance to a non-patient-facing CRM should be stated. |
| D1.4 | Met (answer: Yes) | A web application. |
| D1.4.1 | Partial | Answer: "No, but a plan is in place" is the honest choice. Automated axe-core checks with WCAG 2.0 to 2.2 A and AA tags pass on 11 pages (`tests/e2e/a11y.spec.ts`); automated tools find roughly a third of issues; no manual audit with assistive technology, no screen-reader test, no keyboard-only audit of the drag-and-drop boards. |
| D1.4.2 | Not met | No dated plan. |
| D1.4.3 | **Not met** | No accessibility statement exists, published or drafted. |
| D1.5 | Not met | The service has never run in production, so there are no availability figures. Do not invent one. |

## Supporting documents

| Code | Document | State |
|---|---|---|
| A12 | CQC report | N/A |
| B4 | User journeys and data flows | Data flows partial; no journey map |
| C1.1.1 | PAQ | N/A (not a medical device) |
| C1.2.3, C1.2.4 | Clinical risk management and safety case | Not produced; needed only if C1.2 is Yes |
| C1.2.4 | Hazard log | Not produced |
| C2.2.1 | ICO registration | Not held |
| C2.2.2 | DPIA | Template only |
| C2.2.3 | Privacy notice | Not written |
| C2.2.4 | Terms on use of user data / EULA | Licence only |
| C3.1 | Cyber Essentials certificate | Not held |
| C3.3 | Penetration test summary | Not done |
| D1.1 | User journeys | Partial |

## Actions, cheapest route first

1. **Decide applicability and say so in writing.** Agree with the buyer whether DTAC
   applies. If not, record that decision and stop. If so, write the C1.2.1
   justification (a short document) and state the patients-as-contacts caveat.
2. **Choose the deployment model** (self-hosted software or vendor-hosted SaaS)
   because C2.1, C2.2, C2.2.5, and C3.5.1 turn on it.
3. **Sign the Cyber Security Charter for Suppliers to the NHS** (C3.2). It removes
   C3.3 to C3.6 from the form.
4. **Obtain Cyber Essentials** (C3.1), which is organization-level.
5. **If hosted:** DSPT return, ICO registration, completed DPIA, privacy notice,
   user-data terms. These are legal and organizational documents.
6. **If the Charter is not signed:** commission the independent OWASP Top 10
   penetration test (C3.3, CRM-T42) and write the MFA plan (C3.5).
7. **Accessibility (D1):** run a manual WCAG 2.2 AA audit with a keyboard and a
   screen reader, fix what it finds, then publish an accessibility statement
   (D1.4.3). Consider an alternative to drag-and-drop for the boards.
8. **Close the logging gap** (C3.6): audit reads of deals, accounts, and insights
   (CRM-T40 limits), or state the exclusion in the answer.
9. **Only if a buyer will use it for patient-identifiable data:** the whole of C1.2
   (a Clinical Safety Officer is a registered clinician, so this is not an
   engineering task) and the NHS number, PDS, and NHS login work in C4.2.

## Related documents

- [Compliance evidence pack](../../../../../spec/compliance/index.md): control
  mapping, DPIA template, operations runbook.
- [Production readiness plan](../../../../../spec/plan.md) and the open tasks in
  [tasks.md](../../../../../spec/tasks.md) (T39, T41, T42).
