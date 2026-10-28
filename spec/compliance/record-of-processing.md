# Record of processing activities (GDPR Art. 30)

The controller's record, started from what the software does. The **controller's own details and
decisions are left in brackets**: only the organization running the software can state them, and
nothing here has been reviewed by a DPO. It is not legal advice. Retention periods and the data
held come from the [inventory and schedule](personal-data-inventory.md); the risks are in the
[DPIA](dpia.md).

**Controller:** [name, address, contact]. **Representative (if outside the UK/EEA):** [ ].
**Data protection officer:** [name, contact].

**Security measures common to every activity** *(system facts, Art. 30(1)(g))*: authentication and
attribute-based access control on every route when enabled; masking of money and aggregates for
callers who may not read them; admin-only destructive actions and bridges; a hash-chained audit log
of changes, consent and sensitive reads; field encryption where configured; per-caller and
per-IP rate limits; TLS, backups and key management are the deployer's (see the
[operations runbook](operations-runbook.md)).

| # | Activity | Purpose | Data subjects | Personal data | Recipients | Transfers | Retention |
|---|---|---|---|---|---|---|---|
| 1 | Relationship management | Keep contacts, accounts and their timeline so staff can serve them | Contacts; staff | `person:` reference and display name, job title, owner, preferred channel, activity summaries (free text), tags, custom fields | Staff with access; identity services (display names) | [ ] | Live rows kept; soft-deleted rows hard-deleted after `CRM_RETENTION_DAYS` (365, floor 30) |
| 2 | Marketing email and nurture | Send campaigns and sequences to people who consented | Contacts; leads | Consent state and history, contact pid, delivery state, provider message id and events; a hash of each mailed confirmation or preference link | Email provider [name]; the contact | [ ] | Consent history kept as evidence; delivery and event rows pruned by age once finished with (730 days by default) |
| 3 | Lead capture | Take a lead from a web form or an import, with double opt-in | Leads | Name, email address, source, purpose, confirmation times | Staff; email provider (the confirmation) | [ ] | As leads: soft-deleted then swept; form submissions deleted on erasure |
| 4 | Sales pipeline | Track deals, forecast, and report revenue | Contacts; staff | Deal name (may name a customer), amount, owner, primary contact pid, lost reason (free text) | Staff with access | [ ] | As deals: kept as revenue records after erasure, keyed to a pid that no longer identifies anyone |
| 5 | Quotas, commission and teams | Show where each rep stands against a target | Staff | Worker reference, quota, credited revenue, splits, clawbacks, team and territory membership | Admins only | [ ] | No automatic period (`manual`) |
| 6 | Support | Handle tickets against service-level targets; the help centre | Contacts (requesters); staff (assignees) | Ticket title (free text), contact pid, assignee, times | Staff with access | [ ] | As tickets: soft-deleted then swept |
| 7 | Bridges to other services | Open a delivery idea in the project portfolio service; set teams from the worker service | Staff; contacts (indirectly, through a deal's name) | To PPM: a deal's name, amount, currency and pid. From WPM: a worker's id and department | PPM; WPM [deployer's services] | [ ] | One row per opened deal; no automatic period |
| 8 | Analytics and reporting | Dashboards, the report builder, CLV, attribution | Contacts (aggregated) | Counts and sums; report groups under `CRM_MIN_CELL_SIZE` records left out | Staff with access; masked callers see counts without money | [ ] | Derived on request; saved reports kept until deleted |
| 9 | Audit and security logging | Prove who changed what, detect abuse | Staff; whoever a record was about | Actor, record pid, snapshots of field values, request trace (method, URI with query string, status) | Admins; the log destination [deployer's] | [ ] | Audit rows are evidence and never altered; logs are the deployer's |
| 10 | Event delivery | Tell subscribed systems that something changed | Contacts, staff (through the record) | Event payloads: pids, actor, field values the event carries | Webhook receivers the deployer registered | [ ] | The outbox and webhook deliveries pruned by age once finished with (30 days by default) |

**Lawful basis per activity** [deployer decides, Art. 30 does not require it but the DPIA does].
**Rights handling:** see DPIA section 2. **Last reviewed:** [date].
