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

## Local mail server (for the email send work)

The service does not send real email yet (CRM-T39), but the send path needs a
server to talk to. `scripts/email-dev.sh` runs [Stalwart](https://stalw.art) in
Podman and configures a domain `crm.test` with one account:

```bash
scripts/email-dev.sh up      # start and configure (safe to repeat)
scripts/email-dev.sh check   # send a message and read it back
scripts/email-dev.sh down    # stop; mail is kept
scripts/email-dev.sh reset   # stop and delete all mail
```

Submission is `127.0.0.1:2587` (STARTTLS), login `crm@crm.test` /
`crm-dev-password`; the web admin and JMAP are at `http://127.0.0.1:8081`
(`admin` / `dev-admin-change-me`). **Development only**: fixed passwords, a
self-signed certificate, loopback ports. Details are in
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
