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
| Data masking by role | NIST AC-3(7) | Partial (CRM-T32) | contacts, deals, forecast and snapshots, sales dashboard, insights money views, campaign cost/ROI, bundled contact rows; persona matrix pinned over the reference policy |
| Destructive actions restricted to admin/service | NIST AC-6(9) | Implemented | `DESTRUCTIVE_POST_SUFFIXES` + pins |
| Session management, idle timeout, re-authentication | NIST AC-11, AC-12; HIPAA §164.312(a)(2)(iii) | Partial (CRM-T43) | HttpOnly cookie sessions, BFF, 15-minute client-side idle sign-out; step-up re-auth before destructive actions is a Gap |
| Emergency access procedure | HIPAA §164.312(a)(2)(ii) | Implemented (CRM-T44) | reasoned, time-limited, audited, reviewed by another person; UI at `/break-glass` and `/break-glass/review`; alerting on grants is the deployer's to add |
| Rate limiting, lockout | NIST AC-7, SC-5 | Implemented (CRM-T43), per process | `src/rate_limit.rs`; enforce a global limit at the ingress as well |
| Identity provider, MFA | NIST IA-2(1) | Deployer | Keycloak/OIDC feature or the family auth service |

## Audit and accountability

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Audit of mutations, committed with the mutation | NIST AU-2, AU-12; HIPAA §164.312(b); ISO A.8.15 | Implemented | `models/audit_logs.rs` |
| Tamper-evident, append-only audit trail | NIST AU-9, AU-10; HIPAA §164.312(c)(1) | Implemented (CRM-T31) | hash chain, `GET /api/audits/verify`, append-only trigger, `tests/requests/audit_chain.rs` |
| External anchoring of the chain head | NIST AU-9(3) | Deployer | ship `head` to WORM/off-host storage; see runbook |
| Audit of personal-data reads | NIST AU-2; HIPAA §164.312(b); GDPR 5(2) (the EU and UK example; [by region](#by-region)) | Partial (CRM-T40) | contact, lead, ticket single reads and list summaries; not deals, accounts, insights |
| Audit review and protection from casual readers | NIST AU-6, AU-9(4) | Implemented (CRM-T38) | audit, event, and retention reads are admin/service tier; enforcement test |
| Time source | NIST AU-8 | Deployer | NTP on hosts; timestamps are database time |

## System and communications protection

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Transport encryption | NIST SC-8, SC-13; HIPAA §164.312(e); ISO A.8.24 | Deployer | terminate TLS 1.2+ at the ingress; HSTS is set by both services |
| Encryption at rest | NIST SC-28; HIPAA §164.312(a)(2)(iv) | Deployer, plus field-level for two columns (CRM-T41) | PostgreSQL volume or TDE encryption; AES-256-GCM on contact channel and activity text with key rotation; keys not in a KMS; other columns not covered |
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

The references in this table are the EU GDPR's and the UK's (UK GDPR, PECR), the worked example; they
are *not yet read* (see the [register by region](../regulatory.md#the-register-by-region)). The same
controls for every other region are in [By region](#by-region) below.

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| Lawful basis and consent for marketing | GDPR 6, 7; UK PECR | Implemented in the send path (consent gate, append-only history), opt-in only; other consent regimes are a Gap (CRM-D21, proposed, CRM-T166); legal review is Deployer | `rules/segment.rs`, CRM-G2 |
| Data minimisation | GDPR 5(1)(c) | Implemented | contacts are URN wrappers; identity data stays upstream |
| Right of access | GDPR 15 | Implemented | `GET /api/contacts/{pid}/subject-access` (described as the right of access under the law that applies to the deployer, CRM-T165) |
| Right to erasure | GDPR 17 | Implemented (anonymise, gated) | `POST /api/contacts/{pid}/erase`; coordination with identity services is Deployer |
| Storage limitation | GDPR 5(1)(e) | Implemented | `CRM_RETENTION_DAYS`, sweep; the defaults are no jurisdiction's (`horizon_basis` in `GET /api/retention`, CRM-T165) |
| Processor agreements, ESP | GDPR 28 | Gap | T39 |
| Records of processing, DPIA | GDPR 30, 35 | Deployer | [dpia-template.md](dpia-template.md) |
| Breach notification | GDPR 33; HIPAA §164.404 | Deployer | [operations-runbook.md](operations-runbook.md) |
| Data residency | GDPR Ch. V | Deployer | choose region; service makes no cross-border transfers itself |

## By region

*Added 2026-10-31 (CRM-T165).* One row per region of the [register](../regulatory.md#the-register-by-region). Every regime named is
*not yet read*, so no cell says what a law requires: each says what the CRM gives that row, and where the shape usually described for
the region is one the CRM cannot yet express. Not legal advice. "Same" means the same as the EU row: the CRM's controls do not change
by region.

| Region | Marketing consent | Access and erasure | Retention | Records and impact assessment | Breach notice | Transfers | Each commercial message | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| European Union | opt-in per purpose, double opt-in, send-time check (Implemented) | subject-access export, gated erasure (Implemented) | deployer's horizons; defaults no jurisdiction's (Implemented) | GDPR Art. 30 and 35 forms: [record](record-of-processing.md), [DPIA](dpia.md) (Deployer) | [runbook](operations-runbook.md) (Deployer) | none by the software (Deployer) | unsubscribe link and headers (Implemented); sender identification and address (Gap, CRM-T166) | *not yet read* |
| United Kingdom | same; a soft opt-in for existing customers cannot be expressed (Gap, CRM-T166) | same | same | same forms | same | same | same | *not yet read* |
| United States | stricter than an opt-out regime: nothing is sent without `granted`; an opt-out profile cannot be expressed (Gap, CRM-T166) | same routes serve a state law's access and deletion requests | same | the instrument a state law asks for (Deployer) | each state's law (Deployer) | same | a postal address cannot be carried (Gap, CRM-T166) | *not yet read* |
| Canada | implied consent with an expiry cannot be expressed (Gap, CRM-T166) | same | same | the instrument its law asks for (Deployer) | same | same | sender identification and address (Gap, CRM-T166) | *not yet read* |
| Latin America and the Caribbean | opt-in only (Partial, CRM-T166) | same | same | for example the LGPD's impact report (Deployer) | same | same | same as the EU row | *not yet read* |
| Africa | opt-in only (Partial, CRM-T166) | same | same | the instrument its law asks for (Deployer) | same | same | same as the EU row | *not yet read* |
| Middle East and the Gulf | opt-in only (Partial, CRM-T166) | same | same | the instrument its law asks for (Deployer) | same | some laws localise data (Deployer) | same as the EU row | *not yet read* |
| South Asia | opt-in only (Partial, CRM-T166); calls and texts are not sent by the CRM | same | same | the instrument its law asks for (Deployer) | same | same | same as the EU row | *not yet read* |
| East Asia | opt-in only (Partial, CRM-T166); advertising labels in the subject cannot be added by a rule (Gap, CRM-T166) | same | same | for example a personal-information impact assessment (Deployer) | same | some laws require an assessment before a transfer (Deployer) | same as the EU row | *not yet read* |
| South-East Asia | opt-in only (Partial, CRM-T166) | same | same | the instrument its law asks for (Deployer) | same | same | same as the EU row | *not yet read* |
| Oceania | inferred consent cannot be expressed (Gap, CRM-T166) | same | same | for example a privacy impact assessment (Deployer) | same | same | sender identification (Gap, CRM-T166) | *not yet read* |

A row's consent cell reads **Partial** where the region's shape is not known here: opt-in is the strictest model and is safe, but
whether the region's own shape can be expressed is not known until its law is read.

## Accessibility

| Control | Reference | Status | Evidence / note |
|---|---|---|---|
| WCAG 2.2 A/AA automated checks | Section 508; EN 301 549; UK PSBAR | Partial (CRM-T37) | axe-core over public and list pages in `tests/e2e/a11y.spec.ts`; automated checks find roughly a third of issues, so manual testing with assistive technology is still required |

## Reading this table

Several rows read **Deployer** or **Gap**. That is the honest state: the code
side of production hardening is largely in place after CRM-T30–T40, but the
access model is not complete (masking of insights, read auditing of deals and
accounts, step-up re-authentication), real sending is absent (T39),
and no external assurance exists (T42).
