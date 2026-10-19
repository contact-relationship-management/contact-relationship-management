# For the Chief Technology Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Architecture

- **Service:** Rust on Loco (Axum, SeaORM 2.0), PostgreSQL 18, Redis for cache,
  shared counters and break-glass propagation. `forbid(unsafe_code)`, clippy
  pedantic. Source of truth for behavior is the specification
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **UI:** SvelteKit 3 / Svelte 5, a backend-for-frontend proxy so no token ever
  reaches browser JavaScript, strict CSP with hashes.
- **Auth:** PASETO v4.public verified offline, optional Keycloak; attribute-based
  access control (read/write/delete/destructive actions, a `mask` obligation,
  record-level checks) from a shared engine.
- **Audit:** SHA-256 hash chain written in the same transaction as the change,
  append-only trigger, verifiable at `GET /api/audits/verify`.
- **Data protection:** AES-256-GCM field encryption with key rotation on two
  columns; local keys, no KMS.

## Quality evidence

Unit tests (176), database tests (44 including enforcement, masking, audit,
pagination, caching, idempotency, schema checks), front-end unit (103) and
Playwright (90) including axe
accessibility scans. CI runs fmt, clippy, tests, front-end build/lint and a
supply-chain stage (`cargo audit`, `cargo deny`, `pnpm audit`, CycloneDX SBOMs)
on GitHub Actions and Woodpecker. One command reproduces it: `scripts/ci-check.sh`.

## Technical debt and honest limits

- Pagination uses an offset cursor (rows can skip or repeat if data changes
  mid-walk); no total count by design.
- Idempotency keys are opt-in at the API (T52): a client must send
  `Idempotency-Key` on a create to be protected from a retry duplicating a
  record. The bundled UI sends one on every create.
- OpenAPI schemas are hand-written. They cover requests, lists, detail reads,
  dashboards, insight views, audit, privacy and break-glass; state transitions
  and upserts that return the updated record are untyped. A test fails when a
  route and the document disagree, and others check real responses, masked ones
  included, against the schemas
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Errors are RFC 9457 problem documents (T54) with the earlier `error` and
  `description` members kept; errors from layers outside the router (size
  limit, timeout) are not converted. The versioning and deprecation policy is
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); the
  `Deprecation` and `Sunset` headers are specified but not built.
- Field encryption covers two columns; no KMS integration.
- Rate-limit and cache state need Redis for multi-replica correctness.
- Insight views are masked by role but cached per caller token for 30 s.
- Step-up re-authentication before destructive actions needs identity-provider work.

## Recommended technical sequence

1. KMS-backed field keys and broader field coverage (T41 remainder).
2. Tag a first release to prove keyless signing and verification (T49), and
   have a reviewer check the API standards draft.
3. Build the `Deprecation` and `Sunset` headers with the first deprecation.
4. Commission the independent test (T42) against a staging build.
