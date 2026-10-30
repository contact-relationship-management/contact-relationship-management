# DPIA template (pre-filled)

For the deploying organization to complete. Guidance: UK ICO and EDPB DPIA
guidance; GDPR Art. 35. Replace bracketed text. Items marked *(system fact)*
describe this software and can be reused as written.

## 1. Description of processing

- **Controller:** [organization]. **DPO:** [name, contact].
- **Purpose:** managing relationships with customers, prospects, and
  stakeholders: sales pipeline, consent-based marketing, support tickets,
  analytics. *(system purpose)*
- **Data subjects:** [contacts, leads, ticket requesters]. If patients or
  citizens are recorded as contacts, say so here; that raises the risk level.
- **Data categories held by this system** *(system fact)*: a URN reference to
  the person (identity data stays in the upstream identity service); preferred
  channel; consent history; activities with free-text summaries; deals and
  amounts; tickets; lead source and score. Free-text fields can contain
  anything users type, including special-category data: forbid that by policy
  or treat the system as holding it.
- **Recipients:** [ESP, hosting provider, identity provider]. No real email is
  sent by default.
- **Retention:** `CRM_RETENTION_DAYS` (default 365, floor 30) for soft-deleted
  rows; consent history is retained as compliance evidence. *(system fact)*
- **Transfers outside the deployer's jurisdiction** (for a UK or EU controller, outside the UK/EEA): [none / safeguards].

## 2. Necessity and proportionality

- Lawful basis per processing purpose: [consent / legitimate interest /
  public task / contract]. Marketing email requires recorded consent;
  the send path enforces it. *(system fact)*
- Minimisation: contact detail beyond a URN reference is kept to what the
  relationship needs.
- Transparency: [privacy notice location].
- Rights handling: access, erasure, and retention have endpoints
  (`/api/contacts/{pid}/subject-access`, `/erase`, `/api/retention`). Erasure
  is refused while an open deal, open ticket, or active nurture enrolment
  exists. *(system fact)* Rectification and objection: [procedure].
- Automated decision-making: lead scoring is rule-based and explainable with a
  per-rule breakdown; it does not make decisions with legal or similarly
  significant effect on its own. [Confirm this holds in your use.] *(system fact)*

## 3. Risks and measures

| Risk | Likelihood | Severity | Measures in the system | Residual |
|---|---|---|---|---|
| Unauthorized access to contact data | [ ] | [ ] | authentication enforced in production, ABAC, masking (partial), TLS at ingress | [ ] |
| Alteration of the audit record | [ ] | [ ] | hash chain, append-only trigger, external anchoring (deployer) | [ ] |
| Over-retention | [ ] | [ ] | retention sweep, floor | [ ] |
| Marketing without consent | [ ] | [ ] | consent gate in the send path | [ ] |
| Special-category data in free text | [ ] | [ ] | policy, training, [DLP] | [ ] |
| Processor failure or breach | [ ] | [ ] | [contract, monitoring] | [ ] |
| Loss of availability | [ ] | [ ] | backups, restore tests (runbook) | [ ] |

## 4. Consultation and sign-off

DPO advice: [ ]. Data subject views sought: [ ]. Residual high risk requires
prior consultation with the regulator (Art. 36). Approved by: [name, date].
Review date: [ ].
