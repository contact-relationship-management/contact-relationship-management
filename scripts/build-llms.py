#!/usr/bin/env python3
"""Write llms.txt and llms.json at the repository root from one data table.

Run from anywhere:  python3 scripts/build-llms.py
Both files are generated; edit the table below, not the outputs. Every path
must exist (the script fails if one does not), so the files cannot point at
removed documents.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASE = "https://github.com/contact-relationship-management/contact-relationship-management/blob/main/"

TITLE = "Contact Relationship Management"
SUMMARY = (
    "A demo contact relationship management (CRM) system: sales automation, marketing "
    "automation, customer support and analytics. A Rust JSON API (Loco: Axum, SeaORM, "
    "PostgreSQL) and a Svelte 5 / SvelteKit 3 browser client built on the Lily Design "
    "System, specified in spec/. Demo software: synthetic data only, no real email is "
    "sent by default, no accreditation or certification is claimed."
)
NOTES = [
    "The specification in spec/ is the single source of truth; code and docs follow it.",
    "Requirements are CRM-R<n> (spec/requirements.md); tasks are CRM-T<n> (spec/tasks.md). Every task traces to a requirement.",
    "Be honest about limits: tasks.md records what was verified and what was not. Do not claim more than it does.",
    "Translations (eight UI locales, role briefs in locales/) are machine drafts that no native speaker has reviewed.",
    "Never put real personal data in examples, tests or fixtures; use synthetic data.",
]

# (section, [(title, path, description)])
SECTIONS = [
    ("Start here", [
        ("README", "README.md", "What the project is and how the folders fit together."),
        ("Install and use", "INSTALL.md", "Prerequisites, running the service and the UI, sending email, the guided tour, the whole-repo check."),
        ("Specification index", "spec/index.md", "Entry point to the specification."),
        ("Purpose", "spec/purpose.md", "The problem and what the CRM provides."),
        ("Scope", "spec/scope.md", "What is in and out."),
        ("Regulatory position", "spec/regulatory.md", "Why this is demo software and what that means."),
    ]),
    ("Specification", [
        ("Requirements", "spec/requirements.md", "Numbered requirements (CRM-R1 to CRM-R22) with acceptance criteria."),
        ("Tasks", "spec/tasks.md", "The delivery checklist (CRM-T1 onward) with resolutions, verification and stated limits."),
        ("Architecture", "spec/architecture.md", "Components and data flow."),
        ("Domain model", "spec/domain-model.md", "Entities and relationships, held as relationship wrappers over upstream URNs."),
        ("Design decisions", "spec/design.md", "Numbered design decisions (CRM-D)."),
        ("Authentication and authorisation", "spec/auth.md", "Personas, masking and record-level access."),
        ("Audit", "spec/audit.md", "The audit trail and events."),
        ("Sales automation", "spec/sales-automation.md", "Leads, scoring, pipelines, deals, forecasting."),
        ("Marketing automation", "spec/marketing-automation.md", "Consent, segments, campaigns, nurture."),
        ("Service and support", "spec/service-support.md", "Tickets, SLAs, knowledge base."),
        ("Analytics and reporting", "spec/analytics-reporting.md", "Dashboards and the honesty rules for derived numbers."),
        ("Integrations", "spec/integrations.md", "The upstream services CRM consumes (it holds EntityRef URNs and never duplicates upstream records) and how."),
        ("Testing", "spec/testing.md", "Test strategy."),
        ("Roadmap", "spec/roadmap.md", "What comes beyond the v1 task queue."),
        ("Glossary", "spec/glossary.md", "Terms."),
    ]),
    ("Rust service", [
        ("Service README", "contact-relationship-management-service-with-rust/README.md", "Quick start, configuration, endpoints."),
        ("Service agent guide", "contact-relationship-management-service-with-rust/AGENTS.md", "Working agreements for the service."),
        ("Service changelog", "contact-relationship-management-service-with-rust/CHANGELOG.md", "Changes to the service."),
    ]),
    ("Svelte client", [
        ("UI README", "contact-relationship-management-ui-with-svelte/README.md", "Status, environment variables, stack, tests."),
        ("UI agent guide", "contact-relationship-management-ui-with-svelte/AGENTS.md", "Working agreements for the client (Svelte 5 runes, Lily components, BFF auth, i18n)."),
        ("UI changelog", "contact-relationship-management-ui-with-svelte/CHANGELOG.md", "Changes to the client."),
    ]),
    ("Security and compliance", [
        ("Security policy", "SECURITY.md", "Reporting vulnerabilities."),
        ("Compliance evidence pack", "spec/compliance/index.md", "Control mapping, DPIA template, runbook; not a certification."),
        ("Operations runbook", "spec/compliance/operations-runbook.md", "Hardening, backup, keys, logging, release verification."),
        ("Penetration test scope", "spec/compliance/penetration-test-scope.md", "Brief for an independent tester (no test has been done)."),
        ("Accessibility audit brief", "spec/compliance/accessibility-audit-brief.md", "Test plan for a manual audit (none has been done)."),
        ("First release checklist", "spec/compliance/first-release-checklist.md", "The dry run that proves release signing."),
        ("AI statement", "AI_STATEMENT.md", "How AI was used in this project."),
    ]),
    ("Contributing", [
        ("Contributing", "CONTRIBUTING.md", "How to build, test and propose changes."),
        ("Governance", "GOVERNANCE.md", "A single maintainer; how decisions are made."),
        ("Changelog", "CHANGELOG.md", "Repository-level changes."),
        ("Licence", "LICENSE.md", "Licence terms."),
    ]),
]

OPTIONAL = [
    ("Role briefs and translations", "spec/locale/index.md", "Briefs for executives, security, legal and testers, in many locales (unreviewed)."),
    ("Comparisons", "COMPARISONS.md", "How this project relates to other CRM software (a demonstration, not a competitor)."),
    ("Benchmarks", "BENCHMARKS.md", "No formal benchmarks have been published yet."),
]

COMMANDS = {
    "full check": "scripts/ci-check.sh",
    "service tests": "cd contact-relationship-management-service-with-rust && cargo test && cargo test -- --ignored",
    "ui tests": "cd contact-relationship-management-ui-with-svelte && pnpm install && pnpm test && pnpm exec playwright test",
    "ui live tests (need a running service on :5160)": "pnpm test:live",
    "guided tour (no service needed)": "cd contact-relationship-management-ui-with-svelte && pnpm dev, then sign in as example@example.com",
}


def link(path: str) -> str:
    if not (ROOT / path).exists():
        raise SystemExit(f"missing: {path}")
    return BASE + path


def txt() -> str:
    out = [f"# {TITLE}", "", f"> {SUMMARY}", ""]
    out += [f"- {n}" for n in NOTES] + [""]
    for name, items in SECTIONS:
        out += [f"## {name}", ""]
        out += [f"- [{t}]({link(p)}): {d}" for t, p, d in items] + [""]
    out += ["## Optional", ""]
    out += [f"- [{t}]({link(p)}): {d}" for t, p, d in OPTIONAL]
    out += ["", "## Commands", ""]
    out += [f"- {k}: `{v}`" for k, v in COMMANDS.items()]
    return "\n".join(out) + "\n"


def data() -> dict:
    sec = lambda items: [{"title": t, "url": link(p), "path": p, "description": d} for t, p, d in items]
    return {
        "name": TITLE,
        "summary": SUMMARY,
        "repository": "https://github.com/contact-relationship-management/contact-relationship-management",
        "status": "demo software; synthetic data only",
        "stack": {
            "service": "Rust, Loco (Axum, SeaORM), PostgreSQL",
            "client": "Svelte 5 runes, SvelteKit 3 (SPA mode), Lily Design System",
        },
        "notes": NOTES,
        "sections": [{"name": n, "links": sec(i)} for n, i in SECTIONS],
        "optional": sec(OPTIONAL),
        "commands": COMMANDS,
    }


(ROOT / "llms.txt").write_text(txt(), encoding="utf-8")
(ROOT / "llms.json").write_text(json.dumps(data(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print("wrote llms.txt and llms.json")
