# Requirements

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
  enrolments, and blocks all sends until an explicit re-grant.
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
  v1); deadlines derived at open and re-derived on audited priority
  change; breach flags computed on read and swept by a job emitting
  `sla_breached` once per breach; breaches clear only by meeting
  the metric.

## CRM-R12 — Knowledge base

*As an agent I answer from published articles and link them to
tickets.*

- Article CRUD `draft → published → archived`; published edits bump
  `version` (priors retained read-only); ILIKE keyword search;
  ticket-link logs a note activity.

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
