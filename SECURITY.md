# Security Policy

This is demo software: it holds synthetic data only and sends no real email.
It is not intended for production exposure until the production gates in
[spec/tasks.md](spec/tasks.md) (authentication activation, GDPR/PECR review)
are closed.

## Reporting a vulnerability

Please report privately by email to joel@joelparkerhenderson.com, or by a
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

Dependencies are pinned by lockfile (`Cargo.lock`, `pnpm-lock.yaml`). The Rust
service forbids `unsafe` code where practical and deliberately omits loco's JWT
feature; see the comments in its `Cargo.toml`.
