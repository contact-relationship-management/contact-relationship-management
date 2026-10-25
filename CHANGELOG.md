# Changelog

All notable changes to this repository are summarized here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Each
subproject keeps a detailed changelog:

- [Rust service](contact-relationship-management-service-with-rust/CHANGELOG.md)
- [Svelte front end](contact-relationship-management-ui-with-svelte/CHANGELOG.md)

## [Unreleased]

### Added

- Saved views of the five main lists, per user or shared, in a view bar above each (CRM-R35).

- CSV import of leads with a per-row report, duplicate advice and undo (CRM-R34).

- Server-side search across the main records: accent-blind, typo-tolerant, respecting what the caller may read (CRM-R33).

- Task reminders and a notifications inbox: assigned tasks and SLA breaches notify the assignee once, only for records they may
  read (CRM-R32).

- Web form lead capture (`POST /forms/{slug}`): a sign-up form on a website makes a lead and asks the visitor to confirm
  before anything is mailed; origin check, rate limits and a honeypot protect it (CRM-R31).

- Production-readiness assessment ([spec/plan.md](spec/plan.md)), compliance
  evidence pack ([spec/compliance](spec/compliance/index.md)), tamper-evident
  audit chain, fail-closed production boot, security headers and CSP,
  supply-chain CI stage with SBOMs, hardened Containerfiles, and an axe
  accessibility gate.

- Special files for a public repository: README, LICENSE, CITATION.cff,
  CODEOWNERS, MAINTAINERS, GOVERNANCE, SECURITY, CONTRIBUTING, RFC, INSTALL,
  NEWS, COMPARISONS, BENCHMARKS, AI_STATEMENT.
- Standalone CI for GitHub Actions and Woodpecker.

### Changed

- The repository is self-contained: shared crates come from crates.io or a
  pinned git revision instead of monorepo paths.
- Front-end npm dependencies updated to latest.
