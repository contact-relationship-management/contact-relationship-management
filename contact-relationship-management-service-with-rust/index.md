# Documentation index

Contact Relationship Management — Loco edition. JSON-only back-end
API for sales automation, marketing automation, support, and
analytics.

## Start here

- **[README.md](README.md)** — what this is, quick start, surface,
  configuration.
- **[../INSTALL.md](../INSTALL.md)** — running the service, sending
  email, the bridge settings.
- **[Family bridges tutorial](../docs/tutorial-family-bridges.md)** —
  the PPM and WPM bridges, a signed WPM event with `openssl` and
  `curl`, and the employment check.
- **[../spec/](../spec/index.md)** — the cross-cutting specification
  (single source of truth: domain model, the four modules, auth,
  audit).
- **[spec/](spec/index.md)** — this edition's stack-specific spec.
- **[AGENTS.md](AGENTS.md)** — working agreements for contributors.
- **[CHANGELOG.md](CHANGELOG.md)** — Keep a Changelog format.

## The task queue

Live delivery checklist: [../spec/tasks.md](../spec/tasks.md)
(CRM-T1 to CRM-T162, phased; production gates CRM-G1 and CRM-G2). Most
tasks touch this edition; the latest are the family bridges: CRM-T139
(PPM delivery ideas, WPM teams), CRM-T153 (bridge credentials),
CRM-T158 to CRM-T160 (WPM hire and leave events, their order, the
employment check) and CRM-T161 and CRM-T162 (imported teams and CRM
users named by `person:` reference).
