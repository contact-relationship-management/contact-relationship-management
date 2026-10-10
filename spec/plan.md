# Plan — production readiness for medical and governmental organizations

Assessed 2026-10-10 against the repository as of commit `1d8909b`.
Companion to [tasks.md](tasks.md) (the CRM-T30–T44 rows) and
[regulatory.md](regulatory.md). This is an engineering assessment, not a
legal or accreditation opinion.

## Verdict

**Not ready.** The software is honest demo software and says so. Its design
is a good base (consent-first send path, ABAC with record-level checks on the
main handlers, audit-in-transaction, subject access/erasure/retention,
minimal PII, DB-free rule core, `forbid(unsafe_code)`), but major medical
(HIPAA, NHS DSPT / DCB0129, GDPR special-category) and governmental
(FedRAMP / NIST SP 800-53, UK Cyber Essentials Plus / GovAssure, ISO 27001,
WCAG 2.2 AA / Section 508 / EN 301 549) buyers would fail it today on the
gaps below. Most gaps are known and stated in [auth.md](auth.md); several
others were found in this assessment.

## What is already strong

| Area | Evidence |
|---|---|
| Authorization model | ABAC personas, record-level `authorize_record` on contact/deal reads and writes, destructive-action classification |
| Authentication | Offline PASETO v4.public, optional Keycloak/OIDC, key rotation, no JWT dependency |
| Privacy by design | URN wrappers, consent history append-only, subject access, erasure, retention sweep |
| Audit | Mutation audit rows commit with the mutation; outbox events |
| Engineering hygiene | No `unsafe`, clippy pedantic, lockfiles, CI on two providers, 100+ tests |

## Gap register

Severity: **B** blocks any non-demo deployment; **H** blocks regulated buyers;
**M** expected by procurement checklists.

| # | Gap | Sev | Task |
|---|---|---|---|
| 1 | `CRM_REQUIRE_AUTH` defaults **off**; the service boots wide open in production with no warning | B | T30 |
| 2 | Audit rows are plain inserts: anyone with DB write access can alter or delete history undetected (HIPAA §164.312(b)/(c), NIST AU-9/AU-10) | H | T31 |
| 3 | `mask` obligation not applied to forecast values, campaign ROI/cost, or the activities/deals/tickets bundled in `get_contact` | H | T32 |
| 4 | No security response headers on the service or the UI; production CORS is `enable: true` with no origin list | H | T33 |
| 5 | No dependency-vulnerability, licence, or SBOM gate in CI | H | T34 |
| 6 | No hardened container image (non-root, read-only rootfs, pinned digests) | M | T35 |
| 7 | No control mapping, DPIA template, or operations runbook (backup/restore, key management, encryption at rest, incident response, logging retention) | H | T36 |
| 8 | No accessibility gate (WCAG 2.2 AA / Section 508) | H | T37 |
| 9 | `/api/audits*`, `/api/events/recent`, `/api/retention` are readable by any caller that clears the coarse read gate; `/metrics.prom` and OpenAPI are unauthenticated | H | T38 |
| 10 | Real send path (ESP adapter), processor agreements, bounce/complaint handling absent | B | T39 |
| 11 | Reads of personal data are not audited (spec claims sensitive reads are; only the consent-history read is) | H | T40 |
| 12 | Field-level encryption now covers two columns with local keys; no KMS integration and other columns are plaintext at the field level | M | T41 |
| 13 | No independent penetration test, risk analysis, or accreditation evidence | B | T42 |
| 14 | No rate limiting or lockout; no UI idle-session timeout or re-authentication for destructive actions | M | T43 |
| 15 | Break-glass / emergency access with mandatory reason and after-the-fact review absent | M | T44 |
| 16 | The service is a demo with synthetic data and no clinical or case content; a medical buyer must also decide whether CRM data about patients is PHI in their context | — | scope note below |

## Phases

1. **Fail safe (T30, T31, T33, T34)** — make the unsafe default impossible,
   make the audit trail tamper-evident, add headers and supply-chain gates.
   All code-side; implemented 2026-10-10.
2. **Evidence (T35, T36, T37)** — container hardening, control mapping and
   runbooks, accessibility gate. Implemented 2026-10-10.
3. **Complete the access model (T32, T38, T40, T43, T44)** — masking,
   admin-only operational reads, and read auditing landed 2026-10-10 with
   stated limits (insights unmasked). Rate
   limits and idle sign-out (T43) landed too, without step-up re-auth.
   Break-glass (T44) landed with stated defaults, API and UI.
4. **Real-world exposure (T39, T41, T42)** — ESP integration and contracts,
   field-level encryption with a KMS, independent test and accreditation.
   Open: external parties and legal work, not code.

## Scope note

This CRM stores relationship data, not clinical records. A hospital or health
department that records patients as contacts, or a government body that
records citizens as contacts, takes on the special-category and public-sector
duties for that data. The deployer decides that classification; the plan above
is written to the stricter reading.
