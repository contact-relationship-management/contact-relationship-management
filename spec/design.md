# Design decisions

Numbered, stable; tasks ([tasks.md](tasks.md)) trace to them.

## CRM-D1 — Consumer application, identities by URN

Contacts wrap `person:` records, accounts wrap `organization:`
records, reps/agents are `worker:` URNs. CRM owns relationship
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
dashboards, 13-locale i18n in the front-end from the start (the
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
