# DTAC supporting statements

Drafts for the NHS Digital Technology Assessment Criteria v2.0 form, written
from what the repository does today. The question-by-question assessment is in
[the DTAC assessment](../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md).
These are drafts for the manufacturer to adopt, correct, and sign; they are not
attestations. Dated 2026-10-10 (CRM-T46).

## C1.2.1 — why the product is not in scope of DCB0129

> Contact Relationship Management is a general-purpose system for managing
> relationships with customers, partners, suppliers, and stakeholders: sales
> pipelines, consent-based marketing, service-request tickets with response
> timers, and analytics. It has no clinical purpose. It holds no clinical
> record, observation, order, prescription, or care plan. It does not
> influence, support, or manage real-time or near-real-time direct care, and no
> care workflow depends on it.
>
> This position holds only for the intended use. If an organization records
> patients or service users as contacts, uses tickets to carry care requests, or
> otherwise relies on the system for direct care, it is outside the intended
> use, the product would then be in scope of DCB0129 (manufacturer) and DCB0160
> (deploying organization), and a Clinical Safety Officer must be appointed
> before use. The manufacturer does not support that use today.

Weak points a commissioner may challenge: the product can be configured by a
deployer to hold anything in free-text fields, and `person:` references can
point at patient registries upstream. The statement therefore ties the position
to intended use rather than to a technical impossibility.

## C3.5 — plan to implement multi-factor authentication

- The product holds no passwords and performs no authentication itself. Users
  authenticate at an identity provider; the service verifies signed tokens
  (PASETO v4.public from the family authentication service, or OpenID Connect
  access tokens through Keycloak).
- MFA is therefore enforced by the deployer's identity provider. The product's
  plan is to require it there, via identity federation:
  1. The deployment guide ([operations-runbook.md](operations-runbook.md))
     requires that the identity provider enforce MFA for every account type that
     can reach the product, including administrators and service operators.
  2. The service already refuses to start in production without authentication
     enabled, so there is no unauthenticated path to bypass MFA.
  3. Tokens carry an `attrs` claim, so a deployment can write policy that
     requires an MFA-strength attribute from the identity provider before
     granting privileged actions. The product does not check an MFA claim by
     default; this is a configuration the deployer adds to the policy.
- **Not yet done:** step-up re-authentication before destructive actions (CRM-T43
  remainder); a built-in check that the identity provider's tokens assert MFA.

## C3.6 — logging and reporting requirements

Defined in [audit.md](../audit.md). In summary: every mutation writes an audit
row in the same transaction; consent history is doubly recorded; reads of
contacts, accounts, leads, tickets, and activities are audited (single reads and
list summaries) with the caller, the record, and whether the result was masked;
break-glass grants and reviews are audited; the audit table is a tamper-evident
hash chain, verifiable at `GET /api/audits/verify`. Deal list reads and insights views are audited too, as one summary row per
request. **Exclusions to state in the answer:** single-deal reads do not exist
as a separate endpoint, the deal board polls so the trail grows quickly
(size retention), and the operator is expected to anchor the chain head outside
the database.

## Threat model (summary)

Assets: contact and relationship data (personal and commercial), consent
history, the audit trail, authentication keys.

| Threat | Control today | Residual |
|---|---|---|
| Unauthenticated access to the API | Production start refuses without authentication; blanket guard; lockout after repeated failures | Misconfiguration of the identity provider is out of scope |
| A low-privilege user reads data above their role | ABAC with record-level checks; `mask` on contacts, deals, forecasts, dashboards, insights, campaign cost and ROI | A new endpoint added without masking; the persona matrix is pinned only at policy level |
| Insider alters or deletes the audit trail | Hash chain, append-only trigger, admin-only audit reads | A database owner can disable the trigger; anchor the head externally |
| Database or backup disclosure | Deployer volume/TDE encryption; field-level AES-256-GCM on two columns | Keys are not in a KMS; other columns are plaintext at the field level |
| Credential stuffing, request flooding | Per-client rate limit, failed-auth lockout | Per-process counters; shared anonymous bucket without a trusted proxy header |
| Token theft | Short-lived signed tokens, tokens never in browser JavaScript (BFF cookie), idle sign-out after 15 minutes | No token binding or revocation list beyond expiry |
| Cross-site scripting, framing, MIME sniffing | CSP with hashes, frame denial, `nosniff`, referrer policy | Third-party components are unaudited |
| Vulnerable dependencies | `cargo audit`, `cargo deny`, `pnpm audit`, SBOMs on every change | Zero-day exposure between scans |
| Abuse of emergency access | Reasoned, time-limited, audited, reviewed by another person | Detection depends on the deployer alerting on grants |
| Mass marketing without consent | Consent gate in the send path; append-only consent history | The real send path does not exist yet (CRM-T39) |

This is a summary, not a structured threat-modelling exercise (STRIDE or
similar); that remains to be done by people who can attack the system.

## C3.4 — Software Security Code of Practice

See the mapping table in the DTAC assessment. Gaps to close before confirming:
signed builds and image digests, a published disclosure timeline, and a review
of the evidence against the code by someone other than its author.

## C4.1.2 — GDS API guidance

Present: a published OpenAPI 3 document, version negotiation through the
`Accepts-version` header, conditional requests with ETags, consistent JSON
errors, pagination caps. Not verified: the full GDS API technical and data
standards checklist (for example problem-details error format, API versioning
policy text, rate-limit headers other than `Retry-After`). Treat the answer as
"Cannot confirm" until that review is done.
