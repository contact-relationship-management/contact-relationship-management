# Real email provider: brief for the vendor and legal reviewers (CRM-T39)

This is what to hand a prospective email service provider (ESP) and the people
who sign for it. It states what the software does today, what a real provider
needs from it, and the questions to put to each vendor. It is an engineering
brief, not legal advice, and it picks no vendor.

## What exists today

- **A send seam and an SMTP sender, off by default.** `POST
  /api/campaigns/{pid}/run` sends through `mail::Sender`. The default is
  **simulated**: nothing leaves the process, a touch activity is written per
  contact, and the engagement counters are fixed demo fractions. With
  `CRM_MAIL_MODE=smtp` it submits each message to the SMTP server named by
  `CRM_SMTP_*`. `POST /api/nurture/advance` is still simulated.
- **One delivery row per (campaign, contact).** It is claimed atomically, so a
  re-run or a concurrent run never mails a contact twice. States: pending,
  sending, sent, failed, no_address, suppressed. A transient failure is retried
  up to five times; a permanent one is not. A campaign completes only when
  nothing is left to retry.
- **Consent is checked twice:** in the segment query that lists the audience,
  and again by reading the live contact immediately before each message, so a
  withdrawal between listing and sending is honoured.
- **Content and addresses.** A campaign has an optional `subject` (defaults to
  the name) and a plain-text `body` (required for a real send). The CRM holds no
  email addresses: each is asked for from the person service at send time
  (assumed `email` field; unverified) and not stored.
- **Progress.** `GET /api/campaigns/{pid}/deliveries` gives counts per state.
  Subject access lists a contact's deliveries (state, time, message id).
- **Production guard.** `smtp` mode refuses to start in production unless
  `CRM_MAIL_ALLOW_PRODUCTION=1`, because what follows is not built.
- **Unsubscribe.** Every real message ends with a signed link and carries
  `List-Unsubscribe` and `List-Unsubscribe-Post` headers (RFC 8058).
  `GET /unsubscribe/{token}` shows a page and changes nothing (mail scanners
  follow links); `POST` withdraws marketing consent through the ordinary
  consent change (append-only history, enrolments exited, audited, source
  `unsubscribe-link`) and is idempotent. The token is the contact and campaign
  identifiers plus an HMAC-SHA256 tag, with no expiry. It needs
  `CRM_UNSUBSCRIBE_SECRET` (32+ characters) and `CRM_PUBLIC_URL` (https in
  production); `smtp` mode will not start without them. A withdrawn contact is
  suppressed at the next send, because consent is re-read before each message.
- **Outcome webhook.** `POST /webhooks/email` takes a generic, HMAC-signed
  event (`delivered`, `bounce_hard`, `bounce_soft`, `complaint`) keyed by our
  `Message-ID`, applied once per message and event. A hard bounce or complaint
  suppresses the contact (the send skips them); a complaint also withdraws
  consent. Suppression is per contact because the CRM holds no addresses.
- **Not built:** an adapter from a chosen provider's payload and signature
  scheme to this shape, `opened` and `clicked` events, and nurture sends
  (still simulated).

## A local server to build against

`scripts/email-dev.sh up` starts a Stalwart mail server in Podman with the
domain `crm.test` and one account (see `INSTALL.md`). The service can
send through it (see `INSTALL.md`), so the remaining work can be built and
tested without a vendor: submit over STARTTLS, deliver into a mailbox, read it
back over JMAP. It is **not** a vendor substitute: it
has no public DNS, SPF or DKIM, no reputation, and no bounce or complaint
traffic from real mailbox providers.

## What a real send must do

| Need | Why | Today |
|---|---|---|
| Re-check consent per recipient immediately before handing to the provider | Consent can be withdrawn at any time | Done: the live contact is re-read before each message |
| A per-recipient delivery record (campaign, contact, provider message id, state, time) | Subject access, complaint handling, idempotent retries, honest funnel numbers | Done (`email_deliveries`); no address or body stored |
| Idempotent send per (campaign, contact) | A retry must never email twice | Done for campaigns (atomic claim per row); nurture steps are idempotent per (enrolment, step) but simulated |
| Asynchronous send with retry and back-off | A real send takes minutes and the provider will throttle or fail | A background job sends batch after batch; no back-off, and a stalled campaign waits for the next `run` |
| One-click unsubscribe: a link in the body and the `List-Unsubscribe` and `List-Unsubscribe-Post` headers (RFC 8058) | Bulk-sender rules at the large mailbox providers, and PECR | Built; not yet tried against a large mailbox provider |
| An unsubscribe endpoint that writes a `withdrawn` consent event immediately | The consent log is the source of truth | Built: signed link, public endpoint, tested |
| A suppression list the send path checks, filled by bounces, complaints and unsubscribes | Never mail an address that bounced hard or complained | Built per contact; unsubscribes also suppress through consent |
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

Done: the sender interface, the SMTP sender, the per-recipient delivery table,
idempotent claiming, retry, consent re-check and the progress endpoint.

Remaining, after the vendor is chosen (the webhook format and the suppression
model are the vendor's):

1. An adapter from the provider's payload and signature scheme to the generic
   outcome webhook (the unsubscribe link, headers, suppression list, clearing
   path and generic webhook are built).
2. Send nurture steps the same way (the campaign send is already a
   background job).
3. Agree the address contract with the person service and test it.
5. Run the funnel and scoring tests against both senders.

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
