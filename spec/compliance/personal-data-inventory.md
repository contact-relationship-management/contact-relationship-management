# Personal-data inventory and retention schedule

What personal data the CRM service holds, where, for how long, and what happens to it when a
contact is erased. It is an engineering record for an adopter's data-protection team (see the
[DPIA](dpia.md) and the [record of processing](record-of-processing.md)); it is **not** a legal
opinion, and the lawful basis and the periods an organization adopts are its own decisions.

**Kept true by a test.** `tests/requests/inventory.rs` reads the first column of the table below
and fails if a table exists in the database that is not listed, if a table is listed that does not
exist, or if the tables marked `swept` are not exactly those the retention sweep deletes from
(`SOFT_DELETED_TABLES`). A new table therefore cannot be added without being described here.

Whose data: the CRM references identities held by other services (`person:`, `worker:`,
`organization:` references) and keeps the relationship record, not the demographics. A contact's
name is held only as a display label; no email address is kept on a contact (a lead holds the
address a web form or import gave it).

## Retention schedule

| Class | What happens | Period |
|---|---|---|
| `swept` | A row is kept while it is live. Once soft-deleted it is **hard-deleted** by `POST /api/retention/sweep` (an operator runs it; `GET /api/retention` shows what it would delete). | `CRM_RETENTION_DAYS` after the soft-delete: **365 by default, never below 30** |
| `evidence` | Append-only compliance evidence, kept on purpose and never altered. | Until the deployer's own archive rule |
| `ttl` | Deleted automatically when it is old enough. | 24 hours |
| `manual` | Kept until a person removes it through the API, or until its contact is erased (the last column). Most of these are configuration or hold no personal data. | No automatic period |
| `gap` | Holds personal data or identifiers and **has no deletion path today**. | None: a deployer must archive or delete these by its own means until a task closes the gap |

**Gaps, in plain words.** Email deliveries and events, the event outbox and webhook deliveries
are never pruned. They hold contact pids, message ids and event payloads, not names, but they
are personal data while a pid can be joined back to a person. A retention rule for them is
[CRM-T140](../tasks.md). Until then the deployer's schedule (database job or partition drop) is the
control, and the [operations runbook](operations-runbook.md) is where it belongs.

**What the sweep does not do.** It never erases a contact: a contact whose marketing consent has
been withdrawn past the horizon is only *reported*. Erasing is a separate, gated act
(`POST /api/contacts/{pid}/erase`, refused while there is an open deal, an open ticket or an
active nurture enrolment), and it anonymises rather than deletes, so deals, tickets and consent
history remain under a pid that no longer identifies anyone.

## Inventory

| Table | Personal data held | About | Retention | On erasure of a contact |
|---|---|---|---|---|
| `accounts` | Organization reference, display name, owner reference, stakeholder role | Organizations; a rep (owner) | `swept` | Kept: an organization is not a person |
| `activities` | Free-text summary, actor and assignee references, sentiment | Contacts, reps | `swept` | Summaries about the contact scrubbed; actor and assignee references kept |
| `articles` | None (authored help text) | — | `swept` | Not applicable |
| `attachments` | File name, content type, size, hash, uploader reference; the file itself on disk | Contacts, accounts, deals, tickets | `manual` | Files of the contact deleted with the rows |
| `audit_logs` | Actor, record pid, snapshots of field values at the time | Reps; whoever a changed record was about | `evidence` | Never altered: rows are hash-chained |
| `break_glass_grants` | Subject (a rep) and the reason, reviewer | Reps | `evidence` | Never altered |
| `business_calendars` | None (working hours, holidays) | — | `manual` | Not applicable |
| `campaigns` | Subject and body text; counts only, no recipients named | — | `swept` | Not applicable |
| `commission_plans` | None (tiers) | — | `manual` | Not applicable |
| `consent_events` | Contact pid, purpose, action, source, time: the consent history | Contacts | `evidence` | Kept as evidence, keyed to a pid that no longer identifies anyone |
| `contact_consents` | Contact pid, purpose, state | Contacts | `manual` | Every purpose withdrawn; rows kept |
| `contact_merges` | Surviving and merged contact pids, the merged person reference, actor | Contacts, reps | `manual` | The merged person reference is replaced by a tombstone |
| `contacts` | Person reference, display, given and family name, job title, owner, preferred channel, marketing consent, stakeholder role | Contacts | `swept` | Identity fields scrubbed to a tombstone and the row soft-deleted |
| `custom_field_defs` | Labels the deployer chose; may be set to sensitive | — | `manual` | Not applicable |
| `custom_field_values` | Whatever the deployer's fields hold, by subject | Contacts, accounts | `manual` | Values of the contact deleted |
| `deal_clawbacks` | Worker reference, amount, reason, who set it | Reps | `manual` | Not applicable: a rep, not a contact |
| `deal_deliveries` | Deal pid, the idea's id in PPM, who opened it | Reps | `manual` | Not applicable |
| `deal_splits` | Worker references and shares | Reps | `manual` | Not applicable |
| `deals` | Name (may name a customer), amount, owner reference, primary contact pid, lost reason (free text) | Contacts, reps | `swept` | Kept: revenue records, keyed to a pid that no longer identifies anyone |
| `email_deliveries` | Contact pid, message id, provider id, send state and error text | Contacts | `gap` | Kept |
| `email_events` | Message id and the provider event (delivered, bounced, opened, clicked) | Contacts, through the message id | `gap` | Kept |
| `email_suppressions` | Contact pid and the reason a mailbox is suppressed | Contacts | `manual` | Kept: a suppression must survive |
| `email_templates` | None (template text) | — | `manual` | Not applicable |
| `event_outbox` | Event payloads: record pids, actor, field values the event carries | Contacts, reps | `gap` | Kept |
| `external_links` | URL, external id, label, who created it, by subject | Contacts, accounts, deals | `manual` | Links of the contact deleted |
| `forecast_snapshots` | None (totals per day) | — | `swept` | Not applicable |
| `form_submissions` | Contact pid, lead pid, form, purpose, confirmation times | Contacts | `manual` | Submissions of the contact deleted |
| `fx_rates` | None (exchange rates, who set them) | Reps | `manual` | Not applicable |
| `idempotency_keys` | Cached request and response bodies, which can hold anything the request did | Whoever the request concerned | `ttl` | Expires after 24 hours |
| `kb_daily` | None (daily article counts) | — | `manual` | Not applicable |
| `lead_imports` | Who imported, source note, the pids created | Reps; leads | `manual` | Not applicable |
| `leads` | Display name, email address and its domain, source, score, owner | Leads | `swept` | Name and address scrubbed for a contact's lead |
| `memberships` | Account pid and dates | Organizations | `swept` | Not applicable |
| `notifications` | Recipient reference, the kind and the record it points at | Reps; contacts, through the record | `manual` | Notifications about the contact deleted |
| `nurture_enrollments` | Sequence and contact pid, step and next due time | Contacts | `swept` | Kept |
| `nurture_sequences` | None (sequence name and purpose) | — | `swept` | Not applicable |
| `nurture_steps` | None (message text) | — | `swept` | Not applicable |
| `partnerships` | Account pid, kind, stage, free-text summary | Organizations | `swept` | Not applicable |
| `pipeline_stages` | None | — | `swept` | Not applicable |
| `pipelines` | None | — | `swept` | Not applicable |
| `quotas` | Worker reference, period, amount, plan | Reps | `manual` | Not applicable |
| `record_imports` | Who imported, source note, the pids created | Reps; contacts, leads | `manual` | Not applicable |
| `reports` | Owner reference, name, the report's definition | Reps | `manual` | Not applicable |
| `saved_views` | Owner reference, name, filters | Reps | `manual` | Not applicable |
| `seaql_migrations` | None (schema version) | — | `manual` | Not applicable |
| `segments` | None (a filter definition) | — | `swept` | Not applicable |
| `sla_policies` | None | — | `swept` | Not applicable |
| `taggings` | Tag and subject pids | Contacts, accounts | `manual` | Taggings of the contact deleted |
| `tags` | None (tag names) | — | `manual` | Not applicable |
| `teams` | Team name and its workers' references | Reps | `manual` | Not applicable |
| `territories` | Name, owner reference | Reps | `manual` | Not applicable |
| `territory_accounts` | Organization reference | Organizations | `manual` | Not applicable |
| `tickets` | Title (free text), contact pid, assignee reference, times | Contacts, reps | `swept` | Kept: keyed to a pid that no longer identifies anyone |
| `webhook_deliveries` | Event id and record pid, send state and the receiver's error text | Contacts, through the record | `gap` | Pending ones cancelled; rows kept |
| `webhook_subscriptions` | Receiver URL, signing secrets, who created it | Reps | `manual` | Not applicable |
| `workflow_runs` | Workflow name, the record it ran on, the outcome and steps | Contacts, through the record | `manual` | Kept |
| `workflows` | None (rules) | — | `manual` | Not applicable |
| `working_group_members` | Group and contact pid | Contacts | `manual` | Rows of the contact deleted |
| `working_groups` | None (group name and purpose) | — | `swept` | Not applicable |

## Not in the database

- **Attachment files** are on disk outside the database, under a random name; the rows above say
  whose they are. Erasing a contact deletes its files after the transaction commits.
- **Logs and traces** carry the request trace the framework writes: method, URI **including its
  query string** (so a query that names someone is logged), user agent, status, latency and request
  id. The service writes no request body to them. Where they go and how long they stay is the
  deployer's (see the runbook).
- **Email in transit** is composed from a contact's address held by the person service, sent
  through the configured provider, and not stored here beyond the delivery rows above.
