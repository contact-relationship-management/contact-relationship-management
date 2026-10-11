# For the Chief Risk Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Risk position

**Overall: not ready for regulated data.** The software is well controlled for
its stage, but the main residual risks are unmitigated by testing and process
that only the adopting organization or outside parties can supply.

## Top risks

| # | Risk | Rating basis | Mitigation in the repo | Residual |
|---|---|---|---|---|
| 1 | Undiscovered vulnerabilities | No independent penetration test (T42) | Own tests, supply-chain gates, strict headers/CSP | **High** until tested |
| 2 | Disclosure of sensitive fields to the wrong role | Complex access rules | Role-based masking on records, aggregates and insights; a persona-matrix test; a leak found and fixed on account detail (T47) | Medium: masking is tested, not independently verified |
| 3 | Insider misuse of emergency access | Break-glass elevates reads | Written reason, 60-minute cap, read-only, mandatory second-person review, audited | Medium: no alert is wired; no revoke |
| 4 | Audit trail altered | Needs database access | Hash chain plus append-only trigger; reports the first break | Low-Medium: head hash must be anchored outside the DB |
| 5 | Unlawful marketing | Consent errors | Send path enforces consent, not the UI; immutable consent history | Medium until a real sender, bounce and complaint handling exist (T39) |
| 6 | Availability / abuse | Public API | Rate limit and lockout, shared via Redis | Medium: limits fall back per replica if Redis fails |
| 7 | Accessibility non-compliance | Public-sector duty | Automated axe scans; honest accessibility statement | **Unquantified** until manual audit (T48) |
| 8 | Supply chain | Dependencies and images | `cargo deny/audit`, `pnpm audit`, SBOM, digest pins refreshed by Dependabot, a keyless-signed release workflow | Medium: signing has not run on a real release (T49) |
| 9 | Duplicate records from retries | Idempotency keys are opt-in at the API (T52) | Replay for clients that send the key; the bundled UI always does | Low |
| 10 | Misclassification of data | Deployer decides what a "contact" is | Scope note and DPIA template | Depends on you |

## Gaps that stay open by nature

T39 (real email provider), T42 (independent assurance), T48 (manual
accessibility audit), T49 (a first signed release, a reviewer's check of the API draft). See [tasks.md](../../../../spec/tasks.md)
and [plan.md](../../../../spec/plan.md).

## Suggested risk actions

Register risks 1, 7 and 10 formally; require T42 before any real data; set an
owner for break-glass review and alerting; set the audit-head anchoring cadence.
