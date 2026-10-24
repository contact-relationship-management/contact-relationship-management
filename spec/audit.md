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
