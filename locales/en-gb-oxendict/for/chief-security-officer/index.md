# For the Chief Security Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Security posture in one paragraph

The service **fails safe**, controls access by role and by record, hides
sensitive values from roles that should not see them, and keeps a tamper-evident
audit trail. Those controls are implemented and tested by the authors. They have
**not been tested by anyone independent** (T42), so treat every statement below
as "built and self-tested", not "verified".

## Controls and where to check them

| Area | What is in place | Where to look |
|---|---|---|
| Authentication | PASETO v4.public verified offline, optional Keycloak; production refuses to start with authentication off | [auth](../../../../spec/auth.md) |
| Authorization | Attribute-based: read, write, delete, destructive; a `mask` obligation; record-level checks; admin-only audit, retention and break-glass review | [auth](../../../../spec/auth.md), persona-matrix test |
| Data exposure | Masking on records, aggregates and insights; an account-detail leak was found and fixed (T47) | [tasks](../../../../spec/tasks.md) |
| Emergency access | Written reason, 60-minute cap, read-only, mandatory second-person review, audited, shared through Redis | break-glass tasks T44 |
| Audit | SHA-256 hash chain in the same transaction, append-only trigger, verify endpoint; reads audited | [audit](../../../../spec/audit.md) |
| Abuse resistance | Rate limit (600 a minute) and lockout (10 failures in 5 minutes), shared via Redis; probes exempt | T43 |
| Browser and transport | Strict CSP with hashes, frame denial, `nosniff`, referrer policy, closed CORS, secure headers; no token in browser JavaScript (BFF) | UI `hooks.server.ts` |
| Data protection | AES-256-GCM on two columns with rotation; local keys, no KMS | T41 |
| Supply chain | `cargo audit`, `cargo deny`, `pnpm audit`, SBOMs, hardened non-root images pinned by digest, weekly Dependabot updates, a release workflow that signs images and SBOMs keylessly | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Errors | Every error is one RFC 9457 problem document; a `401` names the accepted scheme | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Memory safety | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Known weaknesses (stated, not hidden)

- **No independent penetration test or threat-modelling exercise** (T42). The
  threat model in [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  is a summary, not a structured one.
- **Release signing has not run on a real release** (T49). The workflow and the
  verification commands exist; until a tag is pushed they are untested, and the
  actions are pinned to version tags, not commit hashes.
- **Field encryption** covers two columns with local keys, no KMS.
- **Step-up re-authentication** before destructive actions is not built.
- **Redis holds personal data** (cached insights, grant keys) and must be protected
  like the database: TLS, password, no public exposure.
- **Break-glass** cannot be revoked, only expire, and no alert is wired.
- **Rate-limit and lockout** fall back to per-replica counts if Redis fails; the
  unauthenticated bucket is shared unless `CRM_TRUST_FORWARDED_FOR=1` behind a
  trusted proxy, so an attacker can lock out other unauthenticated callers.
- **Policy reloads** show in cached insights after up to 30 seconds.
- **Idle sign-out** is client-side.
- **Idempotency keys** are opt-in at the API (T52): a client that does not send
  `Idempotency-Key` is not protected from a retried create duplicating a record.
  The bundled UI sends one.

## What the deployer must do

1. Keep authentication on; supply production keys and `CRM_REDIS_URL`.
2. Alert on `break_glass_granted` and review grants promptly.
3. Anchor the audit chain head outside the database and verify it
   (`GET /api/audits/verify`) on a schedule.
4. Protect PostgreSQL and Redis (network, TLS, credentials, backups, encryption
   at rest); rotate field keys with the provided task.
5. Merge the Dependabot updates; re-run the supply-chain stage; verify a signed
   release before deploying it.
6. Follow the runbook for incident response
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Vulnerability handling

Reports go through [SECURITY.md](../../../../SECURITY.md). Commission the
independent test (T42) against a staging build before any real data, and
re-test after material change.

## Suggested security actions, in order

1. Scope and commission the independent penetration test (T42).
2. Push a first release tag to prove signing and verification, and pin the
   actions by commit hash (T49).
3. Wire alerting for break-glass and for audit-chain verification failures.
4. Put Redis behind TLS and credentials in every environment.
5. KMS-backed field keys and wider field coverage (T41 remainder).

See also the [CTO brief](../chief-technology-officer/index.md) and the
[CRO brief](../chief-risk-officer/index.md).
