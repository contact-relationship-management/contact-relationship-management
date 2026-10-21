# Operations runbook

Written for the team deploying the service and UI in a regulated environment.
Commands assume the container images from the repository's `Containerfile`s.

## 1. Deployment baseline

1. Run the images read-only with a tmpfs for `/tmp`, as non-root, all Linux
   capabilities dropped, `no-new-privileges`, CPU and memory limits set.
2. Terminate TLS 1.2+ (prefer 1.3) at the ingress. Both services send HSTS;
   do not expose either over plain HTTP.
3. Put the service on a private network. Only the UI/BFF and operators reach
   it; browsers never do (the BFF keeps tokens out of JavaScript).
4. Set `LOCO_ENV=production` and `CRM_REQUIRE_AUTH=1`. The service refuses to
   start in production otherwise (`CRM_ALLOW_INSECURE=1` overrides and logs an
   error; do not use it).
5. Mount an ABAC policy (`CRM_ABAC_POLICY_FILE`) derived from
   `config/abac-policy.reference.json` and run
   `cargo test --test enforcement -- --ignored` against it in staging.
6. Configure token verification (`CRM_PASETO_KEYS_URL` or Keycloak settings).
7. Restrict `/metrics.prom` and `/api-docs/*` at the ingress to the monitoring
   network; both are public inside the service.
8. Abuse limits are on by default in production (`CRM_RATE_LIMIT_PER_MIN`,
   `CRM_AUTH_FAIL_LIMIT`, `CRM_AUTH_FAIL_WINDOW_SECS`; `0` disables). With
   `CRM_REDIS_URL` set the counters are shared across replicas through Redis (a
   Redis error falls back to per-replica counters); production requires that
   variable because Loco's cache is Redis, so also limit at the ingress anyway. Set
   `CRM_TRUST_FORWARDED_FOR=1` only behind a proxy that overwrites
   `X-Forwarded-For`, otherwise all unauthenticated callers share one bucket.
9. Set `CRM_CORS_ORIGIN` only if a browser must call the service directly.
10. Run migrations as a separate, privileged step (`auto_migrate` is off in
   production). The runtime database role needs no DDL rights.
    From the image: `podman run --rm --entrypoint /app/crm-service <env> <image> db migrate`
    (the image's entrypoint is `start`, so `db migrate` needs the override). The
    production config refuses to load without `DATABASE_URL`, `JWT_SECRET` and
    `CRM_REDIS_URL`. Checked 2026-10-19 with a locally built image on a fresh
    PostgreSQL 18 and Redis: all 17 migrations applied, the service started,
    `/_ping` answered 200 and `/api/contacts` answered 401 (authentication on).
    A `start` without `--worker` runs no background jobs; run a second container
    with `--worker` for campaign sends (the image's entrypoint is already `start`, so the
    argument is `--worker`, not `start --worker`). **Releases up to v0.1.3 have a bug here (fixed in v0.1.4):**
    a worker-only container sends with the simulated sender and marks the campaign delivered
    while no mail leaves; use `--server-and-worker` in one container on those releases (see the service changelog).

## 2. Database

- PostgreSQL 18, TLS required between service and database, a dedicated role
  with DML only, a separate migration role.
- **Encryption at rest:** use an encrypted volume or managed-database
  encryption with a customer-managed key. The application also encrypts the
  contact channel and activity text (CRM-T41): set `CRM_FIELD_KEYS`
  (`id:base64,...`, 32-byte keys, first is active) or `CRM_FIELD_KEYS_FILE`
  from your secret manager, and `CRM_REQUIRE_FIELD_ENCRYPTION=1` so a missing
  key stops the service instead of storing plaintext. Back up the keys
  separately from the database: losing them makes the data unreadable.
  Audit snapshots and the event outbox may hold copies and are not encrypted.
- **Backups:** continuous WAL archiving or daily full plus frequent
  incrementals; encrypt backups; keep at least one copy off the primary
  account or region. Test a restore at least quarterly and record the result.
- **Audit chain:** after a restore, call `GET /api/audits/verify`; a broken
  chain after restoring from backup means the backup or the live data was
  altered.
- **Retention:** schedule `POST /api/retention/sweep` (admin or service
  credential) at the cadence your policy states, and keep the audit rows it
  writes.

## 2a. Break-glass review

Alert on audit rows with action `break_glass_granted`. A reviewer other than
the grantee works the queue from `GET /api/break-glass` (the `unreviewed`
count) and records the outcome with `POST /api/break-glass/{pid}/review`.
Define your own review deadline (for example one working day) and report
overdue reviews.

## 3. Audit chain anchoring

The chain proves internal consistency. Deleting the newest rows leaves a valid
shorter chain, so anchor the head outside the database:

1. On a schedule (at least daily), call `GET /api/audits/verify`.
2. Alert if `ok` is false.
3. Write `head` and `rows` to storage the database operators cannot alter
   (object-lock bucket, WORM log, or a separate team's system).
4. On investigation, compare the stored `head` with a recomputed chain.

Do not disable the `audit_logs_append_only` trigger. Dropping it needs the
table owner, so keep the owner role out of day-to-day use.

## 4. Key management

- Token verification keys come from the identity provider's key endpoint and
  rotate there. Confirm rotation works in staging before go-live.
- Secrets (database URL, SMTP, any signing keys) come from your secret manager
  at runtime, never baked into images or committed.
- Record who can read each secret and review quarterly.

### Rotating the field keys

1. Add a new key **first** in `CRM_FIELD_KEYS`, keeping the old one after it.
2. Roll the service so every replica has both.
3. Run `cargo loco task rotate_field_keys` (idempotent; it also seals any
   legacy plaintext) and confirm it reports zero on a second run.
4. Only then remove the old key. A value sealed under a removed key cannot be
   read and the service returns an error for that row.

## 5. Logging and monitoring

- Logs are JSON on stdout; ship them to a central store with at least the
  retention your regime requires (HIPAA documentation: six years;
  adjust to local law). Logs carry no request bodies by default.
- Alert on: failed authentication spikes, `ok:false` from audit verify, 5xx
  rate, database connection exhaustion, backup failure, certificate expiry.
- Scrape metrics from the monitoring network only.

## 6. Vulnerability and change management

- CI runs `cargo audit`, `cargo deny`, `pnpm audit`, and publishes SBOMs on
  every change. Review a failure the same day.
- Dependabot (`.github/dependabot.yml`) opens weekly pull requests for Cargo,
  npm, the pinned base-image digests, and the GitHub Actions. Digest pins do
  not update themselves; without these pull requests they would freeze on
  vulnerable layers.
- Patch critical dependency advisories within 72 hours, high within 14 days,
  or record a risk acceptance.
- Every change lands by pull request; the sole-maintainer limit is stated in
  [GOVERNANCE.md](../../GOVERNANCE.md). A deployer needing two-person review
  should add it at their fork or deployment pipeline.

### Verify a release before deploying it

A tag `vX.Y.Z` runs `.github/workflows/release.yml`, which pushes
`ghcr.io/<owner>/<repo>/service` and `/ui`, signs each image by digest with
keyless cosign (the workflow's OIDC identity; no key to store), and attaches a
build-provenance attestation and signed CycloneDX SBOMs. Before deploying,
verify the image you will run:

> **Cosign version.** Releases up to v0.1.1 were signed with cosign 2; from v0.1.2 the
> workflow installs cosign 3 (cosign-installer 4). Verify with cosign 3 or later. Checked
> on `v0.1.2-rc.1` (2026-10-20): cosign 3.1.3 verifies both the cosign 3 signatures and the
> older cosign 2 ones (v0.1.1); cosign 2.5.3 does **not** verify the new ones (it reports
> "no signatures found"). Upgrade the verifier before upgrading what you deploy.

```bash
cosign verify ghcr.io/<owner>/<repo>/service:vX.Y.Z \
  --certificate-identity-regexp '^https://github.com/<owner>/<repo>/\.github/workflows/release\.yml@refs/tags/v' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
gh attestation verify oci://ghcr.io/<owner>/<repo>/service:vX.Y.Z --owner <owner>
```

Deploy by digest (`image@sha256:...`), not by tag. Refuse an image that fails
either check.

## 7. Incident response

1. **Detect and contain:** revoke the affected credentials, isolate the
   service, preserve logs and a database snapshot.
2. **Assess:** run `GET /api/audits/verify`; use `GET /api/audits/{pid}` and
   the owner-scoped audit query to determine what was read or changed. Note
   that most reads are not audited (CRM-T40).
3. **Notify:** supervisory authority within 72 hours of awareness where
   personal data is at risk (GDPR Art. 33); affected individuals without undue
   delay if high risk (Art. 34); HIPAA breach rule: individuals within 60 days
   (§164.404) and HHS per §164.408; sector rules (NHS DSPT, FedRAMP incident
   reporting) as applicable.
4. **Recover and review:** restore from a verified backup if needed, rotate
   secrets, write a post-incident review, update this runbook.

Reporting a vulnerability in the software itself: [SECURITY.md](../../SECURITY.md).

## 8. Before go-live checklist

- [ ] Authentication on; policy mounted; persona matrix tested (CRM-G1)
- [ ] GDPR/PECR and send-path review complete (CRM-G2); ESP contract signed
- [ ] DPIA complete and signed off
- [ ] Encryption at rest and in transit verified
- [ ] Backup restore tested; audit chain verifies after restore
- [ ] Chain-head anchoring running
- [ ] Penetration test done; findings closed (CRM-T42)
- [ ] Accessibility statement published; manual assistive-technology test done
- [ ] Incident contacts and on-call defined

## Chaos test before a release

Run `scripts/chaos.sh` (needs podman, python3 and a built service binary). It uses its own
containers, a small TCP proxy (`scripts/chaos-proxy.py`) between the service and PostgreSQL, and
its own database, and removes them afterwards. It breaks, one at a time, and checks that the
service recovers without a restart and that no merge is left half done (CRM-R25, CRM-T65, CRM-T71):

- Redis stopped and frozen; PostgreSQL stopped and frozen.
- The network to PostgreSQL: packets dropped (no answer), connections reset, and slow (100 ms and
  4 s per chunk).
- Eight merges racing for one contact (exactly one wins).
- The service killed with SIGKILL in the middle of a burst of merges.
- The service killed **just after a COMMIT**, before it can answer: the merge is committed once,
  with one audit row and one outbox event; a retry is refused as already merged. The same for a
  create retried with an `Idempotency-Key`: one contact, and the retry creates no second.
- Failover: the primary is killed, a streaming replica is promoted, and the proxy is pointed at
  it. Writes acknowledged after the replica had caught up all survive; writes acknowledged an
  instant before the kill are counted and reported (informational, because replication is
  asynchronous). Merges and the audit chain verify on the promoted database.

Last run 2026-10-20: 71 checks passed.

**What it does not cover:** a full disk, a clock jump, a replica that lags by minutes, a
failover orchestrated by a real tool (Patroni, a managed service) or an ingress, TLS, or more
than one service replica. One finding to know: a create that crashed after committing, retried
with the same `Idempotency-Key` within two minutes, gets `409` (the first attempt looks still
running). The service takes such a key over after two minutes, and a retry then would run
again and could create a duplicate. That window is in the design (`src/idempotency.rs`); the
script does not wait it out.

### Running a built image

`scripts/image-smoke.sh [IMAGE]` runs an image as a deployment would and uses it: production mode
with a read-only root, no capabilities, non-root, authentication on with real signed tokens (admin,
rep and reader personas), field encryption required, the event outbox, the image's own `db migrate`,
then a workflow (create, merge, unmerge, a link, the audit chain) and a second phase with a mail
server where a campaign run stays queued until a `--worker` container starts. Production refuses
unverified SMTP certificates, so phase 2 runs the same image in development mode. 27 checks. Run it
against a candidate image before tagging. A private ghcr.io package needs a login first
(`gh auth token | podman login ghcr.io -u <user> --password-stdin`).

On 2026-10-20 it ran the released v0.1.3 image: phase 1 passed, and phase 2 found the worker bug
above. It passed 27 of 27 on a local build with the fix. The released image is `linux/amd64` only
(podman warned on an arm64 host and ran it under emulation).

### Migrating a database that holds data

`scripts/migrate-with-data.sh` seeds a database through the API, rolls it back one
migration, migrates forward with the data in place, then merges 150 pairs and checks the
result. Run it before shipping a migration that touches an existing table. On
2026-10-20 it showed migration 17 changing no existing row and taking about 13 ms at 300
contacts (adding a nullable column is a metadata change in PostgreSQL, so it should not
grow with table size; that is reasoning, not a measurement on a large table).
