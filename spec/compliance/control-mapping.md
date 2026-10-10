# Control mapping

Status: **Implemented** (enforced here, pinned by a test or CI check),
**Partial**, **Deployer** (the operating organization supplies it), **Gap**.
Evidence paths are relative to the repository root.

## Access control and identity

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Authenticated access to every non-public route | NIST AC-3, IA-2; HIPAA §164.312(d); ISO A.5.15, A.8.5; GDPR 32 | Implemented when `CRM_REQUIRE_AUTH=1`; production refuses to start without it | `src/auth.rs`, `src/hardening.rs` (CRM-T30), `tests/enforcement.rs` |
| Fail-safe default | NIST SA-8, CM-6 | Implemented | production boot check, T30 |
| Attribute-based authorization, least privilege | NIST AC-6; HIPAA §164.312(a)(1); ISO A.8.2 | Partial | contact and deal reads/writes are record-checked; other handlers use the coarse gate (auth.md known limits) |
| Data masking by role | NIST AC-3(7) | Partial (CRM-T32) | contacts, deals, forecast, sales dashboard, campaign cost/ROI, bundled contact rows; `/api/insights/*` and snapshots not yet |
| Destructive actions restricted to admin/service | NIST AC-6(9) | Implemented | `DESTRUCTIVE_POST_SUFFIXES` + pins |
| Session management, idle timeout, re-authentication | NIST AC-11, AC-12; HIPAA §164.312(a)(2)(iii) | Partial | HttpOnly cookie sessions, BFF; idle timeout and step-up re-auth are a Gap (T43) |
| Emergency access procedure | HIPAA §164.312(a)(2)(ii) | Gap | T44 |
| Rate limiting, lockout | NIST AC-7, SC-5 | Gap at the app; Deployer at the gateway | T43 |
| Identity provider, MFA | NIST IA-2(1) | Deployer | Keycloak/OIDC feature or the family auth service |

## Audit and accountability

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Audit of mutations, committed with the mutation | NIST AU-2, AU-12; HIPAA §164.312(b); ISO A.8.15 | Implemented | `models/audit_logs.rs` |
| Tamper-evident, append-only audit trail | NIST AU-9, AU-10; HIPAA §164.312(c)(1) | Implemented (CRM-T31) | hash chain, `GET /api/audits/verify`, append-only trigger, `tests/requests/audit_chain.rs` |
| External anchoring of the chain head | NIST AU-9(3) | Deployer | ship `head` to WORM/off-host storage; see runbook |
| Audit of personal-data reads | NIST AU-2; HIPAA §164.312(b); GDPR 5(2) | Partial (CRM-T40) | contact, lead, ticket single reads and list summaries; not deals, accounts, insights |
| Audit review and protection from casual readers | NIST AU-6, AU-9(4) | Implemented (CRM-T38) | audit, event, and retention reads are admin/service tier; enforcement test |
| Time source | NIST AU-8 | Deployer | NTP on hosts; timestamps are database time |

## System and communications protection

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Transport encryption | NIST SC-8, SC-13; HIPAA §164.312(e); ISO A.8.24 | Deployer | terminate TLS 1.2+ at the ingress; HSTS is set by both services |
| Encryption at rest | NIST SC-28; HIPAA §164.312(a)(2)(iv) | Deployer | PostgreSQL volume or TDE encryption; field-level encryption is T41 |
| Security response headers, CSP | OWASP ASVS V14; NIST SC-18 | Implemented (CRM-T33) | `config/production.yaml`, `src/lib/server/headers.ts`, `vite.config.ts`; vitest and Playwright pins |
| CORS closed by default | OWASP ASVS V14.5 | Implemented | `config/production.yaml` |
| Request size limits | NIST SC-5 | Implemented | `limit_payload` 2 MB |
| Input validation | NIST SI-10 | Implemented | `src/validation.rs` |
| Memory safety | NIST SA-15 | Implemented | `#![forbid(unsafe_code)]` |
| Unauthenticated operational endpoints | NIST AC-3 | Implemented (opt-in) | `CRM_PROTECT_OPERATIONAL=1` requires auth on metrics and OpenAPI; public by default |

## Supply chain and configuration

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Known-vulnerability scanning | NIST RA-5, SI-2; ISO A.8.8 | Implemented (CRM-T34) | `cargo audit`, `pnpm audit --prod` in CI |
| Dependency licence and source policy | NIST SR-4 | Implemented | `deny.toml`, `cargo deny` in CI |
| Software bill of materials | NIST SR-4; EO 14028 | Implemented | CycloneDX from `scripts/ci-check.sh supply-chain`, uploaded as a CI artifact |
| Pinned, reproducible dependencies | NIST CM-2, SR-3 | Implemented | `Cargo.lock`, `pnpm-lock.yaml`, pinned git revision |
| Hardened runtime image | NIST CM-7; CIS Docker | Implemented (CRM-T35) | `Containerfile`s: distroless, non-root, no shell |
| Independent penetration test | NIST CA-8 | Gap | T42 |

## Privacy

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Lawful basis and consent for marketing | GDPR 6, 7; PECR | Implemented in the send path (consent gate, append-only history); legal review is Deployer | `rules/segment.rs`, CRM-G2 |
| Data minimisation | GDPR 5(1)(c) | Implemented | contacts are URN wrappers; identity data stays upstream |
| Right of access | GDPR 15 | Implemented | `GET /api/contacts/{pid}/subject-access` |
| Right to erasure | GDPR 17 | Implemented (anonymise, gated) | `POST /api/contacts/{pid}/erase`; coordination with identity services is Deployer |
| Storage limitation | GDPR 5(1)(e) | Implemented | `CRM_RETENTION_DAYS`, sweep |
| Processor agreements, ESP | GDPR 28 | Gap | T39 |
| Records of processing, DPIA | GDPR 30, 35 | Deployer | [dpia-template.md](dpia-template.md) |
| Breach notification | GDPR 33; HIPAA §164.404 | Deployer | [operations-runbook.md](operations-runbook.md) |
| Data residency | GDPR Ch. V | Deployer | choose region; service makes no cross-border transfers itself |

## Accessibility

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| WCAG 2.2 A/AA automated checks | Section 508; EN 301 549; UK PSBAR | Partial (CRM-T37) | axe-core over public and list pages in `tests/e2e/a11y.spec.ts`; automated checks find roughly a third of issues, so manual testing with assistive technology is still required |

## Reading this table

Several rows read **Deployer** or **Gap**. That is the honest state: the code
side of production hardening is largely in place after CRM-T30–T40, but the
access model is not complete (masking of insights, read auditing of deals and
accounts, rate limiting, break-glass), real sending is absent (T39),
and no external assurance exists (T42).
