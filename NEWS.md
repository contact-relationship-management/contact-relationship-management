# News

## Updates

- 2026-07-18: Rust service implemented (sales, marketing, support, analytics).
- 2026-09-29: Optional Keycloak bearer-token acceptance added to the service.
- 2026-10-10: Standalone CI added for GitHub Actions and Woodpecker; the
  repository is now self-contained.
- 2026-10-19: The contact matcher crate (the probability that two contact
  records are the same person) and the service's duplicates endpoint.
- 2026-10-28: Bridges to the family's project portfolio service (a won deal
  opens a delivery idea) and worker service (departments become teams), with a
  bearer credential for each (CRM-T139, CRM-T153).
- 2026-10-30: The worker service's hire and leave events pause and resume a
  contact's marketing, in the order they happened. An employment check asks the
  worker service which contacts are its workers. Imported teams and CRM users
  are named by `person:` reference (CRM-T158 to CRM-T162). See the
  [family bridges tutorial](docs/tutorial-family-bridges.md). None of this has
  been run against a running worker service.

For details see [CHANGELOG.md](CHANGELOG.md) and the subproject changelogs.

## Press and contact

Joel Parker Henderson, <joel@joelparkerhenderson.com>
