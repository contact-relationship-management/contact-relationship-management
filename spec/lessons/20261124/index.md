# Lessons from an independent review of a large organization: ideas for CRM

Date: 2026-11-24. **Status: proposed, nothing built.** This file is an assessment and advice. It changes no behaviour and takes no CRM-R, CRM-D, or CRM-T id. An idea becomes work only when the owner chooses it.

## Where the ideas come from, and what is left out

The source is a published independent review of how a large organization sets direction, takes decisions, leads its people, and follows through. **The source is anonymous here on purpose.** This file names no organization, person, place, sector, programme, or document. It uses no quotation and no figure from the source, and it describes patterns in general terms. Keep it that way in any requirement, decision, task, test, demo data, or commit message that comes from it.

**Evidence base.** The whole review was read. What CRM has was checked on 2026-11-24 against `spec/`, the service code, the domain crate, and the migrations, by reading. The points the advice rests on (F1 to F4, and the ticket, tag, reopen, and access findings) were checked again the same day at commit `36bf46c`. No test was run and no service was started. "None" below means a search found nothing, not that its absence is proved.

**Fit.** The review concerns an organization's own people and partners more than its customers. Staff as a subject belong to the workforce service, and programmes to the portfolio service. What transfers to CRM is narrower and useful: **whether people who raise something get an answer, whether issues get stuck, whether promises are kept, and whether a relationship survives a change of owner.** Ideas outside that were left out, or appear under "Not advised".

## Advice, in one paragraph

CRM measures how fast it responds, and hardly measures whether the answer was any good or whether the person heard it. A ticket has a title, a priority, and a status, with no outcome, no theme, and no movement history; reopening it clears the date it was resolved. An organization with the problems in the review could show good service figures here while people lost faith in it. **Build first**: a recorded outcome that the person is told (1), a movement history for tickets (2), and themes with recurrence (3). Then add commitments (4) and ownership continuity (5). Before any of these, fix the four places where the specification promises something the code does not do.

## Fix first: the specification and the code disagree

These are not new ideas. They were found while checking, and the review's main finding is a gap between what is written and what happens.

| # | The specification says | The code does | Where |
| --- | --- | --- | --- |
| F1 | Earlier versions of a knowledge-base article are kept, read-only (`service-support.md`) | The update overwrites title and body and raises a number; no earlier text is kept | `controllers/support.rs`, `update_article` |
| F2 | An article archive carries its reason in the audit snapshot (`audit.md`) | The audit row has no snapshot and no reason is taken | `controllers/support.rs`, `article_status` |
| F3 | Service level attainment over a period (`analytics-reporting.md`) | Open and breached counts for now only | `controllers/dashboards.rs`, `sla_dashboard` |
| F4 | Audit can reconstruct who held what (implied by the actions listed) | An owner change records the field name only; an escalation records `to` and not `from` | `controllers/relationships.rs`, `controllers/support.rs` |

F4 is a reading of intent, so it is a question as much as a fault: field names without values is a stated privacy rule (`audit.md`). Idea 5 gives an answer that keeps the rule.

## Summary

| # | Idea | Pattern it answers | What CRM has | Priority | Size |
| --- | --- | --- | --- | --- | --- |
| 1 | An outcome the person is told | Listened to, never answered | First-response and resolution clocks | First | M |
| 2 | Ticket movement and stuck work | Issues stop moving and nobody knows where | One-step escalation; no reassignment endpoint | First | M |
| 3 | Themes and recurrence | The same problem returns after being closed | Tags on four record kinds, not tickets | First | M |
| 4 | Commitments and whether they were kept | Promises slip without a word | Tasks with a due date | Second | M |
| 5 | Ownership continuity | Contacts must start again with each new owner | One `owner_ref`, no history | Second | M |
| 6 | Outcomes in the leadership pack | Activity reported as success | Pack counts opened and resolved | Second | S |
| 7 | Engagement exercises | Consulted, never involved | None | Second | M |
| 8 | Contact load | Too many messages and requests | Consent and suppression checks only | Second | S to M |
| 9 | Statements with corrections and known gaps | Only good news; decisions not explained | Campaigns, articles | Third | M |
| 10 | Account team and several organizations | Place and function both need a lead | One owner, one account per contact | Second | M to L |
| 11 | Networks with roles and dates | Networks rest on informal goodwill | Working groups with a roster | Second | S |
| 12 | Praise and good practice | Good work unseen | Positive sentiment on an activity | Third | S |
| 13 | Access needs, and who is being reached | Some groups are not heard | Preferred channel | Third | S, owner decision |
| 14 | Follow-up tied to its meeting | Visits with no result | Meetings and tasks, not linked | Third | S |
| 15 | Restricted records | Fear of raising a concern | None | Before any sensitive use | L |

Sizes are estimates: S is days, M is one to two weeks, and L is more, with screens and locales counted.

## Build first

### 1. An outcome the person is told

**Pattern.** People raised issues, gave detailed views, and never learned what happened. In time they stopped.

**CRM has.** `tickets` holds a title, priority, channel, and status, with clocks. There is no description, decision, reason, or record that the person was told. `resolved` is the assignee's word.

**Add.**

- An **outcome** on resolving: `done`, `partly done`, `not done, because`, `passed on to`, or `no action needed`, with a short reason meant for the person to read.
- A **told on** date with the channel, kept apart from `resolved_at`. Consent and channel rules apply as they do now, and nothing is sent by this idea alone.
- The person's answer when they give one: `confirmed`, `not resolved`, or `no reply`. No reply is never counted as confirmed.
- A measure: **resolved and not told**, by age, and the share with `not resolved`, with both numbers shown.

**Accept when:** a ticket cannot be resolved without an outcome; a `not done` without a reason is refused; tickets from before the change show `outcome unknown`; and an auto-close (CRM-R82) is shown as such and not as the person's agreement.

### 2. Ticket movement, and work that is stuck

**Pattern.** Issues went up through layers, came back, and waited. It was not clear who could decide.

**CRM has.** A wait limit with one escalation step (CRM-R64, R69). `PUT /api/tickets/{pid}` changes the title only, so no endpoint reassigns a ticket by hand. `pending` means waiting for the customer and nothing else.

**Add.**

- A reassignment action with a reason, and an append-only movement history (from, to, time, reason, by whom or by the sweep). Opaque references only, as today.
- **Waiting with whom**: `customer`, `assignee`, `another team`, `supplier`, `decision`. A `decision` needs a role and a decide-by date.
- A **stuck** list: moved more than a set number of times, or no movement for a set time, with the list behind each count.
- A status history that survives reopening. Today reopening clears `resolved_at` and keeps no count. Keep each resolution and show the reopen count.

**Guardrail.** Times are for queues, steps, and priorities. No league table of agents.

### 3. Themes, and whether they come back

**Pattern.** Similar problems were found again and again, each time as if new.

**CRM has.** Tags on contact, account, lead, and deal (`rules/tags.rs`). Tickets have no tags, category, or custom fields.

**Add.** A short, deployment-set **theme** on a ticket, engagement exercise, or commitment, chosen by a person. A theme can carry an **action** (what is being changed, owner, date) and a **closed on** date. The recurrence view lists themes seen again after closing, with the tickets behind them, and the number before and after. Untagged records are counted as `no theme`.

**Accept when:** nothing assigns a theme by inference (CRM-D13); counts by account or group follow the small-group rule (CRM-R78); and a theme with no tickets since closing reads `none seen since`, and not `fixed`.

## Then

### 4. Commitments, and whether they were kept

**Pattern.** Dates moved many times. Each move was minor, and together they cost confidence.

**CRM has.** Task activities with assignee, due date, reminder, and done. A reschedule is audited with old and new day. `done` can only be set to true.

**Add.** A commitment made **to** a contact or account: what was promised, by whom, by when, and to whom it was said. States `open`, `kept`, `kept late`, `not kept`, and `withdrawn, because`. Date changes are append-only with a reason and whether the person was told **before** the first date passed. Show open and overdue commitments on the contact and the account, and the ones moved more than once.

### 5. Ownership continuity

**Pattern.** Relationships broke off each time a lead person changed, and partners had to start again.

**CRM has.** One `owner_ref` on contact and account, changed by edit. A lead has no hand-assignment, and a deal's owner is not editable. A leave event from the workforce service pauses marketing and does not touch records the leaver owns.

**Add.**

- **Ownership periods** as their own table (owner reference, from, to, kind: `permanent`, `interim`), so the audit rule of names without values can stay. Give it a row in the [personal data inventory](../../compliance/personal-data-inventory.md), a retention period, and an erasure rule.
- A **handover** for moving a person's records to others: preview, open commitments and tickets listed, a short note per account, and one commit.
- A list of records whose owner has left, or has been interim past a set time. A leave event produces this list and never reassigns on its own.
- On an account: the number of owners in the last 36 months, and the date of the last handover.

**Guardrail.** The count is about the account. It is not shown against an owner.

### 6. Outcomes in the leadership pack

The executive pack counts mostly activity and sales, such as tickets opened and resolved. Add what makes a leader uncomfortable: breached, reopened, stuck, resolved and not told, oldest open, commitments moved or not kept, and themes that came back. Put them on the first page, each with the list behind it. Fix F3 at the same time. No combined health figure: the code already refuses one, and it should stay that way.

### 7. Engagement exercises

**Pattern.** People were consulted, decisions were made elsewhere, and the two were never joined up.

**CRM has.** None. Web forms capture leads and consent.

**Add.** A record of one exercise: the question, what is open to change **and what is not**, the audience (a segment), dates, and who decides. Then the themes heard, and an answer to each: `changed`, `not changed, because`, or `still open`. Closing with an unanswered theme is refused, and the closing statement goes to those who took part and consented. Participation is a fact on the contact's history, with its own consent purpose, and what each person said is not stored against them. Results below the small-group floor are left out, as in CRM-R78.

### 8. Contact load

**CRM has.** Checks on consent, suppression, employment, and segment before a send. The cadence insight flags too little contact and nothing flags too much.

**Add.** A read from `email_deliveries` and exercises: messages and requests per contact in a period, as a distribution. Then an optional cap per contact for non-essential purposes, with skipped sends counted and listed as `over the cap`, in the way the other skips are.

## Later, except 10 and 11

Ideas 10 and 11 are second priority since the owner's decision below; they keep their numbers and place here.

9. **Statements with corrections and known gaps.** Build F1 first. Then, on a published article or announcement: what is known, what is **not** known, when more will be said, and a correction that stays visible and reaches those who received the original. A campaign could ask for a second person's approval where a deployer wants it, and must not need one by default, because extra approval steps were part of the pattern.
10. **Account team and several organizations.** A small set of roles on an account (local lead, specialist lead, executive sponsor), each with an owner, and one role marked accountable. Contacts linked to more than one account, with a role and dates. This is a large change to the domain model, and needs a decision first.
11. **Networks with roles and dates.** `working_group_members` holds a group and a contact. Add role, joined and left dates, removal, a purpose review date, and the decisions a group contributed to. A screen is needed too: none was found.
12. **Praise and good practice.** A `compliment` kind beside complaint-like tickets, passed to the team with the person's agreement, and a list of practices other accounts adopted. It sits beside the hard figures and does not replace them.
13. **Access needs, and who is being reached.** A declared format need (large print, easy read, interpreter) helps people to be heard. CRM chose not to store a contact's language, for a good reason, so this is an owner decision with a data protection assessment. Without it, report reach by channel only.
14. **Follow-up tied to its meeting.** A task has no link to the meeting it came from. Add an optional `follows` reference, and show meetings whose follow-ups are still open. Do not count visits.

## Restricted records come before anything sensitive

The review describes people afraid to raise concerns. CRM should **not** offer a confidential or anonymous concern channel today. The ticket handlers have no record-level access check, so whoever passes the general read gate can read any ticket. Text masking for tickets is not built, and sealing ticket text is open (CRM-T279).

If it is ever wanted, the order is: record-level access on tickets; a restricted flag with a named access list; reads audited and visible to the list; counts that leave restricted records out below the floor; and only then an intake that keeps no identity. Until then, say on the web form where such concerns should go instead.

Record-level access on tickets is worth having for its own sake, and is the one part of this section to schedule now.

## Not advised

- **A trust, relationship health, or sentiment score.** CRM refuses an invented health score and inferred sentiment (CRM-D13). Keep both.
- **Staff as contacts.** Employment belongs to the workforce service (`scope.md`). Staff feedback goes there.
- **A register of reviews and their recommendations.** It belongs to the portfolio service. CRM can hold the relationship with the reviewing body, and link by reference.
- **Themes found by a model.** A person chooses them.
- **Goodwill and discount limits.** Useful in principle, and no current user need was found. Wait for one.
- **Measures of individual agents or owners.** Idea 2 is about queues and idea 5 about accounts.

## Sequencing

1. F1 to F3, and a decision on F4.
2. Ticket status and movement history (2). **Each month without it loses history that cannot be rebuilt.**
3. Outcome and told-on (1), then themes (3).
4. The leadership pack (6), which is small once 1 to 3 exist.
5. The purpose and scope change, then commitments and ownership (4 and 5), contact load (8), engagement exercises (7), networks (11), and account teams (10).
6. Record-level access on tickets, alongside any of these.

## Decided by the owner

**2026-11-24: CRM is for partner and public engagement, promotion, sales, and support.** So ideas 7 to 11 are in scope, and engagement exercises (7) and contact load (8) are no longer conditional. Account teams (10) and networks (11) move from later to second. What this decision does not settle:

- The specification still describes a commercial model ([purpose](../../purpose.md), [scope](../../scope.md)). Both need a change before code for 7, 10, or 11 lands.
- **Promotion and engagement need separate consent purposes.** Agreeing to be consulted is not agreeing to be promoted to, and the reverse. Contact load (8) counts both together, because the person receives both.
- Public engagement raises the priority of record-level access on tickets, and of the note on where sensitive concerns should go.

## Questions for the owner

1. Should a ticket have an outcome the contact can read, or is CRM's support module meant to stay minimal?
2. May ownership periods be kept (idea 5), and for how long?
3. Is F4 a fault, or the privacy rule working as intended?

## Limits of this advice

One review, of one organization, was read. It relied on testimony and documents and says so. Its recommendations were new when it was read, so nothing shows that they work. No user of CRM was asked. Every size is an estimate.
