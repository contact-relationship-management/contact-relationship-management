# Requirements

**Every requirement is read as worldwide** (see [scope](scope.md#worldwide-by-design)): the target is not the United States or the United Kingdom alone, and any standard, protocol, convention or locale a requirement adopts must give equitable capability to countries, regions and federations worldwide.

Numbered requirements with user stories and acceptance criteria.
IDs are stable; design decisions ([design.md](design.md)) and tasks
([tasks.md](tasks.md)) trace to them. The four-module map:
CRM-R1–R5 sales automation, CRM-R6–R9 marketing automation,
CRM-R10–R12 service & support, CRM-R13–R14 analytics & reporting,
CRM-R15–R17 cross-cutting, CRM-R18–R19 insight/engagement views
(added CRM-T19/T20, 2026-07-20 — backfilled here so the "every task
traces to a requirement" rule holds for these two).

## CRM-R1 — Contacts & accounts

*As a rep I keep every person and company I talk to, with the whole
history in one place.*

- Contact CRUD wrapping a required `person:` URN (shape-validated;
  display name best-effort); Account CRUD wrapping a required
  `organization:` URN; ownership by `worker:` URN; soft delete.
- The detail read returns the merged relationship **timeline**
  (activities + deals + campaign touches + tickets, chronological).
- A manual repoint endpoint updates a wrapper's URN after an
  upstream registry merge (audited, reasoned).

## CRM-R2 — Activities

*As a rep I log calls, emails, meetings, notes, and tasks against
any relationship object.*

- Activity CRUD attached to contact / account / lead / deal /
  ticket; task kind carries `due_on` + `done`; actor is a `worker:`
  URN; feeds appear in timelines and dashboards.

## CRM-R3 — Leads & scoring

*As a rep I work the hottest leads first, and I can see why a lead
is hot.*

- Lead CRUD with source + optional campaign attribution; lifecycle
  `new → contacted → qualified → converted | disqualified` enforced
  by the pure core (`422` naming the current state on an illegal
  transition).
- Score 0–100 recomputed on every lead change from the fixed rule
  table ([sales-automation.md](sales-automation.md)); the response
  carries the per-rule breakdown; `hot`/`warm` labels at the
  configured thresholds; the lead queue sorts by score.
- Conversion creates/links the Contact and optionally opens a Deal
  in one transaction, emitting `lead_converted`.

## CRM-R4 — Pipelines & deals (Kanban)

*As a rep I move deals through stages; as a manager I see the board
truthfully.*

- Pipeline + ordered stages with probabilities and terminal
  won/lost flags; Deal CRUD with amount in minor units + ISO-4217.
- Stage moves validate pipeline membership, keep `kanban_position`,
  audit + emit; entering a terminal stage closes the deal (lost
  requires a reason); closed deals immutable except a reasoned
  reopen to the prior stage; concurrent reorders serialize (one
  winner).
- Stalled flag derived at N days (config, default 14) without stage
  move or activity.

## CRM-R5 — Forecasting

*As a sales manager I get a forecast derived from the pipeline, not
typed into it.*

- Live forecast = Σ `amount × probability` over open deals, grouped
  by expected-close period and owner, **per currency** (mixed
  currencies never sum); overflow refused.
- Month-end snapshot persists the roll-up for comparison.

## CRM-R6 — Consent

*As a compliance owner I can show every marketing touch was
consented to.*

- `marketing_consent` per contact with append-only ConsentEvent
  history; unsubscribe withdraws immediately, exits active nurture
  enrolments, and blocks all sends until an explicit re-grant. Consent is held
  per purpose (CRM-R29); `marketing_consent` is the original, default purpose.
- The send path (campaign + nurture) re-checks consent **at send
  time**; segments implicitly require `consent = granted`.

## CRM-R7 — Segments

*As a marketer I define audiences declaratively and preview them
before sending.*

- Segment CRUD (declarative JSON filter over contact fields);
  server-side evaluation; preview returns count + sample; the
  consent AND-gate cannot be expressed away.

## CRM-R8 — Campaigns & ROI

*As a marketer I run email campaigns and see what they returned.*

- Campaign lifecycle `draft → scheduled → running → completed |
  cancelled` (pure core); demo-mode simulated delivery writes
  per-contact touch activities + engagement counters via a `bg_pg`
  job (today inline in the request, through `mail::Sender`: simulated by default, SMTP optional, CRM-T39).
- Funnel view (recipients → delivered → opened → clicked → leads →
  deals → won revenue) + ROI = (attributed won revenue − cost) ÷
  cost, per currency; zero cost reports `null` + absolutes.

## CRM-R9 — Nurture sequences

*As a marketer I enrol contacts in drip sequences that advance
themselves.*

- Sequence CRUD (ordered steps with `delay_hours`); enrolment
  manual / by segment / on lead capture; the `bg_pg` scheduler
  advances due steps idempotently (rerun ⇒ no double-send), logs
  touches, completes after the last step; exit on unsubscribe,
  conversion, or manual exit.

## CRM-R10 — Tickets

*As a support agent I track customer issues from open to closed.*

- Ticket CRUD with contact/account, priority, `worker:` assignment,
  channel; lifecycle `open → pending → resolved → closed`, reopen
  `resolved → open` (pure core); the assignee's first outbound
  call/email activity stamps `first_responded_at`.

## CRM-R11 — SLA tracking

*As a support lead I see promised response times and every breach.*

- SlaPolicy per priority (first-response + resolution minutes, 24×7
  unless a business calendar is named, CRM-R45); deadlines derived at open and re-derived on audited priority
  change; breach flags computed on read and swept by a job emitting
  `sla_breached` once per breach; breaches clear only by meeting
  the metric.

## CRM-R12 — Knowledge base

*As an agent I answer from published articles and link them to
tickets.*

- Article CRUD `draft → published → archived`; published edits bump
  `version` (priors retained read-only); ILIKE keyword search;
  ticket-link logs a note activity. Published articles may also be shown
  to anyone, with a deflection report (CRM-R57).

## CRM-R13 — Dashboards

*As a leader I see live, honest KPIs.*

- Win rate, pipeline by stage, forecast, campaign funnel/ROI, SLA
  health, CLV per account, activity feed — all pure-core
  derivations per [analytics-reporting.md](analytics-reporting.md),
  ETag-conditional, stamped `as_of`, per-currency, ratios with
  numerator/denominator and `null` on zero denominators.

## CRM-R14 — Activity analytics

*As a manager I see per-rep activity counts by kind and period.*

- Derived from Activity rows; scoped by the persona rules
  ([auth.md](auth.md)).

## CRM-R15 — AuthN/Z & masking

- Family stack: offline PASETO verify, blanket `CRM_REQUIRE_AUTH`
  guard (default off), shared ABAC engine; record-level
  `resource.owner`/`status`/`tier` attrs with `$sub` ownership;
  `mask` obligation redacts amounts, forecasts, ROI, and contact
  channel details; the four personas of [auth.md](auth.md)
  expressible as policy.

## CRM-R16 — Audit & events

- Every mutation audited + evented (family envelope,
  `CRM_EVENT_TRANSPORT` memory/outbox); consent history + sensitive
  reads audited; reasoned actions carry their reasons.
- **Fail closed:** when a write's audit row cannot be written, the write is refused (the
  transaction rolls back and the caller gets a `500`); an audited read whose audit row cannot
  be written is refused too. No mutation exists without its audit entry
  ([audit.md](audit.md) Integrity).

## CRM-R17 — Family fixtures

- **API versioning: the path is `/api`, and the version is chosen with the
  `Accepts-version` request header. Never put a version in the path: no
  `/api/v1`, `/api/v2` and so on.** A request without the header gets the
  current version; an unsupported value is `406`. Decided by the maintainer
  (2026-10-21); this deliberately differs from the GDS advice to version in
  the URI (see `compliance/gds-api-review.md`).
- OpenAPI + Swagger, `Accepts-version` negotiation, `/metrics.prom`,
  OTLP tracing, health routes, Podman build,
  `#![forbid(unsafe_code)]`, clippy-pedantic, input caps → `422`,
  unknown-pid → `404`.

## CRM-R18 — Operational insight views

*As a manager or DPO I get read-only rollups that surface work
already recorded, instead of re-deriving it by hand.*

- Seven derived views (`GET /api/insights/*`): stale-deals (aging
  from stage-change audits), follow-ups (overdue + 30-day horizon
  over open activities' `due_on`), pipeline-hygiene (rule-disclosed
  findings), the executive period pack (won/lost, per-currency,
  never merged), forecast-trends (from stored `ForecastSnapshot`
  rows only, no interpolation), the SLA breach register +
  per-assignee workload, and the DPO view (consent coverage +
  withdrawals + duplicate-contact hygiene over shared `person_ref`).
  ETag-conditional, stamped `as_of`, same honesty rules as CRM-R13
  above.

## CRM-R19 — Stakeholder engagement & confederation

*As an account owner I record who matters on an account and how an
innovation partnership or membership is progressing, without the
system guessing at any of it.*

- Declared (never inferred) stakeholder typing: `stakeholder_role`
  on Contact and Account; a 1–5 power–interest grid position on
  Contact. Declared (never inferred) `sentiment` on Activity.
- Partnership CRUD per account with a forward-only lifecycle
  (`scouting → pilot → scaled`, `retired` reachable from any live
  stage); Membership (one per account, `active | lapsed` +
  renewal); WorkingGroup + roster.
- Nine derived views (cadence, engagement workload, the
  audit-derived pipeline funnel, member health, consent-by-account,
  the stakeholder register + grid, the partnership register,
  membership renewals) plus a `kind` filter on the CRM-R18
  follow-ups view for the renewals convention.

## CRM-R20 — Subject rights & retention (the code side of CRM-G2)

*As a DPO I can answer a subject-access request for one contact, honour
an erasure request once no live commercial reason remains to keep the
data, and show that data past its retention horizon does not linger.*

- `GET /api/contacts/{pid}/subject-access`: one audited JSON export of
  every table keyed to the contact (consent history, activities, leads,
  deals as primary contact, tickets, nurture enrolments), with genuine
  exclusions named in the payload rather than silently omitted.
- `POST /api/contacts/{pid}/erase`: anonymise — identity fields
  scrubbed to a tombstone `person:` URN, linked free text scrubbed,
  the row soft-deleted. Refused `422` while the contact has an open
  deal, an open support ticket, or an active nurture enrolment
  ([design.md](design.md) CRM-D14): the field that gates this is
  **not** `Contact::status`, which no endpoint ever transitions.
  Destructive-classified; audited with per-table counts.
- `GET /api/retention` / `POST /api/retention/sweep`: the floored
  horizon report (`CRM_RETENTION_DAYS`, default 365, floor 30) and the
  matching hard-delete sweep across every soft-deleting table, plus a
  read-only count of contacts whose consent has stood withdrawn since
  before the horizon (informational — never auto-scrubbed, since a
  contact always carries the erasure gate above). Destructive-classified;
  audited even when it deletes nothing.

## CRM-R21 — Export from tables

*As a user I can take the rows I am looking at out of any table, to use in a
spreadsheet or another tool.*

- Every data table offers export to **CSV**, **TSV** and **Excel** (`.xlsx`),
  through the export support in `@lilydesignsystem/svelte-data-grid`. The
  app does not write its own file builders.
- An export holds the rows as filtered and sorted on screen, across all pages,
  and the visible columns in their current order. Cells are written as typed
  values (numbers and dates stay numbers and dates), not display strings.
- The file is named after the table, and the export is announced to assistive
  technology (count and file name).
- Export is done in the browser from data the user already holds. It adds no
  service endpoint and no extra read, so masking and record-level access apply
  exactly as they do to the screen.
- The labels of the export controls come from the app's i18n in every locale.

## CRM-R22 — Search and a language in any address

*As a user I can find a record from anywhere, and I can share a link that opens
in a chosen language.*

- The header search finds records in every list the app shows (contacts,
  accounts, leads, deals, tickets, campaigns, the knowledge base): every word
  must appear, in any order, ignoring case and accents. It reads the same lists
  the screens read, so masking and record-level access apply exactly as they do
  there. A list that cannot be read is reported and the others still show.
- Any page opens under a language prefix (`/de-de/contacts`). The prefix sets
  the interface language, is kept by the page links, follows the language
  picker, and does not change which pages need sign-in.

## CRM-R23 — Find the same person twice

*As someone who keeps the contact list clean, I can be told how likely it is
that two contact records are the same person, and why.*

- A library crate, `contact-relationship-management-matcher`, takes two
  contacts and returns the probability that they are one person, a verdict
  (match, review, non-match) and each field's contribution to it.
- The fields are given and family name (case, accents, nicknames, initials and
  swapped order tolerated), email, phone, organisation and job title. A field
  empty on either side counts for nothing, so a sparse record is not punished.
- The same `person_ref` is a certain match. A different one is no evidence.
- It compares one contact with many and ranks the answer, in a stable order.
- The tuning (prior, per-field agreement and coincidence rates, thresholds) is
  a validated configuration that loads from JSON; a bad one is refused.
- It stores nothing and does no input or output: it is handed the personal
  data it needs and keeps none. A match is advice for a person to act on;
  nothing merges or deletes a contact on its own.
- `GET /api/contacts/{pid}/duplicates` returns the other contacts that are
  probably the same person: best first, only match and review verdicts, each
  with the probability and a per-field breakdown that names fields and never
  values. It compares only contacts the caller may read (record-level access
  applies), and the read is audited as a count, not as contact data.
- The CRM holds no email addresses, so a first pass uses what it does hold
  (name, job title, account name, `person_ref`). The best few candidates, by
  that score, then have their addresses fetched from the person service and
  are scored again; each says whether that happened (`email_checked`).

## CRM-R24 — Merge two contacts into one

*Status: built (CRM-T64).*

*As someone who keeps the contact list clean, I can merge a duplicate contact
into the one I keep, knowing the result is never more permissive about consent
than either record was, and that the merge is recorded.*

- `POST /api/contacts/{pid}/merge` with `{ "into_pid", "reason", "dry_run" }`.
  The contact in the path is merged **into** `into_pid`, which survives. With
  `dry_run` it changes nothing and returns what would move. Neither contact is
  chosen by the matcher: a match probability is advice, and a person decides.
- It runs in one transaction. A failure part-way leaves both contacts exactly
  as they were.
- The survivor keeps its identity, `person_ref` and fields. A field the
  survivor leaves empty (account, owner, job title, stakeholder role,
  influence, interest) is filled from the merged contact; nothing the survivor
  holds is overwritten.
- The merged contact's activities, leads, deals (as primary contact), tickets,
  nurture enrolments, working-group memberships and email deliveries move to
  the survivor. Where the survivor already has the same enrolment or
  membership, the survivor's is kept and the other is dropped.
- The merged contact is soft-deleted with a `merged_into_pid` pointer. Its
  `person_ref` is not repointed: the person service is a separate system, and
  two person records stay two until someone merges them there.
- **Consent never widens.** The survivor's marketing consent becomes the
  stricter of the two: `withdrawn` over `never` over `granted`, one ordered
  rule in the code, tested for every pair. A grant
  is never inferred from the other record. Both consent histories are kept,
  each entry with its original source and date, and a consent event records
  the merge. The send path (CRM-D6) then applies to the survivor as it stands.
- Refused: the same contact twice (`422`), a missing or already-merged contact
  (`404`/`409`), an erased contact in either direction (`422`, CRM-R20), and
  any contact the caller cannot write (`403`, record-level access, CRM-T24).
  The response shows masked fields as the caller would see them elsewhere.
- Destructive-classified and audited as one entry naming both contacts, the
  reason, and per-table counts moved; never contact values. It emits an event
  for the seam (CRM-D9) and honours idempotency keys (CRM-T52), so a retry does
  not merge twice.
- A subject-access export (CRM-R20) of the survivor covers the merged-in data,
  and so does an erasure of it.
- A merge is not undone automatically. To make a manual repair possible, the
  merge records the opaque ids of the rows it moved in a merge ledger (ids are
  not personal data); an unmerge is its own later task.
- The contact page offers Merge on a duplicate, behind a confirmation that
  lists what moves and says consent follows the stricter side.

## CRM-R25 — Survives its dependencies failing

*As an operator, I know what the service does when PostgreSQL or Redis stops,
freezes or returns, and when the service itself is killed mid-write.*

- PostgreSQL stopped or frozen: a request fails fast as a `5xx` (never a
  success it did not make, never a hang), the process keeps running, and the
  service serves again when the database returns, without a restart.
- Redis stopped or frozen: reads and writes still succeed. Redis holds the
  rate-limit counters, break-glass grants and the insight cache, which are
  optimisations; each Redis call on a request is bounded (the insight cache
  waits at most 750 ms) and the abuse guard falls back to in-process counters.
- A merge (CRM-R24) is all or nothing: killing the process mid-burst leaves no
  contact half merged, no row keyed to a merged-away contact, no ledger row
  without its closed contact, and an audit chain that still verifies. Of
  several merges racing for one contact, exactly one wins.
- `scripts/chaos.sh` injects each of those faults against a throwaway
  PostgreSQL and Redis and checks them. It is run on purpose, not on every push.

## CRM-R26 — Undo a merge

*As an operator who merged the wrong two contacts, I can undo it, and the undo
cannot expose anyone to marketing they have refused.*

- `POST /api/contacts/{pid}/unmerge` takes the merged-away contact in the path
  and a required `reason`. `dry_run` reports exactly what would be restored and
  changes nothing. It is destructive-classified (`/unmerge`), audited
  (`contacts_unmerged`, counts and reason, never values), and emitted as an event.
- It uses the merge ledger. It moves back only rows that still point at the
  survivor; a row that was since moved on or deleted is **left behind** and
  counted. Work the survivor did after the merge stays with the survivor.
- It restores the nurture enrolments and working-group seats the merge dropped,
  clears the fields the merge filled where the survivor still holds the value it
  was given (a field the survivor has changed is kept), and makes the contact live
  again.
- Consent never widens. The survivor keeps its current state. The returning
  contact gets the stricter of its own state and the survivor's, with a consent
  event `unmerge:<survivor>`; a withdrawal exits its active enrolments.
- Refused: a contact that was never merged or already unmerged (409), a missing
  one (404), an erased survivor or contact (422), a survivor that was itself
  merged away (409: unmerge that first), a merge made before the undo record
  existed (422), a non-admin caller (403). After the retention horizon the merged
  contact's row is gone and the merge can no longer be undone.
- A contact can be merged again after it is unmerged.
- Erasing a survivor (CRM-R20) also scrubs the identity of every contact merged
  into it, and the person reference the ledger kept: they are the same person.

## CRM-R27 — Link to the same record in other systems

*As someone whose organisation still keeps some records in HubSpot, Salesforce,
Attio or another CRM, I can jump from a contact, account, deal or lead here to the
same record there.*

- A contact, account, deal or lead can carry up to 50 links. A link is a system
  (`hubspot`, `salesforce`, `attio`, `pipedrive`, `dynamics` or `other`), a web
  address, and optionally a label and the other system's own id.
- It is only a link. Nothing is fetched from, written to, or synchronised with the
  other system, and no credentials for it are stored.
- The address must be `https`, carry no user name or password, and be at most 2,048
  characters. For a named system it must be on that system's own domains
  (`hubspot.com`, `salesforce.com`, `force.com`, `attio.com`, and so on), so a
  link cannot send a person to a lookalike host. `other` accepts any `https`
  address.
- The same address cannot be added twice to one record (409). Adding and removing
  are audited with the system, never the address.
- Links on a contact move with a merge (CRM-R24) and come back with an unmerge
  (CRM-R26). They are in the subject-access export and deleted by erasure (CRM-R20).
- Not built: importing from, or syncing with, those systems.

## CRM-R28 — Tell other systems when something changes

*As an administrator, I can register a web address that is told when records here
change, so another system can react without polling.*

- An admin or service caller registers a **subscription**: an `https` address, and
  which events it wants (`contact.created`, `deal.*`, or `*`). The service makes up
  the signing secret and shows it once. A subscription can be listed, have its secret
  rotated, and be removed (it is disabled, and its delivery log stays).
- Events come from the event outbox (the `outbox` transport; under `memory` there is
  nothing to deliver). A relay run (`POST /api/webhook-subscriptions/relay`, the same
  sweep pattern as the SLA and nurture sweeps) turns new outbox rows into one delivery
  per matching subscription and sends what is due.
- **A delivery carries ids, never contact values**: the event id, the entity and kind,
  the record's pid and the time. Not the display label and not the event's detail. A
  receiver reads the record back through the API under its own rights.
- **Signed in the Standard Webhooks format**: headers `webhook-id` (the event id),
  `webhook-timestamp` and `webhook-signature` (`v1,` and a base64 HMAC-SHA256 of
  `id.timestamp.body`). While a secret rotates the old one still signs for 24 hours, so
  the header carries both signatures.
- **The address cannot reach inside the network**: `https` only, no user name or
  password, no IP literal in a private, loopback, link-local, multicast or unspecified
  range, and the name is resolved at each send, refused if any answer is private, and
  connected to by the address that was checked. Redirects are not followed.
- **Delivery is at least once.** A delivery that does not get a `2xx` is retried with
  growing delays (30 s, 2 min, 10 min, 1 h, 6 h, 12 h, 24 h) and then marked dead; the
  log keeps each delivery's attempts, last status and last error. A `410 Gone` disables
  the subscription and cancels what is queued. Receivers must dedupe on `webhook-id`.
- Registering, rotating, removing, and a disable by `410` are audited (never the secret
  or the address).
- Erasing a contact (CRM-R20) cancels its queued deliveries, and those of contacts
  merged into it. Nothing personal is stored in a delivery.
- Not built: a retry or replay button, per-subscription rate limits, payload
  customisation, and delivery of events written under the `memory` transport.

## CRM-R29 — Consent per purpose, and double opt-in

*As a compliance owner I can show that each kind of message was consented to
for that kind, and that an address was confirmed by its owner before anything
was sent to it.*

- Consent is held **per purpose**: `marketing` (the original purpose, kept in
  `marketing_consent`, so everything that read it still does), `newsletter`,
  `events` and `product_updates`. A purpose nobody has asked about is `never`.
  Purposes are independent: withdrawing one changes no other.
- A consent state is `granted`, `withdrawn`, `never` or **`pending`**. `pending`
  means a confirmation was asked for and has not come back. **Only `granted`
  allows a send**, for every send path (campaign, nurture, segments): a
  campaign and a nurture sequence each name a purpose (default `marketing`),
  and the contact's state *for that purpose* is checked at send time (CRM-D6).
- Every consent event records its purpose, source and date; history is
  append-only and existing events are `marketing` events.
- **Double opt-in.** `POST /api/contacts/{pid}/consent/request` with a purpose
  and a source moves the purpose to `pending` and returns a confirmation link
  (valid 7 days). The link's page changes nothing on `GET` (mail scanners follow
  links); its button `POST`s and moves `pending` to `granted` with the source
  `double_opt_in`. If the person withdrew in the meantime the link no longer
  works: a confirmation never overrides a withdrawal. Sending the confirmation
  mail is the caller's job (CRM-T98's form does it); this service makes the
  link.
- **Preference page.** `POST /api/contacts/{pid}/preferences-link` returns a
  signed link (valid 90 days). Its page shows each purpose and lets the holder
  grant or withdraw it; each change is an ordinary consent event with the source
  `preference_page`.
- **Unsubscribe** from a message withdraws the purpose of that message's campaign or
  sequence, and no other. A message whose purpose cannot be found withdraws `marketing`.
- **Merge and unmerge** (CRM-R24, CRM-R26) apply the stricter-side rule to every
  purpose separately. The order, strictest first: `withdrawn`, `never`,
  `pending`, `granted`. A pending confirmation held by the merged-away contact
  does not carry over.
- Erasure (CRM-R20) withdraws every purpose of the contact and of contacts
  merged into it.
- Not built: sending the confirmation or preference mail, translating the public
  pages, a purpose list an administrator can edit (the four are fixed in code),
  and consent per channel.

## CRM-R30 — Let an AI agent use the CRM as the person who runs it

*As someone who works with an AI assistant, I can let it look things up in the
CRM, knowing it can see and do exactly what I can and no more, and that I can
see what it did.*

- `POST /mcp` is a Model Context Protocol endpoint (Streamable HTTP, stateless: no
  session, no stream). **Supported protocol versions: `2026-07-28`** (a version on every
  request, `server/discover`) **and, for clients that still open with `initialize`,
  `2025-11-25` and `2025-06-18`.** Older and unknown versions get `400` with
  `UnsupportedProtocolVersionError` listing these. Tools only: no resources, prompts,
  sampling or subscriptions. The MCP specification moves quickly; this names the revision
  built against.
- **A tool is a REST request.** Each tool stands for one documented API operation. A tool
  call is sent through the service's own router with the caller's own `Authorization`
  header, so sign-in, the access policy, masking, idempotency, rate limits and audit decide
  it exactly as they decide the same request from anyone else. There is no second rule set
  and no service account: a reader cannot write through MCP, a stranger gets masked
  values, and a caller with no token gets `401` in the tool result.
- **Read-only by default.** The tools are lookups: contacts, accounts, leads, deals,
  pipelines, forecast, tickets, activities, campaigns, segments (with the consent purpose to
  gate on), knowledge-base search, follow-ups, stale deals, a contact's consent states, and
  likely duplicates. Setting `CRM_MCP_WRITES=on` adds `add_activity`, which is still subject
  to the caller's write rights.
- **Destructive operations are previews.** `preview_merge_contacts` and
  `preview_unmerge_contact` are the merge and undo-merge calls forced to `dry_run`: they
  report and change nothing, and the agent cannot pass `dry_run` itself. A person makes the
  real change in the CRM. No tool erases a contact, reaches break-glass, or touches webhook
  subscriptions.
- **Every call is audited, with an agent marker, before it is made**: entity `mcp`, action
  `mcp_tool_called`, the caller, the tool, whether it reads, previews or writes, and the target
  pid, never the free text the agent passed. If the row cannot be written the call is not made.
  The REST operation behind it audits as it always does.
- **Record text is data.** Every tool description says so, every result opens with a notice
  that what follows is record data and not instructions, and the structured copy carries the
  record unchanged under its own fields. This reduces, and does not remove, the risk that text
  stored in a record steers an agent; the safeguard that matters is that no tool can do harm
  the caller could not already do.
- **Transport hygiene.** `Origin` is checked: a request that carries one must be on
  `CRM_MCP_ALLOWED_ORIGINS` (none by default, so browsers are refused and agents, which send
  none, are not). For `2026-07-28` the `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name`
  headers must match the body (`400`, code `-32020`). Tool arguments are validated against a
  closed schema before any request is made; ids must be UUIDs. An answer is cut at 200 KB.
- Not built: OAuth discovery for MCP clients (the caller presents the same PASETO bearer
  token as any API client), resources and prompts, change notifications, and a tool to erase.

## CRM-R31 — Capture leads from a web form, with the visitor's confirmation

*As someone who runs a website, I can put a sign-up form on it that creates a lead in
the CRM, and I know nobody is mailed marketing until they have confirmed their own
address, and that bots and strangers cannot fill the CRM or use it to spam someone.*

- **The form is configured, not invented by the caller.** `CRM_WEB_FORMS` is a JSON
  list; each entry has a `slug`, the `origins` allowed to post it, an optional `purpose`
  (default `marketing`; any CRM-R29 purpose) and an optional `sequence` (a nurture
  sequence pid). A slug that is not configured is `404`. The visitor cannot choose the
  purpose or the sequence.
- **`POST /forms/{slug}`** (outside `/api`, public by design, like the unsubscribe pages)
  takes `{ "name", "email" }` and a honeypot field `website`. It answers `202` with the
  same body whether the address is new, already known, already confirmed, or the honeypot
  was filled, so the answer cannot be used to find out who is in the CRM.
- **Origin check.** The request must carry an `Origin` on the form's list; one that does
  not, or that carries none, is `403`. An `OPTIONS` preflight is answered for allowed
  origins only, and responses carry `Access-Control-Allow-Origin` for that origin and
  nothing wider. This keeps other websites from posting the form from a visitor's
  browser; it does not stop a script that sets the header, which is what the next two
  items are for.
- **Rate limits.** Each form allows 5 submissions per client in 10 minutes and 300 per
  hour in all (`CRM_WEB_FORM_CLIENT_LIMIT`, `CRM_WEB_FORM_LIMIT`), answering `429` with
  `Retry-After`. Counts are kept in the process, so with several replicas each allows its
  own; clients are told apart by `X-Forwarded-For` only when `CRM_TRUST_FORWARDED_FOR` is
  on, and otherwise share one bucket.
- **Honeypot.** A non-empty `website` is a bot: nothing is created, nothing is sent, and
  the answer is the normal one.
- **The CRM stores no email.** The address goes to the person service, which finds or
  creates the person and returns its id; the CRM keeps the `person:` reference and the
  name. `CRM_UPSTREAM_MODE=http` calls `POST {CRM_PERSON_SERVICE_URL}/api/persons` with
  `{ "name", "email" }` and expects `{ "pid" }` (or `{ "id" }`) back. In the default stub
  mode a process-local person is made (lost on restart, development only). If the person
  service cannot be reached the answer is `503` and nothing is created. The lead carries
  only the address's domain, as before.
- **What is created.** A contact for that person unless one exists, a lead with
  `source=web` (not a second one while the contact has an open web lead), the purpose set
  to `pending` (never touching a purpose already `granted`; a withdrawn one becomes
  `pending` again, since the person is asking), and a record of the submission. Audit rows:
  the contact's creation and consent event, and `web_form_submitted` on the lead naming
  the form and purpose, never the name or address.
- **Double opt-in.** The confirmation mail, which is the only mail sent, goes to the address
  in the request with the CRM-R29 confirmation link; it is not marketing and carries no
  unsubscribe link. At most one is sent per contact and purpose per hour, so a stranger's
  address cannot be flooded by repeat submissions. A failed send is logged and is not shown
  to the visitor. Until the visitor confirms, the purpose is `pending`, which allows no
  send.
- **Optional nurture enrolment** happens when the visitor confirms (in the same transaction
  as the grant), never before. A withdrawal before confirming means the link no longer works
  and nothing is enrolled.
- Erasure (CRM-R20) deletes the contact's submission records.
- Not built: a form builder or embeddable script, CAPTCHA, translating the confirmation
  mail (English only), file fields, and extra fields beyond name and email.

## CRM-R32 — Tasks with reminders, and an inbox that tells me when they are due

*As a salesperson or support agent, I can be assigned a task with a reminder time, and
I see in my inbox when a reminder is due or a ticket of mine breaches its SLA, and only
for records I am allowed to see.*

- **Tasks.** An activity of kind `task` may carry an `assignee_ref` (a `worker:` URN) and a
  `remind_at` time, when created (`POST /api/activities`) and when edited
  (`PUT /api/activities/{pid}`, which also takes them). Other kinds refuse both with `422`.
  Changing `remind_at` makes the task eligible for a new reminder.
- **Notifications.** A notification is a row for one recipient (`worker:` URN): its kind
  (`task_reminder` or `sla_breach`), the record it is about (kind and pid), the activity or
  ticket that caused it, when it was made and when it was read. It carries no record text, so
  nothing in it can leak what the record says; the inbox shows the kind and links to the record.
- **A periodic job makes them.** The server queues `NotificationSweepWorker` every minute (it runs as a
  background job when `workers.mode` is a queue, inline otherwise; Loco has no scheduler of its own, so the
  server does the timing), and `POST /api/notifications/sweep` (admin tier, like the other sweeps)
  runs the same sweep and reports `{ created, suppressed }`. It makes a `task_reminder` for each
  open task whose `remind_at` has passed and that has an assignee, and an `sla_breach` for each open
  ticket whose assignee has a first-response or resolution breach flagged (by `POST /api/sla/sweep`).
  **Each fires once**: a database uniqueness rule on recipient, kind, cause and, for a reminder, the
  `remind_at` it was made for, so a second sweep, a second replica, or a retry makes nothing more.
- **Access is checked twice.**
  1. *When the notification would be written*, the access policy is asked whether the recipient may
     read the record, as a subject known only by the `worker:` id and with no token attributes (the
     sweep has no token; attributes exist only on a request). A record that is gone, or that the policy
     denies to that subject, makes no notification and counts as `suppressed`. This fails closed: a policy that needs
     an attribute to allow a read suppresses reminders for people whose attributes the sweep cannot see.
  2. *When the inbox is read*, the policy is asked again with the caller's own token, record by record, and a
     notification about a record the caller may not read, or that is gone, is left out (and not counted as unread).
- **The inbox.** `GET /api/notifications` (own notifications, newest first, paged; `?unread=true`),
  `PUT /api/notifications/{pid}/read`, `POST /api/notifications/read-all`. A notification of another
  recipient is `404`. With sign-in off there is no caller, so `recipient=worker:…` names whose inbox is read.
  Marking your own notifications read is open to any signed-in caller, since it changes only your inbox.
  The UI has an inbox page and an unread count beside its navigation entry, in all eight locales.
- **Audit.** A sweep that made or suppressed anything writes `notifications_swept` (counts only);
  marking read writes `read` on the notification.
- Erasure (CRM-R20) deletes notifications about the contact.
- Not built: a form in the UI to give a task an assignee or reminder (the API takes them), email or push delivery of
  notifications, per-user preferences, snoozing, recurring tasks, an assignee filter on the follow-ups view, and reading the recipient's real attributes at write time.

## CRM-R33 — Find a record by typing part of its name

*As anyone using the CRM, I can type a few letters of a name, with or without accents and even with a
typo, and be shown the matching contacts, accounts, leads, deals, tickets, campaigns and articles I am
allowed to see.*

- `GET /api/search?q=` searches the name or title of contacts (and job title), accounts (and industry),
  leads (and the domain of their address), deals, tickets, campaigns, and articles (title, keywords and
  body). It uses PostgreSQL full text (`tsvector`, `simple` configuration, every word a prefix) and
  trigram similarity (`pg_trgm`), both over text folded with `unaccent` and lower-cased, so `zoe` finds
  `Zoë` and `lovelase` finds `Lovelace`. Both are indexed (migration `000024` enables the two extensions,
  so the database role that runs migrations must be allowed to).
- `q` is 2 to 100 characters; only letters and digits of at most eight words are used. Anything else is
  `422`. Results are `{ kind, pid, title, detail }`, best match first, paged like the other lists
  (`limit`, `cursor`); a hit carries no more than the name and one plain descriptor (job title, industry,
  status, or an address domain).
- **Access.** A record the caller may not read (the record-level policy for contacts and deals) is left
  out. Search shows names, which the lists already show to a caller whose reads carry the `mask`
  obligation; masked fields (a contact's preferred channel, deal amounts) are neither searched nor
  returned, so masking cannot be got around by searching.
- **Audit.** The read is audited with how many were returned, never the text searched for.
- The UI search page calls this when the service is present instead of filtering the lists in the
  browser.
- Not searchable, because they are encrypted at rest and cannot be indexed: activity summaries and a
  contact's preferred channel. The CRM stores no email address, so none is searched. Pages are cut after
  the policy filter, so a page can hold fewer than `limit` hits while more follow.

## CRM-R34 — Bring in a list of leads from a spreadsheet, and take it back out

*As a salesperson with a list from a trade show, I can load it as leads, see what is wrong
with each row and which rows look like people we already have before anything is saved,
and undo the whole import if it was a mistake.*

- **Scope.** Leads. Contacts and accounts have their own import (CRM-R43). At most 500 rows
  and 1 MB of text per import; a larger list is imported in pieces.
- **Upload and map.** The file is sent as text in a JSON body (`csv`): a header row, comma separated, quoted fields
  allowed. `POST /api/lead-imports/preview` with no `mapping` answers with the columns, how many rows there are, and a
  suggested mapping (a column named `name`, `full name` or `display name`; `email` or `e-mail`; `company` or
  `organization`). With a `mapping` of `display_name` (required), `email` and `company` to column names, it answers one report
  line per row. Nothing is saved by a preview.
- **The row report.** Each row is `valid`, or `invalid` with the reason (no name, a name that is too long or has a line break,
  an address that is not usable, the wrong number of columns). A valid row also lists *likely duplicates*: existing contacts
  the caller may read and existing leads, and earlier rows of the same file, that the matcher (CRM-R23) scores as `match` or
  `review` on name and company. This is advice only; it never blocks a row. Only candidates that share a word of the name are
  compared, so the report is quick.
- **Commit.** `POST /api/lead-imports` with the same body plus `source_note` (where the list came from) and `obtained_on`
  (the date it was obtained), both required. It saves the valid rows in batches of 100 as leads with `source=import`, scored
  like any lead, and answers with the import's pid and counts. `skip_likely_duplicates` leaves out rows with advice. Invalid rows are
  never saved. The import record is written first and keeps the pid of every lead created as each batch commits, so an import that
  stops half way can still be undone.
- **The CRM stores no email.** Like the web form, a lead keeps only the domain of the address; the address is used for the
  syntax check and then dropped.
- **Consent is never inferred.** An imported lead is not a contact and has no consent of any kind; nothing is sent to it. What the
  import does record is its source and the date it was obtained, on the import record and in its audit row. Marketing to these
  people needs consent given through the web form or the consent endpoints.
- **Undo.** `POST /api/lead-imports/{pid}/undo` removes the leads the import created that are still `new` and have no activity;
  leads that have since been worked stay, and the answer says how many of each. Undoing twice is `422`.
- `GET /api/lead-imports` and `GET /api/lead-imports/{pid}` list and show import records (counts, source, who, when, undone).
- **Audit.** `lead_import_previewed` (counts), `lead_import_committed` (counts, source note, obtained date; never a name or
  address), each created lead's `lead_captured`, `lead_import_undone`, and `import_undone` on each removed lead.
- Not built: contacts and accounts import, files over 500 rows, other delimiters or encodings, a column for the lead's source or
  score, updating existing leads, and a background import.

## CRM-R35 — Saved views of the lists

*As someone who works the same slice of the CRM every day, I can save a filtered, sorted view of a list
under a name, and share it with colleagues without sharing anything they could not already see.*

- **What a view is.** A name, the list it is for (`contact`, `account`, `deal`, `lead` or `ticket`), a *filter*, an
  optional *sort* and an optional list of *columns*, stored on the server. A view holds no record data: only the
  names of fields and the values to keep.
- **The filter** has the shape of the segment language (CRM-R7) — a field maps to the values to keep; values of one
  field are alternatives, fields are combined with AND, an empty filter keeps everything — but over a closed set of fields
  per list: contacts `status`, `marketing_consent`; accounts `tier`; deals `currency`, `state` (`open`, `won`,
  `lost`); leads `source`, `status`; tickets `priority`, `status`, `channel`. Each value must be a token the field can
  hold. The segment evaluator itself is not used: it is built to keep only consented contacts, which is right for sending and
  wrong for looking at a list. The sort is one field of the list and a direction. Columns are chosen from the list's
  columns. Anything outside these sets is `422`.
- **Per user and shared.** `POST /api/views` makes a view owned by the caller (a name is unique per owner and list, at most 50
  per owner). `shared: true` lets every signed-in user list and use it. `GET /api/views?kind=` returns the caller's own views and
  everyone's shared ones. Only the owner can change or delete a view (`403` for a shared one, `404` for someone else's private
  one). Making, changing and deleting your own views are not data writes, so a reader may do them.
- **Sharing a view never shares access.** The view says what to ask for; the list it is applied to is still the caller's own
  read, so the rows are decided per caller (record-level policy, masking) exactly as without a view. A masked reader who opens a
  shared view sees masked values, and filtering on a field they cannot see matches nothing rather than revealing it.
- With sign-in off there is no caller; every view belongs to one anonymous owner.
- **Audit.** `created`, `updated` and `deleted` on the entity `view`, with the list and whether it is shared, never the name.
- The UI shows a view bar above each of the five lists: choose a view, save the current filter, sort and columns, share or
  delete your own.
- Not built: views for campaigns, segments or articles; filters on free text or dates; grouping; a default view per user.

## CRM-R36 — Email templates with merge tokens

*As someone who writes marketing email, I can write a message once with the recipient's own details in it, see exactly what a
recipient would get before anything is sent, and be certain every message carries the way to unsubscribe.*

- **What a template is.** A name (unique, ignoring case), a subject, a plain-text body and optional fallbacks, stored on the
  server. `GET/POST /api/email-templates`, `GET/PUT/DELETE /api/email-templates/{pid}`. A template that a campaign or a
  nurture step still refers to cannot be deleted (`409`).
- **Merge tokens** are written `{{name}}` (spaces inside the braces are ignored) from a closed set: `given_name` (the first word
  of the contact's name), `name`, `family_name` (only when the contact has one recorded; never guessed from the display name), `account_name`, `rep_name` (the contact's owner, or the account's when the contact has none),
  `job_title`, and `unsubscribe_url`. Anything else, or a `{{` that is never closed, is `422`; there is no escape for a literal
  `{{`. `GET /api/email-templates/tokens` lists the set with each token's default fallback.
- **Fallbacks.** Every token except `unsubscribe_url` has a declared fallback (`given_name` and `name`: "there"; `account_name`:
  "your organisation"; `family_name`: "customer"; `rep_name`: "the team"; `job_title`: "your role"). A template may declare its own text per token. A value
  that is missing, empty or only white space renders as the fallback, so a message never carries an empty gap or a raw `{{token}}`.
  `unsubscribe_url` has no fallback and cannot be given one.
- **Escaping.** A value is data, never markup. Rendering produces the plain text that is mailed (values as they are) and an HTML
  form in which every value and every piece of the template's own text is HTML-escaped, line breaks become `<br>` and the
  unsubscribe link is the only anchor. Control characters in a value become a space, so a value cannot add a header line or
  a line break to a subject.
- **Unsubscribe.** The body of every template must contain `{{unsubscribe_url}}`; saving one without it is `422`. The token is
  replaced by the contact's signed link (CRM-R8). A message made from a template does not get the generic footer added, because
  the template carries the link where its author put it. The subject cannot contain the token.
- **Preview.** `POST /api/email-templates/preview` takes a draft (subject, body, fallbacks, and optionally `blank`, the tokens to
  treat as missing) and renders it against a *synthetic* contact, never a real one. It answers the rendered subject, text and
  HTML, the tokens that fell back, and `problems`: what would stop the draft being saved. A draft that cannot be parsed is `422`.
  Previewing changes nothing, so a reader may do it.
- **Use.** A campaign (`POST /api/campaigns`) and a nurture step may carry `template_pid` instead of a subject and body; giving
  both is `422`. The template is read when each message is made, so changing it changes the messages not yet sent. A message that
  has already gone out is not changed, and a template missing at send time fails that delivery.
- **Audit.** `created`, `updated` and `deleted` on the entity `email_template`, never the subject or body.
- The UI has a templates page: a list, an editor with the token set, and a live preview that can blank each token. The campaigns page has a creation form: a name, an audience segment, and either a template or the campaign's own subject and message.
- Not built: HTML-bodied email, attachments, per-recipient-language templates, template versions, and tokens from custom fields.

## CRM-R37 — A contact's family name

*As someone who writes formal email, I can address a contact by family name when we know it, and fall back politely when we do not.*

- A contact has an optional `family_name` (at most the usual field length), given on `POST /api/contacts` and returned with the contact.
  The CRM keeps one display name; the family name is **never derived from it**, because names do not split reliably into parts.
- The merge token `family_name` (CRM-R36) fills from it. A contact without one gets the token's fallback ("customer", or the template's own).
- A merge fills it on the survivor when the survivor has none, and an unmerge clears it again, like the other fillable fields (CRM-R24, CRM-R26).
- Erasing a contact (CRM-R20) clears it, on the contact and on contacts merged into it.
- Editing it after creation, and a stored `given_name`, are CRM-R38.

## CRM-R38 — A contact's given name, and editing a contact

*As someone who keeps contact details, I can store a contact's given name and correct a contact's details after creating it.*

- A contact has an optional `given_name`, given on `POST /api/contacts` and returned with the contact. The `given_name` merge token (CRM-R36) fills
  from it; a contact without one still gets the first word of the display name, as before.
- `PUT /api/contacts/{pid}` changes a contact's own fields: `display_name`, `given_name`, `family_name`, `job_title`, `owner_ref`, `account_pid` and
  `preferred_channel`. Only the members sent change; `null` clears an optional one; a blank `display_name`, an unknown member, an unknown account
  or an empty body is refused `422`. The `person_ref` still changes only by `repoint`, and consent only by its own endpoint. A caller needs write
  access to the contact (record-level policy applies).
- The change and its audit row commit together, and the audit row names the fields that changed, never their values, so an erasure leaves no
  personal data in it. A `contact` `updated` event is emitted.
- A merge fills `given_name` on the survivor when it has none, an unmerge clears it again, and erasure clears it (CRM-R24, CRM-R26, CRM-R20).
- The contact page shows both names and an edit form.

## CRM-R39 — Editing an account

*As someone who keeps account details, I can correct an account after creating it.*

- `PUT /api/accounts/{pid}` changes an account's `display_name`, `tier`, `owner_ref` and `industry`. Only the members sent change; `null` clears
  an optional one; a blank name, an unknown tier, an unknown member (the `organization_ref` never changes) or an empty body is refused `422`, and
  an unknown account `404`.
- The change and its audit row commit together; the row names the fields that changed, never their values. An `account` `updated` event is emitted.
- The account page shows an edit form for the name, tier and industry.

## CRM-R40 — Explainable workflow rules

*As someone who wants the CRM to do routine follow-up, I can write a rule from a trigger, conditions and a few actions, try it on today's data first, and read afterwards why it ran and what it did.*

- A workflow has a name (unique, any case), an on/off switch, one **trigger**, **conditions** and one to five **actions**, done in order.
  - Triggers: `contact_created`, `contact_updated`, `deal_stage_changed` (a move to a non-closing stage; the actions are about the deal's primary contact) and `schedule` (with `every_minutes`, 5 to 43 200: the least time between runs on the same contact).
  - Conditions use the segment filter language (`statuses`, `channels`, `account_tiers`; empty means any). An unknown clause is refused, so a typo cannot match everyone. There is no consent clause; consent is checked inside the actions that need it.
  - Actions are a closed set: `create_task`, `notify`, `set_field` (`job_title`, `owner_ref`, `preferred_channel`, `status`), `enrol_nurture` and `send_webhook` (emits `workflow.<event>` to the subscriptions that ask for it, CRM-R28). There is no code and no URL. Anything else is refused `422`.
- A run happens in the transaction of the write that triggered it. Its actions sit in a savepoint: if one fails none stays, the run is logged `failed`, and the triggering write is unharmed. A run log row that cannot be written refuses the write (CRM-R19).
- **Consent:** `enrol_nurture` reads the contact's consent for the sequence's purpose at that moment, not at the trigger, and skips with the reason when it is not `granted`, or when the contact is already enrolled.
- **Loop guard:** a `set_field` is itself a `contact_updated` event, one level deeper; a chain cut at depth 3 is logged `blocked`. A workflow is held back after 60 completed runs in a minute. A held-back run is noted once a minute, not once per attempt.
- **Dry run:** `POST /api/workflows/{pid}/dry-run` looks at up to 500 contacts and says how many meet the conditions and, for the first few, what each action would do (including the consent skip). It changes, sends and logs nothing as a run; the dry run itself is audited.
- **Run log:** each run records the trigger, the record, the depth, an outcome (`done`, `failed`, `blocked`), the reason it fired (trigger and conditions in words) and each step with its result and reason, with no contact data. It is audited as `workflow_run` and survives the workflow's deletion.
- **Schedule:** the minute tick (and `POST /api/workflows/run-due`) works through contacts in id order, at most 50 runs and 500 contacts per workflow per tick, remembering where it stopped; one replica at a time per workflow.
- Saving or trying a workflow needs write access; reading the list does not. Saving is audited.
- Not built: triggers on records other than contacts and deals' stage moves (accounts, tickets, leads, web-form and lead-conversion contacts), conditions on anything but the segment clauses, branching or waiting steps, editing a workflow's actions in place beyond replacing it, and a run log across all workflows.

## CRM-R41 — Typed custom fields

*As someone who keeps details the standard record has no place for, I can declare a field on contacts, accounts, leads or deals, set its values, and trust that it is masked, stored, exported and erased the way its sensitivity says.*

- `POST /api/custom-fields` declares a field: `entity` (`contact`, `account`, `lead`, `deal`), `key` (lower case, at most 40, fixed), `label`, `kind` (`text`, `number`, `date`, `choice`; fixed), `choices` (for a choice, 1 to 20 distinct) and `sensitivity` (fixed). A key is unique per entity; at most 30 fields per entity. `GET /api/custom-fields` lists them; `PUT /api/custom-fields/{pid}` changes the label, choices and position (a choice that values still use cannot be removed, `409`); `DELETE` removes the field and every value it holds. All audited as `custom_field`.
- `GET` and `PUT /api/{contacts|accounts|leads|deals}/{pid}/custom-fields` read every field of the record with its value, and set or clear (`null`) values. Only the fields named change. A value is checked against its kind: text up to 500 characters, a finite number, a `YYYY-MM-DD` date, or one of the choices. A bad value, or an unknown key, is `422` naming the field, and nothing is saved. The audit row names the fields, never the values.
- **Sensitivity** decides what happens to a field's values:
  - `business`: operational data. Visible to a masked read; kept after an erasure.
  - `personal`: about a person. Hidden from a masked read (`value: null`, `masked: true`), included in the subject-access export (CRM-R20), deleted when the contact is erased (and when a lead the erasure scrubs is).
  - `sensitive`: as `personal`, and sealed with the field key in the database (CRM-T41); key rotation re-seals it.
- Writing needs write access to the record (the record-level policy applies to contacts and deals); declaring a field needs write access too.
- A contact's export and an erasure also cover the values set on contacts merged into it. A merge does not copy values to the survivor, and an unmerge has nothing to undo: values stay on the contact they were set on.
- The contact, account, lead and deal pages show the fields as a form; `/custom-fields` declares them.
- Not built: custom fields in saved views and segment filters, in lists and CSV import, in the search index, or in webhooks and the workflow conditions; a field of type multi-choice, URL or lookup; changing a field's kind or sensitivity (delete and declare it again); erasure of account and deal values (they are not personal subjects).

## CRM-R42 — File attachments

*As someone who keeps the documents that go with a relationship, I can attach a file to a contact, account, lead or deal, and trust that only safe kinds are accepted, that it is scanned, and that it goes when the person is erased.*

- `POST /api/{contacts|accounts|leads|deals}/{pid}/attachments?name=` takes the file itself as the body. `GET` lists a record's files (metadata); `GET /api/attachments/{pid}/content` downloads one; `DELETE /api/attachments/{pid}` removes the row and the stored file.
- **Accepted files:** an allow-list by extension (pdf, png, jpg, jpeg, gif, webp, txt, csv, md, docx, xlsx, pptx, odt, ods). Archives, scripts, executables, HTML and SVG are not on it. The stored content type comes from the list, never from the request. For formats with a recognisable start the first bytes must agree with the extension (a `.pdf` that starts with `<script>` is refused). The name is cleaned: no directory part, no control characters, no quotes, at most 120 characters. At most 20 files per record, and a size limit (`CRM_ATTACHMENT_MAX_BYTES`, default 10 MiB, at most 100 MiB). A file refused is `422` and leaves nothing behind.
- **Scan hook:** with `CRM_ATTACHMENT_SCAN_COMMAND` set (a program and arguments, split on spaces, run without a shell, with the file's path last), each upload is written to `incoming/`, scanned, and moved into place only on exit 0. Exit 1 is an infected file (`422`, audited as `refused_infected`); any other result, a timeout or a failure to start is `503` and the file is not stored: an unscannable file is refused, not trusted.
- **Storage:** a directory (`CRM_ATTACHMENT_DIR`); unset, attachments answer `503` and say why. A file is stored under a random id, never its name. Its SHA-256 is kept with it and checked on every download; a file that no longer matches is not served.
- **Downloads** are always `Content-Disposition: attachment` with `X-Content-Type-Options: nosniff`, and are audited as a disclosure.
- **Access:** writing needs write access to the record; reading, listing and downloading need an unmasked read, so a masked reader sees no files.
- **Audit:** `attachment` `uploaded` (type, size and digest, never the name: a name can be personal), `downloaded`, `deleted`, `refused_infected`.
- **Subject access and erasure:** the export lists the files of the contact, of contacts merged into it and of its leads (name, type, size, digest). Erasing the contact deletes those rows in the transaction and the files once it has committed; a file that cannot be removed is logged.
- The four record pages show the files with upload, download and delete.
- Not built: object storage (S3 and similar; the store is one directory, on local disk or a mounted volume), encryption of the files at rest, virus-scan signatures of our own, thumbnails or previews, versions, a file on a ticket or an activity, and a retention sweep of orphan files left by a crash between disk and database.

## CRM-R43 — Bring in contacts and accounts from a spreadsheet

*As someone moving from another system, I can load a list of contacts or accounts, see what is wrong with each row and which rows look like records we already have before anything is saved, and undo the whole import.*

- It works as the lead import does (CRM-R34): the file is sent as text (`csv`), at most 500 rows and 1 MB; `POST /api/{contact|account}-imports/preview` with no `mapping` answers with the columns and a suggested mapping, and with a `mapping` one report line per row; `POST /api/{contact|account}-imports` saves in batches (50 per transaction) and needs `source_note` and `obtained_on`; `GET` lists and reads import records; `POST .../{pid}/undo` takes one back. A preview saves nothing.
- **Contacts.** Mapped columns: `display_name` and `email` (both required), `given_name`, `family_name`, `job_title`. A contact is a reference to a person, so each valid row's name and address are sent to the person service (`clients::resolve_person`), which finds or makes the person; the CRM keeps the `person:` reference and never the address. A row whose person is already a contact here is left out and counted; a row the service cannot answer for is counted as failed and the rest go on. A new contact is `active`, prefers email, and has **no consent** (`never`). An import does **not** start workflows (CRM-R40), so a list of thousands cannot flood them.
- **Accounts.** Mapped columns: `display_name` (required), `organization_ref` (an `organization:` reference), `tier` (`prospect`, `customer`, `partner`, `former`; default `prospect`), `industry`. With no reference in the file, the organization service is asked to find or make the organization by name (`clients::resolve_organization`); an organization already an account here is left out and counted.
- **Duplicate advice** never blocks a row. For contacts it is the matcher's, as for leads (existing contacts the caller may read, leads, and earlier rows, on name; the address contributes its domain only). For accounts it is an exact match on the name, ignoring case and spacing, against existing accounts and earlier rows. `skip_likely_duplicates` leaves such rows out.
- **Record.** `record_imports` is written first and keeps the pid of every record as each batch commits, so an import that stops half way can be undone. Counts: rows, invalid, skipped (duplicates left out, and people or organizations already here), failed (lookups that failed), created.
- **Undo** removes the records the import made that nobody has used since. A contact is kept once it has consent, an activity, a deal, a ticket, a lead, an enrolment, a consent event, an attachment or a custom field value, or has had another contact merged into it. An account is kept once it has a contact, a deal, a ticket, a partnership, a membership, an activity, an attachment or a custom field value. The kept ones are counted.
- **Audit.** The preview (counts only), the commit (counts, source and date), each record created (with the import pid) and the undo.
- The Contacts and Accounts pages link to an import page.
- Not built: linking an imported contact to an account by a company column; an owner column; consent columns (a consent decision needs its own evidence, CRM-R6); updating existing records from a file; more than 500 rows in one import; and a confirmed contract for the person and organization services (the same assumption as the web form's, `spec/integrations.md`: `POST /api/persons` and `POST /api/organizations` answer a `pid` or `id`).

## CRM-R44 — Assign new leads to a worker

*As a sales manager, I want each new lead given an owner by a rule I can read, so that no lead waits for someone to notice it, and I can see why a lead went to a worker.*

- A lead has an optional `owner_ref` (a `worker:` URN). It is returned with the lead, and carries over to the contact when the lead is converted.
- A workflow (CRM-R40) can have the trigger `lead_created`: a lead was captured by `POST /api/leads` or a web form (CRM-R31). A lead brought in by a list import (CRM-R34) starts no workflow, as imported contacts start none (CRM-R43).
- **Conditions** on a `lead_created` workflow are the lead's `sources` (`web`, `referral`, `event`, `import`, `campaign`; empty means any). The contact clauses (status, channel, tier) and `sources` cannot be mixed up: a clause that cannot apply to the trigger is refused `422`.
- **The action `assign_owner`** (for `lead_created` only; `set_field` and `enrol_nurture` are refused there, as they are about a contact) takes a `pool` of one to twenty distinct workers, a `strategy` and `notify`.
  - `round_robin` takes the pool in turn. The workflow keeps a counter, advanced under a row lock, so two leads captured together take different turns. Changing the pool does not reset it.
  - `least_open` takes the worker with the fewest open leads (new, contacted, qualified) and open deals they own; the first in the pool on a tie.
  - A lead that already has an owner is left alone (the step says so). With `notify`, the chosen worker gets an inbox notification about the lead.
- **Explained.** The run log row says why it fired (trigger and source condition) and the step says who was chosen and how. The lead's audit trail has an `assigned` row by `workflow:<pid>` naming the worker and the strategy. A dry run lists the newest leads that meet the conditions and who each would go to, from where the counter now stands, without moving it or assigning anything.
- The loop guard and the rate hold of CRM-R40 apply (60 runs a minute per workflow); a burst of leads beyond that is held back and noted, not assigned.
- The lead page shows its owner, and the workflows page offers the trigger, the source condition and the action.
- Not built: assigning contacts, accounts or deals; hand-assigning a lead (there is no edit of `owner_ref` outside a workflow); weighting or capacity limits; working hours or availability; reassigning a lead that has gone unworked; and assignment of leads imported from a list.

## CRM-R45 — SLA clocks that follow working hours

*As a support lead, I want a promise of "four hours" to mean four working hours, so that a ticket opened on Friday evening is not already late on Monday morning.*

- A **business calendar** (`/api/business-calendars`: list, create, read, replace, delete) has a name (unique, any case), an IANA time zone, the working windows of each weekday (`mon` to `sun`, up to four `{start, end}` windows a day written `HH:MM` in the zone's local time; a day with none is a day off) and a list of holidays. It needs at least one working window; overlapping or backwards windows, an unknown zone and more than 400 holidays are refused `422`. Deleting one that a policy uses is `409`.
- An **SLA policy** may name a calendar (`calendar_pid` on `POST /api/sla-policies`, or `PUT /api/sla-policies/{pid}/calendar`, which also takes `null` to go back to round-the-clock). Without one a policy behaves as before (CRM-R11).
- **Deadlines** of a ticket opened under such a policy are counted in working time from the moment it opens: time outside the windows and the days on the holiday list do not count, and the deadline falls on the instant the target's working minutes have run out. A ticket opened outside working hours starts counting when work next starts. Windows are local to the zone, so they follow its clock changes (summer time). A target the calendar cannot place within about ten years of days falls back to round-the-clock, so a ticket never loses its deadline.
- Deadlines are still **stored on the ticket** (CRM-R11), so breach flags, the sweep, dashboards and notifications are unchanged. They are derived when a ticket is opened and again on an audited priority change; replacing a calendar, or changing which one a policy uses, does **not** move the deadlines of tickets already open.
- Audited as `business_calendar` (`created`, `updated`, `deleted`) and `sla_policy` `calendar_set`.
- Not built: a screen for SLA policies and calendars (they are managed through the API); pausing the clock while a ticket waits for the customer (CRM-R55); different calendars per account or per channel; re-deriving the deadlines of open tickets when a calendar changes; and showing the remaining time in working hours (the countdown is still to the stored deadline).

## CRM-R46 — The screens fit a phone

*As someone using the CRM on a phone, I can read and use every screen without scrolling the page sideways.*

- At a width of 360 CSS pixels no screen makes the page scroll sideways. A wide table may scroll inside its own box; the page may not.
- Below 40 rem the header wraps (the title and menu, the session button and the picker buttons each take the room they need) rather than running past the edge, and a text field, a selector or a file chooser takes at most the width of its container.
- A Playwright test walks the tour's screens and the new ones (workflows, custom fields, templates, imports, and a contact, account, lead, deal and ticket page) at 360 by 740 and fails if any is wider than the viewport.
- Not built: a phone layout for the boards (they scroll inside their own area), a bottom navigation bar, touch-size checks for every control, or a native app.

## CRM-R47 — Tags

*As someone who sorts contacts, accounts, leads and deals my own way, I can label them with tags and then find them, and select them for a campaign, by tag.*

- A **tag** is a short label (1 to 40 characters, no control characters). Names are tidied (spaces trimmed and collapsed) and unique in any case, so `VIP` and `vip` are one tag, which keeps the spelling it was first given. At most 500 tags. `GET /api/tags` lists them with how many records carry each; `POST /api/tags`, `PUT /api/tags/{pid}` (rename; records keep it) and `DELETE /api/tags/{pid}` (it comes off every record) manage them, audited as `tag`.
- A **record's tags** are read with `GET /api/{contacts|accounts|leads|deals}/{pid}/tags` (the names) and set whole with `PUT` of `{ "tags": [names] }`: at most 20, repeats in any case counted once, a name that is not yet a tag becomes one. Writing needs write access to the record. The audit row counts the tags and names none, because a tag can say something about a person.
- **Hidden from a masked read.** A caller whose read of a contact or deal is masked is refused (`403`) when reading or setting its tags. Narrowing a list by tag leaves out the records that caller sees masked, so the filter cannot be used to learn which of them carry a tag.
- **Narrowing a list:** `GET /api/contacts`, `/accounts`, `/leads` and `/deals` take `?tag=<name>` (any case); an unknown tag gives an empty list, a blank one `422`.
- **Segments.** A segment filter may hold `tags`: a contact is in the segment if it has at least one of them, in any case, and the other clauses and the consent gate (CRM-R7) apply as ever. A segment's tags are tidied as a record's are. Workflow conditions (CRM-R40) do not look at tags.
- **Subject access and erasure.** The export (CRM-R20) lists the names of the tags on the contact, on contacts merged into it and on its leads. Erasing the contact deletes those taggings; the tags themselves stay, because they are labels, not the person.
- The contact, account, lead and deal pages show a record's tags with a box to add one; the contact, account and lead lists have a tag selector.
- Not built: tags in saved views (a view filters rows the browser already holds, which do not carry tags); tags in workflow conditions; colours or groups of tags; the tag selector on the deals board; bulk tagging; and tags on tickets, activities or articles.

## CRM-R48 — Reports

*As a manager, I can ask a question of the CRM's own data, such as deals by stage, leads by source and month, or how many contacts carry a tag, get a table, and keep the question to ask again.*

- A **report** is one kind of record (`contact`, `account`, `lead`, `deal`, `ticket`), one thing to **group by**, a **measure**, optional **filters** and an optional date range. `POST /api/reports/run` answers a specification and saves nothing.
  - **Group by and filter by** are chosen from a closed list per kind: contacts by `status`, `marketing_consent`, `account_tier`, `owner`, `created_month`, `tag`; accounts by `tier`, `industry`, `owner`, `created_month`, `tag`; leads by `source`, `status`, `owner`, `created_month`, `tag`; deals by `stage`, `state` (open, won, lost), `currency`, `pipeline`, `owner`, `created_month`, `closed_month`, `tag`; tickets by `status`, `priority`, `channel`, `assignee`, `opened_month`. Months can be grouped by but not filtered. A filter takes one to twenty values per field (alternatives within a field, AND across fields); those fields with a fixed set of values (status, consent, tier, source, state, priority, channel) refuse any other.
  - **Measures:** `count`, and for deals `sum_amount`, which is **always per currency**: every row carries its currency and amounts are never added across currencies.
  - **Dates:** `from` and `to` (inclusive days) apply to the day a record was created (a ticket, opened).
- The query is built from fixed pieces of SQL with the caller's values bound; nothing the caller writes becomes SQL. A specification that asks for something outside the lists is `422` naming the member (`/spec/group_by`, `/spec/filter`, …).
- The answer holds the specification, `as_of`, the rows (the group, the currency for a sum, the value), largest first, at most 200 (`truncated` says there are more), and a note on what the figures mean. A record with several tags is in each tag's group, and one with none in `(no tag)`.
- **Masking.** A caller whose collection read is masked (CRM-T32) may count but not sum amounts or use tags, in the group or the filter: that is `403`. Counts of live records are otherwise as visible as the existing dashboards.
- **Saved reports** (`GET/POST /api/reports`, `PUT/DELETE /api/reports/{pid}`, `GET /api/reports/{pid}/run`): a name unique among the owner's, a specification, and a `shared` flag; at most 50 per owner. Anyone may list and run their own and the shared ones; only the owner changes or deletes (`403` for a shared one, `404` for a private one). Like a saved view a report holds no record data and running it is the runner's own read, so sharing shares no access; and saving one is open to a reader because it changes no CRM data. Running audits `report_run` (the kind, the grouping and the number of groups).
- The Reports page builds, runs, saves and reopens reports, with a bar for each row.
- Not built: reports across kinds (a join of deals and accounts), two groupings, averages, minimums or maximums, custom fields in a report, a chart other than the bar, schedules and e-mailed reports, export (the table is on the page; the table export of CRM-R21 does not yet cover it), and row-level policy in an aggregate (a caller is judged on the collection read, like the other aggregate views).

## CRM-R49 — Multi-touch attribution

*As a marketing manager, I can see how the revenue of won deals is shared among the campaigns that touched the people who bought, under a model I choose, so that a campaign that opened the door is not invisible next to the one that closed it.*

- **Touches.** For a won deal, a touch is a campaign's contact with the deal's **primary contact** before the deal closed: an e-mail the campaign (or a nurture step) actually sent to that contact (`email_deliveries.state = 'sent'`, at its `sent_at`), and a lead of that contact that came from a campaign (at the lead's creation). The deal's own `source_campaign_pid` (v1 source attribution) is always a touch, at the deal's creation. Only touches inside the **window** count: from `window_days` before the deal closed up to the moment it closed (the source campaign is exempt from the window, because someone recorded it on purpose). A deal with no primary contact has only its source campaign.
- **Models.** `first` gives the whole amount to the earliest touch, `last` to the latest, `linear` splits it equally across the touches. Every touch is a touch. A campaign sends a contact at most one e-mail, but it can touch a deal twice or three times (its e-mail, a lead it brought in, and being the recorded source), and under `linear` it then earns that many shares. Ties break by time, then by campaign id, so the answer is the same every time.
- **Exact.** The shares of one deal add up to its amount **to the minor unit**: the leftover minor units of a division go one each to the earliest touches. Nothing is invented and nothing is lost. A deal with no touch at all is counted as **unattributed**, with its amount, not dropped.
- **Per currency.** Credit is summed per campaign **and per currency**; currencies are never added together.
- `GET /api/attribution?model=&window_days=&from=&to=` answers for won deals closed in the date range (inclusive days; all when absent). `model` is `first`, `last` or `linear` (default `linear`); `window_days` is 1 to 365 (default 90). Anything else is `422` naming the member. The answer holds the model, the window, `as_of`, the number of deals considered, the unattributed deals (count and amount per currency), and for each campaign its name, its touches, the deals it touched and its credit per currency, largest first. At most 5000 deals are read (`truncated` says there were more).
- **Masking.** The answer is money, so a caller whose collection read is masked (CRM-T32) gets `403`, as for a report on amounts. It is a read: a reader may ask. Asking audits `attribution_run` (the model, the window and the number of deals).
- The Campaigns area has an Attribution page: pick the model, the window and the dates, and see each campaign's credit with a bar, and the unattributed amount beside it.
- Not built: touches by the deal's account's other contacts; cost per attributed pound; a stored result (every ask is computed from the records as they are now).

## CRM-R50 — Exchange rates and converted reports

*As a finance lead, I can record the exchange rates the organisation uses and see a report's deal sums in one currency, with the rates it used and what it could not convert, so that mixed-currency pipelines have one figure without hiding how it was made.*

- **Rates** are entered by hand; nothing is fetched. A rate is a pair (`from` to `to`, three capital letters each, different), a decimal `rate` (1 `from` buys `rate` `to`; positive, at most 9 decimal places, below 1,000,000), the day it takes effect (`effective_on`), and a `source` in words (up to 80 characters, required: "ECB reference rate", "board-approved budget rate"). One rate per pair and day: setting it again replaces it. `GET /api/fx-rates` lists them, newest first; `PUT /api/fx-rates` sets one; `DELETE /api/fx-rates/{pid}` removes one. Changing rates is a write (a reader cannot), and each change is audited with the pair, the rate, the day and the source.
- **Which rate.** For a day `D` and a pair, the rate is the **latest one taking effect on or before `D`**: the pair's own, or the inverse of the reverse pair's (1 divided by it, not rounded until used); whichever is later, the pair's own on a tie. A rate that takes effect after `D` is never used, and there is no conversion through a third currency. With none, the pair cannot be converted.
- **Converting.** An amount is multiplied by the rate (or divided by the inverse's) and rounded half to even to a minor unit; as everywhere in the CRM, a minor unit is taken as a hundredth.
- **Reports.** A deal report with `measure = sum_amount` may carry `convert_to` (a currency) and `rate_date` (a day, today when absent; a day in the future is `422`; both are refused on any other measure). Each group's sum in each currency is converted **once**, not deal by deal, so the rounding is at most half a minor unit per group and currency; sums already in the target currency are left as they are. Groups then merge, so a row is one group in the target currency. The answer adds `rate_date`, the `rates` used (pair, rate, day, source, whether inverted) and `unconverted`: for every currency with no usable rate, how many groups and how much was left out. **Nothing is dropped silently and nothing is taken at 1.0.** Saved reports keep `convert_to` and `rate_date` (an absent date means the day the report is run).
- The FX rates page lists, adds and deletes rates. The Reports page offers "Show in" and "Rates as of", and shows the rates used and what could not be converted.
- Not built: fetching rates from a provider or a bank feed; converting through a third currency; the rate on the day each deal closed (a report uses one date for all its rows); currencies whose minor unit is not a hundredth; converting the stored forecast snapshots and trend (attribution: CRM-R51; the campaign funnel and ROI: CRM-R52; the live forecast: CRM-R53; CLV: CRM-R54); importing rates in bulk; rate history charts.

## CRM-R51 — Converted attribution

*As a finance lead, I can see each campaign's attributed revenue in one currency, with the rates used and what could not be converted, so that a mixed-currency pipeline ranks its campaigns on one figure.*

- `GET /api/attribution` may carry `convert_to` (three capital letters) and `rate_date` (a day, today when absent; a day in the future is `422`; `rate_date` without `convert_to` is `422`). Rates, picking and rounding are those of CRM-R50.
- Each campaign's credit in each currency is converted **once** and merged, so the rounding is at most half a minor unit per campaign and currency; the unattributed deals are converted the same way. The shares of a deal are still shared exactly in its own currency first: conversion never touches the sharing.
- With `convert_to`, every `credit` and `unattributed` list holds one amount in the target currency (none for a campaign whose credit had no usable rate). The answer adds `convert_to`, `rate_date`, the `rates` used and `unconverted`: for every currency with no usable rate, how many campaign (or unattributed) sums and how much was left out. **Nothing is dropped silently and nothing is taken at 1.0.** Without `convert_to` the answer is as in CRM-R49, with empty `rates` and `unconverted`.
- The Attribution page offers "Show in" and "Rates as of", and shows the rates used and what could not be converted.
- Not built: (campaign ROI: CRM-R52; the forecast: CRM-R53; CLV: CRM-R54); the rate on each deal's close day (one date serves the whole answer); a campaign's credit converted before it is merged across deals.

## CRM-R52 — Converted campaign ROI

*As a marketer, I can count the revenue a campaign won in other currencies at the exchange rate in force, so that its ROI is not understated by deals that closed in dollars or euros.*

- `GET /api/campaigns/{pid}/funnel` may carry `convert=true` and `rate_date` (a day, today when absent; a day in the future is `422`; `rate_date` without `convert=true` is `422`). Without them the answer is as in CRM-R8: revenue in the campaign's own currency, the others listed apart, and no rates.
- With `convert=true`, revenue won in each other currency is converted **once** into the **campaign's own currency**, the one its cost is in, with the rates and rounding of CRM-R50, and added to `won_revenue_minor`; ROI is then computed from that sum. `won_revenue_other_currencies` keeps only what had no usable rate, and `unconverted` names those currencies and amounts. The answer adds `rate_date` and the `rates` used. **Nothing is dropped silently and nothing is taken at 1.0**, so a campaign's ROI is at worst understated by exactly what `unconverted` lists.
- **Masking** is unchanged: a masked caller gets `null` revenue and ROI, and no rates or unconverted amounts.
- The Campaigns page has a checkbox "Count other currencies at the rate in force" beside the funnel, and shows the rates used and what could not be converted.
- Not built: converting the cost (it is already in the campaign's currency); choosing another target currency; the rate on each deal's close day.

## CRM-R53 — Converted forecast

*As a sales lead, I can see the stage-weighted forecast as one total in a currency of my choice, with the rates used and what could not be converted, so that a mixed-currency pipeline has one number.*

- `GET /api/forecast` may carry `convert_to` (three capital letters) and `rate_date` (a day, today when absent; a day in the future is `422`; `rate_date` without `convert_to` is `422`). Without them the answer is as in CRM-R5.
- The per-currency `totals_minor` stay exactly as they were. With `convert_to` the answer adds `converted_minor`: each currency's weighted total converted **once** with the rates and rounding of CRM-R50, and the results added. It also adds `convert_to`, `rate_date`, the `rates` used and `unconverted`: for each currency with no usable rate, the total left out. **Nothing is dropped silently and nothing is taken at 1.0**, so the converted figure is at worst short by exactly what `unconverted` lists.
- **Masking** is unchanged: a masked caller gets `null` totals and `null` `converted_minor`, and no rates.
- The converted figure is a live read, like the forecast itself; the stored snapshots (`POST /api/forecast/snapshot`) and the trend series stay per currency.
- The deal board's forecast strip has a "Show in" box that adds the converted total, the rates used and what was left out.
- Not built: converting the snapshots or the trend; the rate on each deal's expected close day.

## CRM-R54 — Converted customer lifetime value

*As an account manager, I can see an account's lifetime value as one total in a currency of my choice, with the rates used and what could not be converted.*

- `GET /api/accounts/{pid}/clv` may carry `convert_to` (three capital letters) and `rate_date` (a day, today when absent; a day in the future is `422`; `rate_date` without `convert_to` is `422`). Without them the answer is as in CRM-R13.
- The per-currency `clv_minor` stays exactly as it was. With `convert_to` the answer adds `converted_minor`: each currency's CLV converted **once** with the rates and rounding of CRM-R50, and the results added; and `convert_to`, `rate_date`, the `rates` used and `unconverted` (each currency with no usable rate, and the amount left out). **Nothing is dropped silently and nothing is taken at 1.0.**
- **Masking** (CRM-T119): CLV is money, so a caller whose collection read is masked gets the number of won deals and `null` for `clv_minor` and `converted_minor`, as for the forecast; an unmasked caller gets the figures.
- A live read of the won deals now, valued at one date; it says nothing of what the money was worth when each deal closed.
- No page shows CLV yet, so there is no UI.
- Not built: the rate on each deal's close day; margin, discounting or churn (see the roadmap's CLV refinement).

## CRM-R55 — SLA clocks that stop while a ticket waits for the customer

*As a support lead, I want the clock to stop while we are waiting for the customer, so that a ticket is not breached for time we could do nothing with.*

- A ticket moving to `pending` (waiting for the customer) **stops its SLA clock**: `paused_at` records the moment. While it is set, breach is judged at that moment, not at the present, so a deadline that falls while the ticket waits is **not** a breach (the ticket list, the sweep, the SLA dashboard and the support insights all read it so, and a paused ticket is not counted as at risk). A ticket that was already late when it was paused stays late.
- Leaving `pending` (to `open` or `resolved`) **starts it again** and moves each deadline by the waiting that counted against it: in elapsed time on a policy without a calendar, in **working time** on a policy with one (CRM-R45), so a ticket paused on Friday evening and resumed on Monday morning gets back only the working time it waited.
- The waiting counts **only up to the deadline** it counts against, and not at all for a metric already met (a first response given, a ticket resolved). So pausing after a deadline has passed does not undo the breach.
- Each ticket keeps `first_response_paused_seconds` and `resolution_paused_seconds`. Deadlines are always the open time plus the target plus that waiting, so an audited **priority change** (which derives them again from the open time) keeps the waiting already counted. Waiting that is still in progress is counted when the ticket resumes.
- Breach facts already stored (`first_response_breached`, `resolution_breached`) stay as they are: a breach is not cleared by pausing.
- The tickets page marks a waiting ticket "SLA clock stopped" where it would otherwise show nothing.
- Not built: pausing on other states; a customer reply resuming the clock (built in CRM-R63); a limit on how long a ticket may wait (built in CRM-R64; a ticket left `pending` never breaches an SLA deadline); showing the time the clock has been stopped; re-deriving the deadlines when a policy or a calendar changes.

## CRM-R56 — Time-decay and position-based attribution

*As a marketer, I can share the revenue of a won deal by how recent each touch was, or by its place in the journey, so that the first and last campaigns, or the campaigns nearest the sale, are rewarded as my team believes they should be.*

- `GET /api/attribution` accepts two more values of `model`: `time_decay` and `position` (the answer's `model` echoes it). Anything else is `422` naming `/model`, and the message lists all five.
- **`time_decay`**: a touch's weight halves for every **7 days** between the touch and the moment the deal closed, so a touch 0, 7 and 14 days before the close weigh 4 : 2 : 1. Between whole half-lives the weight falls in a straight line, so the arithmetic is exact in integers (within about 6% of the true curve); a touch from after the close counts as having just happened. The half-life is fixed, not asked for.
- **`position`**: 40% to the first touch, 40% to the last, 20% shared equally among those between; one touch takes everything and two take half each.
- **Exact.** Both give a deal's shares **to the minor unit**: each touch gets the floor of its proportion and the leftover minor units go one each to the largest remainders, the earliest touch first on a tie. The method is the one `linear` always used, so `first`, `last` and `linear` give the answers they gave before. The same campaign touching twice earns both shares, as under `linear`.
- Windows, conversion (CRM-R51), masking and audit are unchanged. The Attribution page offers the two models.
- Not built: a half-life or position weights the caller can set; a "W-shaped" model with a mid-journey peak; models that use the kind or the cost of a touch.

## CRM-R57 — Public help centre and deflection report

*As a support lead, I can let customers read our published articles without signing in, and see how many questions they answered for themselves, so that fewer of them become tickets.*

- **Off until asked for.** `CRM_PUBLIC_KB` (off by default) turns on three routes that need no token and answer `404` while it is off, so a deployment publishes nothing by accident: `GET /kb/articles`, `GET /kb/articles/{pid}` and `POST /kb/articles/{pid}/feedback`. They are public paths when `CRM_REQUIRE_AUTH` is on. Each client is limited to `CRM_PUBLIC_KB_CLIENT_LIMIT` requests a minute (default 120, `0` for no limit), counted as the web forms are (the trusted forwarded address when `CRM_TRUST_FORWARDED_FOR` is set); over it is `429` with `Retry-After`.
- **Published only.** A draft, an archived or a deleted article is `404`, the same answer as one that never existed, so nothing is given away. `GET /kb/articles?q=` lists published articles (at most 50, by title; each with the first 200 characters of its text), or those whose title, keywords or text contain `q` (case-insensitive; `%` and `_` are plain characters). `GET /kb/articles/{pid}` returns title, text, keywords, version and last update. The text is **plain**: a client shows it as text, never as markup.
- **Feedback.** `POST /kb/articles/{pid}/feedback` takes `{"helpful": true|false}` and nothing else (an unknown member is `422`); it answers `202`. It is anonymous: nothing identifies who voted.
- **What is kept.** Only counts, per day and per article (and for the portal as a whole): views, "this helped" and "this did not" votes, searches with words in them, and of those the searches that found nothing. **No address, no visitor and no text a visitor typed is stored**, because people type their own names and problems into a search box. The cost is that a search that finds nothing cannot say what was wanted; the count says only that the content has gaps.
- **Report.** `GET /api/insights/kb-deflection?days=` (1 to 365, default 30; anything else `422` naming `/days`; a read, audited as `kb_deflection_read`) answers: the period, views, searches, searches without a result, helpful and not-helpful votes, the tickets opened in the period, the ten most viewed articles with their votes, and `deflection` = `helpful_yes ÷ (helpful_yes + tickets_opened)` as a ratio with its numerator and denominator (`null` value when both are zero). **It is an estimate, not a measure**: a visitor is not identified, so a vote is a visitor's own word, a ticket is not tied to a visit, and a vote can be given by anyone (the rate limit slows that, it does not stop it). The answer says so.
- The UI has `/help` and `/help/{pid}`, open to a signed-out visitor and linked from the footer, and a "Public help centre" panel on the Articles page with the counts.
- Not built: a vote or a count per language; a search that suggests articles as a ticket is typed; counting a ticket as deflected when it was opened after a visit; showing the portal under a customer's own domain or theme; per-article feedback comments; a limit on all clients together (the limit is per client, so many clients can add up); rate-limit state shared between service instances (it is kept in each instance's memory, as for the web forms).

## CRM-R58 — Opens and clicks as attribution touches

*As a marketer, I can count the campaigns whose e-mail a buyer opened or clicked as touches on the sale, so that attribution rewards engagement and not only a send.*

- **Where the data comes from.** The service **never turns tracking on**: it does not ask a provider to add an open pixel or rewrite links (the SendGrid request switches both off so the signed unsubscribe link stays as written). If an operator turns tracking on at the provider, the provider's open and click events arrive on the webhooks already used for delivery outcomes (`POST /webhooks/email`, `/webhooks/email/{provider}`) and are recorded **once per message and kind**. `POST /webhooks/email` accepts the events `opened` and `clicked` beside `delivered`, `bounce_hard`, `bounce_soft` and `complaint`. The vendor names are read as: Brevo `opened` and `click`; Mailjet `open` and `click`; Mailgun `opened` and `clicked`; SendGrid `open` and `click`; SES `Open` and `Click`. Azure's engagement events are not read.
- **Only while the person consents.** An open or click is recorded only if the contact's consent for the campaign's purpose is still `granted`, they exist, and their mailbox is not suppressed. Otherwise the event is acknowledged as `ignored` and nothing is kept. A person who has withdrawn is not profiled by what they open or click.
- **What it adds up.** A first open or click also counts on the campaign's `opened` and `clicked` (a nurture step has no such counter). A repeat of the same event for the same message is `duplicate` and changes nothing. The moment of a touch is when the service **learned** of it (the provider's own timestamp is not read), so a late report is a late touch.
- **Attribution.** `GET /api/attribution` takes `touches`, a comma-separated list of `sent`, `opened` and `clicked` (default `sent`, so an answer is as it was). Anything else is `422` naming `/touches`. `sent` counts the e-mails sent in the window as before; `opened` and `clicked` count the events recorded in the window. Each is a touch, like any other, so a campaign whose e-mail was sent, opened and clicked has three touches under `linear` and earns three shares; first, last, time-decay and position-based treat them in time order as usual. Campaign leads and the recorded source campaign always count. The answer names the `touches` used.
- The Attribution page offers the three kinds as checkboxes (sent by default; with none chosen it cannot run) and says that opens and clicks are what the provider reports and overcount.
- **Not reliable, and said so.** Mail programs that load images in advance (Apple Mail's privacy protection) record an open nobody made, and link scanners record a click nobody made, so opens and clicks overcount; the page says so beside the choice. A person who blocks images is not counted at all.
- Not built: reading a provider's own event time; unique versus repeat opens (only the first of each kind is kept); excluding probable machine opens; touches by the deal's account's other contacts; asking a provider to enable tracking; Azure engagement events; a retention limit on the events beyond what already applies to e-mail events.

## CRM-R59 — Quotas and commission plans

*As a sales lead, I can set what each worker is to win in a period, pay commission on it by a tiered plan, and see where each worker stands, so that targets and pay come from the same won deals the reports use.*

- **Admin only.** `/api/commission-plans`, `/api/quotas` and `GET /api/quotas/attainment` are for admin and service callers with any method, reads included: what a person is paid is neither theirs to set nor everyone's to read. Every write and every attainment read is audited.
- **Plans.** `PUT /api/commission-plans` sets the plan of that name (unique in any case), replacing its tiers; `GET` lists them by name with how many quotas use each; `DELETE` removes one, `409` while a quota is paid under it. Tiers are **marginal**, like tax bands: 1 to 8, the first starting at `0`, each starting above the one before (at most 1000% of the quota), each paying a rate in **basis points** (`500` is 5%, at most `10000`) on the revenue between its start and the next one's.
- **Quotas.** `PUT /api/quotas` sets a worker's (`worker:<id>`) quota for a period (at most 366 days, the end inclusive) in one currency, as an amount in minor units above zero, with an optional plan. Setting the exact same period replaces it; a period that overlaps another of the same worker and currency is `422` naming the member. `GET /api/quotas?owner_ref=` lists latest period first; `DELETE` removes one.
- **Attainment.** `GET /api/quotas/attainment?owner_ref=&at=` gives, for each quota whose period contains `at` (today when absent), the worker's **won revenue in the quota's currency** from deals closed in the period, the deals counted, `revenue ÷ quota` as a ratio with both numbers, what remains (zero once met), the commission and each band's revenue and commission. **Other currencies are listed and never converted or added in.**
- **Exact.** Tier thresholds are whole minor units of the quota (rounded down) so the bands tile with no gap and no overlap; commission is integer arithmetic, rounded half to even once per band.
- The UI has `/quotas` (admin) with the standing, the quotas and the plans.
- Not built: territories; team and manager roll-ups; quotas by product or pipeline; accelerators that reach back to earlier revenue; clawbacks; splitting a deal between workers (built in CRM-R65); a quota in one currency measured from another through the exchange rates.

## CRM-R60 — Margin and a discounted look-ahead on customer lifetime value

*As an account manager, I can see what an account's lifetime value is after the margin the business keeps, and what the account is still expected to bring if it stays, discounted to today, so that CLV is more than the revenue already won.*

- `GET /api/accounts/{pid}/clv` takes `margin_bps` (0 to 10000), `retention_bps` (0 to 10000) with `years` (1 to 20), and `discount_bps` (0 to 10000, `0` when absent, only with those two). Anything else is `422` naming the member; `retention_bps` and `years` go together. Without them the answer is as in CRM-R13 and CRM-R54.
- **Margin.** With `margin_bps`, `margin_minor` gives each currency's `clv_minor × margin ÷ 10000`, rounded half to even. `clv_minor` is unchanged.
- **Look-ahead.** With `retention_bps` and `years`, `projection` gives, per currency: `annual_base_minor` (revenue, or its margin when `margin_bps` is given, of deals won in the 365 days to `as_of`), `projected_minor` and `total_minor` (what has been won, as margin when given, plus `projected_minor`). Year `t` is year `t-1` × `retention ÷ (1 + discount)`, in integers with each year rounded half to even, and `projected_minor` is the sum of the years. Currencies are never added or converted.
- **Masking** as before: a masked caller gets `null` for `margin_minor` and `projection`.
- The account page shows them in a panel (CRM-T127), typing percentages that become basis points; `convert_to` does not apply to `margin_minor` or `projection`.
- Not built: converting the margin or projection (built in CRM-R62); a retention taken from the account's own history (built in CRM-R61); a margin per product or deal; the rate on each deal's close day; discounting the revenue already won back to its close day.

## CRM-R61 — Retention taken from an account's own history

*As an account manager, I can have the look-ahead on an account's lifetime value use the retention the account has actually shown, so that I do not have to guess a number.*

- `GET /api/accounts/{pid}/clv` takes `retention_from=history` in place of `retention_bps`, with `years` (and `discount_bps`) as in CRM-R60. Any other value, the two together, or `years` without a retention is `422` naming the member.
- **Retention** per currency is the revenue won in the 365 days to `as_of` divided by the revenue won in the 365 days before those, in basis points, rounded half to even and held at 10000: history is never read as growth. Currencies are never added or converted.
- Where nothing was won in the year before, there is no retention to find: it is `null` in `projection.retention_history_bps`, nothing is projected for that currency (`projected_minor` is `0`) and what was won still counts in `total_minor`. A number is never guessed.
- `projection.retention_bps` is `null` when the retention was read from history; `retention_history_bps` is `null` when it was supplied. Masking, margin and `convert_to` are as in CRM-R60.
- The CLV panel on the account page has a checkbox for it.
- Not built: a window other than the two years; weighting recent quarters; growth above 100%; retention by deal count or by customer.

## CRM-R62 — Converting the CLV margin and look-ahead

*As an account manager with deals in several currencies, I can see the margin and the look-ahead of an account's lifetime value in one currency, so that the whole account reads as one figure.*

- With `convert_to` (and `rate_date`) as in CRM-R54, `GET /api/accounts/{pid}/clv` also gives `margin_converted_minor` (when `margin_bps` is given) and `projection.converted` with `annual_base_minor`, `projected_minor` and `total_minor` (when the look-ahead is). Each currency's figure is converted **once** at the rate in force on `rate_date` and the results added; the per-currency figures stay as they were.
- The same rates and the same currencies as the CLV, so `rates` and `unconverted` describe all of them; a currency with no rate is left out of every converted figure and never taken at 1.0.
- Without `convert_to` the new members are `null`; a masked caller gets `null`.
- The CLV panel on the account page has a "Show in" field.
- Not built: converting at each deal's close day; converting before the margin is taken.

## CRM-R63 — A customer reply resumes a waiting ticket

*As a support agent, I can record that the customer has answered, so that a ticket stops waiting and its SLA clock runs again without my choosing a status by hand.*

- `POST /api/tickets/{pid}/customer-reply` (no body). A ticket in `pending` goes to `open`, and the clock starts again exactly as for `POST /api/tickets/{pid}/status` (CRM-R55): the waiting is added to each deadline it counted against, working time on a calendar.
- A ticket already `open` is returned as it is and nothing is audited. A ticket that is `resolved` or `closed` is `422`: a reply does not reopen it. An unknown ticket is `404`.
- The audit entry is the ordinary status change with `via: customer_reply` beside `from`; the same event is emitted.
- The tickets page shows "Customer replied" on a waiting ticket.
- Not built: recognising a reply from a channel other than e-mail (the caller says so; e-mail is CRM-R88); reopening a resolved ticket on a reply; a limit on how long a ticket may wait.

## CRM-R64 — A limit on how long a ticket may wait for the customer

*As a support lead, I want to be told when a ticket has waited too long for the customer, so that a ticket that is stopped on the clock is not forgotten.*

- `POST /api/sla-policies` takes an optional `max_wait_minutes` (above zero, otherwise `422` naming the member): the most a ticket of that priority may spend in `pending`. Left out, there is no limit.
- `POST /api/sla/sweep` finds each `pending` ticket whose policy has a limit and which has waited longer than it (elapsed minutes since `paused_at`, not working time). It stores the moment in `wait_limit_exceeded_at`, emits `ticket_wait_limit_exceeded` **once**, and counts them in `new_wait_limits_exceeded`. A ticket already flagged, one without a limit and one inside it are not touched.
- **Nothing else changes**: the ticket stays `pending`, its clock stays stopped and no SLA breach is recorded; the flag is a fact for people to act on, not a closure.
- Leaving `pending` (a status change or a customer reply, CRM-R63) clears `wait_limit_exceeded_at`.
- The tickets page marks a flagged ticket "Waiting too long".
- Not built: closing or escalating the ticket by itself; a limit in working time; a limit per account or per customer; setting it on an existing policy (a policy is created with it); a warning before the limit.

## CRM-R65 — Splitting a won deal between workers

*As a sales manager, I can credit a deal to more than one worker, so that quota attainment and commission follow who did the work and not only who owns the record.*

- `PUT /api/quotas/splits/{pid}` (the deal's pid) with `shares`: from 1 to 8 workers (`worker:<id>`) named once, each from 1 to 10000 basis points, adding up to exactly 10000. It replaces any earlier split. Anything else is `422` naming `/shares`; an unknown or deleted deal is `404`. Audited.
- `GET /api/quotas/splits/{pid}` gives the shares (none when the deal is credited wholly to its owner); `DELETE` removes the split (`404` when there is none) and the owner is credited again. All three are admin only, like the quotas (CRM-R59).
- **Attainment** (`GET /api/quotas/attainment`): a won deal that is split is credited to each worker named, **not** to its owner unless they are named; one that is not split is credited to its owner as before. A worker's share of the amount is exact: each share gets the floor of its proportion and the leftover minor units go one each to the largest remainders, the earliest share first on a tie, so the shares always add up to the amount. Currencies are never converted. A split deal counts once in each named worker's `won_deals`.
- Splits are not retroactive bookkeeping: they change today's reading of attainment for any period the deal closed in.
- The `/quotas` page can split a won deal and remove the split.
- Not built: splitting an open deal ahead of its close; splits by role or by percent of margin; shares that differ by line item; the deal record itself showing its split; clawing a share back; a split history beyond the audit trail.

## CRM-R66 — Articles suggested for a ticket

*As a support agent, I am shown the help articles that may answer the ticket I am looking at, so that I can point to one instead of writing the answer again.*

- `GET /api/tickets/{pid}/suggested-articles` gives the **published** articles (never a draft or an archived one) that share words with the ticket's title, best first, at most 5. Each has `article_pid`, `title`, `score` and `matched` (the shared words, alphabetical). An unknown ticket is `404`; a ticket that shares nothing with any article gets `[]`.
- **Words** are the title lower-cased and cut at anything that is not a letter or digit; words under three characters and a short list of common English words are dropped. Each word counts once. A word found in an article's title or keywords scores 2; one found only in its text scores 1. Ties go to the earlier article. Plain overlap, in integers: the same ticket and articles always give the same answer, and the score can be explained from `matched`.
- Nothing is recorded: a suggestion is not a view, not a deflection (CRM-R57) and not an audit event.
- The tickets page lists them under the selected ticket.
- Not built: suggesting as a ticket is typed (there is no new-ticket form in the client); other languages or stemming; the ticket's description and replies (a ticket has only a title); learning which suggestions were used; suggestions on the public help centre.


## CRM-R67 — Teams and their quota roll-up

*As a sales manager, I can group workers into teams and see a team's attainment, so that I can read how a team is doing without adding up each person by hand.*

- `PUT /api/quotas/teams` with `name` (up to 80 characters) and `members`: from 1 to 50 workers (`worker:<id>`) named once. A team of that name in any case is replaced, not duplicated. Anything else is `422` naming `/name` or `/members`. Audited.
- `GET /api/quotas/teams` lists the teams by name; `DELETE /api/quotas/teams/{pid}` removes one (`404` when there is none) and leaves its workers' quotas alone. Admin only, like the quotas (CRM-R59).
- `GET /api/quotas/team-attainment?at=` (today when absent) adds the members' standings (CRM-R59, with splits, CRM-R65) **per currency**: quotas, credited won revenue, won deals, attainment as a ratio, what remains, and how many workers have a quota in that currency. Members with no quota containing the day are listed in `without_quota`; their revenue in a currency the team has no quota in is listed in `other_currencies`. Currencies are never added together or converted. An audited read.
- The `/quotas` page keeps teams and shows each team's roll-up.
- Not built: a team inside a team; a worker's team over time; a team's own quota; territories and clawbacks.

## CRM-R68 — Territories of named accounts

*As a sales manager, I can name the accounts that make up a territory, so that I can see what has been won in each one.*

- `PUT /api/quotas/territories` with `name` (up to 80 characters), an optional `owner_ref` (`worker:<id>`) and `accounts`: from 1 to 200 organizations (`organization:<id>`, the `organization_ref` an account carries) named once. A territory of that name in any case is replaced. An account is in **at most one territory**: naming one that is in another territory is `422` naming `/accounts` and that territory. Anything else is `422` naming `/name`, `/owner_ref` or `/accounts`. Audited.
- `GET /api/quotas/territories` lists them by name with their accounts; `DELETE /api/quotas/territories/{pid}` removes one (`404` when there is none) and frees its accounts. Admin only, like the quotas (CRM-R59).
- `GET /api/quotas/territory-attainment?from=&to=` (the 90 days to today when absent; `422` if `from` is after `to` or the period is over 366 days) adds up the **won, live deals closed in the period** whose account's `organization_ref` is in the territory: revenue and deals **per currency**, never added or converted. A deal counts **whole** in the territory that holds its account, whoever it is credited to (splits, CRM-R65, do not apply). Revenue on accounts in no territory, and on deals with no account, is listed as `unassigned`. An audited read.
- A territory has no quota of its own; it reports what was won.
- Not built: territories by country, region or tag (the CRM holds no geography); a territory's own quota; rules that route a new account to a territory; moving open deals.
- The `/quotas` page keeps territories and shows what was won in each.

## CRM-R69 — What happens to a ticket that waits too long

*As a support lead, I choose what the sweep does with a ticket left waiting past its limit, so that a ticket stopped on the customer is dealt with and not only flagged.*

- `POST /api/sla-policies` takes an optional `wait_limit_action` beside `max_wait_minutes` (CRM-R64): `flag` (the default and the earlier behaviour), `resolve` or `escalate`. An action other than `flag` needs a `max_wait_minutes` to act on; anything else is `422` naming `/wait_limit_action`.
- `escalate` also takes `escalate_to`, a worker (`worker:<id>`), required for it and refused with any other action (`422` naming `/escalate_to`). The sweep flags the ticket as in CRM-R64 and **reassigns it to that worker**; it stays `pending` and its clock stays stopped. Audited as `ticket_escalated` with `via: wait_limit`.
- `resolve` resolves the ticket as a status change from `pending` does (CRM-R55): the clock resumes, `resolved_at` is set, audited with `via: wait_limit`. A customer reply cannot reopen a resolved ticket (CRM-R63), but a status change to `open` can. It is **resolved, not closed**: `closed` is final and can only follow `resolved`, so an automatic step stops short of it and a person closes the ticket.
- Each ticket is acted on **once**, as before. `POST /api/sla/sweep` counts them in `new_wait_limits_exceeded`, with `new_wait_limit_resolved` and `new_wait_limit_escalated` saying how many of those were resolved or reassigned. `ticket_wait_limit_exceeded` is emitted for each, with the `action` taken.
- Not built: closing a ticket automatically (see CRM-R82 for closing one left resolved); escalating by raising its priority; a limit or an action on an existing policy (a policy is created with them); a warning before the limit; telling the customer.

## CRM-R70 — CLV at the rate of the day each deal closed

*As a finance analyst, I can have an account's lifetime value converted at the rate in force when each deal closed, so that the figure does not move with today's rate.*

- `GET /api/accounts/{pid}/clv?convert_to=GBP&rate_basis=close_day` converts **each won deal** at the rate in force on the day it closed (the latest on or before that day, the pair's own or the inverse of the reverse pair's, as in CRM-R50), rounds it **once per deal** (half to even) and adds the results as `converted_minor`. A deal already in the target currency is kept as it is. `rate_basis` is `close_day` or `422`.
- `rate_date` is `null` (each deal has its own day) and `rate_basis` is `close_day`. `rates` lists each **distinct** rate used, with the day it took effect; `unconverted` names, per currency, the deals with no rate in force on their close day and their amounts: **never taken at 1.0, never dropped quietly**. The per-currency `clv_minor` is unchanged.
- It is `422` without `convert_to`, with `rate_date`, or with `margin_bps` or a look-ahead (`retention_bps`, `retention_from`, `years`): the margin and the look-ahead stay on the one date of CRM-R62.
- A masked reader gets `null` as for any CLV figure.
- The account page has a checkbox "Each deal at the rate of the day it closed", available once a currency is chosen and no margin or look-ahead is asked for.
- Not built: the margin and look-ahead at close-day rates; the same for the other converted reports (attribution, ROI, forecast); converting through a third currency.

## CRM-R71 — Clawing credit back from a worker

*As a sales manager, I can take an amount back from what a worker is credited for a won deal (a refund, a cancellation), so that attainment and commission do not pay for revenue that was returned.*

- `PUT /api/quotas/clawbacks/{pid}` (the deal's pid) with `owner_ref` (`worker:<id>`), `amount_minor` and an optional `reason` (up to 200 characters). The amount is above zero and **no more than that worker's credit for the deal** (their whole amount, or their share of a split, CRM-R65); the deal must be **won** and credit that worker. Otherwise `422` naming `/amount_minor` or `/owner_ref`; an unknown or deleted deal is `404`. A worker has at most one clawback per deal: setting another **replaces** it. Audited. It answers every clawback on the deal.
- `GET /api/quotas/clawbacks/{pid}` lists them (`404` for an unknown deal); `DELETE /api/quotas/clawbacks/{pid}?owner_ref=` gives the credit back (`404` when there is none). All three are admin only, like the quotas (CRM-R59).
- **Attainment** (`GET /api/quotas/attainment`): the worker's credit for the deal is reduced by the clawback, never below zero, **for them alone**; other workers on a split keep theirs. A deal whose credit to a worker is clawed back whole no longer counts in that worker's `won_deals`. Currencies are never converted.
- The deal itself, its amount and its other uses (forecast, win rate, CLV) are untouched: a clawback is about what a worker is credited with.
- Not built: a clawback reaching into a period already paid; a clawback in another currency; clawing back commission directly; a list of clawbacks on the `/quotas` page (it has a form to set and remove one); automatic clawbacks when a deal is reopened or lost.

## CRM-R72 — A won deal opens a delivery idea in PPM

*As a sales manager, I can hand a won deal to delivery, so that the project portfolio service has an idea for the work that was sold.*

- `POST /api/bridges/ppm/deals/{pid}` (no body) sends **one idea** to the project portfolio service: `POST {CRM_PPM_SERVICE_URL}/api/ideas` with a `title` (`Deliver: <deal name>`, up to 200 characters), a `pitch` (the deal's pid, amount in minor units and currency) and the tags `crm` and `won-deal`. Nothing else about the deal, its account or its contacts is sent. PPM answers `{pid}`.
- It is **once per deal**: the idea's id is kept, and asking again answers it with `already_opened: true` and sends nothing. An unknown or deleted deal is `404`; a deal that is not won is `422`; PPM unreachable, refusing or answering without an id is `502` and nothing is kept. Audited as `deal_delivery` / `opened`.
- **Stub-first**, like the other upstream clients (CRM-D11): unless `CRM_UPSTREAM_MODE=http`, nothing is sent and nothing is kept; the answer is `mode: stub`, `sent: false` and the idea that would have gone. The routes under `/api/bridges` are **admin only**, since they send data out.
- PPM's contract (`POST /api/ideas`) was read from its source and checked against it again on 2026-10-28 (`{title, pitch, tags}` in, `{pid}` out; a title of 1,024 bytes at most, ours is 200 characters; tags of 64 at most); it is exercised against a stand-in in the tests, not against a running PPM; the idea is PPM's to promote or drop. A bearer token is sent when `CRM_PPM_SERVICE_TOKEN` is set (CRM-R87); without one it reaches a PPM only while that has authentication off.
- A won deal's page has a button "Open a delivery idea in the project portfolio" and says what happened.
- Not built: a project (PPM has plans and ideas, not projects); opening one automatically when a deal is won; following the idea's fate in PPM; sending line items or contacts.

## CRM-R73 — Teams from the worker service's departments

*As a sales manager, I can have teams made from the worker service's departments, so that a team's roll-up (CRM-R67) follows who is employed where without my keeping both lists.*

- `POST /api/bridges/wpm/teams-import` (no body) reads the **active** workers of the worker service (WPM) — `GET {CRM_WORKER_SERVICE_URL}/api/workers?status=active&limit=200&offset=`, page by page, at most 100 pages — and sets **a team per department**, named for it, holding that department's workers (`worker:<pid>`), as `PUT /api/quotas/teams` would (CRM-R67). Only each worker's `pid` and `department` are read; pay and employment details are never taken.
- **All the teams are set together or none**; each is audited with `via: wpm`. Teams the import does not name are left as they are; **nothing is written to the worker service**. Running it again replaces the same teams.
- Left out and reported in `skipped`, with the reason: a department of **more than 50** workers (a team's limit); a department that differs from another only in case; a worker whose id is not a reference. Active workers with no department are counted in `without_department`.
- `502` when the worker service cannot be reached or answers something that is not a list. **Stub-first** and admin only, as CRM-R72: without `CRM_UPSTREAM_MODE=http` nothing is read and the answer is `mode: stub` with no teams.
- WPM's contract (`GET /api/workers`) was read from its source and checked against it again on 2026-10-28 (a bare JSON array ordered by id, `pid` and `department` members, `status=active` a real status, a page of at most 500, an offset of at most 10,000, so at most 51 pages of 200 are read); it is exercised against a stand-in, not a running WPM. A bearer token is sent when `CRM_WORKER_SERVICE_TOKEN` is set (CRM-R87); WPM filters the list by the caller's organizations when it has authentication on, so the import sees the workers that token may see, not necessarily all.
- The `/quotas` page has a button to run it and says how many teams were set.
- Not built: teams by manager and reports; removing a team when its department disappears; importing on a schedule; any other worker fields.

## CRM-R74 — A personal-data inventory, a retention schedule, a DPIA and a record of processing

*As a data protection officer at an adopting organization, I can see what personal data the service holds, how long, and what it does with it, so that I can complete the organization's own assessment and record.*

- `spec/compliance/personal-data-inventory.md` lists **every table** with the personal data in it, whom it is about, its retention class and what erasure does to it. The classes are `swept` (hard-deleted by the retention sweep after `CRM_RETENTION_DAYS`), `evidence` (kept on purpose), `ttl` (expires by itself), `manual` (kept until a person removes it) and `gap` (personal data and no deletion path).
- A test fails if a table has no row, a row has no table, a class is unknown, or the `swept` tables are not exactly those the sweep deletes from. A new table cannot be added without being described.
- `dpia.md` is the DPIA template filled in as far as the software goes: system facts, an engineering assessment of likelihood and severity as shipped, and the deployer's own decisions left open. `record-of-processing.md` is the Art. 30 record, started from ten processing activities, with the controller's details left open. Neither is a sign-off, legal advice or reviewed by anyone.
- The gaps the inventory names are not hidden: email deliveries and events, the event outbox and webhook deliveries have no deletion path (CRM-T140).
- Not built: filling the deployer's decisions; a generated record from the code; per-field classification beyond the table.

## CRM-R75 — A translation-review register

*As someone responsible for what a person reads in their own language, I can see which locales have been reviewed by a native speaker and which have not, so that no draft is mistaken for reviewed wording.*

- `spec/translation-register.md` in the UI folder has a row per locale file: language, status (`source`, `draft` or `reviewed`), method, reviewer, date reviewed, the number of strings and how many are identical to English. A `reviewed` row names the reviewer and the date. Every locale other than English is `draft`: **none has been reviewed by a native speaker**.
- A test fails if a locale file has no row or a row no file, a count is out of date, a `reviewed` row lacks a reviewer or a date, a locale lacks a key English has or has one it does not or has an empty string, or a string carries different `{placeholders}` from the English one.
- Not built: a review workflow in the UI; marking individual strings reviewed; pluralisation rules per language.

## CRM-R76 — No page shows the service's error text

*As a reader, I see an error in my own language and in plain words, so that nothing the service wrote for a developer (a record's id, a rule, a person) appears on my screen.*

- A failed call becomes a **fixed, translated message chosen by the kind of failure**: not accepted (`400`, `422`), sign in again (`401`), no permission (`403`), not found (`404`), conflicts with what is there (`409`), too many requests (`429`), service unreachable or unavailable (`5xx` and a failed `fetch`), and a generic one for anything else. The same message is shown for the same kind of failure in all eight locales.
- The service's `description`, a problem's `title` and `detail`, and an error's own `message` are **never shown**. `fieldErrors` still names which members were not accepted (so a page can mark them) but says only "Not accepted" for each, never the service's `detail`.
- A page that has a more specific message of its own passes it as the fallback, or throws a `PageError`, whose text is already the page's own and in the reader's language.
- Enforced three ways: `errorMessage` and `fieldErrors` in `src/lib/errors.ts` are the only code that reads an error; an ESLint rule (`no-restricted-properties` on `message`, `no-restricted-syntax` on `String(error)`) fails lint in the routes and components; and a unit test scans them for the same patterns and checks every kind of failure in every locale.
- This does not touch what the service *returns as data* (a workflow step's detail, a search hit's detail, an audit finding): that is content, not an error.
- Not built: showing a request id so a person can quote it to support; a different message per endpoint.

## CRM-R77 — Every page is scanned for accessibility

*As a person who uses assistive technology, I can expect no page to have been left out of the automated check, so that a new page cannot ship with a barrier an automatic scan would have found.*

- `tests/e2e/a11y.spec.ts` runs axe-core with the WCAG 2.0, 2.1 and 2.2 A and AA rules over **every page route**. A signed-in page is scanned in the tour with its sample data (tables, forms, records and a calendar on the page, not empty lists); the public pages are scanned signed out. A violation fails the build.
- The pages and how to reach them are one table, `tests/e2e/a11y-routes.ts`: the route as the file tree names it, a real address (a sample id in place of `[pid]`, a real locale in place of `[locale]`), and how to get there. A page that is not scanned stays in the table with a **written reason**.
- A unit test fails if a page exists under `src/routes` that the table does not list, an entry names a page that does not exist, a dynamic page has no real address, or an exclusion has no reason. A new route therefore cannot go unscanned.
- Running it over the pages that had never been scanned found five barriers, all fixed: error text too faint to read (`.error`), contact links too small to hit and links in running text told apart by colour alone, a pipeline drop-down with no name, and calendar days whose name did not contain their visible text.
- This is an automatic check, which finds only a part of what is wrong. It is **not** an accessibility audit: the manual audit with screen readers and keyboards (CRM-T48) has not been done.
- Not built: scanning the states a page reaches after an action (a dialog open, an error shown); scanning each locale and each theme.

## CRM-R78 — Small groups are left out of analytics

*As a data protection officer, I want a report never to show a group so small that it describes one person or one deal, so that an aggregate cannot be used to read a record.*

- A report (CRM-R48, saved or ad hoc) **leaves out every group with fewer than `CRM_MIN_CELL_SIZE` records behind it**: 5 by default, `0` for none, at most 1000. It is left out, not shown as a small number or as zero.
- What counts as the group's size: for a count, the count; for a sum of amounts, the **number of deals summed** in that group and currency (a sum over one deal is that deal's amount, however large the figure); with `convert_to`, the deals in the group across all its currencies, since a conversion adds them up.
- The answer carries `min_cell_size` (what was applied) and `suppressed` (how many cells, or with `convert_to` groups, were left out), so a short table is not mistaken for a complete one. A filter that narrows a group to a few records does not get round it. The audit entry for a run records the same two numbers.
- Applies to every caller, administrators included: a data-protection measure is not a privilege. It applies to the report builder, where a caller can group person-level records; the fixed dashboards (win rate, pipeline by stage, SLA, activity) are totals over the whole book, not groupings by a person-level field, and are unchanged.
- **Primary suppression only.** An answer carries no total, so a left-out cell cannot be got by subtraction from it; but two reports asked in different ways (one grouped by status, another by status and then filtered to one owner) can still be compared by a determined caller. Secondary suppression across reports is not built.
- The Reports page says how many groups were left out and why.
- Not built: secondary suppression, a different threshold per kind or per caller, rounding of counts, noise; the fixed dashboards and the attribution, funnel and attainment views.

## CRM-R79 — Confirmation and preference links work once, and are kept only as a hash

*As a person who is mailed a link, I want it to work once and no longer, so that a copy of the email, or of the database, cannot be used to confirm or change what I agreed to.*

- The **confirmation** link (double opt-in, CRM-R29) and the **preference-page** link (CRM-R29) carry a random token of 256 bits, 43 URL-safe characters, made by the operating system's generator. The database keeps only its **SHA-256** (`link_tokens`: the hash, the kind, the contact pid, the purpose index, the times); the token itself exists only in the mailed link, so a copy of the table opens nothing.
- A token **works once**. It is spent by the request that acts on it, **in the same transaction as the act**: a confirmation that fails leaves the link usable, and two simultaneous requests are decided by the database, so exactly one wins and the other finds it spent. A spent, expired, unknown, mistyped or wrong-kind token is the same `404` page as before.
- **Opening the page spends nothing** (`GET /confirm/{token}` and `GET /preferences/{token}`), because mail scanners follow links. The button spends it.
- A **preference choice** spends the link and the page that answers carries a **fresh** token in its buttons, expiring when the first would have, so a person can make several choices but the link in the email works for the first only. A choice that is refused spends nothing.
- A token expires as before (confirmation and preference links are valid for the days they were issued for). The rows of tokens that expired or were spent **more than a week ago are deleted** by the hourly job that also drops idempotency keys, and **erasing a contact deletes its tokens**.
- The **unsubscribe link is not one of these**: it is a signed, stateless token that never expires and is not single-use, deliberately, so a recipient can leave from any old message; replaying it only withdraws consent again.
- Not built: single-use for the unsubscribe link; a keyed hash (a 256-bit random token needs none); revoking a person's links on demand other than by erasure; a "this link was already used" page that differs from "not valid or has expired".

## CRM-R80 — ESLint and Prettier are tests

*As a maintainer, I want a lint or formatting fault to fail the tests, not only a separate command, so that no one has to remember to run a second tool and the project's own lint rules are themselves tested.*

- `pnpm test` runs ESLint and Prettier through their programming interfaces (`tests/unit/lint.test.ts`, in Node, not the browser build): **the whole project must lint with no error and no warning**, and **every file Prettier can format must be formatted** (`src`, `tests`, `scripts` and the configuration files at the top; `.svelte` files have no Prettier parser here, and `.prettierignore` is honoured). The test fails naming each file and fault.
- The project's **own lint rule** (no page reads an error's text, CRM-R76) is tested: a sample page that reads an error's `message`, or prints an error as a string, is refused; the same in a script and in a component; and the one module that may read an error, and ordinary code, are left alone.
- `pnpm lint` and `pnpm format` cover the same files (`src`, `tests`, `scripts` and the top-level configuration files), where they covered only `src`; CI's front-end job already runs `pnpm run lint`, and its coverage run runs this test.
- Not built: Prettier for `.svelte` files (needs a plugin the project does not use); a format check of the Markdown or the locale files; the Rust side, whose `rustfmt` and `clippy` are CI steps already.

## CRM-R81 — Delivery and event logs are pruned by age

*As a data protection officer, I want the delivery and event logs to have a retention period like everything else, so that contact pids, message ids and event payloads are not kept for ever.*

- `POST /api/retention/sweep` (and its report, `GET /api/retention`) also covers four logs, each with a rule for which rows are **finished with**:
  - **email deliveries**: sent, or failed for good (five attempts), more than the horizon ago, and of a campaign that has `completed` or been `cancelled`, or of no campaign (a nurture step); a pending, retryable or running-campaign delivery is never deleted, since a run could still send or retry it.
  - **email events**: received more than the horizon ago.
  - **the event outbox**: published more than the horizon ago **and** handed to the webhooks; an event not yet published or fanned out is still owed to a subscriber and stays.
  - **webhook deliveries**: no longer `pending`, last changed more than the horizon ago.
- Two horizons, both settings: `CRM_EVENT_RETENTION_DAYS` for the outbox and webhook deliveries (30 by default, never below 7) and `CRM_MAIL_RETENTION_DAYS` for email deliveries and events (730 by default, **never below 400**, because attribution reads opens and clicks up to 365 days back from a deal's close). A value below its floor is raised to it.
- The report adds `event_horizon_days`, `mail_horizon_days` and `logs_past_horizon` (rows by table); the sweep adds `logs_deleted`, the two horizons and counts them in `rows_deleted`; the audit entry records them. The sweep stays an operator's act: nothing deletes on a timer.
- The inventory marks these four tables `aged`, and its test requires `aged` to be exactly the tables the sweep prunes by age.
- Not built: running the sweep on a schedule; separate horizons per table; archiving instead of deleting.

## CRM-R82 — A ticket left resolved is closed after its days

*As a support lead, I want a resolved ticket that nobody has reopened to be closed after a set number of days, so that finished tickets do not sit open on the board and the closed count means something.*

- `POST /api/sla-policies` takes an optional `auto_close_days`, a whole number from 1 to 365, refused otherwise (`422` naming `/auto_close_days`). Left out, nothing is closed automatically and a person closes the ticket, as before.
- `POST /api/sla/sweep` closes each live ticket whose status is `resolved` and whose `resolved_at` is **more than** that many days ago, for the policy of its priority. It is the same status change a person makes (`resolved` to `closed`, CRM-R63's lifecycle), audited as `ticket_closed` with `via: auto_close`, and emits `ticket_closed` like any other.
- The days count from the **last** resolution: reopening clears `resolved_at` and resolving again sets it afresh, so a reopened ticket is not closed on its first resolution's clock. A ticket the wait-limit action resolved (CRM-R69) counts from that moment.
- A closed ticket is final, so each ticket is closed **once**. The answer adds `new_auto_closed`.
- Not built: closing from the `pending` state or any state but `resolved`; a warning to the customer before closing; a different number of days per category; changing the days on an existing policy (a policy is created with them); running the sweep on a schedule.

## CRM-R83 — A report converts each deal at the rate of the day it closed

*As a finance lead, I want a report of deal amounts shown in one currency at the rate in force when each deal closed, so that a figure for last year is not restated at today's exchange rate.*

- `POST /api/reports/run` (and a saved report's spec) takes an optional `rate_basis` of `close_day`, beside `convert_to` (CRM-R50), as CRM-R70 does for an account's CLV. Anything else is refused (`422` naming `/spec/rate_basis`); so is a basis with no `convert_to`.
- It is for **closed deals only**: the `state` filter must be present and limited to `won` and `lost`, because an open deal has no close day. Otherwise `422` naming `/spec/filter`; the report is not quietly narrowed.
- It cannot be used with `rate_date`: each deal has its own day (`422` naming `/spec/rate_date`).
- The database adds up each group's amounts **per currency and per closing day**; each of those sums is converted once at the rate in force on its day (the latest on or before it, or the reverse pair inverted), then the groups merge. The rounding is therefore at most half a minor unit per group, currency and day, **not per deal**, which differs from CRM-R70 where a handful of deals are converted one by one. A sum already in the target currency is kept as it is; a sum with no rate on its day is **left out and counted** in `unconverted`, never taken at one-to-one.
- The answer's `rate_date` is `null` (no one day stands for it); `rates` lists each distinct rate used, so the same currency can appear more than once, with its effective day.
- Small-cell suppression (CRM-R78) judges a group by all its records, over all its days.
- A report that would read more than 20,000 group, currency and day cells is refused (`422`) and asks for a narrower one, rather than being cut off.
- Money, so a masked reader is refused as for any sum (CRM-R50).
- Not built: close-day rates for attribution, campaign ROI and the forecast; conversion per deal rather than per day sum; a day other than the one the deal closed (such as the invoice date).

## CRM-R84 — A starting hazard analysis in the manner of ISO 14971

*As an integrator who gives the software a medical purpose, I want the hazards it could contribute to, and the controls it has, set out against the ISO 14971 process, so that I start my risk management file from facts and not from a blank page.*

- `spec/compliance/risk-management.md` maps the clauses of ISO 14971:2019 to what is the project's, what is the deployer's and what is a gap; lists the foreseeable misuse; and lists fourteen hazards with the control in the software, the test that pins it where there is one, and a status in the evidence pack's own terms (Implemented, Deployer, Gap).
- It **claims no conformity**, is not a risk management file, says it has not been reviewed against the standard's text, and invents no severity, probability or acceptability: those depend on a clinical use the project does not have.
- It says the shipped product is not a device, points to the MHRA for that question in Great Britain, and says which controls (the wait-limit action, automatic closing) should not be switched on for care.
- Not built: a risk management plan, estimates, or a residual-risk judgement; a review by a clinical safety officer or a regulatory specialist; a test that every control named is still there.

## CRM-R85 — Attribution converts each deal's share at the rate of the day it closed

*As a marketing lead, I want the credit each campaign earns shown in one currency at the rate in force when each deal closed, so that last year's campaigns are not restated at today's exchange rate.*

- `GET /api/attribution` takes an optional `rate_basis=close_day` beside `convert_to` (CRM-R51), as an account's CLV (CRM-R70) and the report builder (CRM-R83) do. Anything else is refused (`422` naming `/rate_basis`); so is a basis with no `convert_to`, and one beside `rate_date` (`422` naming `/rate_date`).
- Every attribution is of **won deals**, so every deal has a close day and no further condition is needed.
- Each deal's **share** to a campaign, and the amount of a deal no campaign touched, is converted **once** at the rate in force on the day that deal closed (the latest on or before it, or the reverse pair inverted), then each campaign's converted shares are added. The rounding is at most half a minor unit per deal and campaign. A share already in the target currency is kept; one with no rate on its day is **left out and counted** in `unconverted`, never taken at one-to-one.
- The answer's `rate_date` is `null` and `rate_basis` says `close_day`; `rates` lists each distinct rate used, so one currency can appear more than once with its effective day.
- Unchanged: the models, the window, the touches, money-masked readers refused (CRM-R49, CRM-R58).
- Not built: close-day rates for campaign ROI and the forecast; a day other than the day a deal closed.

## CRM-R86 — A campaign's ROI converts each deal at the rate of the day it closed

*As a marketing lead, I want a campaign's won revenue in other currencies counted at the rate in force when each deal closed, so that the ROI of a campaign that ran last year is not restated at today's exchange rate.*

- `GET /api/campaigns/{pid}/funnel` takes an optional `rate_basis=close_day` beside `convert=true` (CRM-R52), as CLV (CRM-R70), the report builder (CRM-R83) and attribution (CRM-R85) do. Anything else is refused (`422` naming `/rate_basis`); so is a basis without `convert=true`, and one beside `rate_date` (`422` naming `/rate_date`).
- Each won deal attributed to the campaign in **another currency** is converted **once** at the rate in force on the day it closed (the latest on or before it, or the reverse pair inverted) and added to the campaign's own-currency revenue, which ROI then uses. Deals already in the campaign's currency are untouched.
- A deal with **no rate on its day** is left out, counted in `unconverted`, and its amount stays in `won_revenue_other_currencies`. So with this basis `won_revenue_other_currencies` holds **only what had no rate on its day**: a currency can be partly converted, and what is shown is the part that was not.
- The answer's `rate_date` is `null` and `rate_basis` says `close_day`; `rates` lists each distinct rate used, so one currency can appear more than once with its effective day. Cost, revenue and ROI stay masked for a masked reader.
- The tour has no foreign-currency deals, so it accepts and validates the option and answers as it does without it; the service's behaviour is tested against the real database.
- Not built: close-day rates for the forecast (live and snapshots) and the trend; a day other than the day a deal closed.

## CRM-R87 — The bridges can send a credential

*As an operator whose PPM and WPM have authentication on, I want the bridges to present a token, so that a won deal reaches PPM and the team import reaches WPM without switching their authentication off.*

- `CRM_PPM_SERVICE_TOKEN` and `CRM_WORKER_SERVICE_TOKEN` are optional. When one is set, the call to that service carries it as `Authorization: Bearer <token>`; when it is not, nothing is attached, as before. Each service gets **only its own** token.
- A token is **sent only over `https`, or over `http` to this machine** (`localhost`, `127.0.0.1`, `::1`). To any other `http` address the call fails with a short reason, before anything is sent, and the bridge answers as for any upstream failure (`502`). The client follows no redirect, so the token goes to the address configured and nowhere else.
- The token is **not logged, audited or returned**: the reason a call failed names neither it nor the address.
- The token is the deployer's to obtain from their identity provider and to renew: it is read when the service starts, so a renewal is a restart. The service does not fetch, refresh or validate it.
- What the receiving service lets the token do is that service's decision: WPM filters its worker list by the token's organizations, so the import sees the workers that token may see.
- Not built: fetching or refreshing a token; a token per caller (one is used for all); sending the caller's own token on; a credential for the person and organization lookups (CRM-R7).

## CRM-R88 — A customer's reply by e-mail resumes the ticket it answers

*As a support agent, I want a customer's e-mailed answer to restart a waiting ticket by itself, so that I need not press "Customer replied" for every message.*

- `POST /webhooks/inbound-email` (public, outside `/api`) takes `{"from", "subject"}` from a mail service the deployer has set to forward replies. It is **signed like `/webhooks/email`** (`X-CRM-Timestamp`, `X-CRM-Signature` over the body, `CRM_WEBHOOK_SECRET`): `401` when the signature is wrong or stale, `404` while no secret is set, `400` for a body that is not that shape. Any other member of the body (the text, the headers) is **not read and not kept**.
- A ticket's **reference** is `T-` and the first eight hex digits of its pid (`T-0a1b2c3d`). It is given as `reference` by `GET /api/tickets/{pid}` and shown on the tickets page; a reply finds the ticket by the first `[T-xxxxxxxx]` in its subject, in any case and after any `Re:` or `Fwd:`.
- The message resumes a ticket only if **all** hold: the subject names a reference; exactly one live ticket has it; the ticket has a contact; and the sender's mailbox (the part inside `<...>` when there is one, in lower case) **is the mailbox the person service gives for that contact** (CRM-R7), not an address the message names for itself. A ticket with no contact, or a contact with no known mailbox, is not resumed.
- The answer is always `200` with a `result`, so a mail service does not retry what it was right to ignore: `resumed` (a `pending` ticket goes to `open` exactly as for CRM-R63, the clock running again; audited with `via: inbound_mail`, no address in it), `already_open`, `ignored` (a resolved or closed ticket: a reply does not reopen it), `no_reference`, `unknown_ticket`, `ambiguous_reference` or `unverified_sender`.
- A repeat of the same message changes nothing (`already_open`), so no message id is kept.
- A `From` header can be forged, and the mail service's own verdict (SPF, DKIM) is not read: someone who knows a reference and the customer's address could restart a waiting ticket's clock. That is all it can do (CRM-R63's limits stand), and the hazard is written in `compliance/risk-management.md` (H14).
- Not built: a vendor's own inbound format (SendGrid Inbound Parse, Mailgun routes, Postmark, SES): the mail service or a small relay must send the two members above; matching by `In-Reply-To` or `References` (the service sends no ticket mail of its own to match); a reply to a resolved or closed ticket opening a new one; attachments; reading the sender's authentication verdict; a reference that cannot be guessed.

## CRM-R89 — Attribution settings: a time-decay half-life, and leaving out opens that come too soon

*As a marketer, I want to choose how fast time decay forgets and to leave out opens a mail program made ahead of the reader, so that attribution does not reward a machine.*

- `GET /api/attribution` takes `half_life_days` (4 to 90, default 7, the earlier fixed value). It is for the `time_decay` model alone: with any other model, or a value outside the range, `422` naming `/half_life_days`. The answer's `half_life_days` is the one used for `time_decay` and `null` for the other models. The shortest is 4 days because the longest window (365 days) then makes at most 91 halvings, inside the 100 a weight is held to, so a touch never weighs nothing and the shares still add up to the amount.
- `GET /api/attribution` takes `min_open_seconds` (1 to 86,400): an e-mail **open reported less than that many seconds after the e-mail was sent** is taken for a mail program loading it ahead of the reader (CRM-R58) and **does not count**. It needs `touches` to include `opened` and is refused otherwise (`422` naming `/min_open_seconds`), as is a value outside the range. The answer's `min_open_seconds` says what was asked, `null` when nothing was.
- It applies to **opens only**. Sends, clicks, campaign leads and the recorded source are untouched: a click by a link scanner is the same kind of error but is not what a delay can tell from a person, and a click is deliberate on its face.
- It is **a rule of thumb, not a detector**: a real reader who opens within the delay is left out, and a program that loads the message later is counted. The page says what it does; a deal left with no touch is reported as unattributed as ever, never dropped.
- The audit entry of a run names both settings.
- Not built: weights for the position model; discounting an open by how soon it came rather than leaving it out; a delay for clicks; unique versus repeat opens.

## CRM-R90 — Position-model weights: the share of each end

*As a marketer, I want to choose how much the first and last touches earn under the position model, so that the model fits how we sell.*

- `GET /api/attribution` takes `end_percent` (1 to 49, default 40, the earlier fixed value): each of the first and last touches earns that share and the touches between share the rest. It is for the `position` model alone: with any other model, or a value outside the range, `422` naming `/end_percent`. The answer's `end_percent` is the one used for `position` and `null` for the other models; the audit entry names it.
- The most is 49 because two ends at 50 would leave the middle nothing and the rest of the model would then be `first` and `last` in disguise; at 49 a middle touch still earns something.
- With one or two touches the share is not used: they split equally, as before. The shares add up to the amount exactly at every value.
- Not built: weights per touch position beyond the two ends (a separate share for the second touch, say); discounting an open by how soon it came; a delay for clicks.

## CRM-R91 — Attribution settings: leaving out clicks that come too soon

*As a marketer, I want to leave out clicks a link scanner made ahead of the reader, so that attribution does not reward a machine.*

- `GET /api/attribution` takes `min_click_seconds` (1 to 86,400): an e-mail **click reported less than that many seconds after the e-mail was sent** is taken for a security scanner following the links (CRM-R58) and **does not count**. It needs `touches` to include `clicked` and is refused otherwise (`422` naming `/min_click_seconds`), as is a value outside the range. The answer's `min_click_seconds` says what was asked, `null` when nothing was; the audit entry names it.
- It applies to **clicks only**, as `min_open_seconds` (CRM-R89) applies to opens only; the two are independent and may be used together. Sends, campaign leads and the recorded source are untouched.
- It is **a rule of thumb, not a detector**, like CRM-R89: a real reader who clicks within the delay is left out. A deal left with no touch is reported as unattributed, never dropped.
- Not built: weights for the touches between the ends; discounting a touch by how soon it came rather than leaving it out; unique versus repeat clicks.

## CRM-R92 — A contact who becomes an employee gets no marketing until they leave

*As a marketer, I want a contact who joins the organization to stop getting campaigns, and to be back where they were when they leave, so that we do not market to our own staff and do not lose a customer who once worked here.*

- The worker service (WPM) posts its hire and leave events to `POST /api/bridges/wpm/events` (WPM-R151, WPM-R207). The body is `{id, kind, occurred_at, subject, data?}`; for `worker_hired`, `worker_terminated` and `worker_retired`, `data` holds `person_ref` (a `person:` URN), `organization_ref` (an `organization:` URN) and `effective_on` (a date), with `from` and `to` beside them for the endings, which are not read. Nothing else is read, and no name arrives.
- **No session; a signature.** The route is outside the session check and is authenticated by `x-wpm-signature: t=<unix>,v1=<hex HMAC-SHA256(secret, "<t>.<raw body>")>`, with the secret in `CRM_WPM_WEBHOOK_SECRET` (32 characters or more, its own setting: the provider secret does not open it). The MAC is compared in constant time; a `t` more than five minutes from now either way is refused. Missing, wrong or stale: `401`. While the secret is unset the route answers `404`, as the other signed routes do. A body over 16 KiB is `413`; one that is not an event (no `id` or `kind`) is `400`; a hire or leave whose `data` is there but malformed (a reference of the wrong kind, a date that is not `YYYY-MM-DD`) is `422`. WPM gives up on a `4xx`, so these are not retried.
- **Once per event.** The `id` is kept with the kind and what it did (`wpm_events`); the same `id` again answers `200` `duplicate` and changes nothing, however it is signed. WPM retries for at most about two days; the ids are pruned by the event-log horizon (30 days, never below 7; CRM-R81).
- **What a hire does.** Every live contact whose `person_ref` is the event's is marked an employee of `organization_ref` (`contact_employments`: contact, person reference, organization reference, `since` = `effective_on`, the event id; nothing more). A second hire for the same contact and organization keeps the first record (`already_paused`). A person the CRM has no contact for is `no_contact` and **nothing is kept about them**.
- **What it means.** While a contact has an employment, **no segment matches them** (the evaluator leaves them out, as it leaves out a contact without consent: CRM-D6), a campaign skips them with the reason "an employee, by the worker service: marketing paused", and a nurture enrolment is **held**, not exited: its step stays due and goes when the pause lifts. Their consent is not touched.
- **What a leave does.** `worker_terminated` or `worker_retired` removes the employment by that organization (`resumed`; `not_paused` when there was none). Marketing resumes **only under the consent the contact already had**: nothing is granted or re-requested, and a contact who withdrew while employed stays withdrawn. A campaign that skipped them is not sent again.
- **`effective_on` in the future** takes effect at once (CRM-D15): the pause starts when the event arrives and `since` records the date WPM gave.
- **Audit.** Each change is audited on the contact as `employment_recorded` or `employment_ended`, with `via: wpm`, the organization, `effective_on` and the event id.
- **Ignored, not refused.** Every other kind (`worker_activated`, `leave_approved`, and kinds WPM may add) answers `200` `ignored` with the reason `not_used` or `unknown_kind`; a hire or leave **without `data`** (a WPM from before WPM-R207) is `ignored`, `no_data`. Each is still kept by id.
- **Seen on the contact.** `GET /api/contacts/{pid}` carries `employments` (`organization_ref` and `since`), shown on the contact's page as "Employee — marketing paused"; the subject-access export carries the same; erasing the contact deletes them.
- The contract was read from WPM's source on 2026-10-30 (`rules/webhook.rs`, `rules/bridge.rs`, `tasks/webhook_deliver.rs`) while WPM was adding `data`; the tests sign requests with the same algorithm, **not against a running WPM**.
- Not built: applying a future `effective_on` only when it is reached; ordering events (built since: CRM-R93); a contact created after the hire is not marked by the events (WPM's overlap check, WPM-R208, finds them: CRM-R94); several secrets at once for rotation; matching by `subject` (`worker:`) rather than `person_ref`.

## CRM-R93 — A hire or leave older than the last one applied changes nothing

*As a marketer, I want a hire that WPM delivers late, after the leave that followed it, not to pause the contact again, so that a person who has left is marketed to under their consent and not held back by a retry.*

- A hire or leave (`worker_hired`, `worker_terminated`, `worker_retired` with `data`) is placed by its **stamp**: the event's `occurred_at` (RFC 3339; WPM sends whole seconds in UTC), then, for two in the same second, `data.effective_on`. A hire or leave without a well-formed `occurred_at` is `422`, as malformed `data` is; the other kinds do not need it.
- The CRM keeps, **per person reference and organization reference**, the stamp, kind (`hired` or `left`) and id of the last hire or leave it applied (`employment_last_events`). An event whose stamp is **older** is answered `200` `stale`, changes no contact, writes no audit entry, and is kept in `wpm_events` with the outcome `stale`. An event with the **same** stamp is applied (the last to arrive wins, as before).
- An applied event replaces the stamp even when it changes no contact (a leave for a contact who was not paused, `not_paused`, still records the leave, so a late hire before it is `stale`).
- Two events for the same person and organization are judged one after the other (a transaction-scoped lock), so two arriving together cannot both read the old stamp.
- **Only for contacts.** The stamp is written only when a live contact carries the person; an event for anyone else is `no_contact` and leaves nothing, as in CRM-R92.
- **Seen and erased.** The subject-access export carries `employment_last_events` (organization, kind, `occurred_at`, `effective_on`); erasing the contact deletes the rows of its person reference and those of the duplicates merged into it (audited as `employment_last_events_deleted`). The retention sweep deletes a leave's row once older than the event-log horizon (30 days, never below 7; WPM has stopped retrying by then) and a hire's row once that old and no live contact carries the person.
- A duplicate event id is still `duplicate` before ordering is looked at.
- Not built: ordering by anything WPM does not send (it has no per-employment sequence number); judging an event that arrives more than the horizon after a leave (by then the leave's stamp may be pruned, and WPM retries for about two days).

## CRM-R94 — The worker service can be asked which contacts are its employees

*As an administrator, I want to ask the worker service which of our contacts work for us today, so that a contact who became a contact after their hire, or was hired before the events were wired, stops getting marketing too.*

- `POST /api/bridges/wpm/employment-check` (no body; admin only, under `/api/bridges`) sends WPM's overlap check (WPM-R208) the **`person:` references of the live contacts**, each once, sorted, **at most 500 a call**: `POST {CRM_WORKER_SERVICE_URL}/api/bridges/overlap` with `{organization_ref, person_refs}`. **No name or other field is sent.** An erased contact's tombstone and anything that is not a `person:` URN are left out (WPM refuses a whole call for one).
- The organization is **`CRM_WPM_ORGANIZATION_REF`** (an `organization:` URN; anything else is unset) and the credential **`CRM_WORKER_SERVICE_TOKEN`**, which the deployer obtains for an HR identity of that organization in WPM (CRM-R87's rules: `https` or this machine only, never logged). CRM-D16 says why not the caller's own token.
- WPM answers `{organization_ref, results: [{person_ref, worker}]}`. The answer is read strictly: another organization, an entry without a reference or a yes or no, a reference not asked about or answered twice is a bad answer. **Every call is answered before anything is written**; one failed call, refusal or bad answer is `502` and records nothing. An unset organization in `http` mode is `502` before anything is sent.
- **A yes** marks every live contact carrying the reference an employee of the organization, the same row a hire event writes (CRM-R92), `since` the day of the check, with the same effect (no segment, campaign or nurture step); audited on each contact as `employment_recorded` with `via: wpm`, `source: employment_check`, the organization and the check's id. A contact already marked is counted `already_paused`. A yes for a person whose **last applied event was a leave** (CRM-R93) is skipped and counted `left_by_event`.
- **A no changes nothing.** In particular it **never ends an employment**, whether recorded from a hire event or by an earlier check; only a leave event does (CRM-D16).
- The check writes no event order (`employment_last_events`); a later leave event ends an employment the check recorded, as it ends one from a hire.
- The check itself is audited once (`employment_checked`, `via: wpm`) with the organization and the counts asked, batches, workers and recorded, never a reference. WPM audits each call by count too.
- The answer: `{mode, organization_ref, asked, batches, workers, recorded, already_paused, left_by_event}`. **Stub-first**: without `CRM_UPSTREAM_MODE=http` nothing is sent and the answer says how many references would be asked in how many calls.
- The `/quotas` page, beside the teams import, has a button to run it and says how many contacts were marked (or, in the tour, how many would be asked about).
- WPM's contract was read from its source on 2026-10-30 (`controllers/bridges.rs`, `rules/bridge.rs`); it is exercised against a stand-in that answers only an HR bearer token, **not a running WPM**.
- Not built: running it on a schedule or after a contact is created; asking about one contact; ending an employment on a no; more than one organization; the caller's own token (WPM-D135).
