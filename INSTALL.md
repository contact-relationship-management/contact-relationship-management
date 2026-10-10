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
