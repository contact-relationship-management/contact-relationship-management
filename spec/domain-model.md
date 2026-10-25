# Domain model

Every owned record has a public UUID `pid`, timestamps, soft delete,
audit + events. Upstream references are EntityRef URNs. Money is
minor units (`i64`) + ISO-4217. Text/array input caps per the family
security invariants.

## Relationship layer

**Contact** — a person the business talks to.
`person_ref` (`person:` URN, required), `account_pid` (nullable),
`owner_ref` (`worker:` URN), `status` (`active | inactive`),
`job_title`, `preferred_channel`, `marketing_consent`
(`granted | withdrawn | never | pending`; the state of the default purpose
`marketing`, CRM-R29), `consent_changed_at`, cached display name. The other
purposes (`newsletter`, `events`, `product_updates`) are rows of
`contact_consents` (contact, purpose, state, changed_at); no row is `never`.
A web form submission (CRM-R31) is a row of `form_submissions` (form slug, contact, lead, purpose, the
nurture sequence to start on confirmation, when the confirmation mail went out and when the visitor confirmed). It holds no
name and no address, and erasure deletes it.
A task activity may carry `assignee_ref` (`worker:` URN) and `remind_at` (CRM-R32). A notification is a row of `notifications`
(recipient, kind `task_reminder | sla_breach`, the record it is about, the activity or ticket that caused it, a `source_key` that
makes each fire once, `suppressed`, `read_at`); it holds no record text and erasure of the contact deletes it.
A lead import (CRM-R34) is a row of `lead_imports`: who, where the list came from (`source_note`), the date it was obtained, counts,
status (`running | completed | undone`) and the pid of every lead it made. Imported leads have `source=import`, no contact and no consent.
A contact (CRM-R37) may carry an optional `family_name`, set when the contact is made. It is never derived from `display_name`, is filled by a
merge when the survivor has none (and cleared again by an unmerge), and is erased with the contact. The same holds for the optional `given_name` (CRM-R38); a
contact's own fields can be changed after creation with `PUT /api/contacts/{pid}`.
An import of contacts or accounts (CRM-R43) is a row of `record_imports` (kind, source, obtained date, counts, and the pid of every record it made), like a lead import (CRM-R34).
An attachment (CRM-R42) is a row of `attachments` (record kind and pid, cleaned name, content type, size, SHA-256, uploader); the bytes are a file in `CRM_ATTACHMENT_DIR` named by the row's pid.
A custom field (CRM-R41) is a row of `custom_field_defs` (entity, key, label, kind, choices, sensitivity); its values are rows of `custom_field_values` keyed by field and record pid, as text, sealed when the field is `sensitive`.
A tag (CRM-R47) is a row of `tags` (a name, unique in any case); a `tagging` puts one on a contact, account, lead or deal. Tags are personal data when they are on a person: masked, exported and erased with the contact.
A business calendar (CRM-R45) is a row of `business_calendars` (time zone, working windows per weekday, holidays); an SLA policy may name one (`calendar_pid`), and the deadlines it derives are then in working time.
A lead (CRM-R44) may have an `owner_ref`, given by a `lead_created` workflow's `assign_owner` action, which keeps a counter (`assign_cursor`) for its round-robin.
A workflow (CRM-R40) is a row of `workflows`: a trigger, conditions in the segment filter language and a closed list of actions; each run is a row of `workflow_runs` that keeps the workflow's name, the reason it fired and its steps.
An email template (CRM-R36) is a row of `email_templates`: a name (unique, any case), a subject, a plain-text body that holds merge tokens and
must contain `{{unsubscribe_url}}`, and the template's own fallbacks (token to text). A campaign or nurture step may carry its `template_pid`
instead of a subject and body.
A saved view (CRM-R35) is a row of `saved_views`: owner (`worker:` URN), the list it is for, a name (unique per owner and list), a filter
(field to values), an optional sort and columns, and whether it is shared. It holds no record data.

**Account** — an organization relationship.
`organization_ref` (`organization:` URN, required), `owner_ref`,
`tier` (`prospect | customer | partner | former`), `industry`,
`annual_revenue_minor` + `currency` (optional), cached display name.

**Activity** — one interaction, attachable to contact / account /
lead / deal / ticket (`subject_kind` + `subject_pid`).
`kind` (`call | email | meeting | note | task`), `occurred_at`,
`actor_ref` (`worker:` URN), `summary`, `due_on` + `done` (task
kind only).

## Sales

**Lead** — an unqualified prospect.
`source` (`web | referral | event | import | campaign`),
`campaign_pid` (nullable attribution), `contact_ref` (optional
`person:` URN once known), `email_domain`, `score` (derived, 0–100),
`status`: `new → contacted → qualified → converted | disqualified`.
Conversion creates/links a Contact and optionally opens a Deal in
one transaction.

**Pipeline** — a named stage list. `name`, ordered **PipelineStage**
rows: `name`, `position`, `probability_percent` (0–100),
`is_won` / `is_lost` terminal flags.

**Deal** — a revenue opportunity.
`account_pid`, `primary_contact_pid`, `owner_ref`, `pipeline_pid`,
`stage_pid`, `amount_minor` + `currency`, `expected_close_on`,
`kanban_position`, `source_campaign_pid` (nullable), `closed_at`,
`won` (set only by entering a terminal stage), `lost_reason`.
Stage moves are audited; a deal in a terminal stage is immutable
except reopening to its prior stage with a reason.

**ForecastSnapshot** — optional persisted roll-up per period/owner
(the live forecast is derived; snapshots freeze month-end).

## Marketing

**Campaign** — `kind` (`email` v1), `name`, `status`
(`draft → scheduled → running → completed | cancelled`),
`cost_minor` + `currency`, `segment_pid`, counters (recipients,
delivered, opened, clicked, unsubscribed — simulated in demo mode).

**Segment** — a saved audience filter over contacts (declarative
JSON filter: consent = granted required always).

**NurtureSequence** — ordered **NurtureStep** rows (`position`,
`delay_hours`, `template_ref`); **NurtureEnrollment** per contact
with `current_step`, `next_due_at`, `status`
(`active | completed | exited`); exit on unsubscribe or conversion.

**ConsentEvent** — append-only consent history per contact
(`granted | withdrawn | pending`, `purpose`, `source`, `occurred_at`) — the audit
substrate behind every purpose's state. A **Campaign** and a **NurtureSequence**
name the `purpose` they are sent under (default `marketing`).

## Support

**Ticket** — `contact_pid`, `account_pid` (derived default),
`assignee_ref` (`worker:` URN), `priority`
(`low | normal | high | urgent`), `channel`, `status`:
`open → pending → resolved → closed` (reopen: `resolved → open`),
`sla_policy_pid`, derived `first_response_due_at`,
`resolution_due_at`, `first_responded_at`, `resolved_at`, breach
flags.

**SlaPolicy** — per priority: `first_response_minutes`,
`resolution_minutes`, business-hours flag (v1: 24×7 clocks).

**Article** — knowledge base. `title`, `body`, `keywords`,
`status` (`draft → published → archived`), `version` (bumped on
published edits), `ticket links` (Activity of kind `note` with
`article_pid`).

## Engagement & confederation (CRM-T20)

Declared stakeholder typing and a forward-only partnership
lifecycle — none of it inferred; a rating with no entry stays
`null`, never defaulted.

**Stakeholder typing** — `stakeholder_role` (one of
`customer | partner | regulator | sponsor | community | media |
member`) on both **Contact** and **Account**; **Contact** also
carries a declared power–interest grid position, `influence` +
`interest` (1–5 each, both nullable).

**Activity.sentiment** — `positive | neutral | negative`, recorded
on the activity, never derived from its text.

**Partnership** — an account-scoped innovation-partnership record.
`account_pid`, `kind` (`university | startup | vendor | accelerator
| other`), `stage` (`scouting → pilot → scaled`, forward one step at
a time; `retired` reachable from any live stage), `summary`,
`started_on`.

**Membership** — one confederation-membership record per account.
`account_pid` (unique), `joined_on`, `status`
(`active | lapsed`), `renewal_on`.

**WorkingGroup** — a cross-member collaboration group. `name`,
`purpose`; roster in **WorkingGroupMember** (`group_pid` ×
`contact_pid`).

## Derived views (never stored as editable data)

Win rate (won ÷ closed), pipeline value by stage, stage-weighted
forecast (Σ `amount × probability` over open deals), campaign ROI
((attributed won revenue − cost) ÷ cost), CLV per account
(Σ won deal amounts), SLA health (open breaches by priority),
activity feed. All ETag-conditional; all carry `as_of`.

**Insight views (CRM-T19)** — seven read-only derived views under
`/api/insights/*`: stale-deals (days-in-stage from
`deal_stage_changed` audits), follow-ups (open activities by
`due_on`: overdue aging + next-30-days), pipeline-hygiene
(rule-disclosed findings — no amount / no expected close / past
expected close / no recent activity / unworked leads), executive
(period pack: won/lost per-currency, lost reasons verbatim, leads /
tickets / activities / campaigns-started / consent withdrawals),
forecast-trends (stored `ForecastSnapshot` series only, no
interpolation), SLA (breach register + per-assignee workload, 4h
at-risk window disclosed), DPO (consent coverage + withdrawals +
per-source counts + duplicate-contact hygiene over shared
`person_ref`).

**Engagement views (CRM-T20)** — nine further derived views:
cadence (untouched contacts/accounts + no-next-touch), engagement
workload (activity kinds + recorded sentiment), the audit-derived
pipeline funnel (entered-per-stage from `to_stage` audits, honest
ratios), member health (+ silent-member list), consent-by-account,
the stakeholder register + power–interest grid (declared scores
only), the partnership register, membership renewals; follow-ups
(above) gains a `kind` filter for the renewals convention.

## Subject rights & retention (CRM-R20 — CRM-D14)

Not record types but the lifecycle over all of them: the
**subject-access export** gathers every table keyed to one contact
(exclusions named in the payload); **erasure anonymises** (identity
fields scrubbed to a tombstone `person:` URN, linked free text
scrubbed, row soft-deleted; deals/tickets/consent history remain —
CRM has no monetary field on Contact itself, so nothing analogous to
a salary field needs clearing) and is refused while the contact holds
an open deal, an open ticket, or an active nurture enrolment (its
`status` field is not the gate — see CRM-D14); the **retention sweep**
hard-deletes soft-deleted rows past the floored horizon
(`CRM_RETENTION_DAYS`, default 365, floor 30) across the 19-table
list, and the report separately discloses (never auto-scrubs) contacts
whose consent has stood withdrawn since before the horizon.

## Event kinds

`lead_captured`, `lead_scored`, `lead_converted`, `deal_opened`,
`deal_stage_changed`, `deal_won` / `deal_lost`,
`campaign_started` / `campaign_completed`, `consent_granted` /
`consent_withdrawn` / `consent_pending` (each for one purpose), `nurture_step_sent`, `ticket_opened`,
`ticket_first_response`, `ticket_resolved` / `ticket_closed`,
`sla_breached`, `article_published`.
