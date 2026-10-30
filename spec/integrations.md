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

## Family bridges

The bridges between the CRM, the project portfolio service (PPM) and the worker service (WPM), as of
2026-10-30. Each carries references and the few facts named, never records.

| Bridge | Direction | Contract | State in the CRM |
|---|---|---|---|
| Won deal → delivery idea | CRM → PPM | `POST {CRM_PPM_SERVICE_URL}/api/ideas`; CRM-R72, token CRM-R87 | Built (`POST /api/bridges/ppm/deals/{pid}`), admin only, stub-first; tested against a stand-in |
| Teams from departments | CRM ← WPM (CRM reads) | `GET {CRM_WORKER_SERVICE_URL}/api/workers`; CRM-R73, token `CRM_WORKER_SERVICE_TOKEN` (CRM-R87) | Built (`POST /api/bridges/wpm/teams-import`), admin only, stub-first; tested against a stand-in. **Unresolved:** the teams hold `worker:<pid>` references while the hire and leave events and WPM's overlap check speak of `person:` references, so a team member and a contact cannot be matched; WPM's spec names the same mismatch (WPM cross-app-bridges, "What CRM was read to say") |
| Hire and leave | WPM → CRM (WPM posts) | WPM's outbound webhooks (WPM-R151) with the handoff of WPM-R207; [below](#inbound-wpm-hire-and-leave-events-crm-r92) | Built (`POST /api/bridges/wpm/events`, CRM-R92); tested with requests signed as WPM signs, not with a running WPM |
| Pool link, demand and supply | PPM ↔ WPM | WPM-R210 to R213, WPM-D138 to D141 | Not the CRM's: built in PPM and WPM |
| Overlap check | CRM → WPM | `POST {WPM}/api/bridges/overlap` (WPM-R208): up to 500 `person:` refs in, a yes or no per ref out, under an HR caller's token | Available in WPM; **the CRM client is not built** (next: it would find contacts who were employees before CRM-R92 was running, or who became contacts after their hire) |

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

## Inbound: WPM hire and leave events (CRM-R92)

WPM's `webhook_deliver` task posts each queued event to the subscription's address with
`content-type: application/json`, `x-wpm-event` (the kind), `x-wpm-delivery` (the delivery's id,
new per subscription) and `x-wpm-signature: t=<unix>,v1=<hex>`, the HMAC-SHA256 of `"<t>.<raw
body>"` under the subscription's secret (WPM reads it from `WPM_WEBHOOK_SECRET_<NAME>`; the CRM from
`CRM_WPM_WEBHOOK_SECRET`; the deployer sets both to the same value). It retries a 5xx, 408, 429 or
network failure with a doubling back-off up to a day, at most ten attempts, re-signing each time,
and gives up on any other 4xx. So the CRM answers `200` for every well-formed event it does not act
on, and dedupes on the body's `id` (the event id, the same on every retry), not on `x-wpm-delivery`.

To wire it: in WPM, an administrator creates a webhook subscription to
`https://<crm>/api/bridges/wpm/events` for the kinds `worker_hired`, `worker_terminated` and
`worker_retired`; in the CRM, `CRM_WPM_WEBHOOK_SECRET` is set. Not done here: running the two
together.

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

## Web form lead capture (CRM-R31)

`POST /forms/{slug}` is public by design and sits outside `/api`, like the unsubscribe and confirmation pages (`auth::is_public_path`).
The forms are configuration, not data: `CRM_WEB_FORMS` is a JSON list of `{slug, origins, purpose?, sequence?}`, read into
`Settings` (an entry that is not valid is left out, a list that is not JSON gives no forms). Nothing the caller sends chooses the
purpose or the sequence.

What stands between the internet and the CRM, in order: the form must exist (`404`), the `Origin` must be on its list (`403`,
and a missing `Origin` is refused, so a script must set one: the origin check keeps other websites from using a visitor's
browser, and the limits are what stop a script), the per-client and per-form limits (`429`), then the honeypot, then validation.
The honeypot is checked before validation so a bot gets the same answer however malformed its other fields are.

Address handling: the CRM stores none. `clients::resolve_person` hands name and address to the person service
(`POST /api/persons`, expecting `{pid}`; this is the contract this service assumes of it and should be confirmed with that service)
and keeps the `person:` reference. The address is then used once more, in memory, as the recipient of the confirmation mail, and
dropped. The lead keeps the address's domain only, which scoring already used. In stub mode the person is process-local, so a
restart loses the address and the confirmation cannot be re-sent: development only.

Two things stop the form being used to mail a stranger: the mail is the confirmation only (no marketing until the visitor
presses the button, since `pending` allows no send), and at most one goes to a contact per purpose per hour however many
submissions name the address. The answer never says whether the address was known.

Limits are counted in the process (`rules::forms::Counter`), so with several replicas each allows its own count. Moving them to
the shared Redis store is possible (the abuse guard already does it) and was left out to keep the first version small. Clients are
told apart by `X-Forwarded-For` only with `CRM_TRUST_FORWARDED_FOR`; without it every client shares one bucket, which is safe and
strict.

The nurture enrolment is made by the confirmation itself (`forms::on_confirmed`, called inside the transaction that grants), from the
`form_submissions` row the submission left. A sequence of another purpose than the form's is skipped.

## Search (CRM-R33)

`GET /api/search?q=` runs one `UNION ALL` over seven tables (`src/search.rs`). The text searched in each table is a single
expression, `immutable_unaccent(lower(...))`, defined once in the migration crate (`search_docs::SOURCES`) so the two GIN indexes
per table (full text on `to_tsvector('simple', ...)`, trigrams on the same expression) are exactly the ones the query can use. If
they drifted the results would stay correct but the query would stop using its indexes, which no test notices; check `EXPLAIN`
after changing an expression.
`unaccent` is only STABLE in PostgreSQL, so it cannot be indexed; the migration wraps it in an IMMUTABLE SQL function that names the
dictionary, which is safe because that dictionary is fixed. The migration enables the `unaccent` and `pg_trgm` extensions, which
needs a role allowed to (both are "trusted" extensions from PostgreSQL 13, so the database owner is enough).

A match is a word prefix (`tsquery` of `word:*` terms, all required) or trigram `word_similarity` of at least 0.45 (set per query
with `SET LOCAL`; the default 0.6 is too strict for a typo). The score is `ts_rank + word_similarity`, ties broken by pid so a
page boundary is stable. Each branch fetches `limit + offset + 1` rows, so deep pages cost more; the page size is capped at 50.

Access is applied after the query, per row, for the two kinds with record-level attributes (contacts, deals); the other kinds are
decided on the entity by the blanket guard. That is why a page can be shorter than `limit` while more follow.

## Lead import (CRM-R34)

The CSV travels as text in a JSON body rather than as a multipart upload: the service has no multipart dependency, the limit is small
(500 rows, 1 MB), and the same body serves the preview and the commit, so a client holds the text and sends it twice. The commit reads
and judges the file again rather than trusting a report the client sends back.

Duplicate advice reuses the CRM-R23 matcher with the name and company only. The matcher compares one record with many by scoring all
of them, which is quadratic over a file, so each row is compared only with the people who share a word of its name (three letters or
more), at most 200 of them. A pair with no shared word is never compared; that is the price of speed, and the advice is advice. The
people compared are contacts the caller may read (the same filter as the duplicates endpoint), the 2000 newest leads, and the
earlier rows of the same file.

A commit writes the import record in its own transaction first, then saves leads in transactions of 100, appending their pids to the
record in the same transaction as the leads. A crash between batches leaves a `running` record naming exactly the leads that exist, so
undo is still correct. Undo locks the record row, so two undo calls cannot both remove leads.
