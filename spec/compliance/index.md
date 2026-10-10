# Compliance evidence pack

Supports the assessment in [../plan.md](../plan.md) (CRM-T36). These are
engineering documents for an adopter's security, privacy, and procurement
teams. They state what the software does and does not do; they are not a
certification, audit, or legal opinion, and no accreditation is claimed.

- [control-mapping.md](control-mapping.md) — controls from NIST SP 800-53
  rev 5, the HIPAA Security Rule, ISO/IEC 27001 Annex A, and GDPR Art. 32,
  mapped to what is implemented, what is the deployer's duty, and what is a gap.
- [dpia-template.md](dpia-template.md) — a data protection impact assessment
  skeleton pre-filled with this system's data flows.
- [operations-runbook.md](operations-runbook.md) — deployment hardening,
  backup and restore, encryption, key management, logging, incident response.

Ground rules used throughout: a control is **Implemented** only when code or
configuration in this repository enforces it and a test or check pins it;
**Deployer** when the organization running the software must supply it;
**Gap** when neither is true today (each gap names its task in
[../tasks.md](../tasks.md)).
