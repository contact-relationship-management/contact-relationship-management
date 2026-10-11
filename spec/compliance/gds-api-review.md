# API review against UK government API standards (draft)

Status: self-review, 2026-10-10, for CRM-T49. **Caveat:** I wrote this from
the standards' well-known principles and from reading this repository. I did
not fetch or quote the standards' text, so a reviewer must check each row
against the current published wording before relying on it. This is not an
assessment outcome.

Scope: the REST API in `contact-relationship-management-service-with-rust`
(`/api/*`).

| Area | Position | Evidence | Gap |
|---|---|---|---|
| Machine-readable description | Partly met | OpenAPI 3.0.3 at `/api-docs/openapi.json`, Swagger UI | Hand-written and summary-level: operations have summaries but no request or response schemas |
| Documentation completeness | Met after this review | Before it, 22 mounted paths (all `insights`, `groups`, `partnerships`, `membership`, `audits/verify`) were missing; now only the docs endpoints themselves are undocumented | Nothing enforces it: no test compares the route table to the document |
| Authentication stated | Met after this review | `bearerAuth` scheme and global `security` added; probes, metrics and docs stay public | none |
| Error responses stated | Met after this review | 401, 403, 406 and 429 added to every `/api/*` operation | Bodies are Loco's `{error, description}` shape, not RFC 9457 `application/problem+json` |
| Versioning | Met, with a risk | `Accepts-version` header, `406` for an unsupported value, response stamped with the version | A client that sends no header gets the current version, so it breaks silently when 2.0 ships; no deprecation or sunset policy is written down |
| Pagination | Met after this review (CRM-T51) | The seven list endpoints take `limit` and `cursor` and answer `Link: rel="next"` | Offset cursor (can skip or repeat a row if data changes mid-walk); no total count by design; timelines and feeds are bounded, not paged |
| Idempotent retries | **Not met** | Create endpoints (`POST`) have no idempotency key | A retried create after a timeout makes a duplicate |
| Rate limiting | Partly met | `429` with `Retry-After`; shared across replicas when Redis is configured | No `RateLimit-*` headers telling a client its budget |
| Caching | Partly met | ETag and `If-None-Match` on dashboards and some insights | Not applied to ordinary list and detail reads |
| Data formats | Met (by design) | Money in minor units with ISO 4217; JSON throughout | Date and time conventions are not stated in the OpenAPI document |
| Security | Met as far as tested | See `control-mapping.md`; CORS closed, secure headers, audit chain | Independent testing is CRM-T42 |
| Operability | Met | `/_health`, `/_ping`, `/_readiness`, `/metrics.prom` | none |

## Changes made with this review

- Pagination (CRM-T51) was built after this review; see `tasks.md`.
- `src/openapi.rs`: added the 22 missing paths, the `bearerAuth` scheme and
  global `security`, and the shared 401, 403, 406 and 429 responses; the unit
  test now pins them.

## Recommended next steps

Each is a new open task in `spec/tasks.md`:

1. Idempotency keys (CRM-T52) for `POST` creates.
2. Request and response schemas in the OpenAPI document, and a test that
   fails when a mounted route is undocumented.
3. Decide on RFC 9457 error bodies and a written deprecation policy.
