# Security Policy

This is the security policy of the Contact Relationship Management project: how to report a
vulnerability, what the project does with a report, and when a fix and an advisory are published.
It is kept to by the sole maintainer ([MAINTAINERS.md](MAINTAINERS.md)). The response times below
are targets, not contractual commitments. This policy is not legal advice and has not been reviewed
by a lawyer.

A machine-readable summary is published at
<https://contact-relationship-management.github.io/.well-known/security.txt> (RFC 9116).

## Status of the software

This software is not ready for production ([path to production](locales/en-us/path-to-production/index.md)).
It holds synthetic data only, and it sends no real email unless a real mail mode is configured
(see [INSTALL.md](INSTALL.md#sending-email)). It is not intended for production exposure until the
production gates in [spec/tasks.md](spec/tasks.md) (CRM-G1, authentication activation; CRM-G2,
GDPR/PECR review) are closed. No independent penetration test has been done (CRM-T42); see the
[brief for an independent security tester](locales/en-us/for/independent-security-tester/index.md).

A vulnerability is still worth reporting now: the policy applies to the code as it stands.

## Scope

In scope:

- The Rust service (`contact-relationship-management-service-with-rust`) and the matcher library
  (`contact-relationship-management-matcher-with-rust`).
- The Svelte UI (`contact-relationship-management-ui-with-svelte`), including its server-side proxy.
- The project website (`contact-relationship-management.github.io`).
- The container images published from this repository, the release workflow and its signatures and
  provenance, and the other GitHub Actions workflows.
- The default configuration, the demo `compose.yaml`, and the installation instructions, where
  following them as written leaves a deployment exposed.

Out of scope:

- A deployment run by someone else. Report it to whoever runs it.
- A vulnerability in a dependency that this project does not make worse. Report it upstream; a
  report here is still welcome when this project uses the dependency in an exposed way.
- The demo settings (`CRM_REQUIRE_AUTH=0`), which are for synthetic data only and are documented as
  open, unless they can be turned on by accident or reached from a production configuration.
- Volume denial of service, social engineering, physical access, and findings that need a
  compromised maintainer account or machine.
- Missing hardening with no demonstrated effect, such as a header the project has chosen not to
  send, unless it leads to an attack.

## Reporting a vulnerability

Report privately, by email to <joel@joelparkerhenderson.com>. When GitHub private vulnerability
reporting is enabled for the repository, its "Report a vulnerability" form on the Security tab
works too. Do not open a public issue, discussion or pull request for a vulnerability.

Include:

- what the vulnerability is, and what an attacker gains;
- the affected component and the version or commit;
- the steps to reproduce it, with a proof of concept if there is one;
- whether it is known to anyone else, or is being exploited;
- how you would like to be credited, if at all.

Use synthetic data in a report. The project has no encryption key for reports; if the details are
sensitive, say so in a first email without them, and a way to send them will be agreed.

## What happens after a report

| Step | Target |
| --- | --- |
| Acknowledgement of the report | 7 days |
| Triage: confirmed or not, with a severity | 14 days |
| A fix or mitigation for a critical issue | 14 days after triage |
| A fix or mitigation for a high issue | 30 days after triage |
| A fix or mitigation for a medium issue | 90 days after triage |
| A fix for a low issue | the next release |

The reporter is kept informed at each step, and told if a target will be missed and why. A report
that is not accepted is answered with the reason.

## Severity

Severity is rated with the Common Vulnerability Scoring System, version 4.0 (CVSS v4.0), base
score, for the software as documented, with its production settings:

| Rating | CVSS v4.0 base score |
| --- | --- |
| Critical | 9.0 to 10.0 |
| High | 7.0 to 8.9 |
| Medium | 4.0 to 6.9 |
| Low | 0.1 to 3.9 |

A finding that exposes personal data or consent records, defeats authentication or authorization,
breaks the audit chain, or sends mail to a contact without recorded consent is rated at least high,
whatever its score. These are the controls the project's
[risk management](spec/compliance/risk-management.md) depends on.

## Coordinated disclosure

- A vulnerability is disclosed when a fix is released, or 90 days after the report, whichever comes
  first. The date can be moved by agreement with the reporter.
- A vulnerability being exploited is disclosed, with any mitigation, within 7 days of the project
  learning of it.
- Each fixed vulnerability gets a GitHub security advisory on this repository, with a CVE requested
  through GitHub, and an entry in [CHANGELOG.md](CHANGELOG.md) and the component's changelog. The
  advisory names the affected and fixed versions, and credits the reporter if they want it.
- The reporter is asked to keep the details private until the advisory is published.

## Supported versions

Only the latest commit on `main` is supported. A fix lands on `main` first and is released in the
next tag; earlier releases are not patched. The latest release, v0.1.5, predates the move to axa
(CRM-T228), so it differs from `main` in its configuration and probes.

## Safe harbour

The maintainer will not pursue or support legal action against research done in good faith under
this policy: research that stays in scope, uses only accounts and data of your own or synthetic data,
does not degrade a service for others, stops and reports as soon as it reaches data that is not
yours, and keeps the details private as described above. This applies to the maintainer's own
rights only. It cannot bind anyone else who runs this software, and it does not authorize testing a
deployment you do not run.

## How the project keeps itself secure

- Dependencies are pinned by lockfile (`Cargo.lock`, `pnpm-lock.yaml`), and new releases wait out a
  cooldown before they are taken. `scripts/ci-check.sh supply-chain` checks known vulnerabilities,
  licences and sources, and writes the SBOMs to `sbom/`.
- GitHub Actions are pinned to a commit and images to a digest (`scripts/check-actions-pinned.py`),
  workflows pass `zizmor`, and history is scanned for secrets (`.gitleaksignore` holds the reviewed
  findings).
- Release images are built on GitHub's runners, signed, and carry build provenance
  ([operations runbook](spec/compliance/operations-runbook.md)).
- The Rust service and its migration crate forbid `unsafe` code (`#![forbid(unsafe_code)]`), and the
  service has no JWT code, because tokens in this family are PASETO.
- Production refuses to start without authentication.

## Inbound requests that are not signed in

These routes are public even with `CRM_REQUIRE_AUTH` on, and each checks a secret of its own:

- `POST /webhooks/email` (`CRM_WEBHOOK_SECRET`; `X-CRM-Timestamp` and `X-CRM-Signature`), and the
  provider callbacks under `/webhooks/email/` (see [INSTALL.md](INSTALL.md#sending-email) for how
  each is checked);
- `POST /api/bridges/wpm/events` (`CRM_WPM_WEBHOOK_SECRET`; `x-wpm-signature`, HMAC-SHA256 over
  `<t>.<body>`, five-minute window), see the [family bridges tutorial](docs/tutorial-family-bridges.md).

`POST /webhooks/email` and `POST /api/bridges/wpm/events` answer `404` while their secret is unset.
