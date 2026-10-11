# For the Chief Information Officer

> Engineering briefing, not legal advice, certification or an audit. It states
> what the repository shows as of 2026-10-11. Nothing here claims accreditation.

## Information and data view

This CRM holds **relationship data**: contacts and accounts as wrappers around
identifiers held in other systems, activities, deals, tickets, consent history
and an audit log. Demographics deliberately stay in the identity registries
([domain-model](../../../../spec/domain-model.md)). If your organisation records patients or
citizens as contacts, the classification of that data is **yours** to decide;
the plan is written to the stricter reading ([plan](../../../../spec/plan.md)).

## Where data goes

| Store | Holds | Protection in the repo |
|---|---|---|
| PostgreSQL | All records, audit chain, break-glass grants | Field encryption on two columns; append-only audit trigger; encryption at rest is the deployer's |
| Redis | Cache of insight responses (30 s), rate-limit counters, break-glass grant keys | Keyed per caller; **needs TLS, a password and no public exposure** |
| Logs/metrics | Request metadata, Prometheus metrics | Retention is the deployer's |
| Email provider (future) | Message content and recipients | Not built (T39) |

## Integration and identity

- Sign-in comes from an identity service issuing PASETO tokens, or Keycloak.
  Joiner/mover/leaver, MFA and session policy are therefore **your identity
  provider's**; the CRM enforces what the token says and refuses expired ones.
- The API is documented in OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)),
  versioned by header, paginated with `Link` headers, and returns errors as
  RFC 9457 problem documents. The change and deprecation rules are in
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOMs (CycloneDX) are produced for service and UI in CI; the release workflow
  signs them and the images.

## Information-governance duties that remain with you

Backup and restore evidence, retention schedules per table, anchoring the audit
chain head outside the database, KMS custody of field keys, data-processing
agreements for any supplier, and the data-protection impact assessment
([dpia-template](../../../../spec/compliance/dpia-template.md)). The control mapping
([control-mapping](../../../../spec/compliance/control-mapping.md)) marks each control as
Implemented, Deployer, or Gap.
