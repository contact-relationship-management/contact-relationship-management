# Penetration test: scope and rules for the testers (CRM-T42)

What to hand an independent tester. It is a brief, not a result. **No
penetration test has been done.** The software's own tests are not a substitute:
they were written by the people who built it. Accreditation (ATO, DSPT, HIPAA,
ISO 27001) and a formal risk analysis are separate work for the adopter; see
[control-mapping.md](control-mapping.md).

## Targets

- The Rust service (`contact-relationship-management-service-with-rust`), its
  REST API (`/api/...`, OpenAPI at `/api-docs/openapi.json`) and the webhooks
  (`/webhooks/email/...`).
- The Svelte front end, including its server-side proxy (`/api/proxy/...`),
  sign-in, sessions and the idle timeout.
- The container images and the release workflow's signing, if in scope.
- **Out of scope** unless the adopter says otherwise: the identity provider,
  the email vendors, the host and network, denial-of-service load tests.

## Run against a copy, never production

Use a disposable deployment with synthetic data. Start the service in
production mode (`CRM_REQUIRE_AUTH=1`, which the service requires, and the
`production.yaml` settings) so the tester sees what an adopter would. Mail
stays simulated unless the adopter sets a vendor.

## Controls the testers should try to defeat

- **Authentication and authorisation.** Bearer tokens against a key set; a
  per-record rule engine (owner sees own, others masked); masked callers get
  `null` for money and free text; break-glass elevation with a reason, a time
  limit and a review report. Try horizontal and vertical privilege escalation,
  token reuse and expiry, and changing a record's owner to read it.
- **Abuse resistance.** Rate limit and lockout after repeated `401`; the
  limits are per process and unauthenticated callers share a bucket unless the
  proxy header is trusted. Try to lock out other users, and to bypass the
  limit with forwarded headers.
- **Input handling.** Payload-size limit, JSON validation, idempotency keys on
  creates, RFC 9457 error bodies (do they leak internals?), pagination links,
  the board and reschedule `PUT` endpoints, and anything that reaches SQL.
- **Webhooks.** Signed outcome webhook (HMAC with a five-minute window), the
  Mailgun signature, SES/SNS subscription confirmation (restricted to AWS
  hosts; SNS message signatures are **not** verified), SendGrid (ECDSA
  signature **not** verified). These are known gaps: confirm they are
  exploitable and how far.
- **Unsubscribe links.** Signed, expiring? Try forging or replaying.
- **Data at rest.** Field encryption of two columns with a keyring (keys from
  the environment); the audit log's hash chain and append-only trigger; the
  subject-access export and erasure endpoints.
- **Front end.** CSP and security headers, cookie flags, the proxy forwarding
  (does it let a caller reach other paths or hosts?), open redirects after
  sign-in, XSS through card titles, names, notes and imported text, CSRF on the
  `PUT` and `POST` calls.
- **Supply chain.** Dependency advisories (`cargo deny` passes; one unmaintained
  crate is ignored with a reason), pinned image digests, cosign signatures.

## Known limits the adopter already accepts

Only two database columns are field-encrypted; counters are per process; the
idle timer is client-side; the audit chain cannot detect deletion of the newest
rows without an external anchor; edits are last-write-wins; the email adapters
have not run against live vendor accounts. See the task list for each.

## Report

Findings with severity (CVSS), steps to reproduce, affected version (commit),
and a fix suggestion; a retest after fixes. Close CRM-T42 only with the
tester's signed report and the adopter's risk acceptance decisions.
