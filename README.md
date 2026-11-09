# Contact Relationship Management

A central hub for managing interactions with customers and prospects across
the relationship lifecycle: sales automation, marketing automation, customer
support, and analytics.

> ⚠️ **Demo software.** Not a production CRM. No real email is sent unless
> a real mail mode is configured, and the data is synthetic. See
> [spec/regulatory.md](spec/regulatory.md).

## Contents

- [contact-relationship-management-service-with-rust](contact-relationship-management-service-with-rust/) —
  JSON API in Rust on Axum and axa (PostgreSQL through sqlx and SeaORM).
- [contact-relationship-management-matcher-with-rust](contact-relationship-management-matcher-with-rust/) —
  library: the probability that two contact records are the same person.
- [contact-relationship-management-ui-with-svelte](contact-relationship-management-ui-with-svelte/) —
  Svelte client for sales, marketing, and support.
- [spec](spec/index.md) — the cross-cutting specification, the single source
  of truth for both editions.
- [contact-relationship-management.github.io](contact-relationship-management.github.io/README.md) —
  the documentation site for GitHub Pages (static SvelteKit), published at
  <https://contact-relationship-management.github.io/>.
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

## GitHub Pages site

[contact-relationship-management.github.io/](contact-relationship-management.github.io/README.md)
is a static site of this README, INSTALL, the family-bridges tutorial, the
specification index, the locale status and the role briefs in all 11 locales
(Arabic right to left), with a landing page, a tutorial and comparison pages,
published at <https://contact-relationship-management.github.io/>. It is built and tested here
(`scripts/ci-check.sh site`) and exported to the read-only repository
`contact-relationship-management/contact-relationship-management.github.io`,
which deploys it with GitHub Actions.

To publish a change, from a clone with the `github-pages` remote:

1. After editing any document the site shows, run `pnpm run sync` in the site's
   directory (CI fails on a stale copy).
2. Commit, then run `make github-pages` from the repository root. That runs
   `bin/make-github-pages`, which checks the remote, the site's copy of the
   documents and a clean directory, then runs
   `git subtree push --prefix=contact-relationship-management.github.io github-pages main`.
3. Watch the deploy run in the exported repository.

The remote lives in each clone, so a new clone adds it once:
`git remote add github-pages git@github.com:contact-relationship-management/contact-relationship-management.github.io.git`

Never work in the exported repository. See
[spec/github-pages-site.md](spec/github-pages-site.md).

## Works on its own

This repository builds, runs and is tested **without** the project-portfolio (PPM) or workforce-planning (WPM)
repositories: nothing here depends on their code. The bridges are opt-in and off until configured
(`CRM_PPM_SERVICE_URL`, `CRM_WORKER_SERVICE_URL`, `CRM_WPM_WEBHOOK_SECRET`; see [INSTALL.md](INSTALL.md)); the inbound
webhook answers `404` while its secret is unset. A bridge carries references and counts, never a record.
Walk-through: [docs/tutorial-family-bridges.md](docs/tutorial-family-bridges.md).

## Documents

[INSTALL](INSTALL.md) · [CONTRIBUTING](CONTRIBUTING.md) · [RFC](RFC.md) ·
[NEWS](NEWS.md) · [CHANGELOG](CHANGELOG.md) · [COMPARISONS](COMPARISONS.md) ·
[BENCHMARKS](BENCHMARKS.md) · [GOVERNANCE](GOVERNANCE.md) ·
[MAINTAINERS](MAINTAINERS.md) · [SECURITY](SECURITY.md) ·
[AI_STATEMENT](AI_STATEMENT.md) · [CITATION.cff](CITATION.cff) ·
[LICENSE](LICENSE.md)

## Contact

Joel Parker Henderson <joel@joelparkerhenderson.com>
