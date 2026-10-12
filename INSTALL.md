# Install and use

## Prerequisites

- Rust 1.96 (see `rust-toolchain.toml`)
- PostgreSQL 18 (or `docker compose`)
- Node.js and pnpm, for the front end

## Rust service

```bash
cd contact-relationship-management-service-with-rust
docker compose -f compose.test.yaml up -d   # test PostgreSQL
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
| `CRM_MAIL_MODE` | `simulated` (default), `smtp`, `brevo` or `mailjet` |
| `CRM_BREVO_API_KEY` | `brevo` mode: the API key (sent as `api-key`) |
| `CRM_MAILJET_API_KEY`, `CRM_MAILJET_API_SECRET` | `mailjet` mode: the API key pair (basic auth) |
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
basic auth), with `CRM_WEBHOOK_SECRET` as the secret. These adapters follow the
vendors' documentation and have not been run against live accounts.

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

```bash
cd contact-relationship-management-ui-with-svelte
pnpm install
pnpm dev
```

See its [README](contact-relationship-management-ui-with-svelte/README.md).

## Whole-repo check

```bash
scripts/ci-check.sh
```

This is demo software; use synthetic data only.
