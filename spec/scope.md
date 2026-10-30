# Scope

## Worldwide by design

The target is **worldwide**, not the United States or the United Kingdom
alone. Any standard, protocol, convention, locale, format or regulation
the CRM adopts must give **equitable capability to countries, regions and
federations worldwide**: no country's or region's convention is the
default that the rest are adapted to. In practice:

- Names, addresses, phone numbers, dates, calendars, currencies, number
  formats, scripts and writing directions are not assumed to follow one
  country's shape (a family name may come first, be absent, or be
  several; an address may have no postcode or state).
- Where a regime applies (data protection, consent, retention, tax,
  accessibility), it is a **configurable, named set of rules** beside
  others, not a hard-coded one; GDPR, UK GDPR, CCPA and the like are
  examples, not the list.
- Standards are chosen for being international and open (ISO, IETF,
  Unicode CLDR, BCP 47) and, where a national one is used, a
  counterpart for other regions is built or recorded as a gap in the
  roadmap rather than left unmentioned.
- A feature that works for one region and not another is recorded as a
  gap, with the regions it fails, and is not described as complete.

## In scope (v1)

- Contacts and accounts as **relationship wrappers** over `person:`
  and `organization:` URNs, with lifecycle status and ownership.
- Leads with rule-based scoring and conversion; deals with
  configurable pipelines/stages, Kanban ordering, and won/lost close;
  activities (call, email-logged, meeting, note, task) on any of
  contact/account/lead/deal/ticket.
- Stage-weighted forecasting per period/owner.
- Campaigns (email kind in v1) with segments, simulated sends,
  engagement counters; nurture sequences (ordered delayed steps)
  advanced by the `POST /api/nurture/advance` sweep (run by a caller,
  not on a timer); marketing consent + unsubscribe.
- Tickets with priority/assignment/lifecycle, SLA policies with
  derived first-response/resolution deadlines and breach flags;
  knowledge-base articles (draft → published, versioned).
- Dashboards: win rate, pipeline by stage, forecast, campaign ROI,
  SLA health, CLV per account; activity feeds. ETag conditional.
- Family fixtures: auth (PASETO + ABAC + `CRM_REQUIRE_AUTH`), audit,
  events (memory/outbox), OpenAPI, `Accepts-version`, OTLP, Podman.

## Out of scope (v1)

- Real email delivery, deliverability, bounce handling (simulated;
  ESP adapter is roadmap).
- Quoting, CPQ, contracts, invoicing, payments.
- Telephony/CTI, chat channels, social listening.
- ML lead scoring or predictive forecasting (rules + arithmetic
  only).
- Duplicate detection of contacts/accounts — that is the person /
  organization registries' matcher job; CRM links to the surviving
  record.
- Territory management, quota plans, commission calculation.

These lists are the v1 scope as first drawn. Several out-of-scope items
were built later and are now in scope: real email delivery and
bounce handling through SMTP and provider adapters (CRM-T39; the legal
and vendor review, CRM-G2, is still open), duplicate advice with the
family matcher (CRM-R23; merging stays upstream for identities, the
CRM merges only its own contact wrappers, CRM-R24), and quotas,
commission plans and territories (CRM-R59, CRM-R68).

## Boundary with the family

| Concern | Owner |
|---|---|
| Who a person/company *is* (demographics, dedup, merge) | person / organization services |
| Sales rep & support agent identity | authentication-service (the sign-in subject, a person id; the CRM names a user `person:<sub>`, CRM-D18) |
| Employment: departments, hires and leaves | worker-service (WPM), through the bridges (CRM-R73, CRM-R92–R94) |
| Login, sessions, tokens, attributes | authentication-service |
| The relationship, its state and history | **CRM (this project)** |

Contacts and accounts carry a **required** `person:` /
`organization:` URN. When the upstream registry merges two records,
CRM repoints its wrapper on the `merged` event (roadmap; manual
repoint endpoint in v1). The governmental **`case` registry is not
the ticket store**: a CRM ticket is operational support state and
never becomes a `case` record.
