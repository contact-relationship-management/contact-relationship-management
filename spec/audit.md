# Audit & events

## Audit trail

Family conventions: every mutation writes an audit row (entity
kind, pid, action, actor, snapshot). CRM-specific emphases:

- **Consent** is doubly recorded: the append-only `ConsentEvent`
  history *is* domain data, and each change also writes the normal
  audit row — GDPR accountability evidence from two angles.
- **Stage moves, reasoned actions** (deal reopen, lost reason,
  priority change, nurture exit, article archive) carry their
  reason in the snapshot.
- **Sensitive reads audited**: consent history views and unmasked
  deal-amount/forecast reads.

## Events

Family envelope (kinds in [domain-model.md](domain-model.md)),
deduped by consumers on `event_id`; transactional outbox under the
`outbox` transport. `sla_breached` is emitted once per breach by
the sweep job (idempotent on the breach fact).

Webhook subscriptions (CRM-R28) audit under the entity `webhook_subscription`:
`webhook_subscription_created` (with the event patterns), `webhook_secret_rotated`,
`webhook_subscription_removed` (with how many queued deliveries were cancelled) and
`webhook_subscription_disabled` (the receiver answered `410 Gone`; no actor). Never the address
or the secret.

A web form submission (CRM-R31) audits as the actor `web_form:<slug>`: the contact's `created` (when new) and
`consent_pending`, the lead's `lead_captured`, and `web_form_submitted` on the lead with the form and purpose. Never the
name or the address. A submission the honeypot caught, one over a limit and one from a refused origin write nothing.

Notifications (CRM-R32) audit under the entity `notification`: `notifications_swept` (counts of created and suppressed, only
when something happened; no actor when the periodic job ran it), `read` (one notification) and `read_all` (with how many).
No record text.

Search (CRM-R33) audits as the entity `search`, action `search`, with how many hits were returned. Never the text searched for.

Lead imports (CRM-R34) audit under the entity `lead_import`: `lead_import_previewed` (counts), `lead_import_committed` (counts,
the source note and the date obtained) and `lead_import_undone` (removed and kept), plus the usual `lead_captured` for each lead made and
`import_undone` on each removed. Never a name or an address.

A contact edited with `PUT /api/contacts/{pid}` (CRM-R38) audits as `contact` `updated`, with the names of the fields changed, never their values.

An account edited with `PUT /api/accounts/{pid}` (CRM-R39) audits as `account` `updated`, with the field names only.

Contact and account imports (CRM-R43) audit under the entity `record_import`: `contact_import_previewed` and `account_import_previewed` (counts), `..._committed` (counts, source note, date) and `..._undone` (removed, kept). Each record made has its own `created` row carrying the import pid, and an undone one an `import_undone` row.

Attachments (CRM-R42) audit under the entity `attachment`: `uploaded` (subject, type, size, digest; never the name), `downloaded` (an audited disclosure), `deleted` and `refused_infected`. Erasing a contact records how many files went.

Custom fields (CRM-R41) audit under the entity `custom_field` (`created` with entity, key, kind and sensitivity; `updated`; `deleted` with the count of values removed). Setting values audits as the record's own entity with `custom_fields_updated` and the keys, never the values.

Tags (CRM-R47) audit under the entity `tag` (`created`, `renamed`, `deleted` with how many taggings went). Setting a record's tags audits as the record's own entity, `tags_set`, with the count and no names; erasing a contact records how many taggings went.

Business calendars (CRM-R45) audit under the entity `business_calendar` (`created`, `updated`, `deleted`); a policy changing calendar audits as `sla_policy` `calendar_set`.

A lead given an owner by a workflow audits as `lead` `assigned` (CRM-R44), by the actor `workflow:<pid>`, with the worker and the strategy.

Workflows (CRM-R40) audit under the entity `workflow`: `created`, `updated`, `deleted` (trigger and on/off, never the actions' text) and `dry_run` (counts). Each run audits as `workflow_run` with `fired`, `failed` or `blocked`, the workflow, the record and the step count, and the actor `workflow:<pid>` is the actor of every write the run makes.

Email templates (CRM-R36) audit under the entity `email_template`: `created`, `updated`, `deleted`; never the subject or body.

Saved views (CRM-R35) audit under the entity `view`: `created`, `updated`, `deleted`, with the list and whether it is shared; never the
name or the filter.

## Integrity

**A write whose audit row cannot be written is refused.** The audit row is part of the
write, not a side effect of it: if inserting it fails, the transaction rolls back, the caller
gets an error, and nothing changed, so no mutation exists without its audit entry. A handler
never logs an audit failure and carries on. The same holds for a read that is audited because
it discloses personal data (the contact, account, deal-list and insight reads, the consent
history, the subject-access export): when its audit row cannot be written the read is
refused, because an undisclosed disclosure is worse than an error. The failure is a server
error (`500`), never a silent success.

State transitions + audit + outbox share one transaction; lead
conversion (lead → contact + deal) is atomic; the nurture scheduler
is idempotent per (enrolment, step); Kanban reorder races serialize
on the deal row lock (`FOR UPDATE`).

## Tamper evidence (CRM-T31)

Each audit row stores `prev_hash` and `hash`: SHA-256 over the row's id,
previous hash, entity, pid, action, actor, snapshot, and timestamp. Altering,
deleting, or reordering a row breaks the chain from that row on.
`GET /api/audits/verify` recomputes it and returns `ok`, `rows`,
`legacy_rows` (written before the chain), `head`, and `first_break`. On
PostgreSQL an append-only trigger also refuses `DELETE` and any `UPDATE`
other than filling an empty `hash`. Limits: audited writes are serialized by
a transaction advisory lock; deleting the newest rows leaves a valid shorter
chain, so anchor `head` outside the database
([compliance/operations-runbook.md](compliance/operations-runbook.md)); a
table owner can drop the trigger.

Personal-data reads (CRM-T40): single reads of contacts, leads, and tickets
write `contact_read`, `lead_read`, `ticket_read`; list reads write one summary
row (`*_list_read`, with the returned count). Account reads (single and list) are audited too (CRM-T47). Deal list reads (`deal_list_read`) and
every `GET /api/insights/*` view (`insight_read`, naming the view) are audited
too. The trail grows with read traffic, so size
retention for it.
