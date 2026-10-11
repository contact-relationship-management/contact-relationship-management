# For the Chief Legal Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Legal posture in brief

The repository is **demo software with synthetic data**. It does not claim
compliance with any regime. It implements technical features that support
several, and lists what a deployer must add ([regulatory](../../../../spec/regulatory.md)).

## Regimes touched

| Regime | What the software supports | What you must supply |
|---|---|---|
| UK/EU GDPR | Consent history, subject access export, erasure (refused while open obligations exist), retention sweep, data minimization, audit of reads | Lawful basis per purpose, DPIA ([template](../../../../spec/compliance/dpia-template.md)), records of processing, retention schedule, controller/processor roles |
| PECR / e-privacy | Consent-gated sends, immediate unsubscribe | Sender identification and soft-opt-in decisions for a real sender |
| HIPAA (if US health data) | Audit controls, access control, integrity, transmission security features | A business associate agreement, risk analysis, policies, workforce training. The software does not make you compliant |
| NHS DTAC / DCB0129/0160 | Evidence drafts ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Named clinical safety officer, hazard log, certifications. Assessment says DTAC probably does not apply |
| Accessibility (WCAG 2.2 AA, public sector) | Automated checks and an honest statement | A manual audit (T48) before any compliance claim |
| Government security (NIST 800-53, ISO 27001, Cyber Essentials) | Control mapping ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certification and independent assurance |

## Contracts you will need that do not exist yet

1. Data-processing agreement and international-transfer terms with the email provider (T39).
2. Statement of work and confidentiality terms with the penetration-test firm (T42).
3. Hosting and sub-processor terms for PostgreSQL, Redis and logs.
4. License: see [LICENSE.md](../../../../LICENSE.md); the repository also carries
   security, governance and AI-use statements.

## Specific legal watch-points

- **Erasure** anonymizes the contact and is refused while an open deal, ticket
  or active nurture enrollment exists; the audit chain keeps records of actions,
  so settle how that interacts with erasure requests in your DPIA.
- **Break-glass** reads of masked data are lawful only on your documented basis;
  the reason field is evidence, not authority.
- **Scoring** of leads: if it ever influences significant decisions about
  people, automated-decision rules apply ([regulatory](../../../../spec/regulatory.md)).
- **API commitments:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  sets a 180-day notice period before an API change is removed. That is the
  maintainer's choice; if customers will rely on it, decide whether it belongs
  in a contract.
- **Public claims:** do not state compliance, certification or approval.
