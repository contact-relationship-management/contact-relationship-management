# Risk management in the manner of ISO 14971

A starting point for an organization that gives this software a **medical purpose**
and so has to run a risk management process on it. It follows the structure of
ISO 14971:2019 (*Medical devices: application of risk management to medical devices*) and
cites the guidance in ISO/TR 24971. It is **not** a risk management file, makes no
claim of conformity with either, and no one has reviewed it against the standard's
text: read the standard itself before relying on it.

## 1. Status of this document

- **As shipped, the software is not a medical device.** It has no clinical purpose
  ([DTAC supporting statements](dtac-supporting.md), C1.2.1; the
  [AI statement](../special-files-for-public-repos/AI_STATEMENT.md), section 2). ISO 14971 applies
  to a medical device, so it does not bind this project. In Great Britain the regulator
  for that question is the
  [MHRA](https://www.gov.uk/government/organisations/medicines-and-healthcare-products-regulatory-agency);
  this project has not asked it for a view ([regulatory.md](../regulatory.md)).
- **If an integrator gives it a medical purpose** (patients as contacts, tickets carrying
  care requests, anything a care workflow depends on), the integrator becomes the
  manufacturer of that product, owns the risk management file and must decide the
  acceptability criteria. Severity, probability and acceptability are **left to them**:
  this document names hazards and the controls the software has, and invents no numbers.
- The ground rules are those of the [evidence pack](index.md). A control is **Implemented**
  only when code or configuration in this repository enforces it **and a test pins it**;
  **Deployer** when the organization running it must supply it; **Gap** when neither is
  true. Each gap names its task in [../tasks.md](../tasks.md).

## 2. The process, clause by clause

| Clause (2019 edition) | What it asks | Where it stands here |
|---|---|---|
| 4 General: process, management responsibilities, competence, plan, file | A defined process, top management commitment, qualified people, a plan and a risk management file | **Deployer.** The manufacturer's. Nothing in this repository can supply it. |
| 5.2 Intended use and foreseeable misuse | State what it is for and how it may be misused | The intended use is in the DTAC statement; the misuse that matters is in section 3 |
| 5.3 Safety-related characteristics | Identify characteristics that could affect safety | Section 4 |
| 5.4 Hazards and hazardous situations | List them | Section 4 |
| 5.5 Risk estimation | Severity and probability of harm | **Deployer.** Depends on the clinical use, which this project does not have. |
| 6 Risk evaluation | Compare with the acceptability criteria | **Deployer.** |
| 7 Risk control: options, implementation, residual risk, benefit-risk, risks from the controls, completeness | Reduce risk, verify the control, check it created no new hazard | The controls and their evidence are in section 4; the risks the controls themselves create are in section 5 |
| 8 Overall residual risk | Judge it across all hazards | **Deployer.** |
| 9 Review | Review before release | **Deployer.** |
| 10 Production and post-production | Collect and review information after release | Partly: audit, logs and the [runbook](operations-runbook.md) give the data; no one reviews it for safety. **Gap**, the deployer's. |

## 3. Foreseeable misuse

- **Care use.** Recording a patient as a contact or a care request as a ticket, then
  relying on the ticket clock. The software does not stop it: free-text fields hold anything
  and a `person:` reference can point at a patient registry upstream
  ([dtac-supporting.md](dtac-supporting.md), the weak points paragraph).
- **Taking the tour for the product.** The tour (demo mode) holds invented data in memory and
  mirrors the service only in outline.
- **Switching a safety-neutral automation on for clinical work**: the wait-limit action and
  automatic closing below.

## 4. Hazards the software could contribute to

These are the hazards a **care** use would add to. The harm column is the integrator's to
confirm; the controls are what the software has.

| # | Hazardous situation | Possible harm in a care use | Control in the software | Status |
|---|---|---|---|---|
| H1 | A ticket carrying a concern is **resolved or closed by the sweep** because the customer did not answer in time | A concern is dropped unseen | Both are off by default: a policy must set `max_wait_minutes` with `wait_limit_action` `resolve` (CRM-R69) or `auto_close_days` (CRM-R82). Resolving is reversible by a status change to `open`; closing is final. Each is audited with its `via`. Tests: `tickets` wait-limit and auto-close request tests in `ticket_pauses.rs`. | Implemented; **do not enable for care** |
| H2 | An SLA clock **pauses** while a ticket waits on the customer (CRM-R55) and the ticket is forgotten | A request is answered late | The wait limit flags, optionally escalates (CRM-R64, CRM-R69), and the SLA dashboard (`GET /api/dashboards/sla`) shows open tickets by priority and breach state | Implemented for flagging; the choice to act on it is the deployer's |
| H3 | A message is sent to a person who has **withdrawn consent** | Unwanted contact; in a care setting, distress or a breach of confidence | Consent is rechecked at send time, not only at queue time; a run after a withdrawal honours it. Test: `consent_withdrawn_between_runs_is_honoured` (`email_delivery.rs`) and `enrolment_rechecks_consent_inside_the_action` | Implemented |
| H4 | A message goes to the **wrong person** because two contacts were merged wrongly | A message or record about one person reaches another | A merge is recorded and moves everything in one transaction (`a_merge_moves_everything_and_is_recorded`); identity matching and de-duplication stay with the upstream person registry | Implemented for the record of it; matching quality is **Deployer** (upstream) |
| H5 | **Erasure** removes data that was still needed | A record needed for care or a dispute is gone | Refused while a deal, ticket or nurture enrolment is open (`subject_rights_round_trip`); anonymised, not deleted, so a history keyed to a pid stays | Implemented; what must be kept is a **Deployer** decision |
| H6 | A **small group** in a report identifies a person | Disclosure of who is in a rare group | Groups with fewer records than `CRM_MIN_CELL_SIZE` (default 5) are left out and counted (CRM-R78); primary suppression only | Implemented; secondary suppression is a **Gap**, recorded in CRM-R78 |
| H7 | A reader sees **more than their role allows** | Disclosure of amounts or personal data | Attribute-based access control with masking; masked readers get counts but no sums or tags; admin-only prefixes; `CRM_REQUIRE_AUTH` gate with a production check (`hardening` tests) | Implemented; the gate must be on, **Deployer** |
| H8 | A **non-English screen** says something different from the English | A person misreads an instruction | The non-English strings are machine drafts; a register records which locale has been reviewed by a native speaker, and a test keeps every locale to the same keys and placeholders (CRM-R75) | Partly: the **review itself is a Gap** (none done) |
| H9 | An error message from the service leaks detail or confuses | Misleading text shown to a person | No page shows the service's error text; a fixed localized message by status (CRM-R76), enforced by a lint rule | Implemented |
| H10 | A figure is **wrong in money**: summed across currencies, or converted at the wrong rate | A wrong decision on cost | Minor units; per-currency lines, never summed silently; conversions name the rates used and what was left out (CRM-R50 to CRM-R54, CRM-R70, CRM-R83) | Implemented |
| H11 | A link in a message is **replayed or forwarded** | A stranger changes a person's consent or preferences | Confirmation and preference links are random, hashed at rest, work once and expire (CRM-R79); the unsubscribe link is deliberately reusable | Implemented, with that stated exception |
| H12 | The data is **unavailable or lost** | A request cannot be read when it is needed | Backup and restore are in the [runbook](operations-runbook.md); no clinical availability target is set | **Deployer**; **Gap** for any clinical target |
| H13 | **Software change** breaks a control without anyone noticing | Any of the above | CI runs the unit, request and end-to-end tests, clippy and lint as tests; releases are signed; changes are spec, code and tests together ([AGENTS.md](../../contact-relationship-management-service-with-rust/AGENTS.md)) | Implemented; a clinical change-control process is **Deployer** |
| H14 | A **forged sender** names a ticket's reference and restarts its clock | A waiting ticket is put back to `open`, so a customer who has not answered looks as if they had | A reply resumes a ticket only when its sender is the ticket's contact by the mailbox the person service gives, never by an address the message names for itself; it never reopens a resolved or closed ticket and changes nothing else; each resume is audited `via: inbound_mail`. Test: `a_reply_by_mail_resumes_the_ticket_it_names_when_it_comes_from_the_customer`. A `From` can be forged and the mail service's own verdict (SPF, DKIM) is not read | Implemented for the check; the forgery risk is the deployer's to judge |

## 5. Risks the controls themselves create

- **Suppression hides a real signal.** A report leaves out a small group, so a short table can
  look complete. The answer states how many groups it left out so it is not mistaken for whole.
- **Consent checks block a wanted message.** Rechecking at send time stops a message the person
  now declines, and also one a clinician might have wanted sent. The system sends nothing
  without consent by design; a clinical need to contact someone who has withdrawn is outside
  its intended use.
- **A wait-limit flag becomes noise.** A flag raised on every long wait can be ignored. The
  count in the sweep answer is a measure of it, not a guarantee anyone reads it.
- **Fixed error messages hide what went wrong.** A person sees a short, localized sentence by
  status; the cause is in the audit and the logs, not on screen.

## 6. What an integrator would still have to do

1. Decide whether the product has a medical purpose and, in Great Britain, whether it is a
   device at all, with the regulator if in doubt.
2. If it is: appoint the responsible people, write the risk management plan and file, set
   the acceptability criteria, estimate severity and probability for the table above and for
   what the integrator adds, evaluate the overall residual risk and review before release.
3. Check which of the software's controls their clinical use needs **switched on or off**
   (H1 above all).
4. Run the post-production loop: complaints, incidents and audit entries reviewed for safety.
5. Obtain a native-speaker review of every locale they show a patient or clinician.

Read with [dtac-supporting.md](dtac-supporting.md), the [control mapping](control-mapping.md) and
the [DPIA](dpia.md), which cover the clinical-safety standards for health IT (DCB0129 and
DCB0160) and data protection from the same facts.
