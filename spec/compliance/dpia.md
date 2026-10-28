# Data protection impact assessment (completed as far as the software goes)

GDPR Art. 35 / UK GDPR; guidance from the ICO and the EDPB. This is the
[template](dpia-template.md) filled in with everything the software can answer. Three kinds of
entry, so a reader knows what to trust:

- **System fact** — enforced by code in this repository, with a test or check behind it.
- **Engineering assessment** — a likelihood or severity the engineers rate for the software *as
  shipped, with the defaults on*; the deployer's context (who the contacts are, what they type into
  free text) can change it.
- **Deployer decides** — only the organization running the software can answer; left in brackets.

It is not legal advice and not a sign-off. Nothing here has been reviewed by a DPO or a regulator.
Companion documents: the [personal-data inventory and retention schedule](personal-data-inventory.md)
and the [record of processing](record-of-processing.md).

## 1. Description of processing

- **Controller:** [deployer decides]. **DPO:** [deployer decides].
- **Purpose** *(system fact)*: managing relationships with customers, prospects and stakeholders:
  contacts and accounts with their timeline, a sales pipeline with forecasts, quotas and
  commission statements, consent-based marketing email and nurture sequences, support tickets with
  service-level tracking, a help centre, and the analytics derived from them.
- **Data subjects** *(system fact)*: contacts, leads (including web-form and imported ones), and
  ticket requesters; and the organization's own staff (reps, agents, assignees, owners) whose
  references, quotas and activity the system records. [Deployer decides whether patients, citizens
  or children can be contacts: if so the risk level rises and section 3 must be redone.]
- **Data held** *(system fact)*: see the [inventory](personal-data-inventory.md), table by table.
  In short: a reference to the person (`person:<id>`; names and addresses live in the identity
  service, the CRM keeps a display label), preferred channel, consent history, activity summaries
  and ticket titles (free text), deals and amounts, lead source and score, and for a lead the name
  and address a form or import gave. **Free-text fields can hold anything a user types**, including
  special-category data; the system cannot tell.
- **Recipients** *(system fact)*: the email provider the deployer configures (real email is **off by
  default**: the simulated sender sends nothing); the receivers of webhook subscriptions the
  deployer registers; the project portfolio service (PPM), which is sent a deal's name, amount and
  currency when an admin opens a delivery idea (CRM-R72, off unless `CRM_UPSTREAM_MODE=http`); the
  worker service (WPM), which is only read (CRM-R73) and receives nothing; the identity services,
  which are asked for a display name or an address. [Deployer decides: hosting provider, backup
  provider, identity provider, and whether each is a processor.]
- **Retention** *(system fact)*: the [schedule](personal-data-inventory.md#retention-schedule).
  Soft-deleted rows are hard-deleted by an operator-run sweep after `CRM_RETENTION_DAYS` (365,
  never below 30); consent history and the audit log are kept as evidence; a 24-hour cache expires
  by itself; **email delivery and event rows, the event outbox and webhook deliveries are never
  pruned today** (CRM-T140).
- **Transfers outside the UK/EEA:** [deployer decides]. The software adds none of its own; the
  email provider, the hosting and any bridge target are where a transfer would arise.

## 2. Necessity and proportionality

- **Lawful basis per purpose:** [deployer decides: consent for marketing; contract or legitimate
  interest for the rest]. *(System fact)* Marketing email needs a recorded consent for the
  campaign's purpose, checked again at send time, and every message carries a working unsubscribe;
  withdrawing stops it. Consent history is append-only.
- **Minimisation** *(system fact)*: the CRM references identities by URN and keeps no address on a
  contact; a bridge sends PPM only a deal's name, amount and tags; the worker import reads only a
  worker's id and department and never pay.
- **Transparency:** [deployer decides: privacy notice location]. A tracking pixel (open and click
  tracking at the email provider) is a matter of its own and is the deployer's to disclose before
  it is turned on.
- **Rights** *(system fact)*: access (`/api/contacts/{pid}/subject-access`), erasure
  (`/api/contacts/{pid}/erase`, which anonymises, and is refused while a deal, a ticket or a nurture
  enrolment is open) and retention (`/api/retention`) have endpoints. What erasure leaves is listed
  per table in the inventory. **Rectification** is an ordinary edit; **objection** and **portability
  beyond the access export**: [deployer decides the procedure].
- **Automated decisions** *(system fact)*: lead scoring is rule-based with a per-rule breakdown and
  is advice to a person; nothing here decides on its own with a legal or similarly significant
  effect. [Deployer confirms this holds in its use.] Quota attainment and commission are
  *statements for people to act on, not payments*.

## 3. Risks and measures

Rated for the software as shipped. **L** = likelihood, **S** = severity, each Low / Medium / High.

| Risk | L | S | Measures in the system | Residual |
|---|---|---|---|---|
| Unauthorized access to contact data | M | H | Authentication enforced when `CRM_REQUIRE_AUTH` is on (it is a deployer's switch, off for demos); attribute-based access control on every route; record-level masking for aggregate reads and money; destructive actions and the routes that send data out (`/api/bridges`, quotas, webhooks) admin-only; TLS at the ingress is the deployer's | M if auth is on and the policy is reviewed; **High if it is left off** |
| Alteration of the audit record | L | H | Hash-chained, append-only audit log; the chain is verifiable by an endpoint; external anchoring is the deployer's | L |
| Over-retention | M | M | Retention sweep with a floor; **gaps**: email deliveries and events, the event outbox and webhook deliveries are never pruned (CRM-T140) | M until CRM-T140 closes, or the deployer prunes them |
| Marketing without consent | L | H | Consent gate in the send path and again at send time; segments require `consent = granted`; suppression survives; unsubscribe in every message | L |
| Special-category data in free text | M | H | Policy and training (deployer); the system cannot detect it; erasure scrubs activity summaries and a lead's name | M, **High if patients or citizens are contacts** |
| Data sent to another service by a bridge | L | M | Admin only; stub unless `CRM_UPSTREAM_MODE=http`; only a deal's name, amount and tags are sent, once per deal; every opening is audited | L |
| Re-identification from small analytics groups | M | M | Aggregates and money are masked for callers who may not read them; the report builder **leaves out any group with fewer than `CRM_MIN_CELL_SIZE` records** (5 by default) and says how many it left out (CRM-R78); **primary suppression only**: two differently asked reports can still be compared, and the fixed dashboards, attribution and attainment views are not suppressed | L–M |
| Processor failure or breach | [ ] | [ ] | [Deployer decides: contracts, monitoring, breach process; the runbook has incident response] | [ ] |
| Loss of availability | L | M | Backups and restore tests are the deployer's (runbook); the data model has no single store that cannot be rebuilt from the database | L |
| A link token used twice or guessed | L | M | Confirmation and preference links are 256-bit random tokens that work **once** and expire; the database keeps only a hash, so a copy of it opens nothing; a preference choice spends the link and the answer carries the next one; looking at a page spends nothing, since mail scanners follow links; erasing the contact deletes its links. The unsubscribe link is signed and stateless and deliberately neither expires nor is single-use (a recipient must be able to leave from any old message; replaying it only withdraws consent again) | L |

## 4. Consultation and sign-off

DPO advice: [ ]. Data subject views sought: [ ]. A residual **High** risk needs prior consultation
with the regulator (Art. 36): with authentication left off, or with patients or citizens as
contacts and no policy on free text, the rows above stay High. Approved by: [name, date]. Review
date: [at least yearly, and when a bridge, a provider or the schedule changes].

## 5. What would change this assessment

- A new table, or a new use of an existing one: the inventory test fails until it is described, and
  this assessment is revisited.
- Turning on real email, open and click tracking, or either bridge.
- Treating the software as a system of record for a regulated population.
