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
| `ttl` | Deleted automatically when it is old enough. | 24 hours for `idempotency_keys`; a week after it expires or is spent for `link_tokens` |
| `manual` | Kept until a person removes it through the API, or until its contact is erased (the last column). Most of these are configuration or hold no personal data. | No automatic period |
| `aged` | A delivery or event log. Rows that are **finished with** are hard-deleted by the same sweep once they are older than their own horizon; a row something may still act on is never touched (see below). | `CRM_EVENT_RETENTION_DAYS` for the event outbox, webhook deliveries and the worker service's event ids (**30 by default, never below 7**); `CRM_MAIL_RETENTION_DAYS` for email deliveries and events (**730 by default, never below 400**: attribution looks back 365 days) |
| `gap` | Holds personal data or identifiers and **has no deletion path today**. | None: a deployer must archive or delete these by its own means until a task closes the gap |

**What `aged` deletes, and what it never does** (CRM-R81). Email deliveries: only a delivery
that was sent, or failed for good (five attempts), of a campaign that has completed or been
cancelled, or of no campaign (a nurture step); a pending, retryable or running-campaign delivery
stays. Email events: any older than the horizon. The event outbox: only an event that has been
published **and** handed to the webhooks; one still owed stays. Webhook deliveries: any that is no
longer pending. The worker service's event ids (`wpm_events`): any older than the horizon; WPM
stops retrying an event within about two days, so an id is no longer needed by then. A row is never deleted for being old alone if something may still act on it.

`contact_employments` has no period of its own: a row lives while the worker service says the
person is employed, and goes when WPM sends the leave, or when the contact is erased. If WPM never
sends the leave (a subscription removed, a delivery that gave up), the row stays and the contact
stays out of marketing: the failure is the safe way round, and a person can see it on the contact.

**Gaps.** None are open at the moment: every table that holds personal data has a deletion path or
is kept as evidence on purpose. A new table has to be classed here before it exists (the test).

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
| `contact_employments` | Contact pid, person reference, employer's organization reference, first day, the WPM event id (CRM-R92) | Contacts who are employees | `manual` | Rows of the contact deleted |
| `contact_merges` | Surviving and merged contact pids, the merged person reference, actor | Contacts, reps | `manual` | The merged person reference is replaced by a tombstone |
| `contacts` | Person reference, display, given and family name, job title, owner, preferred channel, marketing consent, stakeholder role | Contacts | `swept` | Identity fields scrubbed to a tombstone and the row soft-deleted |
| `custom_field_defs` | Labels the deployer chose; may be set to sensitive | — | `manual` | Not applicable |
| `custom_field_values` | Whatever the deployer's fields hold, by subject | Contacts, accounts | `manual` | Values of the contact deleted |
| `deal_clawbacks` | Worker reference, amount, reason, who set it | Reps | `manual` | Not applicable: a rep, not a contact |
| `deal_deliveries` | Deal pid, the idea's id in PPM, who opened it | Reps | `manual` | Not applicable |
| `deal_splits` | Worker references and shares | Reps | `manual` | Not applicable |
| `deals` | Name (may name a customer), amount, owner reference, primary contact pid, lost reason (free text) | Contacts, reps | `swept` | Kept: revenue records, keyed to a pid that no longer identifies anyone |
| `email_deliveries` | Contact pid, message id, provider id, send state and error text | Contacts | `aged` | Kept |
| `email_events` | Message id and the provider event (delivered, bounced, opened, clicked) | Contacts, through the message id | `aged` | Kept |
| `email_suppressions` | Contact pid and the reason a mailbox is suppressed | Contacts | `manual` | Kept: a suppression must survive |
| `email_templates` | None (template text) | — | `manual` | Not applicable |
| `event_outbox` | Event payloads: record pids, actor, field values the event carries | Contacts, reps | `aged` | Kept |
| `external_links` | URL, external id, label, who created it, by subject | Contacts, accounts, deals | `manual` | Links of the contact deleted |
| `forecast_snapshots` | None (totals per day) | — | `swept` | Not applicable |
| `form_submissions` | Contact pid, lead pid, form, purpose, confirmation times | Contacts | `manual` | Submissions of the contact deleted |
| `fx_rates` | None (exchange rates, who set them) | Reps | `manual` | Not applicable |
| `idempotency_keys` | Cached request and response bodies, which can hold anything the request did | Whoever the request concerned | `ttl` | Expires after 24 hours |
| `kb_daily` | None (daily article counts) | — | `manual` | Not applicable |
| `lead_imports` | Who imported, source note, the pids created | Reps; leads | `manual` | Not applicable |
| `leads` | Display name, email address and its domain, source, score, owner | Leads | `swept` | Name and address scrubbed for a contact's lead |
| `link_tokens` | A hash of each mailed confirmation or preference link (never the token), the contact pid it names, its purpose index and times | Contacts | `ttl` | Deleted with the contact; the rest a week after they expire or are spent |
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
| `webhook_deliveries` | Event id and record pid, send state and the receiver's error text | Contacts, through the record | `aged` | Pending ones cancelled; rows kept |
| `webhook_subscriptions` | Receiver URL, signing secrets, who created it | Reps | `manual` | Not applicable |
| `workflow_runs` | Workflow name, the record it ran on, the outcome and steps | Contacts, through the record | `manual` | Kept |
| `workflows` | None (rules) | — | `manual` | Not applicable |
| `working_group_members` | Group and contact pid | Contacts | `manual` | Rows of the contact deleted |
| `wpm_events` | None: the worker service's event id, kind and what was done (CRM-R92) | — | `aged` | Not applicable |
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
