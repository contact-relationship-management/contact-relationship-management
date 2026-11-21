#!/usr/bin/env bash
# One entry point for every CI check, so GitHub Actions and Woodpecker run
# byte-identical commands.   Usage: scripts/ci-check.sh <stage>
#   fmt | clippy | test | test-db | front-end | e2e | site | scripts | lint | pins | secrets | mutants | mutants-diff | bench | supply-chain | semver | api-types | coverage | all | pre-push
# scripts/check-ci-entry-point.py fails the build if a CI step checks anything outside this file.
set -euo pipefail
cd "$(dirname "$0")/.."

SVC=contact-relationship-management-service-with-rust
FE=contact-relationship-management-ui-with-svelte
MATCHER=contact-relationship-management-matcher-with-rust
# The domain rules (CRM-T260), a crate in the service's workspace.
DOMAIN="${SVC}/contact-relationship-management-domain"
SITE=contact-relationship-management.github.io
# `keycloak` gates the optional Keycloak bearer path and the `test-idp`
# dev-dependency feature; without it CI never compiles that code.
FEATURES=(--features keycloak)

# What `all` runs, cheapest first. `mutants` is left out: it takes hours (spec/local-ci-cd.md);
# `bench` too, as a reading of the machine rather than a check.
ALL_STAGES=(fmt lint scripts pins secrets semver clippy test api-types coverage front-end e2e site supply-chain test-db)

stage="${1:?usage: $0 <fmt|clippy|test|test-db|front-end|e2e|site|scripts|lint|pins|secrets|mutants|mutants-diff|bench|supply-chain|semver|api-types|coverage|all|pre-push>}"

case "${stage}" in
  fmt)
    (cd "${SVC}" && cargo fmt --all --check)
    (cd "${MATCHER}" && cargo fmt --check)
    ;;
  clippy)
    (cd "${SVC}" && cargo clippy --workspace --all-targets "${FEATURES[@]}" -- -D warnings)
    # The Keycloak-only build (no PASETO) the manifest documents.
    (cd "${SVC}" && cargo clippy --all-targets --no-default-features --features keycloak -- -D warnings)
    # The domain crate without its `postgres` feature, as its own tests and benchmarks build it.
    (cd "${DOMAIN}" && cargo clippy --all-targets -- -D warnings)
    (cd "${MATCHER}" && cargo clippy --all-targets -- -D warnings)
    ;;
  test)
    (cd "${SVC}" && cargo test --workspace "${FEATURES[@]}")
    # The service depends on the published matcher (the image builds from the
    # service directory alone), so also test it against the matcher in this
    # repository: a matcher change that breaks the service fails here, before
    # the matcher is published. The patch rewrites Cargo.lock, so it is put
    # back afterwards.
    (cd "${SVC}" && cp Cargo.lock Cargo.lock.ci-keep \
      && trap 'mv Cargo.lock.ci-keep Cargo.lock' EXIT \
      && cargo test "${FEATURES[@]}" --lib \
        --config "patch.crates-io.contact-relationship-management-matcher.path=\"../${MATCHER}\"")
    (cd "${MATCHER}" && cargo test)
    ;;
  test-db)
    # Needs a Postgres reachable with the PG_* settings (defaults match
    # the service's compose.test.yaml: podman compose -f compose.test.yaml
    # up -d --wait), and a Redis at REDIS_HOST:REDIS_PORT.
    : "${PG_HOST:=localhost}" "${PG_PORT:=5432}" "${PG_USER:=postgres}" "${PG_PASSWORD:=postgres}"
    db=ci_contact_relationship_management
    export PGPASSWORD="${PG_PASSWORD}"
    dropdb --if-exists -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" "${db}"
    createdb -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" "${db}"
    psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d "${db}" -q -c \
      'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
       CREATE EXTENSION IF NOT EXISTS citext;
       CREATE EXTENSION IF NOT EXISTS unaccent;' || true
    # axa (0.11) reads the database from `DB_URL`; sqlx's query macros and `cargo sqlx` read
    # `DATABASE_URL`. Both name the one database this stage created.
    url="postgres://${PG_USER}:${PG_PASSWORD}@${PG_HOST}:${PG_PORT}/${db}"
    # The query cache a build reads must match the migrated schema and the queries (CRM-D24). A
    # compiler cache such as kache answers without running the macros, so nothing is compared and
    # the check passes: clear it, and treat sqlx's warning about unseen queries as that failure.
    (cd "${SVC}" \
      && DB_URL="${url}" DATABASE_URL="${url}" \
         cargo run "${FEATURES[@]}" -- db migrate)
    prepared="$(cd "${SVC}" \
      && DB_URL="${url}" DATABASE_URL="${url}" \
         RUSTC_WRAPPER='' CARGO_BUILD_RUSTC_WRAPPER='' \
         cargo sqlx prepare --check -- "${FEATURES[@]}" 2>&1)" || { echo "${prepared}"; exit 1; }
    if grep -q 'potentially unused queries' <<<"${prepared}"; then
      echo "${prepared}"
      echo "test-db: the query macros did not run, so the cache was not compared" >&2
      exit 1
    fi
    (cd "${SVC}" \
      && DB_URL="${url}" DATABASE_URL="${url}" \
         cargo test "${FEATURES[@]}" -- --ignored --test-threads=1)
    # The Redis tests skip silently when CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL is unset, so run them
    # on their own, against a real Redis. Not set for the run above: the rate
    # limiter would then share counters across the whole suite.
    : "${REDIS_HOST:=localhost}" "${REDIS_PORT:=6379}"
    (cd "${SVC}" \
      && CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL="redis://${REDIS_HOST}:${REDIS_PORT}" \
         cargo test "${FEATURES[@]}" redis_store -- --ignored --test-threads=1)
    ;;
  front-end)
    # CRM-T234: a dependency both front-end projects use is on one range in both
    # (`.syncpackrc.json`; the root package.json holds only syncpack).
    pnpm install --frozen-lockfile
    pnpm exec syncpack lint
    cd "${FE}"
    pnpm install --frozen-lockfile
    pnpm run check
    # CRM-T274: Vitest enforces the 95% executable-line floor in vite.config.ts.
    pnpm exec vitest run --coverage --coverage.reporter=text-summary
    pnpm run build
    pnpm run lint
    ;;
  api-types)
    # The UI's generated API types (src/lib/api/schema.d.ts) must be what the service's
    # OpenAPI document says today; regenerate with `OPENAPI=... pnpm api:types`.
    doc="$(mktemp)"
    trap 'rm -f "${doc}"' EXIT
    (cd "${SVC}" && cargo run -q --example openapi) > "${doc}"
    cd "${FE}"
    pnpm install --frozen-lockfile
    pnpm exec openapi-typescript "${doc}" -o src/lib/api/schema.d.ts
    git diff --exit-code -- src/lib/api/schema.d.ts \
      || { echo "schema.d.ts is out of date: run pnpm api:types" >&2; exit 1; }
    pnpm run check
    ;;
  coverage)
    # CRM-T273: the independently versioned matcher has its own complete line census.
    (cd "${MATCHER}" \
      && export RUSTC_WRAPPER='' CARGO_BUILD_RUSTC_WRAPPER='' \
      && cargo fetch --locked \
      && cargo llvm-cov clean --workspace \
      && cargo llvm-cov --offline --workspace --all-targets \
        --summary-only --fail-under-lines 95)
    # CRM-T275: one clean profile merges DB-free service-workspace tests, the normal integration
    # tests, PostgreSQL request tests and the Redis adapter tests before enforcing the executable-
    # line floor. This part therefore needs the same services as test-db.
    : "${PG_HOST:=localhost}" "${PG_PORT:=5432}" "${PG_USER:=postgres}" "${PG_PASSWORD:=postgres}"
    db=ci_contact_relationship_management_coverage
    export PGPASSWORD="${PG_PASSWORD}"
    dropdb --if-exists -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" "${db}"
    createdb -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" "${db}"
    psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d "${db}" -q -c \
      'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
       CREATE EXTENSION IF NOT EXISTS citext;
       CREATE EXTENSION IF NOT EXISTS unaccent;' || true
    url="postgres://${PG_USER}:${PG_PASSWORD}@${PG_HOST}:${PG_PORT}/${db}"
    (cd "${SVC}" && DB_URL="${url}" DATABASE_URL="${url}" cargo run "${FEATURES[@]}" -- db migrate)
    : "${REDIS_HOST:=localhost}" "${REDIS_PORT:=6379}"
    if [[ -n "${REDIS_PASSWORD:-}" ]]; then
      redis_url="redis://:${REDIS_PASSWORD}@${REDIS_HOST}:${REDIS_PORT}"
    else
      redis_url="redis://${REDIS_HOST}:${REDIS_PORT}"
    fi
    (cd "${SVC}" \
      && export RUSTC_WRAPPER='' CARGO_BUILD_RUSTC_WRAPPER='' \
      && cargo fetch --locked \
      && cargo llvm-cov clean --workspace \
      && cargo llvm-cov --offline --workspace --lib "${FEATURES[@]}" --summary-only \
      && cargo llvm-cov --no-clean --offline --test main "${FEATURES[@]}" --summary-only \
      && DB_URL="${url}" DATABASE_URL="${url}" \
         cargo llvm-cov --no-clean --offline --test main "${FEATURES[@]}" --summary-only -- \
         --ignored --test-threads=1 \
      && CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL="${redis_url}" \
         cargo llvm-cov --no-clean --offline --workspace --lib "${FEATURES[@]}" \
           --summary-only --fail-under-lines 96.5 -- \
           redis_store --ignored --test-threads=1)
    ;;
  e2e)
    # The Playwright suites (demo-data tour, accessibility, smoke): they build
    # and serve the UI themselves and need no service.
    cd "${FE}"
    pnpm install --frozen-lockfile
    pnpm exec playwright install --with-deps chromium
    pnpm exec playwright test
    ;;
  site)
    # The GitHub Pages site (spec/github-pages-site.md): its copy of the documents is current, then
    # svelte-check, the static build (every page prerendered; a broken link or anchor fails it), and
    # Playwright with axe over the built pages (light and dark, right to left, the locale switch).
    cd "${SITE}"
    node scripts/sync-content.mjs --check
    pnpm install --frozen-lockfile
    pnpm run check
    pnpm run build
    pnpm exec playwright install --with-deps chromium
    CI=true pnpm exec playwright test
    ;;
  scripts)
    # Warnings and errors only; the style and info notes are not gated.
    shellcheck -S warning scripts/*.sh
    scripts/test-pre-push.sh
    scripts/test-machine-idle.sh
    scripts/test-attest.sh
    ;;
  lint)
    # Spelling (`_typos.toml` lists what is not a typo and leaves the translations out), and the
    # security audit of the GitHub workflows: template injection, credentials left in the
    # workspace, caches in jobs that publish. Offline, so it needs no token.
    typos
    zizmor --offline .github
    # Markdown (`.rumdl.toml`: line length and the links to sibling projects are not checked).
    rumdl check .
    # The supplier assurance pack's matrix names real requirements and control-mapping rows (CRM-T276).
    python3 scripts/check-assurance-pack.py
    ;;
  pins)
    # Every action and image is pinned, and every CI step goes through this file.
    python3 scripts/check-actions-pinned.py
    python3 scripts/check-ci-entry-point.py
    python3 scripts/test_check_ci_entry_point.py
    ;;
  secrets)
    # Reviewed findings, with reasons, are in `.gitleaksignore`. Needs the whole history.
    gitleaks detect --source . --redact --no-banner
    ;;
  mutants)
    # CRM-T94: change each pure rule a little and check that a unit test notices. A mutant that
    # survives is a rule no test pins. Scheduled weekly, not on every push: it takes a while.
    # The rules are the domain crate (CRM-T260), whose unit tests need nothing of the service.
    # CRM-T259: the matcher too, whole: it takes minutes, so a sharded run takes it in shard 0 only.
    if [[ -z "${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_SHARD:-}" || "${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_SHARD}" == 0/* ]]; then
      (cd "${MATCHER}" && cargo mutants -j "${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_JOBS:-2}" --no-shuffle --output "${PWD}/mutants-out")
    fi
    (cd "${DOMAIN}" && cargo mutants -j "${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_JOBS:-2}" --no-shuffle --output "${PWD}/mutants-out" ${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_SHARD:+--shard "${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_SHARD}"} "${@:2}")
    ;;
  bench)
    # CRM-T266: the criterion benchmarks, the matcher's and the domain rules'. A reading of this
    # machine for BENCHMARKS.md, not a gate, so no hosted runner's noisy neighbours run it.
    # CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_SAVE=<name> saves the run as a named baseline (in each crate's target/criterion);
    # CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_BASELINE=<name> compares with one saved earlier, and criterion prints each change.
    # Further arguments go to criterion, such as a filter: scripts/ci-check.sh bench '^calendar/'.
    bench_args=()
    if [[ -n "${CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_SAVE:-}" ]]; then bench_args+=(--save-baseline "${CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_SAVE}"); fi
    if [[ -n "${CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_BASELINE:-}" ]]; then bench_args+=(--baseline-lenient "${CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_BASELINE}"); fi
    bench_args+=("${@:2}")
    (cd "${MATCHER}" && cargo bench --bench matching -- ${bench_args[@]+"${bench_args[@]}"})
    (cd "${DOMAIN}" && cargo bench --bench rules -- ${bench_args[@]+"${bench_args[@]}"})
    ;;
  mutants-diff)
    # CRM-T259: only the mutants in lines changed since a base commit (CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_BASE, or the
    # argument, default origin/main), in the matcher and the domain rules, so a rule that no
    # test pins fails the change that adds it rather than the weekly run.
    base="${2:-${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_BASE:-origin/main}}"
    if [[ "${base}" =~ ^0+$ ]] || ! git rev-parse --verify --quiet "${base}^{commit}" >/dev/null; then
      echo "mutants-diff: no base commit (${base}), so nothing to compare; skipped" >&2
      exit 0
    fi
    # cargo-mutants reads the diff's paths from the workspace root, so each crate's diff is relative
    # to its own root: the matcher is one, and the domain crate's is the service's directory. A diff
    # relative to the wrong directory matches no file and passes having tested nothing.
    status=0
    for each in "${MATCHER}:${MATCHER}" "${SVC}:${DOMAIN}"; do
      root="${each%%:*}" crate="${each#*:}"
      diff="$(mktemp)"
      git diff --relative="${root}/" "${base}" -- "${crate}/src" >"${diff}"
      if [[ -s "${diff}" ]]; then
        (cd "${crate}" && cargo mutants -j "${CONTACT_RELATIONSHIP_MANAGEMENT_MUTANTS_JOBS:-2}" --no-shuffle --in-diff "${diff}" --output "${PWD}/mutants-diff-out") || status=$?
      else
        echo "mutants-diff: ${crate}/src is unchanged since ${base}"
      fi
      rm -f "${diff}"
    done
    exit "${status}"
    ;;
  supply-chain)
    # CRM-T34: known-vulnerability, licence, and source gates, plus SBOMs
    # (CycloneDX) written to sbom/ for the release record.
    mkdir -p sbom
    (cd "${SVC}" && cargo audit && cargo deny check advisories bans licenses sources)
    # The matcher is published on its own, so it has its own gate.
    (cd "${MATCHER}" && cargo audit && cargo deny check advisories bans licenses sources)
    (cd "${SVC}" && cargo cyclonedx --format json --spec-version 1.5 --override-filename service.cdx \
      && mv service.cdx.json ../sbom/service.cdx.json)
    (cd "${FE}" && pnpm install --frozen-lockfile && pnpm audit --prod \
      && pnpm sbom --sbom-format cyclonedx --prod > ../sbom/ui.cdx.json)
    ;;
  semver)
    # The matcher is published to crates.io, so a breaking change to its public
    # API must come with a new version. Compares against the last published
    # release; run it before publishing. Needs cargo-semver-checks 0.51 or
    # later to read Rust 1.99's rustdoc output.
    (cd "${MATCHER}" && cargo semver-checks)
    ;;
  pre-push)
    # What scripts/pre-push-gate.sh runs on the commit being pushed. These stages, and Prettier and
    # ESLint from `front-end`, caught most of the failed jobs in CRM-T184; a minute or so when warm.
    for each in fmt lint scripts pins secrets clippy; do
      "${PWD}/scripts/ci-check.sh" "${each}"
    done
    (cd "${SVC}" && cargo test --workspace "${FEATURES[@]}" --lib)
    (cd "${MATCHER}" && cargo test --lib)
    (cd "${FE}" && pnpm install --frozen-lockfile && pnpm run lint)
    ;;
  all)
    # Every stage, in one process each, on after a failure so one run shows every problem. test-db
    # needs PostgreSQL and Redis: see its comment above. A pass on a clean tree is recorded by
    # scripts/attest.sh; a dirty tree or a failed stage records nothing.
    # Vite will not load test files from under .git, so `front-end` would fail after minutes of work.
    if [[ "${PWD}/" == */.git/* ]]; then
      echo "ci-check: run all from a checkout outside .git, not ${PWD}" >&2
      exit 2
    fi
    failed=()
    attest_commit="$("${PWD}/scripts/attest.sh" start)"
    attest_stages="$(mktemp)"
    trap 'rm -f "${attest_stages}"' EXIT
    for each in "${ALL_STAGES[@]}"; do
      started="${SECONDS}"
      code=0
      "${PWD}/scripts/ci-check.sh" "${each}" || code=$?
      if ((code == 0)); then outcome=passed; else outcome=FAILED; failed+=("${each}"); fi
      echo "ci-check: ${each} ${outcome} in $((SECONDS - started))s"
      echo "${each} ${code} $((SECONDS - started))" >>"${attest_stages}"
    done
    if ((${#failed[@]} > 0)); then
      echo "ci-check: failed: ${failed[*]}" >&2
      exit 1
    fi
    echo "ci-check: all ${#ALL_STAGES[@]} stages passed"
    if [[ -n "${attest_commit}" ]]; then
      "${PWD}/scripts/attest.sh" write "${attest_commit}" "${attest_stages}"
    fi
    ;;
  *)
    echo "unknown stage: ${stage}" >&2
    exit 2
    ;;
esac
