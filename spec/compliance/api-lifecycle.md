# API errors, versioning and deprecation

This is the written policy the GDS API review asked for. It covers what an
error looks like, what counts as a breaking change, and how an endpoint,
field or version is retired. It is a policy adopted by the maintainer for
this demo; the notice period below is a choice, not a requirement from any
standard.

## Errors

Every `4xx` and `5xx` response is an
[RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem document with the
media type `application/problem+json`. One middleware (`src/problem.rs`)
produces it for every source: controllers, the auth and rate-limit
middleware, the version check, and the router's own `404` and `405`.

| Member | Meaning |
| --- | --- |
| `type` | `about:blank` when the status says it all; otherwise `urn:crm:problem:<code>` (for example `urn:crm:problem:validation`). The URN identifies the kind of problem; it is not a web page |
| `title` | The status reason phrase |
| `status` | The HTTP status |
| `detail` | The human-readable message for this occurrence |
| `instance` | The request path |
| `error` | The stable machine code (for example `validation`, `forbidden`, `not_found`) |
| `description` | The same text as `detail` |

`error` and `description` are the members the service sent before this
change, and they stay so existing clients keep working. Only the media type
changed (`application/json` to `application/problem+json`). A client should
branch on `status` and `type`, and show `detail`. Members a source adds, such
as `supported` on a refused API version, are kept as extension members.

A `401` carries `WWW-Authenticate: Bearer`. A `429` carries `Retry-After`. A
`409` for an idempotency key still in flight carries `Retry-After`.

**Limit:** the tests cover the application and router errors listed above and
the body-size limit's `413`. The request timeout and the panic handler are not
tested, so whether they produce a problem document is unverified.

## Versioning

The API version is chosen with the `Accepts-version` request header (today
only `1.0`). URLs carry no version, by decision: the path stays `/api`, never `/api/v1` (the GDS guidance prefers a version in the URI; this is a recorded deviation). A request that names an unsupported
version gets `406` with the supported list. The response names the version
served.

## What is a breaking change

A change that can break a correct client is breaking and needs a new major
version:

- removing or renaming a path, method, query parameter, header, field or
  response member;
- making an optional request field required, or narrowing what a field accepts;
- changing a field's type, units or meaning (for example money moving off
  minor units);
- changing a status code or the machine code in `error` for a given situation;
- removing a value from a closed list, or changing a state machine so a
  previously legal transition is refused.

These are not breaking, and clients must tolerate them:

- new endpoints, new optional request fields, new response members;
- new values in a response list of tokens (a new status, kind or channel);
- new `4xx` situations that were already covered by an existing code;
- tightening a bug (a validation that the documentation already promised).

Clients must ignore response members they do not know.

## Deprecation

Retiring something takes these steps, in order:

1. **Announce.** Record it in `CHANGELOG.md` and the service changelog, and
   mark the operation or field `deprecated: true` in the OpenAPI document with
   a description naming the replacement.
2. **Signal in responses.** Send `Deprecation: @<unix time>` from the day it
   is deprecated ([RFC 9745](https://www.rfc-editor.org/rfc/rfc9745)), a
   `Link: <url>; rel="deprecation"` header pointing at the replacement or
   migration note, and, once the removal date is fixed, `Sunset: <HTTP date>`
   ([RFC 8594](https://www.rfc-editor.org/rfc/rfc8594)).
3. **Notice period.** At least 180 days from step 1 to the sunset date. A
   security fix may shorten this, and the changelog says so when it does.
4. **Overlap.** For a version change, both versions are listed in
   `SUPPORTED_API_VERSIONS` until the sunset date, so a client moves by changing
   its `Accepts-version` header.
5. **Remove.** After the sunset date the old behaviour is removed in the next
   major version, and the changelog records the removal.

## Status

- Nothing is deprecated today, so the service sends no `Deprecation` or
  `Sunset` headers and the OpenAPI document has no `deprecated: true` entries.
- The headers are not built yet. Building them is part of the first
  deprecation: add a table of deprecated operations, a middleware that adds
  the headers, and a test that fails when an operation is marked deprecated in
  the document without them.
- The `error` and `description` members fall under this policy. Removing them
  is a breaking change and follows the steps above.
- This policy has not been checked against the standards' published text by a
  reviewer (see [gds-api-review.md](gds-api-review.md)).
