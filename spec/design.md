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

*Confirmed by the product owner, 2026-10-31, as the accepted design.* The difference from WPM-D135 named below is
accepted, not open: the CRM keeps one deployer-held HR token (`CRM_WORKER_SERVICE_TOKEN`) and does not forward the
caller's own. WPM's own spec still mentions the gap; it was not edited from this repository.

The employment check (CRM-R94) calls WPM's overlap check (WPM-R208), which answers only a privileged
caller of the organization (HR), and WPM-D135 says an inbound call acts under **the caller's own
token**. The CRM does not forward its caller's token: that token is issued for the CRM's audience
(`CRM_TOKEN_AUDIENCE`), so WPM's verifier would refuse it, and a token good for both would let any
CRM caller act in WPM. Like the teams import (CRM-R73, CRM-R87), the check sends
**`CRM_WORKER_SERVICE_TOKEN`**, which the deployer obtains for an **HR identity of the organization
in WPM**; WPM's own policy then decides, and the CRM's side is admin only and audited. This is
closer to WPM-D135 than a shared key and **not the same as it**; the difference is named in the
integrations table and in WPM's spec, and is accepted (confirmed 2026-10-31, above).

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

**What WPM sends is what is read.** `GET /api/workers` used to answer whole worker records (name,
job, manager, pay unless WPM's policy masks it) to the import's token. Since WPM-R216 the list takes
`fields`, and the import asks for `pid,department,person_ref` only, so WPM sends no name, job or pay
and records no salary read. The CRM keeps `department` and `person_ref` and reads `pid` only to
recognise a row; anything more is dropped unread. A WPM from before WPM-R216 answers `422`; the
import then fails rather than asking for the whole record, which is the trade-off chosen: a failed
import is visible and safe, a silent fallback would bring the pay back.

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

## CRM-D19 — Jurisdiction is data, never code; a standard is adopted as a profile, never as the only shape

*Confirmed by the product owner, 2026-10-31* (drafted 2026-10-30 by the AI assistant; implements CRM-R95; mirrors WPM-D143 and the
project portfolio app's spec §2.0). A rule that comes from a law takes the jurisdiction's parameters as an input and names no country;
federations and regions are first-class; names, phone numbers, dates, time zones, working weeks, reporting periods and money (the ISO 4217
exponent, no built-in currency) follow the locale or the jurisdiction; international standards first, national ones as profiles; locales
are equal, including what a data subject reads; claims name their jurisdiction; and a jurisdiction-specific rule is tested with a second,
differently shaped profile. The full text is in [worldwide.md](worldwide.md#decision).

## CRM-D20 — A stalled deal is derived on read; its days are an organization setting

*Decided 2026-10-31 by the AI assistant under the product owner's instruction to build CRM-R4's stalled flag (CRM-T183).*

**The rule** (`rules::stalled`, pure): an open deal is **stalled** when at least N whole days (N × 24 hours of elapsed time) have
passed since its **last touch**, the later of its last stage move and its last activity. A closed deal is never stalled. Elapsed time,
not calendar days in a zone, so the answer is the same wherever the reader is and no zone is the default (CRM-D19).

- **Last stage move** is a new column, `deals.stage_moved_at`, set when the deal is created, on every stage move (open, won or lost)
  and on a reopen. Migration `000058` fills it for existing deals from the newest `deal_stage_changed`, `deal_won`, `deal_lost` or
  `deal_reopened` audit row, else the deal's `created_at`, without moving `updated_at`. Rejected: reading the audit log on each list
  (the stale-deals insight does, for every deal; a column is one read and keeps the derivation off the hash-chained log). An edit of
  the name, amount or close date, a tag or a card's place on the board is not a stage move.
- **Last activity** is the newest `occurred_at` of a live activity logged against the deal itself (`subject_kind = deal`) that is
  not in the future. Activities on the deal's contact or account do not count: they may be about something else. A future
  `occurred_at` (a planned call) is not a touch until it has happened.
- **N lives in `organization_settings`** (CRM-T171's one row), as `stalled_after_days`, a whole number from 1 to 365, default 14,
  read and set with the calendar at `/api/settings/organization` (an administrator sets it; a `PUT` without it keeps the stored
  value). Rejected: a `CRM_*` environment setting, because N is a sales-process choice an administrator makes and changes, not a
  deployment fact, and the organization's other process settings already live in that row.
- **Where it shows:** `GET /api/deals` gives each deal `stalled` and `last_touched_at`; the UI's Kanban board marks a stalled card
  with the board's non-colour flag and the deal page says since when. Nothing is stored: the flag is derived on every read (CRM-D4).

The stale-deals insight (`/api/insights/stale-deals?days=`, days in the current stage) is a different, older view and is unchanged.

## CRM-D21 — Consent and e-marketing rules are jurisdiction profiles; opt-in stays the default

**Proposed, needs product-owner confirmation.** *Drafted 2026-10-31 by the AI assistant for CRM-T166 (CRM-R95, CRM-D19), as a design
only: no code, migration or test is changed by it, and nothing below is built. The questions at the end are the product owner's.* Not
legal advice: the regime shapes below are the ones the [register by region](regulatory.md#the-register-by-region) names as commonly
described, and every region is *not yet read*.

### What exists today

Consent is held per contact and per purpose (CRM-R6, CRM-R29): four purposes fixed in code (`rules::tokens::PURPOSES`: `marketing`,
`newsletter`, `events`, `product_updates`), states `granted`, `withdrawn`, `never` and `pending`, an append-only `consent_events`
history, and **one rule for every contact on every send path: only `granted` sends** (the campaign job, the nurture scheduler and the
segment evaluator, which ANDs it structurally; CRM-D6). Consent has no channel (only email is sent), no expiry and no jurisdiction. A
suppressed mailbox, an employment pause (CRM-D15) and an erased contact block a send whatever the consent. Each message carries an
unsubscribe link and the `List-Unsubscribe` and `List-Unsubscribe-Post` headers, and **no sender identification or postal address**
(worldwide audit code rows 1 and 4, spec row 12). That is the strictest model, applied everywhere: safe, but no jurisdiction's own
shape can be expressed.

### The model

The rule is CRM-D19's: the law's parameters are an input and the code names no country. The shape is WPM-D144's, so the three apps
hold jurisdiction rules alike:

- **Jurisdiction code.** WPM-D144's: ISO 3166-1 alpha-2 for a country, ISO 3166-2 for a subdivision, a deployer-extended list for a
  supranational region (`EU`, others). Each code has a layer (`supranational`, `national`, `subnational`) and a parent, held per
  organization, so a federation, a bloc or a free zone with its own law is a chain.
- **Parameter registry** (a list in the spec and in `rules::consent_regime`, one changes with the other; a key outside it is refused).
  Each entry fixes its type, its validation and its combination rule across the layers of a chain:

  | Key | Value | Combination across layers |
  | --- | --- | --- |
  | `consent.regime.<channel>` | `opt_in` \| `soft_opt_in` \| `implied` \| `opt_out` | `strictest` (in that order, `opt_in` strictest) |
  | `consent.soft_opt_in.window_days.<channel>` | whole days, 1 to 3,660, since the existing-customer evidence | `strictest_max` (the shortest) |
  | `consent.implied.expiry_days.<evidence>` | whole days, 1 to 3,660, per kind of evidence (`won_deal`, `inquiry`) | `strictest_max` (the shortest) |
  | `message.sender_identification` | `required` \| `not_required` | `strictest` |
  | `message.postal_address` | `required` \| `not_required` | `strictest` |
  | `message.advertising_label` | text per language (a mark some regimes ask for in the subject), or none | `most_specific` |

  As in WPM-D144, **a parameter has no default value**. Unlike WPM, where an unknown is shown as unknown, an unknown consent regime
  resolves to `opt_in`, today's rule, because for a send the safe answer to "no rule configured" is the strictest one.
- **Profile row.** WPM-D144's: `(organization, jurisdiction code, parameter key, value, effective_from, effective_to?, source,
  checked_on, status)`, `status` one of `read`, `unverified` or `example`; append-only (a change is a new row), set by an
  administrator, audited with the code and the key. `resolve(parameter, chain, as_of)` is one pure function (the same signature and
  answer as WPM's: the value, the code it came from, its status and source), DB-free and clock-free.

### Which profile applies to a send

**Per contact, else per send.** A contact gains an optional `consent_jurisdiction` (a code), set by staff, by an import column or by
the web form's configuration, never inferred from a phone prefix, an address, a language, a name or an IP address. A campaign and a
nurture sequence gain an optional `consent_jurisdiction` too. A recipient is under the chain of their own code when it is set, else the
send's, else **none, which is today's opt-in**. The applicable code and the resolved parameters are recorded on each delivery, so a
send can be explained by the rule in force that day.

### What a regime allows

One pure decision, `rules::consent_regime::basis(regime, state, evidence, now) -> Basis | Blocked(reason)`, used by every send path
and by the segment evaluator (which today ANDs `granted`; it would AND "has a basis"):

| Regime | Sends when | Never sends when |
| --- | --- | --- |
| `opt_in` (today, and the default) | the purpose and channel are `granted` | anything else |
| `soft_opt_in` | `granted`; or `never` or `pending` with existing-customer evidence (a won deal naming the contact as primary contact) inside the window, and the purpose marked eligible for it | `withdrawn` for that purpose and channel, ever since the evidence |
| `implied` | `granted`; or `never` or `pending` with evidence of a kind the profile lists, younger than that kind's expiry | `withdrawn`; evidence expired |
| `opt_out` | anything but `withdrawn` | `withdrawn` |

In every regime, as today: a suppressed mailbox, an employment pause (CRM-D15), an erased contact and a withdrawal block the send; an
unsubscribe withdraws the message's purpose (on its channel) at once; the double opt-in and the preference page work as they do. The
delivery records its **basis** (`granted`, `soft_opt_in`, `implied` with the evidence's pid and expiry, or `opt_out`), so CRM-R6's
evidence becomes "every marketing touch had a recorded basis under the profile in force" (question 12).

### Purposes an administrator edits; consent per channel

- **Purposes** move from code to a table: a key, a name and a description per locale (the UI's eight, English fallback marked as such),
  `active`, and `soft_opt_in_eligible`. The four of today are its first rows. `marketing` stays (it is `contacts.marketing_consent`
  and the default purpose) and cannot be retired; another purpose can be retired (no new campaign, sequence or preference-page row
  names it) but never deleted, because history names it. A new purpose starts `never` for everyone.
- **Channel.** Consent state and events gain a channel: `email`, `sms`, `phone`, `post` (`rules::tokens::CHANNELS` without `none`, plus
  `sms`). Only email is sent; the others are recorded so a deployer's other systems can read them, and so a regime can differ by
  channel. A campaign sends on `email` and checks only `email`.

### Sender identification and postal address

The organization's settings row (`organization_settings`, CRM-T171) gains a **sender name**, a **postal address** (free text in any
script and any number of lines, no country's format assumed, as WPM-D152 keeps addresses) and a **contact link or address**, set by an
administrator. When set, every commercial message carries them in its footer, in the template's language (the labels translated like
the public pages, CRM-T175), in both the text and the HTML part; the advertising label, when the profile has one, prefixes the
subject. When the applicable profile says `required` and one is unset, the campaign's `run` and the nurture step are refused (`422`
naming the setting), before anything is queued. With no profile, nothing is refused that is sent today.

### Migration and default behaviour

- **Opt-in stays the default, and nothing blocked today becomes sendable without a profile an administrator loaded.** With no profile
  rows, every decision equals today's, for every state, purpose and contact; a test holds that exhaustively over the states.
- Existing consent becomes the `email` channel's (the only one ever sent); `sms`, `phone` and `post` start `never`. Existing events
  become `email` events. `marketing_consent` keeps meaning `marketing` on `email`.
- No deployment-wide setting (`CRM_*`) chooses a regime: that would make one jurisdiction the deployment's.
- Loosening is explicit: an administrator sets a contact's or a send's code and loads the rows; a campaign's preview shows how many
  recipients would be sent to on each basis and under which profile and status before it runs.

### The two test profiles

Synthetic codes from the ISO 3166 user-assigned range, so no test names a country (CRM-D19):

- **`XA`, one national layer, soft opt-in.** `email`: `soft_opt_in`, window 730 days; `sms`: `opt_in`; sender identification and postal
  address `required`; no label. In the shape the register describes for the UK's PECR.
- **`XB` and `XB-01`, a federation of two layers.** `XB` (national): `email` `opt_out`, postal address `required`; `XB-01`
  (subnational): `email` `implied`, `won_deal` 730 days and `inquiry` 180 days, an advertising label in two languages. The chain
  resolves to `implied` (the stricter), the address required (from the parent), the label (from the child). In the shapes the
  register describes for the US CAN-SPAM Act and Canada's CASL, joined as one federation only to exercise the layers.
- Both run against every regime row above, plus: the same contact under each profile and under none gets three different answers;
  a withdrawal blocks under all of them; no profile equals today.

### Order of work (once confirmed)

1. Pure `rules::consent_regime` (the registry, `resolve` over a chain, `basis`), with the two profiles and the "no profile is today"
   test.
2. The tables (jurisdictions, parameter rows, purposes, the channel column), administrator-only, audited.
3. The send paths and the segment evaluator read the basis; deliveries record it.
4. The sender settings and the footer.
5. The UI: profiles, purposes, a contact's and a campaign's code, the preview by basis.

### Questions for the product owner

Each with the assistant's recommendation; none is decided.

1. **Per contact or per send?** Recommended: the contact's `consent_jurisdiction` when set, else the campaign's or sequence's, else
   none (opt-in). Alternatives: per send only; per contact only.
2. **Combining layers for the regime.** Recommended: `strictest` (whichever layer is stricter wins, a state over a looser federation or a federation over a looser state).
   Alternative: `most_specific` (the deepest layer wins even if looser).
3. **Where a contact's code comes from.** Recommended: set by staff, an import column or the web form's configuration, never
   inferred from a phone prefix, address, language, name or IP. Alternative: also inferred, flagged as inferred (contrary to CRM-D13).
4. **Who edits profiles and purposes.** Recommended: an administrator, every row audited and append-only. Alternatives: the deployer
   only (configuration files); an administrator with a second person's review.
5. **What counts as an existing customer for a soft opt-in.** Recommended: a won deal naming the contact as primary contact, inside
   the profile's window, for a purpose marked eligible. Alternatives: also an account of tier `customer`; also a negotiation (an open
   deal).
6. **What counts as evidence for implied consent.** Recommended: the profile lists the kinds (`won_deal`, `inquiry`, the latter a web
   form submission or a ticket the contact opened), each with its own expiry. Alternative: one expiry for all evidence.
7. **The channels.** Recommended: `email`, `sms`, `phone` and `post`; existing consent migrates as `email` only, the rest `never`.
   Alternative: `email` only until another channel is sent.
8. **Sender identification and address.** Recommended: one set per organization, carried whenever set, required (send refused) only
   where the applicable profile says so. Alternatives: required for every commercial message once the feature ships (sends that work
   today would be refused until set); per campaign or per brand.
9. **May an `example` or `unverified` profile loosen a send?** Recommended: yes, only after an administrator acknowledges that profile
   (audited), and the preview and each delivery show its status. Alternative: only `read` rows may loosen; the others may only tighten.
10. **The demo seed.** Recommended: it loads `XA`- and `XB`-shaped example profiles under clearly synthetic codes and assigns them only
    to newly seeded contacts, so the existing seeded contacts and the tour's sends give today's answers. Alternative: no profiles in
    the seed.
11. **`pending` under a soft opt-in or implied regime.** Recommended: treated as `never` (a confirmation was asked for and not
    refused), so evidence can still give a basis. Alternative: `pending` always blocks, as today.
12. **CRM-R6's wording.** Recommended: "every marketing touch had a recorded basis under the profile in force; with no profile the
    only basis is opt-in consent". Alternative: keep "consented to" and allow only `opt_in` and `soft_opt_in` (no `implied`, no
    `opt_out`).
