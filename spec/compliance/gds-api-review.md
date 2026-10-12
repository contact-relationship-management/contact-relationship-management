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
| Machine-readable description | Met for the main resources | OpenAPI 3.0.3 at `/api-docs/openapi.json`, Swagger UI; request schemas for every operation that takes a body; resource schemas for the 11 lists; response schemas for the detail reads, dashboards, forecast, insight views, audit, retention, privacy and break-glass; a shared `Error` schema | Hand-written, checked against real responses (including masked ones). Operations whose response is the updated record (state transitions) are still untyped |
| Documentation completeness | Met, and enforced | Before this review, 22 mounted paths were missing; later two more (the `stakeholder` routes) were found by the new test. `every_mounted_route_is_documented_and_the_reverse` compares Loco's real route table with the document in both directions | Only the probes and the documentation endpoints are exempt, by an explicit list in the test |
| Authentication stated | Met after this review | `bearerAuth` scheme and global `security` added; probes, metrics and docs stay public | none |
| Error responses stated | Met after this review | 401, 403, 406 and 429 added to every `/api/*` operation | Bodies are RFC 9457 `application/problem+json` (CRM-T54), with the earlier `error` and `description` members kept. The body-size `413` is converted and tested; the timeout and panic handler are untested |
| Versioning | Met, with a risk | `Accepts-version` header, `406` for an unsupported value, response stamped with the version | A client that sends no header gets the current version, so it breaks silently when 2.0 ships; no deprecation or sunset policy is written down |
| Pagination | Met after this review (CRM-T51) | The seven list endpoints take `limit` and `cursor` and answer `Link: rel="next"` | Offset cursor (can skip or repeat a row if data changes mid-walk); no total count by design; timelines and feeds are bounded, not paged |
| Idempotent retries | Met after this review (CRM-T52), opt-in | `POST` accepts `Idempotency-Key`; a retry replays the first success | Clients must send the key; keys are per caller and kept 24 hours; only `2xx` responses are replayed |
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

Idempotency keys (CRM-T52), request, resource and response schemas, the
route-coverage test (CRM-T53), and RFC 9457 error bodies with a written
deprecation policy (CRM-T54, [api-lifecycle.md](api-lifecycle.md)) were built
after this review. What remains is a reviewer's check of this draft against
the standards' published text, and building the `Deprecation` and `Sunset`
headers when something is first deprecated.
