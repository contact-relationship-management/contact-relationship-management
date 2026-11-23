# Changelog — Android app

## Unreleased

- First version (CRM-T303, spec/mobile-apps): contacts with a name filter, a contact's details, activities, deals and
  tickets, logging an activity with an `Idempotency-Key`, deals by pipeline and stage, tickets with SLA breach badges,
  a demo-server warning, and OIDC sign-in with PKCE in the browser. Records are kept in memory only; the server URL, the
  issuer URL and the client ID are the only things stored. Built and unit-tested; not yet run on an emulator or against
  a real identity provider.
