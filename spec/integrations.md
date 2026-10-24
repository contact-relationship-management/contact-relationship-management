# Integrations

CRM is a consumer of the Main X Index family; it holds **EntityRef
URNs** and never duplicates upstream records.

| Service | Used for | How |
|---|---|---|
| [person-service](../../person/person-service-with-loco/) | contacts | required `person:` URN on Contact (and optional on Lead); display names resolved best-effort |
| [organization-service](../../organization/organization-service-with-loco/) | accounts | required `organization:` URN on Account |
| [worker-service](../../worker/worker-service-with-loco/) | reps & agents | `owner_ref` / `assignee_ref` / `actor_ref` URNs |
| [authentication-service](../../authentication/authentication-service-with-loco/) | SSO + ABAC attrs | offline PASETO via `authentication-verifier`; persona attributes |

## Contract the person service must meet for email

Sending needs a mailbox, and the CRM stores none. At send time it reads the
person record at `GET {CRM_PERSON_SERVICE_URL}/api/persons/{id}` and accepts
`{"email": "a@b"}` or `{"emails": [...]}` (strings, or objects with `address`,
`email` or `value`; an entry marked `primary` or `is_primary` is preferred). A
record with no usable address is recorded as `no_address`. **This contract is
assumed**: the person service was not available to check against, so confirm
the field names, or change `clients::extract_email`, before a real send.

Client modules follow the stub-first pattern (patient-flow
`clients.rs`): read-only, cached, best-effort display-name lookups
that never block writes. The cache is in memory, per process: a name is kept
for 10 minutes and at most 10,000 are held, the oldest going first. A display
name is personal data, so it is not kept for the life of the process, and an
erasure or a rename upstream shows within 10 minutes (CRM-T81).

## Dedup lives upstream

CRM performs **no** contact/account matching: creating a contact
requires an existing `person:` record, so duplicate humans are the
person-service matcher's problem. When upstream merges records and
emits `merged {pid, merged_from}`, CRM repoints its wrappers —
manual repoint endpoint in v1, event-driven on the durable bus as
roadmap.

## Events

Family envelope via the `CRM_EVENT_TRANSPORT` seam (default
`memory`; Postgres outbox rows share the mutation's transaction
under `outbox`). Event kinds are listed in
[domain-model.md](domain-model.md).

## Outbound webhooks (CRM-R28)

Two tables: `webhook_subscriptions` (address, event patterns, the signing secret sealed
with the field keyring, the previous secret and when it stops signing, `enabled` and why
not) and `webhook_deliveries` (one row per subscription per event: status `pending`,
`delivered`, `dead` or `cancelled`, attempts, next attempt, last status and error). A
column `event_outbox.webhook_fanned_out_at` marks outbox rows already turned into
deliveries, so fan-out is a single transaction and a re-run does nothing.

The relay run does two steps: **fan out** (unmarked outbox rows, oldest first, one
delivery per enabled subscription whose patterns match `entity.kind`) and **send** (due
deliveries, `FOR UPDATE SKIP LOCKED`, so two runs never send the same delivery twice at
once). The body is built at send time from the outbox row's ids; it is not stored. The
network is behind a `Sender` trait: the real one resolves, checks and pins the address;
tests use one that records. A subscription's address is checked for shape when it is registered (https, no
credentials, no private IP literal, no local name). Names cannot be judged until they are
resolved, and DNS can change, so the resolution check runs at every send, and the call
connects to the addresses it checked.

Why at least once and not exactly once: a send can succeed and its record fail to be
written. Receivers dedupe on `webhook-id`, which is the event id and is the same on every
retry; `webhook-timestamp` changes per attempt, so a replayed capture is refused by a
receiver that checks the timestamp window.

## MCP for AI agents (CRM-R30)

`POST /mcp` (outside `/api`, so the blanket sign-in layer does not stand in front of it) builds the REST request a tool stands
for and sends it through the service's own router, the same value `after_routes` returns, cloned before the MCP route is
added to it. The caller's `Authorization` header (and `X-Forwarded-For`, so the rate limiter counts the same client) is
copied; nothing else of the original request is. Because the router is the whole stack (version check, abuse guard,
sign-in, access policy, insight cache and audit, idempotency, problem rendering), a tool call is refused, masked,
rate-limited and audited exactly as the REST call it is. The endpoint adds only the protocol, an argument check against a
closed schema, the `mcp_tool_called` audit row (written first), the data notice, and the cut at 200 KB.

Settings: `CRM_MCP_WRITES` (off) lists `add_activity`; `CRM_MCP_ALLOWED_ORIGINS` (none) lists browser origins. A new tool is
one entry in `src/mcp/tools.rs`; a test fails if it names a route that is not documented, if a non-GET tool is neither a
dry run nor the write tool, or if its path mentions erase, webhooks or break-glass.

Why not a separate MCP process calling the HTTP API: it would need its own copy of the caller's token handling and could not
stay in step with the access policy by construction. Why stateless: the 2026-07-28 revision removed sessions, so there is
nothing to keep per client, and for the older versions no `Mcp-Session-Id` is issued, which the specification allows.
