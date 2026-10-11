# Governance, Risk, Compliance

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Where to start

The evidence pack is [spec/compliance](../../../../spec/compliance/index.md). Its
ground rule is the one to hold the team to: a control is **Implemented** only
when code or configuration enforces it and a test or check pins it; **Deployer**
when your organisation must supply it; **Gap** when neither is true today.

| Document | Use it for |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Annex A, GDPR Art. 32, each marked Implemented, Deployer or Gap |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Data protection impact assessment, pre-filled with this system's data flows |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Hardening, backup and restore, keys, logging, incident response |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Draft NHS DTAC statements for the manufacturer to adopt |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Self-review against UK government API standards (a draft, not checked against the published text) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Error format (RFC 9457), what counts as a breaking change, the deprecation steps and notice period |
| [NHS DTAC v2.0 assessment](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Question-by-question result: probably out of DTAC scope, and it would fail the applicable sections for lack of organisational evidence |
| [plan](../../../../spec/plan.md) | Readiness verdict (not ready) and the gap register |
| [tasks](../../../../spec/tasks.md) | Each gap's task, resolution and stated limits |

## Governance

- **Decision rights:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Contribution and AI-assistance rules are in
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) and [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Change control:** the specification is the source of truth; code and tests
  cite it. CI runs on two providers, and `scripts/ci-check.sh` reproduces it.
  Rule in the repo: do not weaken a test or gate to make something pass.
- **Disclosure:** [SECURITY.md](../../../../SECURITY.md) is the reporting route.

## Risk

The risk view is in the [CRO brief](../chief-risk-officer/index.md). In short: the highest
residual risks are untested-by-outsiders security (T42), unquantified
accessibility (T48), and data classification, which is the adopter's call.

## Compliance: what is real and what is not

| Area | Position |
|---|---|
| Access control | Implemented and tested: role-based masking, record-level checks, admin-only operational reads, break-glass with mandatory second-person review |
| Audit | Implemented and tested: hash-chained, append-only, in-transaction, read auditing including lists, deal lists and insights. **Deployer:** anchor the chain head outside the database |
| Privacy rights | Implemented: subject access export, erasure (refused while obligations are open), retention sweep. **Deployer:** lawful basis, retention schedule, records of processing |
| Encryption | Partly: field-level AES-256-GCM on two columns with local keys, no KMS. Encryption at rest and in transit are the deployer's |
| Supply chain | Implemented: `cargo audit`, `cargo deny`, `pnpm audit`, SBOMs, base images pinned by digest, Dependabot updates. **Gap:** the signed-release workflow has not run on a real release (T49) |
| Accessibility | Partly: automated axe scans and an honest public statement. **Gap:** no manual audit (T48), translations unreviewed |
| Independent assurance | **Gap:** none (T42) |
| Email and marketing law | Consent-first send path implemented; real sending, processor agreement, bounce and complaint handling are absent (T39) |

## Controls that need an owner in your organisation

1. Review and alerting for break-glass grants (alert on `break_glass_granted`).
2. Anchoring the audit chain head and verifying it on a schedule
   (`GET /api/audits/verify`).
3. Backup and restore evidence, and key rotation rehearsal.
4. Joiner, mover and leaver handling in your identity provider.
5. Retention for the audit table, which grows with read traffic.
6. Protecting Redis like the database: it holds cached insight data.
7. Merging Dependabot updates, and verifying a signed release before deploying it.

## What to refuse to sign off on

Any statement that the system is compliant, certified, HIPAA-compliant or NHS
approved. The defensible statement is narrower: controls are mapped, the
implemented ones are tested, and the gaps are named and tracked.
