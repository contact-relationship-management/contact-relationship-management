# Independent Security Tester

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Why this exists

Every security control in this project was built and tested by its authors
(CRM-T42). That shows the controls work as the authors understood them. It does
not show that nothing is missing. Hospitals and government buyers usually require
a test by someone independent before they hold real data, and the honest claim
until then is "built to a high standard, untested by outsiders"
([CEO brief](../chief-executive-officer/index.md),
[security brief](../chief-security-officer/index.md)).

This page is what to hand a testing firm, and what to expect back.

## What passing means, and what it does not

A clean report says a named team, in a named time, with a named scope, found no
serious issues. It is **not** a certification, not an accreditation and not a
guarantee. Do not describe the system as "secure", "certified" or "penetration
tested to a standard" beyond what the report states
([GRC brief](../governance-risk-compliance/index.md)).

## Choosing the tester

- Independent of the authors, and of anyone who configured the deployment.
- Staffed by named, experienced testers, with relevant accreditation for your
  market where one applies (for example CREST, or the NCSC CHECK scheme for
  UK government work). Ask which accreditation covers the individuals doing the
  work, not just the company.
- Agrees in writing to scope, rules of engagement, confidentiality, data
  handling, and liability before any testing starts. The contract is the legal
  team's to agree ([CLO brief](../chief-legal-officer/index.md)).

## What to test

| Area | What the testers should try | Where the design is described |
|---|---|---|
| Authentication | Forged, expired, replayed and wrong-audience tokens; downgrade to unauthenticated; the start-up check that refuses to run with authentication off | [auth](../../../../spec/auth.md) |
| Authorization | Reaching records or actions a role should not: horizontal and vertical escalation, record-level checks, destructive actions, admin-only reads (audit, retention, break-glass review) | [auth](../../../../spec/auth.md) |
| Data exposure | Masked values leaking through lists, bundled detail reads, insights, aggregates, error messages, caching, or the cache key | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Emergency access | Taking, extending or abusing break-glass; reviewing your own grant; reviewing twice | [auth](../../../../spec/auth.md) (T44) |
| Audit | Altering, deleting or reordering history; bypassing the chain; hiding reads | [audit](../../../../spec/audit.md) |
| Abuse resistance | Rate-limit and lockout bypass, locking out other users, header spoofing for the client key | T43 |
| Input handling | Injection (SQL, JSON, header), oversized bodies, malformed cursors and idempotency keys, replaying or colliding keys across callers | T51, T52 |
| Web front end | Cross-site scripting, request forgery, session handling, the backend-for-frontend proxy, content security policy, clickjacking | UI `hooks.server.ts` |
| Infrastructure as shipped | Container images, exposed ports, Redis and PostgreSQL reachability, secrets handling, configuration defaults | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Supply chain | Dependency and image provenance, the SBOMs, unsigned builds | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Business logic | Consent bypass in the send path, erasure that leaves data, scoring manipulation, duplicate creation | [regulatory](../../../../spec/regulatory.md) |

**Out of scope unless you agree otherwise:** the identity service that issues
tokens (it is outside this repository), the hosting platform, and denial-of-service
at volume.

## What to give the testers

1. A **staging build** that matches what will run in production: production
   configuration, authentication on, TLS, a real Redis and PostgreSQL. Do not
   test the demo defaults.
2. **Synthetic data only.** The seed task builds a book of business with no real
   people. Real patient or citizen data must never be in a test environment.
3. **One account per role**, so authorization can be tested from each side. The
   reference policy is `config/abac-policy.reference.json`.
4. The documents linked above, the OpenAPI document at `/api-docs/openapi.json`,
   and read access to the repository, so the test can combine black-box and code
   review.
5. The **known weaknesses**, so testers spend their time on what is not already
   written down:
   - release signing is built but has not run on a real release (T49);
   - field encryption covers two columns with local keys, no KMS;
   - step-up re-authentication is not built;
   - break-glass cannot be revoked, only expire, and no alert is wired;
   - policy reloads show in cached insights for up to 30 seconds;
   - idle sign-out is client-side;
   - rate-limit state falls back to per replica if Redis fails.
   Ask them to confirm or refute each, not to skip them.

## Rules of engagement to agree in writing

- Named testers, dates and source addresses; a 24-hour contact on your side.
- What may be attempted (for example, no destruction of the test data store
  without notice) and what must stop at proof of exposure.
- Who is told at once of a critical finding, and how, before the final report.
- How the report, the evidence and any tester notes are stored and destroyed.

## What you should get back

- A report with scope, method, dates, tester names and accreditation.
- Each finding with severity, evidence, steps to reproduce, affected component
  and a recommended fix.
- A clear statement of what was **not** tested.
- A retest of the fixed findings, included in the engagement or priced.

## After the report

1. Triage every finding in `spec/tasks.md` as a task with an owner and a date.
2. Fix critical and high findings before any real data, then have the testers
   confirm the fixes.
3. Add a regression test for each fix. The project rule is not to weaken a test
   or gate to make something pass.
4. Record the result honestly: the report date, the scope, the findings and
   their status. Update the readiness plan ([plan](../../../../spec/plan.md)) and
   the compliance pack ([compliance](../../../../spec/compliance/index.md)).
5. Re-test after material change, and on a schedule your risk owner sets.
   One test is a snapshot, not a state.

## Order of work

1. Close what is cheap and known first, so the testers do not spend the budget
   rediscovering it: a first signed release (T49), alerting on break-glass
   and on audit verification failures, Redis behind TLS and credentials.
2. Prepare the staging build, accounts and seed data.
3. Agree scope, contract and rules of engagement.
4. Test; receive critical findings at once.
5. Fix, retest, and record.
6. Only then update external claims, and only to what the report supports.

## Decisions for you

1. Who commissions and who owns the findings (see the
   [CRO brief](../chief-risk-officer/index.md)).
2. Which accreditation your market expects of the testers.
3. The budget and the date, set by what a delay costs rather than by what is
   convenient.
4. Whether to test once before launch or on a recurring schedule.
