# For the Chief Technology Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Architecture

- **Service:** Rust on Loco (Axum, SeaORM 2.0), PostgreSQL 18, Redis for cache,
  shared counters and break-glass propagation. `forbid(unsafe_code)`, clippy
  pedantic. Source of truth for behavior is the specification
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **UI:** SvelteKit 2 / Svelte 5, a backend-for-frontend proxy so no token ever
  reaches browser JavaScript, strict CSP with hashes.
- **Auth:** PASETO v4.public verified offline, optional Keycloak; attribute-based
  access control (read/write/delete/destructive actions, a `mask` obligation,
  record-level checks) from a shared engine.
- **Audit:** SHA-256 hash chain written in the same transaction as the change,
  append-only trigger, verifiable at `GET /api/audits/verify`.
- **Data protection:** AES-256-GCM field encryption with key rotation on two
  columns; local keys, no KMS.

## Quality evidence

Unit tests (113), database tests (17 including enforcement, masking, audit,
pagination, caching), front-end unit (35) and Playwright (39) including axe
accessibility scans. CI runs fmt, clippy, tests, front-end build/lint and a
supply-chain stage (`cargo audit`, `cargo deny`, `pnpm audit`, CycloneDX SBOMs)
on GitHub Actions and Woodpecker. One command reproduces it: `scripts/ci-check.sh`.

## Technical debt and honest limits

- Pagination uses an offset cursor (rows can skip or repeat if data changes
  mid-walk); no total count by design.
- Idempotency keys for creates are missing (T52).
- OpenAPI is hand-written and has no request/response schemas; nothing fails the
  build when a route is undocumented ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Errors are Loco's `{error, description}`, not RFC 9457.
- Field encryption covers two columns; no KMS integration.
- Rate-limit and cache state need Redis for multi-replica correctness.
- Insight views are masked by role but cached per caller token for 30 s.
- Step-up re-authentication before destructive actions needs identity-provider work.

## Recommended technical sequence

1. Idempotency keys (T52) and OpenAPI schemas plus a route-coverage test.
2. KMS-backed field keys and broader field coverage (T41 remainder).
3. Release-build signing (T49) once key custody is decided.
4. Commission the independent test (T42) against a staging build.
