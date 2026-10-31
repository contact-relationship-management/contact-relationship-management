# AGENTS.md — Contact Relationship Management

Entry point for AI coding agents at the repository root. Three projects live here; each has its own, longer
`AGENTS.md`. [`llms.txt`](llms.txt) and [`llms.json`](llms.json) list every document.

| Project | Directory | Guide |
| --- | --- | --- |
| Service (Loco REST API, PostgreSQL) | `contact-relationship-management-service-with-rust` | [AGENTS.md](contact-relationship-management-service-with-rust/AGENTS.md) |
| Matcher (pure Rust library) | `contact-relationship-management-matcher-with-rust` | [README.md](contact-relationship-management-matcher-with-rust/README.md) |
| Client UI (SvelteKit, Svelte 5) | `contact-relationship-management-ui-with-svelte` | [AGENTS.md](contact-relationship-management-ui-with-svelte/AGENTS.md) |

Rust 1.99.1 (pinned in `rust-toolchain.toml`), Node 26, pnpm, Podman.

## Works on its own

This repository must build, run and be tested **without** the project-portfolio or workforce-planning repositories.
Nothing may depend on their code or paths. A bridge to them is opt-in, configured by URL, off while unset, and carries
references and counts, never a record ([`docs/tutorial-family-bridges.md`](docs/tutorial-family-bridges.md)). A change
that makes a sibling required is a bug.

## How work is done here

1. **Spec first.** [`spec/`](spec/index.md) is the source of truth; the live task queue is
   [`spec/tasks.md`](spec/tasks.md) (`CRM-T*`, traced to `CRM-D*` and `CRM-R*`). A behavioural change is one commit with
   a spec edit, a code edit and a test edit.
2. **Stage named files**, never `git add -A`, and look at CI after a push.
3. **Say plainly what is not built or not tested.** Every change records what was not done or not verified. Do not
   weaken a guard or a census test to get green; fix the cause or record why.
4. **Security gates are part of the change.** Actions are pinned to a commit SHA and images to a digest
   (`python3 scripts/check-actions-pinned.py`), history is secret-scanned (reviewed findings, with reasons, in
   `.gitleaksignore`), and workflows pass `zizmor`. Do not add a finding to `.gitleaksignore` without reading it.
5. Strings in other languages are machine-written until a speaker reviews them. Never claim a translation is reviewed.

## Gates

`scripts/ci-check.sh <stage>` is the one entry point for every CI check, so GitHub Actions and Woodpecker run the same
commands. The stages are `fmt`, `clippy`, `test`, `test-db`, `front-end`, `e2e`, `site`, `scripts`, `lint`, `mutants`,
`supply-chain`, `semver`, `api-types` and `coverage`.

```bash
scripts/ci-check.sh fmt && scripts/ci-check.sh clippy && scripts/ci-check.sh test
scripts/ci-check.sh lint            # typos, zizmor, rumdl
python3 scripts/check-actions-pinned.py
# database tests need PostgreSQL 18 and Redis 8: podman compose -f contact-relationship-management-service-with-rust/compose.test.yaml up -d
scripts/ci-check.sh test-db
```

A one-command demo (sign-in off, synthetic data only) is `podman compose up --build` ([`compose.yaml`](compose.yaml)).

## Things that are easy to get wrong

- A contact is a person-service record: store the `person:<uuid>` reference, never a copy of a name or address. See
  the scope boundary in [`spec/scope.md`](spec/scope.md).
- Consent comes first: nothing is sent to a contact without recorded consent, and a bridge never grants it.
- Production refuses to start without authentication. Demo settings (`CRM_REQUIRE_AUTH=0`) are for synthetic data only.
- Use Podman in instructions and scripts; CI still builds with Docker.
