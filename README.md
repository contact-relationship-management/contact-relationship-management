# Contact Relationship Management

A central hub for managing interactions with customers and prospects across
the relationship lifecycle: sales automation, marketing automation, customer
support, and analytics.

> ⚠️ **Demo software.** Not a production CRM. No real email is sent unless
> a real mail mode is configured, and the data is synthetic. See
> [spec/regulatory.md](spec/regulatory.md).

## Contents

- [contact-relationship-management-service-with-rust](contact-relationship-management-service-with-rust/) —
  JSON API in Rust on Loco (Axum, SeaORM, PostgreSQL).
- [contact-relationship-management-matcher-with-rust](contact-relationship-management-matcher-with-rust/) —
  library: the probability that two contact records are the same person.
- [contact-relationship-management-ui-with-svelte](contact-relationship-management-ui-with-svelte/) —
  Svelte client for sales, marketing, and support.
- [spec](spec/index.md) — the cross-cutting specification, the single source
  of truth for both editions.
- [docs](docs/) — tutorials and reference material:
  [the family bridges](docs/tutorial-family-bridges.md) (the project portfolio
  and worker services, signed WPM events, the employment check), and the NHS
  [DTAC 2.0](docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)
  notes.

## Quick start

See [INSTALL.md](INSTALL.md). In short: `cd
contact-relationship-management-ui-with-svelte && pnpm install && pnpm dev`,
then sign in as `example@example.com` for a guided tour on fictitious data, with
no service needed.

## Documents

[INSTALL](INSTALL.md) · [CONTRIBUTING](CONTRIBUTING.md) · [RFC](RFC.md) ·
[NEWS](NEWS.md) · [CHANGELOG](CHANGELOG.md) · [COMPARISONS](COMPARISONS.md) ·
[BENCHMARKS](BENCHMARKS.md) · [GOVERNANCE](GOVERNANCE.md) ·
[MAINTAINERS](MAINTAINERS.md) · [SECURITY](SECURITY.md) ·
[AI_STATEMENT](AI_STATEMENT.md) · [CITATION.cff](CITATION.cff) ·
[LICENSE](LICENSE.md)

## Contact

Joel Parker Henderson <joel@joelparkerhenderson.com>
