# Security Policy

This is demo software: it holds synthetic data only, and sends no real email
unless a real mail mode is configured (see [INSTALL.md](INSTALL.md#sending-email)).
It is not intended for production exposure until the production gates in
[spec/tasks.md](spec/tasks.md) (CRM-G1, authentication activation; CRM-G2,
GDPR/PECR review) are closed.

## Reporting a vulnerability

Please report privately by email to <joel@joelparkerhenderson.com>, or by a
GitHub private security advisory if the repository has them enabled. Do not
open a public issue for a vulnerability.

Include what you found, how to reproduce it, and the affected version or commit.

## What to expect

- Acknowledgement within 7 days.
- An assessment and a fix or mitigation plan, as soon as practical, from the
  sole maintainer.
- Credit in [CHANGELOG.md](CHANGELOG.md) if you want it.

## Supported versions

Only the latest commit on `main` is supported.

## Supply chain

Dependencies are pinned by lockfile (`Cargo.lock`, `pnpm-lock.yaml`).
`scripts/ci-check.sh supply-chain` checks known vulnerabilities, licences and
sources, and writes the SBOMs to `sbom/`. The Rust service and its migration
crate forbid `unsafe` code (`#![forbid(unsafe_code)]`), and the service
deliberately omits loco's `auth` (JWT) feature; see the comment in its
`Cargo.toml`.

## Inbound requests that are not signed in

These routes are public even with `CRM_REQUIRE_AUTH` on, and each checks a
secret of its own:

- `POST /webhooks/email` (`CRM_WEBHOOK_SECRET`; `X-CRM-Timestamp` and
  `X-CRM-Signature`), and the provider callbacks under `/webhooks/email/`
  (see [INSTALL.md](INSTALL.md#sending-email) for how each is checked);
- `POST /api/bridges/wpm/events` (`CRM_WPM_WEBHOOK_SECRET`; `x-wpm-signature`,
  HMAC-SHA256 over `<t>.<body>`, five-minute window), see the
  [family bridges tutorial](docs/tutorial-family-bridges.md).

`POST /webhooks/email` and `POST /api/bridges/wpm/events` answer `404` while
their secret is unset.
