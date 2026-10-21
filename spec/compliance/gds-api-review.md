# API review against UK government API standards (draft)

Status: self-review, 2026-10-10; checked against the published text on
2026-10-21 (section below), for CRM-T49. The check was made by me, not by an
independent reviewer, and it is not an assessment outcome. The table below is
the original self-review; where the published text disagrees, the section
"Check against the published standard" is the later and correct position.

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


## Check against the published standard (2026-10-21)

Read from [GDS API technical and data standards](https://www.gov.uk/guidance/gds-api-technical-and-data-standards)
and [Setting API service levels](https://www.gov.uk/guidance/setting-api-service-levels)
on 2026-10-21. A search showed the older "v1 2018" and "v2 2019" publications
are superseded by this guidance page; I did not open them. The page has no
pagination, idempotency or HSTS requirement, so the table's claims on those
are good practice, not conformance. Findings come from reading the repository
(`grep` over `src/` and `config/`); I did not run requests to confirm them.

| Published requirement | Position | Note |
|---|---|---|
| Use JSON, UTF-8, ISO 8601 dates | Met, not stated | The OpenAPI document does not say dates are ISO 8601 |
| Consistent resource names, persistent resources | Met as far as read | Plural collection names throughout; pids are stable |
| Standard HTTP status codes, documented, consistent, no attacker-useful detail | Met | RFC 9457 bodies; codes documented in OpenAPI |
| TLS 1.2 or above | Not the service's job | The service speaks plain HTTP behind a proxy; the runbook must say the proxy enforces TLS 1.2+. Check it says so |
| Authentication on every request; avoid anonymous endpoints | Met | Only probes, metrics and docs are public |
| Use OAuth 2.0; never basic auth; avoid long-lived API keys | **Gap** | The service accepts bearer JWTs it can mint itself; there is no OAuth 2.0 authorisation server, client-credentials or PKCE flow. Whether the tokens are short-lived should be stated. Deployers normally put an OAuth provider in front |
| Check scope on each call | Met | Role and scope checks per route (see control-mapping.md) |
| Version the API; add the version to the URI; avoid custom headers or media types, which "can lead to your API being blocked by proxies or firewalls" | **Contradicted** | We version with the `Accepts-version` header and put no version in the URL. The earlier table row called this "Met, with a risk"; against the published text it is a deviation. The cost of moving is a breaking change to every client, so decide it deliberately |
| Keep few active versions; tell users when a version is retired | Met (policy) | api-lifecycle.md; `Deprecation`/`Sunset` headers are not built yet |
| Reject unknown attributes (schema validation on de-serialising endpoints) | **Gap** | No `deny_unknown_fields` anywhere in `src/`, so serde ignores unknown request members. A misspelt field is silently dropped |
| Enforce content types and reject others | Probably met, untested | Axum's `Json` extractor answers 415 for a wrong `Content-Type`; no test here pins it |
| Only allow the verbs each resource supports | Met | Router answers 405 as a problem document (tested) |
| Rate limit anonymous endpoints | Partly met | Rate limiting exists; whether the public endpoints (unsubscribe, webhooks, consent) are covered should be checked |
| Tell users when they approach limits (for example `X-RateLimit-Remaining`) | **Gap** | No rate-limit headers; only `429` with `Retry-After` |
| Make responses cacheable; state policy with `Cache-Control`/`Vary`; never cache sensitive or user-specific data | **Gap, by design** | No `Cache-Control` anywhere in `src/`. Responses hold personal data, so caching is rightly off, but the policy is not stated: send `Cache-Control: no-store` on authenticated responses |
| Log when personal data is provided and to whom | Met | Hash-chained audit log of reads and writes (verify with `/api/audits/verify`) |
| Monitor for unusual activity | Partly met | Metrics and break-glass alerts; no alerting rules shipped |
| Provide a test service with no real personal data | Partly met | The demo mode serves fictitious data; no hosted test service |
| Document the API; OpenAPI early | Met | Enforced by the route-coverage test |
| Use an API management system or gateway | Not applicable | A deployer's choice |
| Host on an `api.gov.uk` name | Not applicable | Deployment |

### What the check changed

- The versioning row moves from "met" to "deviates from the published text".
- Four gaps are real and small: unknown-field rejection, `Cache-Control:
  no-store`, rate-limit headers, and a statement of the OAuth 2.0 position.
- The "no pagination or idempotency in the standard" finding means T51 and
  T52 stay as good practice but are not conformance evidence.

### Open after this check

Nothing here is fixed in code yet. Candidates, in order of value: add
`Cache-Control: no-store` to authenticated responses; reject unknown request
members; add `RateLimit-*` headers; decide the versioning question; write the
OAuth 2.0 deployment pattern into the runbook. An independent reviewer is
still wanted before anyone describes this as conformant.
