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

## Integrity

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

Correction to the section above: only consent-history and subject-access reads
are audited today. Auditing other personal-data reads is CRM-T40 (open).
