# Path to production

> Contact Relationship Management is not ready for production. This page says what production means here, and how
> far each part has got.

The software runs on synthetic data. Email is simulated unless a real mail mode is configured. Do not put real
personal data in it.

## What production means

A production deployment holds real people's data and sends real messages to them. For this project that needs all
of the following:

- **Real sign-in.** Every request is authenticated, and access rules match the deploying organization's roles.
- **Lawful sending.** Each message has recorded consent and a lawful basis under the law of every place it is sent
  to, and goes through a real email provider that handles bounces, complaints and unsubscribes.
- **Independent assurance.** People outside the project have tested the security and the accessibility, and the
  findings are fixed or accepted.
- **Operations.** Backups that have been restored, monitoring, an incident process, and someone on call.
- **Evidence of stability.** Long runs under realistic load, on hardware like the deployment's, without errors or
  slowdown.

Production readiness is decided per deployment. A deployer can close some gates only for its own organization and
recipients. The engineering assessment behind this page is [spec/plan.md](../../../spec/plan.md). The open gates are
in [spec/tasks.md](../../../spec/tasks.md).

## Readiness

Each percentage is an estimate made on 2026-11-13 from the task list, not a measurement.

- **Authentication and access control** (70%): Sign-in by PASETO tokens or Keycloak/OIDC, attribute-based access
  rules with record-level checks, and a production mode that refuses to start without authentication are built. No
  real deployment has turned them on with its own attributes, and the enforcement matrix does not yet cover every
  handler (CRM-G1).
- **Email delivery** (40%): A simulated sender is the default, and an SMTP sender exists. There is no email provider
  adapter with bounce and complaint webhooks or a suppression list (CRM-T39).
- **Data protection and e-marketing law** (15%): Consent records, subject access, erasure and retention are built.
  No region's law has been read for this project, and the review of the real send path is open (CRM-G2,
  [regulatory](../../../spec/regulatory.md)).
- **Independent security assurance** (10%): A penetration-test scope is written. No penetration test, formal risk
  analysis or accreditation has been done (CRM-T42). The AI security audits are not independent assurance.
- **Accessibility** (50%): Automated axe-core checks against WCAG 2.2 A and AA pass on every page route. No manual
  audit with keyboard-only use, screen readers or 400% zoom has been done (CRM-T48).
- **Translations** (30%): The interface is available in eight languages. Strings in languages other than English are
  machine-written, and no speaker has reviewed them.
- **Operations** (30%): An operations runbook covers backup and restore, key management, encryption at rest,
  incident response and log retention. No restore has been tested, and the runbook's go-live checklist is open.
- **Performance and stability** (20%): Load tests and a long-running stability test exist. The stability test has
  run for two minutes against the demo stack; no long run and no baseline on production-like hardware exist.
- **Release integrity** (85%): Release images are multi-architecture, signed and attested, and their signatures
  verify. Actions and images are pinned by digest, and history is scanned for secrets. The amd64 half of a pulled
  release image has not been checked, and there is no second code host yet (CRM-T192).
- **Continuous integration** (60%): One script runs every check, on GitHub Actions and Woodpecker. Coverage floors,
  scheduled mutation testing and other gates are still open (CRM-T94).
- **Demo safeguards** (60%): Each tour visitor gets a separate copy of the demo data, capped at 200 copies. The tour
  is still on unless `CRM_DEMO_TOUR=off` is set (CRM-T92).

## How this page is kept

When a gate closes, change the estimate and its evidence in the same commit as the work, and link the task.
