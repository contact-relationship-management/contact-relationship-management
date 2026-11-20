# Data flow

*Written 2026-11-20 (CRM-T276) from the settings the code reads.* The software as shipped. Where each part runs, and in
which region, is the hosting supplier's ([CRM-R106](../../requirements.md#crm-r106--data-stays-in-the-region-the-customer-names)).

```text
 customer's staff (browser)
        │  https; HttpOnly session cookie, no token in page script
        ▼
 UI (SvelteKit server) ───────────────► authentication service  AUTH_API_URL
        │  Bearer PASETO v4.public      (or Keycloak / OIDC: CRM_KEYCLOAK_URL)
        ▼
 service (Rust) ──► PostgreSQL  DB_URL           all records, the audit trail, the outbox
        │       ──► Redis       CRM_REDIS_URL    counters, break-glass grants, cached insight answers
        │       ──► file store  CRM_ATTACHMENT_DIR   attachments, on local disk or a mounted volume
        │
        ├──► person / organization services   display names; an address at send time
        ├──► worker service (WPM), project portfolio service (PPM)   the family bridges
        ├──► mail provider or SMTP relay       each message sent to a contact
        └──► customer-registered webhook addresses   event envelopes
 inbound to the service: WPM hire and leave events, mail-provider events, public web forms,
 unsubscribe and preference pages, MCP tool calls
```

## Every connection

"Off when unset" means nothing leaves the process until the setting is given. The upstream services and bridges are
also stub-first: without `CRM_UPSTREAM_MODE=http` they send nothing (CRM-D11).

| Connection | Setting | Direction | What crosses it | Personal data | Off when unset |
| --- | --- | --- | --- | --- | --- |
| Browser and UI | the deployment's address | both | every page a person sees | yes, all the person may see | no |
| UI to service | `CRM_API_URL` | out | every API call, with the person's token | yes | no |
| UI to authentication service | `AUTH_API_URL` | out | magic-link request and verify, token exchange, sign-out | the signing-in person's address | no |
| Keycloak / OIDC | `CRM_KEYCLOAK_URL` | out | sign-in and its tokens | the signing-in person | yes |
| Token keys | `CRM_PASETO_KEYS_URL` | out | public keys only | no | yes |
| Database | `DB_URL` | out | every record | yes | no: required in production |
| Redis | `CRM_REDIS_URL` | out | rate-limit and lockout counters, break-glass grants, cached insight answers | yes, in the cached answers | no: required in production |
| File store | `CRM_ATTACHMENT_DIR` | local | attachments ([CRM-R42](../../requirements.md)) | yes; not encrypted by the software | yes |
| Person service | `CRM_PERSON_SERVICE_URL` | out | a `person:` id; back, a display name, and a mailbox at send time | yes | yes |
| Organization service | `CRM_ORGANIZATION_SERVICE_URL` | out | an `organization:` id; back, a display name | an organization's name | yes |
| Worker service (WPM) | `CRM_WORKER_SERVICE_URL` | out | teams import (departments, `person:` refs), overlap check (`person:` refs) | references only | yes |
| Project portfolio (PPM) | `CRM_PPM_SERVICE_URL` | out | a won deal's name, id, amount and currency | no | yes |
| Mail | `CRM_MAIL_MODE` (a provider's name or `smtp`) | out | recipient address and name, subject, body, unsubscribe link | yes | yes: `simulated`, nothing is sent |
| Outbound webhooks | addresses an admin registers | out | each matching event's envelope, signed | ids of the records an event is about | yes: none registered |
| WPM events | `CRM_WPM_WEBHOOK_SECRET` | in | hire and leave events for `person:` refs | references only | yes |
| Mail-provider events | per provider | in | delivery, bounce and complaint reports | the mailbox concerned | yes |
| Web forms | a published form | in | what a member of the public types into it | yes | yes: no form published |
| Unsubscribe and preference pages | `CRM_UNSUBSCRIBE_SECRET` and `CRM_PUBLIC_URL` | in | a one-time link and the choice made | the contact's consent | yes: no links are made |
| MCP | `POST /mcp` | in | tool calls, routed as the REST calls they stand for | yes, as the REST call | no |

The mail providers the service can call directly are Brevo, Mailjet, Amazon SES, Azure Communication Services, Mailgun
and SendGrid, or any SMTP relay. Each one chosen is a sub-processor ([sub-processors.md](sub-processors.md)).

## Where personal data rests

PostgreSQL, Redis, the file store, the backups of each, and the logs (the operations runbook says what the logs hold).
The tables and their retention are in the [personal-data inventory](../personal-data-inventory.md). Two columns are
sealed with AES-256-GCM by the software (CRM-T41); everything else relies on the volume's encryption
([CRM-R102](../../requirements.md#crm-r102--encryption-in-transit-at-rest-and-of-sensitive-fields)).

## Not done

No diagram of a particular cloud's layout (that is the supplier's); the connections are read from the settings the code
reads, not observed on a running deployment.
