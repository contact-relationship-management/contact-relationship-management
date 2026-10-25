# Changelog

All notable changes to this repository are summarized here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Each
subproject keeps a detailed changelog:

- [Rust service](contact-relationship-management-service-with-rust/CHANGELOG.md)
- [Svelte front end](contact-relationship-management-ui-with-svelte/CHANGELOG.md)

## [Unreleased]

### Added

- File attachments on contacts, accounts, leads and deals: an allow-list of types checked against the file's start, a size limit, an optional malware-scan command, integrity-checked downloads that are always attachments, subject access and erasure (CRM-R42).
- Typed custom fields on contacts, accounts, leads and deals, with a sensitivity that decides masking, sealing, subject access and erasure (CRM-R41).
- Validation errors name the field in the contact and account edit forms; the executive pack and pipeline hygiene read less from the database; the sign-in tests share a process with the others (CRM-T78 to CRM-T80). The matcher is 0.2.0 in the tree, not published (CRM-T87).
- Explainable workflow rules: a trigger, conditions in the segment filter language and a closed list of actions, with a dry run, a loop guard, consent re-checked inside enrolment, and a run log that says why each run happened (CRM-R40).
- `PUT /api/accounts/{pid}` to edit an account after creation (audited by field name), with an edit form on the account page (CRM-R39).
- A contact's optional `given_name`, and `PUT /api/contacts/{pid}` to edit a contact after creation (audited by field name), with an edit form on the contact page (CRM-R38).
- Email templates with merge tokens, a live preview for a made-up contact, and the unsubscribe link required in every body; campaigns and nurture steps can use one (CRM-R36).

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
