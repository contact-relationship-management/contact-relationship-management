# Compliance evidence pack

Supports the assessment in [../plan.md](../plan.md) (CRM-T36). These are
engineering documents for an adopter's security, privacy, and procurement
teams. They state what the software does and does not do; they are not a
certification, audit, or legal opinion, and no accreditation is claimed.

- [control-mapping.md](control-mapping.md) — controls from NIST SP 800-53
  rev 5, the HIPAA Security Rule, ISO/IEC 27001 Annex A, and GDPR Art. 32,
  mapped to what is implemented, what is the deployer's duty, and what is a gap.
- [dpia-template.md](dpia-template.md) — a data protection impact assessment
  skeleton pre-filled with this system's data flows (written as the GDPR's; under another law, the
  assessment that law asks for).
- [personal-data-inventory.md](personal-data-inventory.md) — every table, the personal data in it,
  its retention class and what erasure does to it; a test keeps it complete.
- [dpia.md](dpia.md) — the DPIA filled in as far as the software goes, with the deployer's own
  decisions left open.
- [record-of-processing.md](record-of-processing.md) — the record of processing (GDPR Art. 30 in the
  worked example; the equivalent under the law that applies), started from what the software does.
- [risk-management.md](risk-management.md) — the ISO 14971 process mapped to what the software does, with the hazards a medical use would add to and their controls (not a risk management file; no conformity claimed).
- [operations-runbook.md](operations-runbook.md) — deployment hardening,
  backup and restore, encryption, key management, logging, incident response.

Ground rules used throughout: a control is **Implemented** only when code or
configuration in this repository enforces it and a test or check pins it;
**Deployer** when the organization running the software must supply it;
**Gap** when neither is true today (each gap names its task in
[../tasks.md](../tasks.md)).

**Generic and national assurance.** [control-mapping.md](control-mapping.md) is the generic mapping
(international or widely used frameworks, and a [row per region](control-mapping.md#by-region)). A
scheme of one country or one body lives under `docs/<country-or-body>/<scheme>/<version>/` (lower case,
names in full, hyphens), beside the generic one and never as the default: today only the United
Kingdom's, in `docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/`,
with the two UK drafts below. The rule is stated in the
[register by region](../regulatory.md#the-register-by-region).

- [dtac-supporting.md](dtac-supporting.md) — draft statements for the NHS DTAC form (United Kingdom).
- [gds-api-review.md](gds-api-review.md) — self-review against UK government API standards (a draft; United Kingdom).
- [api-lifecycle.md](api-lifecycle.md) — error format (RFC 9457), versioning, and the deprecation policy.
- [email-provider-brief.md](email-provider-brief.md) — what to hand an email provider and its legal reviewers (CRM-T39).
- [first-release-checklist.md](first-release-checklist.md) — the dry run that proves release signing and verification (CRM-T49).
- [accessibility-audit-brief.md](accessibility-audit-brief.md) — the manual test plan for CRM-T48 (no audit has been done).
- [penetration-test-scope.md](penetration-test-scope.md) — scope and rules for an independent tester, CRM-T42 (no test has been done).
