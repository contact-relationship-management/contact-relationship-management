# For the Chief Operating Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## What running this takes

The service is one Rust process per replica plus PostgreSQL 18 and Redis; the
front end is a Node process. Both ship as hardened, non-root container images
with base images pinned by digest. The operations runbook is
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Operational facts to plan around

- **Fails safe at start.** In production the service refuses to start if
  authentication is off, if the audit chain is unusable, or if required keys and
  `CRM_REDIS_URL` are missing. A misconfigured deploy stops rather than running open.
- **Health and metrics.** `/_health`, `/_ping`, `/_readiness` are never
  rate-limited, so a lockout cannot restart a healthy pod. Metrics at
  `/metrics.prom`.
- **Abuse limits** are on in production: 600 requests a minute per client and a
  lockout after 10 failed sign-ins in 5 minutes. They are shared across
  replicas through Redis; if Redis fails they fall back to per-replica counts.
- **Emergency access** (break-glass): any signed-in user can take a reasoned
  read elevation for up to 60 minutes. Someone else must review every grant.
  **Process you must run:** alert on `break_glass_granted` audit rows and
  review the unreviewed count (admin page `/break-glass/review`).
- **Audit growth.** Reads are audited, so the audit table grows with traffic.
  Size retention for it. A client that polls will grow it fast.

## Open operational work

| Item | State |
|---|---|
| Backup/restore drills, key rotation rehearsal | Written in the runbook; **you** must run and evidence them |
| Anchoring the audit chain head outside the database | Your duty; the API reports the head hash |
| Refreshing pinned image digests | Needs a schedule (Renovate or Dependabot) or they go stale |
| Real email provider (T39) | Open; changes the incident and supplier picture |
| Redis holds insight data | Protect it like the database: TLS, password, no public exposure |

## Known limits worth telling the on-call team

Idle sign-out is client-side; a revoked token is refused at once, but a policy
reload shows in cached insights after up to 30 seconds; break-glass cannot be
revoked, only expire.
