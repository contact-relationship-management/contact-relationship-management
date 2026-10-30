# Design decisions

Numbered, stable; tasks ([tasks.md](tasks.md)) trace to them.

## CRM-D1 — Consumer application, identities by URN

Contacts wrap `person:` records, accounts wrap `organization:`
records, reps/agents are `person:` URNs, the sign-in subject (CRM-D18; they
were `worker:` URNs before). CRM owns relationship
state only; dedup/merge of identities stays upstream, and wrappers
repoint (manual v1, event-driven roadmap) after upstream merges.

## CRM-D2 — Normalized relational schema

Pipelines, stage positions, SLA clocks, consent history, and money
sums are constraint-heavy — normalized SeaORM tables, not
DTO-as-JSONB. All-plural table names (the loco pluralization
lesson).

## CRM-D3 — Every lifecycle is a pure-core state machine

Lead, deal (incl. terminal immutability + reasoned reopen),
campaign, ticket (incl. reopen), article: one transition table each
in DB-free `rules/` modules, exhaustively unit-tested; controllers
only wire them; illegal transition ⇒ `422` naming the current
state.

## CRM-D4 — Derived numbers, never stored opinions

Score, forecast, ROI, CLV, win rate, SLA deadlines/breaches are
pure-core derivations from recorded facts. No editable KPI fields
exist; snapshots (forecast) are frozen outputs, not inputs.

## CRM-D5 — Explainable scoring

Lead scoring is a fixed deterministic rule table with per-rule
breakdown in the response (the family matcher score-breakdown
posture). Weights config-tunable, rule set fixed in v1, no ML.

## CRM-D6 — Consent gates the send path

Consent is enforced where sends happen (campaign job, nurture
scheduler), not in the UI; segments AND `consent = granted`
structurally; ConsentEvent history is append-only. Unsubscribe
exits everything immediately.

## CRM-D7 — Money in minor units, per-currency honesty

`i64` minor units + ISO-4217; overflow refused; mixed currencies
report per-currency lines, never a silent sum; no FX in v1 (the report builder
alone can convert a deal sum, from hand-entered rates, naming them: CRM-R50).

## CRM-D8 — Jobs on `bg_pg`, idempotent by key

Nurture advancement (idempotent per enrolment+step), the SLA breach
sweep (one `sla_breached` per breach fact), and simulated campaign
sends run inline in the request handler (no job queue, no external broker),
in batches, through `mail::Sender` (simulated by default; SMTP with
`CRM_MAIL_MODE=smtp`, CRM-T39). A background job is still future work.

## CRM-D9 — Transactional integrity

Lead conversion, stage moves, consent changes, and every
audit/outbox write share the mutation's transaction; Kanban
reorders and close/reopen races serialize on the deal row lock
(`FOR UPDATE`).

## CRM-D10 — Personas are policy, not code

One API surface; rep/manager/marketing/support are ABAC profiles
over `attrs` + `resource.owner` (`$sub` ownership) + record attrs;
commercial and channel masking is the `mask` obligation.

## CRM-D11 — Stub-first upstream clients

Display-name lookups behind traits with `http` + `stub`
implementations, config-selected, cached, best-effort — boots and
tests with no siblings running.

## CRM-D12 — Family fixtures from day one

Loco-idiomatic layout, forbid-unsafe + clippy-pedantic, OpenAPI +
Swagger, `Accepts-version`, OTLP + `/metrics.prom`, Podman, input
caps, `404` mapping at `find_by_pid` call sites, enforcement tests
that scope the settings they change (the OnceLock lesson), ETag-conditional
dashboards, eight-locale i18n in the front-end from the start (the
PPM lesson).

## CRM-D13 — Declared data is never inferred

Stakeholder role, power–interest grid position, and activity
sentiment (CRM-R19, [requirements.md](requirements.md)) are
operator-entered fields with no scoring or NLP behind them: a
rating with no entry stays `null` and renders as undeclared, never
defaulted or guessed at from other data. This mirrors CRM-D4
(derived numbers are never stored opinions) from the other
direction — where CRM-D4 keeps a computed number from being typed
in as an opinion, CRM-D13 keeps a human judgement from being
computed. The distinction matters for the insight/engagement views
(CRM-R18/R19): every one of them is either a pure derivation over
recorded facts, or a verbatim echo of a declared field — never a
blend of the two presented as one number.

## CRM-D14 — Erasure gates on live engagement, not on `Contact::status`

WPM's analogous rule (WPM-D22) gates employee erasure on the
employment `status` field, because that field genuinely transitions
through an offboarding lifecycle and *is* the lawful basis. CRM's
`Contact::status` (`active`/`inactive`) is not that: it is set once at
creation and no endpoint in this crate ever transitions it, so gating
erasure on it would refuse erasure forever. CRM-R20's `erasable`
predicate instead checks the signals that actually change over a
contact's lifetime and actually represent a live reason to keep the
data: an open deal naming the contact primary contact, an open
support ticket, or an active nurture enrolment. Erasure anonymises
(tombstone `person:` URN, scrubbed identity fields and linked free
text, soft-deleted row) rather than deletes, mirroring WPM-D22's
"neither pretends" framing: deals/tickets/consent history remain for
reporting and compliance-evidence continuity, keyed to a pid that no
longer identifies anyone, and the retention sweep (floored horizon,
`CRM_RETENTION_DAYS`) is the separate, generic hard-delete path for
soft-deleted rows past that horizon — it never bulk-anonymises a
contact, because a contact carries the erasure gate above and the
sweep has no way to evaluate it per row.

## CRM-D15 — Employment pauses marketing; it is not consent

A contact who becomes an employee (CRM-R92) is kept out of marketing by **a fact beside consent,
not by changing consent**. Withdrawing consent on a hire would have to be undone on leaving, and
the CRM could not then tell a withdrawal the person made from one it made for them; granting it
again on leaving would be marketing without a consent the person gave. So the hire writes a row
(`contact_employments`) and the leave deletes it; the segment evaluator and the send path read it
the way they read consent and suppression, and consent history is never written by the bridge.

**A future `effective_on` takes effect at once.** WPM emits the hire when HR records it, which may be
before the first day. The CRM pauses marketing on arrival and records the date as `since`. The
alternative, storing the event and applying it when due, needs a sweep and an answer to what a
leave before the start means; pausing a few days early costs a marketer a few sends to someone who
has accepted a job, and is the honest simple choice. The same holds for a leave: marketing may
resume from the day the event arrives.

**Only contacts the CRM already has are marked**, and nothing is kept for a person it has no
contact for: WPM's events would otherwise make the CRM a copy of the employee list.

**Events are put in order by when WPM recorded them** (CRM-R93). WPM retries a failed delivery for
up to two days, so a hire can arrive after the leave that followed it. The CRM keeps, per person and
organization, the stamp of the last hire or leave it applied (`occurred_at`, then `effective_on`
for two in the same second, since WPM sends whole seconds) and an event with an older stamp changes
nothing (`stale`). The order is kept **by person, not by contact**, because that is what an event is
about, but **only while the person is a contact**: an event for anyone else still leaves nothing
behind, and erasing the contact deletes the rows of its person references. An equal stamp is
applied, so two events WPM cannot tell apart behave as before (the last to arrive wins). Rejected:
ordering by arrival (the bug), by `effective_on` first (a rehire can take effect before an earlier
leave was recorded), and keeping the ended employment row with an end date (every reader of
`contact_employments` would have to learn to skip it; a separate row keeps "a row is a pause").

## CRM-D16 — The employment check asks under one HR token, for one configured organization

The employment check (CRM-R94) calls WPM's overlap check (WPM-R208), which answers only a privileged
caller of the organization (HR), and WPM-D135 says an inbound call acts under **the caller's own
token**. The CRM does not forward its caller's token: that token is issued for the CRM's audience
(`CRM_TOKEN_AUDIENCE`), so WPM's verifier would refuse it, and a token good for both would let any
CRM caller act in WPM. Like the teams import (CRM-R73, CRM-R87), the check sends
**`CRM_WORKER_SERVICE_TOKEN`**, which the deployer obtains for an **HR identity of the organization
in WPM**; WPM's own policy then decides, and the CRM's side is admin only and audited. This is
closer to WPM-D135 than a shared key and **not the same as it**; the gap is named in the integrations
table and in WPM's spec.

**The organization is a setting, `CRM_WPM_ORGANIZATION_REF`**, not a field of the request, because
the token belongs to HR of one organization: asking about another would be refused by WPM, and a
field would invite asking anyway. It is the CRM operator's own employer, whose staff are not to be
marketed to.

**A yes records; a no does nothing.** A yes writes the same `contact_employments` row a hire event
writes, dated the day of the check, with `overlap:<check id>` in place of an event id. A no never
ends an employment: one recorded from a hire event is ended by its leave event, and a no today may
be a hire whose first day has not come (CRM-D15 pauses from the hire's arrival). A yes for a person
whose last applied event was a leave is skipped (`left_by_event`), since WPM says "worker" until the
leave's day while the CRM lifts the pause when the leave arrives. The check writes no order
(`employment_last_events`): it has no event time, and a later leave must still apply. Rejected:
mirroring WPM (a no ending the pause), which would make the check and the events fight.

## CRM-D17 — Imported teams name their members by `person:` reference; old members are replaced, not migrated

The teams import (CRM-R73) made each member `worker:<pid>` from WPM's worker id, while everything
else in the family names a person by `person:` URN: CRM keys contacts and employments by
`person_ref`, and WPM's hire and leave events and overlap check carry `person_ref`. WPM's list,
which the import already reads, carries each worker's `person_ref`, so the import now keeps that
(validated as a `person:` URN) and nothing else new. **A worker without a valid `person_ref` joins
no team and is counted** (`without_person_ref`); it never falls back to `worker:`, since a team
mixing the two kinds could not be compared with anything.

**Teams already stored with `worker:` members are not migrated.** The next import replaces the
members of every team it names, as it always has; a team set by hand, or one whose department WPM
no longer lists, keeps what it holds. Rejected: a migration that asks WPM for each `worker:<pid>`'s
person (a call from a migration, to a service that may be unreachable or unconfigured, to repair
data the next import rewrites anyway); and a migration that deletes `worker:` members (it would
empty hand-set teams, whose `worker:` members are what CRM-R67 asks for).

**The cost, named:** quotas and deals are owned by `worker:` references (CRM-R59), so a team's
roll-up (CRM-R67) cannot match an imported team's `person:` members to any quota and lists them in
`without_quota`. Whether the old `worker:<wpm pid>` members ever matched depended on the CRM's sign-in
subject being WPM's worker id, which nothing guaranteed. The mapping from a person to the worker
reference that owns deals is not built, and `PUT /api/quotas/teams` still takes `worker:` members
only, so an imported team cannot be re-saved by hand. *(Resolved by CRM-D18: the CRM's users are
`person:` references, so quotas and imported members now meet.)*

**What WPM sends is more than is read.** `GET /api/workers` answers whole worker records (name, job,
manager, pay unless WPM's policy applies its `mask` obligation) to the import's token, and does not
audit a salary-bearing list as it audits a single read. The CRM keeps `department` and `person_ref`
and reads `pid` only to recognise a row; the rest is dropped unread. Narrowing the answer is WPM's to
decide (a field list or a bridge route), recorded for WPM's owner in the integrations table; WPM is
not changed here.

## CRM-D18 — A CRM user is named by `person:` URN, the sign-in subject; `worker:` is read as `person:` for now

The CRM named its own users (owners, assignees, quota holders, team members, the recipient of a
notification, whoever set something) `worker:<id>`, and wrote the signed-in caller as
`worker:<sub>` (`owner_of`). **The family's sign-in subject is a person id**, and was checked in all
three apps before this change:

- The shared design (`agents/share/authentication-sessions.md` §5) says only that `sub` is the
  authentication service's *user pid*; it does not say which entity that pid is.
- **WPM** treats it as a person id everywhere: `/api/me` finds the caller's worker by
  `person_ref = person:<sub>` (`controllers/me.rs`), organization memberships are keyed
  `person:<sub>` (`models/memberships.rs`), as are organizations, expenses and announcements, and
  its self rules are `resource.person = $sub` where `resource.person` is the bare id of a worker's
  `person_ref` (`auth.rs`, WPM `spec/auth.md`, WPM-R8). A WPM worker pid is a separate id
  (`tasks/seed.rs` gives each worker a `pid` and a different `person_ref`).
- **PPM** addresses a `sub` as `person:<sub>` (`discussion.rs`, `sub_as_recipient`) and treats
  `person:<id>` and `worker:<id>` with the same id as one person (`subjects.rs`, `approvals.rs`).
- **The CRM** compared a deal's owner with `$sub` by the bare id (`auth/records.rs`), so for
  ownership to work at all, `worker:<id>` had to carry the sign-in subject, a person id.

So `worker:<sub>` was `person:<sub>` under the wrong prefix, and it could never meet what the WPM
teams import stores (`person_ref`, CRM-D17). **Every field that names a CRM user now holds
`person:<id>`** (contacts', accounts', leads', deals' and territories' owners; activities' actor
and assignee; tickets' assignee and an SLA policy's `escalate_to`; notification recipients; quota,
split and clawback holders; team members; workflow assignees, recipients and owner pools; `set_by`,
a saved view's and a report's owner, `opened_by`), and `owner_of` gives `person:<sub>`
(`rules::user_ref`).

**`worker:<id>` is accepted on input and stored as `person:<id>`, the same id**, in every body and
`owner_ref` filter that names a user, for a deprecation period. Answers only ever say `person:`.
Accepting is faithful because in the CRM's API `worker:<id>` always meant the sign-in subject (the
ownership check read it so), and it matches PPM, which takes both forms as one person. Rejected:
refusing `worker:` with a `422` (it would break every client and stored workflow at once for a
spelling); keeping `worker:` and mapping a person to a worker (CRM-D17's open gap: it needs a WPM
call per person, and the id is the same anyway). A client sending a real WPM worker pid as
`worker:<pid>` gets `person:<pid>`, which names nobody, exactly as it named nobody before. **When
the alias is removed is not decided**; no `Deprecation` header is sent.

**Stored data is rewritten by migration 54** in the columns above and in the user references
inside workflow actions (only `assignee_ref`, `recipient_ref`, `pool` and a `set_field` of
`owner_ref`; a task summary that happens to read `worker:…` is text and stays). **Teams:** the
members of a team whose latest `set` audit says `via: wpm` are left alone, because an import before
CRM-T161 stored `worker:<WPM worker pid>`, which is not a person id (the next import replaces them,
CRM-D17); a team set by hand is rewritten, because its `worker:` members were validated against the
same sign-in-subject convention. **Not rewritten:** history (the hash-chained audit log, the outbox,
workflow run logs, stored idempotent answers) and WPM's own `worker:` event subjects, which do name
WPM workers. A row whose rewrite would collide with an existing `person:` row on a unique key is
left (none can come from the API). `updated_at` does not move: the reference is spelt differently,
nobody edited the record. `down` reverses it except for the import's `person_ref` members.

**A side effect:** the `{{rep_name}}` email-template value now asks the person service for the
owner's name (it asked the worker service for `/api/workers/<sub>`, which is not a worker id).
