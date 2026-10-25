# Install and use

## Prerequisites

- Rust 1.99 (see `rust-toolchain.toml`)
- PostgreSQL 18 (or `podman compose`)
- Node.js and pnpm, for the front end

## Rust service

```bash
cd contact-relationship-management-service-with-rust
podman compose -f compose.test.yaml up -d   # test PostgreSQL and Redis
cargo build
cargo test                                   # DB-free unit and request tests
cargo test -- --ignored                      # database-backed tests
```

See the service [README](contact-relationship-management-service-with-rust/README.md)
for the quick start, configuration, and endpoints.

## Sending email

By default the service sends nothing: campaign sends are simulated. To send
real mail, set `CRM_MAIL_MODE=smtp` and the `CRM_SMTP_*` variables. The
service refuses to start with a bad combination.

| Variable | Meaning |
|---|---|
| `CRM_MAIL_MODE` | `simulated` (default), `smtp`, `brevo`, `mailjet`, `ses`, `azure`, `mailgun` or `sendgrid` |
| `CRM_BREVO_API_KEY` | `brevo` mode: the API key (sent as `api-key`) |
| `CRM_MAILJET_API_KEY`, `CRM_MAILJET_API_SECRET` | `mailjet` mode: the API key pair (basic auth) |
| `CRM_SES_REGION`, `CRM_SES_ACCESS_KEY_ID`, `CRM_SES_SECRET_ACCESS_KEY` | `ses` mode (optional `CRM_SES_SESSION_TOKEN`, and `CRM_SES_CONFIGURATION_SET`, which SES events need). Static keys only; no instance-role lookup |
| `CRM_AZURE_ENDPOINT`, `CRM_AZURE_ACCESS_KEY` | `azure` mode: the Communication Services endpoint and its base64 access key; `CRM_MAIL_FROM` must be a verified sender address |
| `CRM_MAILGUN_API_KEY`, `CRM_MAILGUN_DOMAIN` | `mailgun` mode (`CRM_MAILGUN_API_BASE=https://api.eu.mailgun.net` for the EU region) |
| `CRM_MAILGUN_SIGNING_KEY` | Mailgun's webhook signing key; Mailgun callbacks are verified with it (not with `CRM_WEBHOOK_SECRET`) |
| `CRM_SENDGRID_API_KEY` | `sendgrid` mode |
| `CRM_MAIL_API_URL` | Override the vendor endpoint (a test double); `https` in production |
| `CRM_SMTP_HOST`, `CRM_SMTP_PORT` | The server; port defaults to 587 (465 with `implicit`) |
| `CRM_SMTP_TLS` | `starttls` (default), `implicit` or `none` |
| `CRM_SMTP_USERNAME`, `CRM_SMTP_PASSWORD` | Set together, or neither |
| `CRM_MAIL_FROM` | The `From` mailbox, for example `CRM <crm@example.org>` |
| `CRM_MAIL_BATCH` | Contacts sent per batch (default 200); the job runs batches until done |
| `CRM_SMTP_ACCEPT_INVALID_CERTS` | Development only; refused in production |
| `CRM_UNSUBSCRIBE_SECRET` | 32+ characters; signs the unsubscribe link in every message. Required with `smtp` |
| `CRM_PUBLIC_URL` | The address recipients reach this service at, for the link (`https` in production). Required with `smtp` |
| `CRM_WEBHOOK_SECRET` | 32+ characters; enables `POST /webhooks/email` for provider outcomes (delivered, hard or soft bounce, complaint). Requests carry `X-CRM-Timestamp` and `X-CRM-Signature: sha256=` (HMAC-SHA256 of `timestamp.body`); five-minute window |
| `CRM_MAIL_ALLOW_PRODUCTION` | Required to use any real mode in production, because the vendor, agreement and legal review is open (CRM-T39) |
| `CRM_MAIL_DEV_DOMAIN` | Development only: in `stub` upstream mode, mail goes to `<person id>@<domain>` |

With a real sender, `POST /api/campaigns/{pid}/run` checks the campaign and
hands the sending to a background job, answering `202` while it runs (`200`
once complete). In production, `workers.mode` is `BackgroundQueue` (jobs are
kept in PostgreSQL), so run a worker: `cargo run -- start --server-and-worker`
in development, or a separate `start --worker` process. A campaign whose
remaining sends all fail stops (stays `running`); call `run` again to resume.
Simulated mode, and the test configuration, still finish inside the request.

`POST /api/nurture/advance` mails due nurture steps through the same sender;
each step then needs a `body`.

`brevo` and `mailjet` send through the vendor's HTTP API with `CRM_MAIL_FROM`
and the same unsubscribe and webhook settings. Point the vendor's callback at
`POST {CRM_PUBLIC_URL}/webhooks/email/brevo` (bearer token, or a custom header
`X-CRM-Webhook-Token`) or `/webhooks/email/mailjet` (`https://crm:SECRET@host/…`
basic auth), with `CRM_WEBHOOK_SECRET` as the secret. For the clouds:
SES publishes events to an SNS topic (add an HTTPS subscription to
`/webhooks/email/ses` with `https://crm:SECRET@host/…`; the service confirms the
subscription itself, for AWS hosts only; SNS message signatures are not
verified); Azure sends delivery reports through Event Grid (a webhook
subscription to `/webhooks/email/azure` with `X-CRM-Webhook-Token` as a
delivery property). Mailgun callbacks go to `/webhooks/email/mailgun` and
SendGrid's Event Webhook to `/webhooks/email/sendgrid` (basic auth in the URL;
SendGrid's ECDSA signature is not verified). Google Cloud has no email-sending
service of its own: use SendGrid or Mailgun, or its SMTP relay with
`CRM_MAIL_MODE=smtp` (no outcome callbacks). These adapters follow the vendors'
documentation and have not been run against live accounts.

The CRM holds no email addresses. In `CRM_UPSTREAM_MODE=http` it asks the
person service for an `email` field at send time and stores nothing.

### Local mail server

`scripts/email-dev.sh` runs [Stalwart](https://stalw.art) in Podman and
configures a domain `crm.test` (with a catch-all) and one account:

```bash
scripts/email-dev.sh up      # start and configure (safe to repeat)
scripts/email-dev.sh check   # send a message and read it back
scripts/email-dev.sh live    # run the service's real-SMTP test against it
scripts/email-dev.sh down    # stop; mail is kept
scripts/email-dev.sh reset   # stop and delete all mail
```

To send a campaign through it from the service:

```bash
CRM_MAIL_MODE=smtp CRM_SMTP_HOST=127.0.0.1 CRM_SMTP_PORT=2587 \
CRM_SMTP_USERNAME=crm@crm.test CRM_SMTP_PASSWORD=crm-dev-password \
CRM_MAIL_FROM='CRM <crm@crm.test>' CRM_SMTP_ACCEPT_INVALID_CERTS=1 \
CRM_MAIL_DEV_DOMAIN=crm.test \
CRM_UNSUBSCRIBE_SECRET=dev-only-unsubscribe-secret-0123456789 \
CRM_PUBLIC_URL=http://localhost:5150 cargo run -- start
```

Mail lands in the `crm@crm.test` mailbox; read it over JMAP at
`http://127.0.0.1:8081` (`crm@crm.test` / `crm-dev-password`) or IMAPS on
`2993`. The web admin is the same address (`admin` / `dev-admin-change-me`).
**Development only**: fixed passwords, a self-signed certificate, loopback
ports. Details are in
`contact-relationship-management-service-with-rust/compose.email.yaml`.

## Svelte front end

**Guided tour:** start the UI and sign in as `example@example.com`. No email
is sent and no service is needed; every screen shows fictitious data (set
`CRM_DEMO_TOUR=off` to remove this). Each sign-in gets its own copy of the
data, held in memory (at most 200 at a time).

```bash
cd contact-relationship-management-ui-with-svelte
pnpm install
pnpm dev
```

See its [README](contact-relationship-management-ui-with-svelte/README.md).

## Whole-repo check

```bash
scripts/ci-check.sh <stage>
```

GitHub Actions and Woodpecker run the same stages, so a local pass is the CI pass. The stages:

| Stage | What it checks |
|---|---|
| `fmt`, `clippy` | Rust formatting and lints (warnings are errors), both feature sets |
| `test`, `test-db` | unit tests; the request tests against PostgreSQL |
| `front-end` | `svelte-check`, unit tests with coverage, the build, then `pnpm lint` (Prettier and ESLint) |
| `e2e` | Playwright in a browser, including the guided tour |
| `lint` | spelling (`typos`), GitHub workflow security (`zizmor`) and Markdown (`rumdl`) |
| `mutants` | `cargo mutants` over `src/rules`: it changes each rule a little, and a change no test notices is reported as missed. Slow (hours), so GitHub runs it weekly; run it on one file with `scripts/ci-check.sh mutants --no-config -f src/rules/<file>.rs -- --lib` |
| `api-types` | the UI's generated API types match the service's OpenAPI document |
| `supply-chain` | known vulnerabilities, licences and sources for Rust and the UI, and the SBOMs written to `sbom/` |
| `semver` | the matcher crate's public API against its last published release |
| `scripts` | `shellcheck` on `scripts/*.sh` |
| `coverage` | a Rust line-coverage report from the unit tests (a report, not a gate) |

`typos`, `zizmor` and `rumdl` are installed with `cargo install typos-cli zizmor rumdl`.

This is demo software; use synthetic data only.
