# Real email provider: brief for the vendor and legal reviewers (CRM-T39)

This is what to hand a prospective email service provider (ESP) and the people
who sign for it. It states what the software does today, what a real provider
needs from it, and the questions to put to each vendor. It is an engineering
brief, not legal advice, and it picks no vendor.

## What exists today

- **No email leaves the system.** `POST /api/campaigns/{pid}/run` simulates a
  send inside the request: it lists the segment, writes one `email` touch
  activity per contact, sets `delivered`, `opened` and `clicked` to fixed
  fractions of the audience, and marks the campaign completed. `POST
  /api/nurture/advance` does the same for nurture steps.
- **There is no sender trait and no background job.** The specification once
  said the send was "a trait seam" run as a Postgres-backed job. It is neither;
  that wording has been corrected. A real adapter needs both built first.
- **Consent is enforced at send time, in the segment query**
  (`evaluate_segment`): only contacts with `marketing_consent = granted` are
  listed, so a withdrawal between scheduling and sending is honoured. Consent
  history is an append-only log of `granted` and `withdrawn` events with source
  and time.
- **There is no unsubscribe link, suppression list, bounce handling,
  complaint handling, or per-recipient send log.** Campaign counters are
  aggregates. The subject-access export names this as an exclusion.
- **The funnel, ROI and lead score read the counters.** A real provider's
  numbers would replace the simulated ones.

## A local server to build against

`scripts/email-dev.sh up` starts a Stalwart mail server in Podman with the
domain `crm.test` and one account (see `INSTALL.md`). It lets the engineering
work below be built and tested without a vendor: submit over STARTTLS, deliver
into a mailbox, read it back over JMAP. It is **not** a vendor substitute: it
has no public DNS, SPF or DKIM, no reputation, and no bounce or complaint
traffic from real mailbox providers.

## What a real send must do

| Need | Why | Today |
|---|---|---|
| Re-check consent per recipient immediately before handing to the provider | Consent can be withdrawn at any time | Done for the audience list; not per message |
| A per-recipient delivery record (campaign, contact, provider message id, state, time) | Subject access, complaint handling, idempotent retries, honest funnel numbers | None |
| Idempotent send per (campaign, contact) | A retry must never email twice | Nurture steps are idempotent per (enrolment, step); campaigns are not |
| Asynchronous send with retry and back-off | A real send takes minutes and the provider will throttle or fail | Inline in the request |
| One-click unsubscribe: a link in the body and the `List-Unsubscribe` and `List-Unsubscribe-Post` headers (RFC 8058) | Bulk-sender rules at the large mailbox providers, and PECR | None |
| An unsubscribe endpoint that writes a `withdrawn` consent event immediately | The consent log is the source of truth | The event can be written by API; no public link |
| A suppression list the send path checks, filled by bounces, complaints and unsubscribes | Never mail an address that bounced hard or complained | None |
| Signed, authenticated webhooks for delivered, bounced, complained, opened and clicked | The provider reports outcomes asynchronously | None |
| Sender identification and a postal or contact address in each message | PECR and equivalent rules | None |
| SPF, DKIM and DMARC aligned for the sending domain | Delivery and anti-spoofing | A deployer task |
| Message content and recipient data held for a stated period, then deleted | Data minimisation | Not defined |

## Questions for each vendor

1. **Role and contract.** Will you sign a data-processing agreement as a
   processor? Do you act as a controller for any of the data (for example
   for abuse prevention or deliverability analytics)? Provide the standard
   terms and the sub-processor list.
2. **Data location and transfers.** Where is recipient data stored and
   processed? Which transfer mechanism covers any transfer out of the UK or
   EEA?
3. **Retention.** How long do you keep message bodies, recipient addresses
   and event data? Can we set it and delete on request?
4. **Security evidence.** ISO 27001 or SOC 2 report, penetration-test summary,
   breach-notification time.
5. **Suppression and consent.** Do you keep your own suppression list? How
   does it sync with ours, and which wins on conflict?
6. **Webhooks.** How are they signed? Replay window? Retry policy? Event
   ordering and duplicates?
7. **Unsubscribe.** Do you support one-click `List-Unsubscribe-Post`? Do you
   host the unsubscribe page or call us?
8. **Tracking.** Can open and click tracking be turned off per campaign? Open
   tracking is unreliable and, in some regimes, needs its own consent.
9. **Limits and cost.** Rate limits, warm-up rules, dedicated IP options,
   price per thousand, overage terms.
10. **Exit.** How do we export our data and remove it if we leave?

## What the engineering work is

1. Define a sender interface (send one message, return a provider id) with a
   simulated implementation that keeps today's behaviour for demo mode.
2. Add a per-recipient delivery table and move the send into a background
   job, idempotent per (campaign, contact), with retry.
3. Add the unsubscribe link and headers, a public unsubscribe endpoint that
   writes the consent event, and the suppression list.
4. Add authenticated webhook endpoints for outcomes, updating the delivery
   table and the campaign counters from real events.
5. Add the delivery table to the subject-access export and the erasure and
   retention rules.
6. Update the funnel and scoring tests to run against both senders.

Estimate this after the vendor is chosen, because the webhook format and the
suppression model are the vendor's.

## Decisions for the business

- Which vendor, and who signs the agreement (see the CLO brief).
- Whether open tracking is used at all.
- The retention period for message content and delivery records.
- The sending domain, and who owns its DNS records.
- Whether anything beyond marketing (transactional notices) will go through
  the same provider, which changes the lawful basis and the consent rules.

## Acceptance tests to require

- A contact who withdraws between scheduling and the send is not mailed.
- The same campaign run twice sends each contact one message.
- A hard bounce and a complaint add the address to the suppression list, and
  a later run skips it.
- A forged or replayed webhook is rejected.
- The unsubscribe link works without signing in and writes a consent event
  within seconds.
- Subject access lists every message sent to the contact.
