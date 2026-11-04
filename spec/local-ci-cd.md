# Local-first CI/CD (CRM, proposed 2026-11-04)

> **Status: proposed, not built.** This file is an assessment and a plan. Nothing in it changes how the build runs today, and every decision it lists is the owner's (CRM-D22). The owner's reason: GitHub-hosted CI is unreliable and costs money, so do more of it locally.

## Advice, in one paragraph

Make **the local machine the place CI runs first and the place its result is recorded**, and make every hosted runner a second opinion rather than the gate. Do that in this order, because each step pays for itself before the next: (1) one entry point for every check, called by both the developer and the workflow, so "it passed locally" and "it passed in CI" mean the same commands; (2) a pre-push gate that runs the fast stages and an on-demand `all` that runs the rest, in Linux containers where the service needs PostgreSQL, Redis or Keycloak; (3) a signed record of what passed for which commit, posted as a commit status to the hosts that accept one, so branch protection can require it without a hosted runner; (4) narrow the hosted workflow to a weekly clean-room run, tags and manual dispatch. Only after those, and only if one laptop proves a bottleneck, add a self-managed runner (GitLab's is the cheapest fit) or a small always-on Linux machine. **Do not make `act` or a Forgejo runner the plan.**

## Where CRM stands

- **The model the other two should copy.** `scripts/ci-check.sh <stage>` is "one entry point for every CI check, so GitHub Actions and Woodpecker run byte-identical commands" (stages: fmt, clippy, test, test-db, front-end, e2e, site, scripts, lint, mutants, supply-chain, semver, api-types, coverage). `.github/workflows/ci.yml` (216 lines) and `.woodpecker.yml` both call it, and `mutants.yml` and `release.yml` are separate.
- **Remotes.** `origin` is GitHub only (HTTPS), so there is no second host yet.
- **Machine.** 16 cores, 128 GB, Apple silicon, with `docker`, `colima`, `podman`, `dagger` and `just` installed.

## Options assessed

| Option | Cost | Fit for CRM | Verdict |
|---|---|---|---|
| **One entry point** (`scripts/ci-check.sh <stage>`, or `just`) that the workflow also calls | Free; a few days | Already proven in CRM (GitHub and Woodpecker call the same commands). The other projects inline their steps | **Adopt first.** Everything below depends on it |
| **Pre-push gate** (a fast stage set) plus `all` on demand | Free | The fast stages are fmt, clippy, lib tests and the doc scripts; minutes on 16 cores | **Adopt** |
| **Local attestation**: a git note or signed file of commit, tree hash, stages, durations, tool versions, host; and a **commit status** posted to the host | Free; needs a token per host | Lets branch protection require "local CI passed" on GitHub, GitLab and Forgejo without a hosted runner. Trust is one maintainer's machine | **Adopt**, with the limits under "Risks" |
| **Parallel database suites**: one throwaway database per test binary, binaries run concurrently | Free | The database binaries run serially today. On 16 cores the 20-odd binaries could overlap, for a several-fold speed-up | **Adopt**, after the entry point |
| **Linux containers locally** (colima or docker, digests pinned as in the workflow) | Free | Already installed. Apple silicon runs `arm64`; CI is `amd64`, so run the few parity-sensitive jobs with `--platform linux/amd64` | **Adopt** |
| **Nightly and weekly jobs by `launchd`** (chaos, mutants, `cargo deny` advisories, gitleaks over history, ZAP) | Free | These are the jobs that cost the most hosted minutes and need no push | **Adopt** |
| **GitLab Runner on this machine** (shell executor) for the GitLab mirror | Free | Registers to GitLab.com; no hosted minutes. Runs only while this machine is on and logged in | **Optional** second opinion |
| **Woodpecker on Codeberg** | Free, by request, "as-is", may break | Needs approval; a different pipeline syntax (CRM already carries a `.woodpecker.yml`) | **Optional**; do not depend on it |
| **Dagger** | Free engine | Installed. One pipeline in code for both worlds, a shared cache. Reports of per-step overhead on a Rust workspace | **Pilot** on one project, measured against plain cargo; do not migrate blind |
| **`act`** (run the GitHub workflow locally) | Free | Needs Docker, images lack GitHub's tooling, service containers and API-calling actions are the weak points, and it would run the workflow YAML rather than the shared entry point | **Do not rely on it**; keep it only for debugging a workflow edit |
| **Forgejo Actions runner** | Free | Needs your own Forgejo (Codeberg does not host Actions); not compatible with GitHub Actions; a macOS runner is built from source | **Skip** unless a Forgejo is hosted anyway |
| **GitHub self-hosted runner** | A per-minute platform charge was announced, then reported postponed | Policy has already moved once; building the plan on it repeats the unreliability the owner is trying to leave | **Do not depend on it** |
| **A small always-on Linux machine** running a runner | A few euros a month | Removes the laptop as a single point of failure and gives a native `amd64` clean room | **Decide later**, on evidence from the first weeks |

## The design: local-first, hosted-as-audit

1. **One entry point per project**, with the stages of the current jobs as names. The workflow's steps become `run: scripts/ci-check.sh <stage>`; the service containers stay in the workflow and in the local helper that starts them. A new check is added once.
2. **Two local gates.** `pre-push` runs the fast stages (format, lint, unit, the doc and link scripts). `all` runs every stage, starts PostgreSQL 18, Redis and Keycloak in containers, and writes the attestation.
3. **The attestation** (`refs/notes/ci`, plus a `local-ci` commit status) names the commit, the tree hash, each stage with its exit code and duration, the toolchain and image digests, and the host. It is written only when every stage passed on a clean tree. A dirty tree is never attested.
4. **Hosted CI narrows** to `workflow_dispatch`, tags, and a weekly schedule. The weekly run is the clean room: a fresh clone building with the documented commands on `amd64`, which a laptop cannot be.
5. **Release stays reproducible.** Build the Linux binaries on the host with the musl target the specs already describe, build and sign the images, and keep the hosted release workflow as the manual fallback.
6. **Speed comes from the machine we have.** A shared `CARGO_TARGET_DIR`, `sccache` or equivalent, `cargo nextest` where it helps, and the database binaries in parallel. Measure first (CRM-T184); do not guess.

## Risks

| Risk | What follows | Mitigation |
|---|---|---|
| One machine is a single point of failure, and a sleeping laptop runs nothing | Nightly jobs miss; a push waits | `launchd` with wake, a status page of the last attestation, and the optional always-on machine |
| macOS is not Linux (case, paths, libc, locale, `arm64`) | A pass here, a failure there | Run the service and database stages in Linux containers; the weekly hosted clean room catches the rest |
| An attestation is only as honest as its host | A self-signed "passed" proves little to a second maintainer | State it plainly; sign with a key held off the repository; require the hosted weekly run for any release |
| Secrets on the developer machine | A leaked token posts false statuses | A token that can only post commit statuses; never reuse the push key |
| Two systems drift | Local green, hosted red | Both call the same entry point; a test fails the build if a workflow step is not a call to it |
| Hosted terms change again | A plan built on a free tier breaks | Nothing in this plan needs a hosted service to be free |

## What to measure first

Before any change beyond the entry point, time every job's commands on this machine, cold and warm, and compare with the hosted runs' durations from the workflow history. The decision to add a runner or a machine rests on those numbers and on how often a hosted run was down or queued (CRM-T184).

## Sources

Read on 2026-11-04. These are search results and vendor pages, not trials: the claims about `act`, Dagger's cache overhead and Codeberg's current terms are **not verified by running them** here, and the last four rows of the options table say what to test first.

- GitHub's pricing announcement: [Update to GitHub Actions pricing](https://github.blog/changelog/2025-12-16-coming-soon-simpler-pricing-and-a-better-experience-for-github-actions/) (a platform charge on self-hosted runners from 1 March 2026) and [GitHub's own summary](https://resources.github.com/actions/2026-pricing-changes-for-github-actions/). A secondary report says the self-hosted charge was postponed: [feedbagel](https://feedbagel.com/post/github-postpones-price-increase-for-self-hosted-runners-in-github-actions). **Re-check GitHub's billing page before relying on either.**
- `act`: [a usage guide listing its limits](https://tessl.io/registry/pantheon-ai/github-actions-toolkit/1.0.1/files/validator/references/act_usage.md) (needs Docker, not fully compatible, secrets supplied by hand, actions that call GitHub's API can fail, default images lack tools GitHub's runners have).
- Forgejo Actions: [differences from GitHub Actions](https://forgejo.org/docs/next/user/actions/github-actions/) (familiar, not compatible; a smaller default image; some `github` context keys and the `permissions` key are ignored) and [a macOS runner built from source](https://angu-software.codeberg.page/devops/2026/03/01/forging-a-mac-os-compatible-forgejo-actions-runner.html).
- Codeberg: [CI](https://docs.codeberg.org/ci/) (Woodpecker, by request, as-is, reasonable use; Forgejo Actions needs your own runner).
- GitLab: [Install GitLab Runner on macOS](https://docs.gitlab.com/runner/install/osx/) (a self-managed runner registers to GitLab.com; the shell executor is the macOS recommendation; the service runs while a user is logged in).
- Dagger: [the engine](https://dagger.io/dagger-engine) (the same pipeline code locally and in CI; a shared layer cache).
- Woodpecker: [local execution](https://woodpecker-ci.org/docs/usage/local-execution) (`woodpecker-cli exec`; pick the Docker backend explicitly; secrets by flag).

## Decision

CRM-D22 (proposed, 2026-11-04). Run CI locally first and record the result as an attestation and a commit status; narrow hosted CI to a weekly clean room, tags and manual dispatch.
