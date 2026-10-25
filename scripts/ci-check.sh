#!/usr/bin/env bash
# One entry point for every CI check, so GitHub Actions and Woodpecker run
# byte-identical commands.   Usage: scripts/ci-check.sh <stage>
#   fmt | clippy | test | test-db | front-end | e2e | scripts | lint | mutants | supply-chain | semver | api-types | coverage
set -euo pipefail
cd "$(dirname "$0")/.."

SVC=contact-relationship-management-service-with-rust
FE=contact-relationship-management-ui-with-svelte
MATCHER=contact-relationship-management-matcher-with-rust
# `keycloak` gates the optional Keycloak bearer path and the `test-idp`
# dev-dependency feature; without it CI never compiles that code.
FEATURES=(--features keycloak)

stage="${1:?usage: $0 <fmt|clippy|test|test-db|front-end|e2e|scripts|lint|mutants|supply-chain|semver|api-types|coverage>}"

case "${stage}" in
  fmt)
    (cd "${SVC}" && cargo fmt --check)
    (cd "${SVC}/migration" && cargo fmt --check)
    (cd "${MATCHER}" && cargo fmt --check)
    ;;
  clippy)
    (cd "${SVC}" && cargo clippy --all-targets "${FEATURES[@]}" -- -D warnings)
    # The Keycloak-only build (no PASETO) the manifest documents.
    (cd "${SVC}" && cargo clippy --all-targets --no-default-features --features keycloak -- -D warnings)
    (cd "${SVC}/migration" && cargo clippy --all-targets -- -D warnings)
    (cd "${MATCHER}" && cargo clippy --all-targets -- -D warnings)
    ;;
  test)
    (cd "${SVC}" && cargo test "${FEATURES[@]}")
    # The service depends on the published matcher (the image builds from the
    # service directory alone), so also test it against the matcher in this
    # repository: a matcher change that breaks the service fails here, before
    # the matcher is published. The patch rewrites Cargo.lock, so it is put
    # back afterwards.
    (cd "${SVC}" && cp Cargo.lock Cargo.lock.ci-keep \
      && trap 'mv Cargo.lock.ci-keep Cargo.lock' EXIT \
      && cargo test "${FEATURES[@]}" --lib \
        --config "patch.crates-io.contact-relationship-management-matcher.path=\"../${MATCHER}\"")
    (cd "${SVC}/migration" && cargo test)
    (cd "${MATCHER}" && cargo test)
    ;;
  test-db)
    # Needs a Postgres reachable with the PG_* settings (defaults match
    # compose.test.yaml: scripts/test-db.sh up).
    : "${PG_HOST:=localhost}" "${PG_PORT:=5432}" "${PG_USER:=loco}" "${PG_PASSWORD:=loco}"
    db=ci_contact_relationship_management
    export PGPASSWORD="${PG_PASSWORD}"
    dropdb --if-exists -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" "${db}"
    createdb -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" "${db}"
    psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d "${db}" -q -c \
      'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
       CREATE EXTENSION IF NOT EXISTS citext;
       CREATE EXTENSION IF NOT EXISTS unaccent;' || true
    (cd "${SVC}" \
      && DATABASE_URL="postgres://${PG_USER}:${PG_PASSWORD}@${PG_HOST}:${PG_PORT}/${db}" \
         cargo test "${FEATURES[@]}" -- --ignored --test-threads=1)
    # The Redis tests skip silently when CRM_REDIS_URL is unset, so run them
    # on their own, against a real Redis. Not set for the run above: the rate
    # limiter would then share counters across the whole suite.
    : "${REDIS_HOST:=localhost}" "${REDIS_PORT:=6379}"
    (cd "${SVC}" \
      && CRM_REDIS_URL="redis://${REDIS_HOST}:${REDIS_PORT}" \
         cargo test "${FEATURES[@]}" redis_store -- --ignored --test-threads=1)
    ;;
  front-end)
    cd "${FE}"
    pnpm install --frozen-lockfile
    pnpm run check
    # Coverage is a report for now (CRM-T94); no floor is enforced.
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
    # A report, not a gate: unit tests only (the database suite is in test-db), so the figure
    # understates what the whole suite reaches. Floors come once the numbers have been watched.
    (cd "${SVC}" && cargo llvm-cov --lib "${FEATURES[@]}" --summary-only)
    ;;
  e2e)
    # The Playwright suites (demo-data tour, accessibility, smoke): they build
    # and serve the UI themselves and need no service.
    cd "${FE}"
    pnpm install --frozen-lockfile
    pnpm exec playwright install --with-deps chromium
    pnpm exec playwright test
    ;;
  scripts)
    # Warnings and errors only; the style and info notes are not gated.
    shellcheck -S warning scripts/*.sh
    ;;
  lint)
    # Spelling (`_typos.toml` lists what is not a typo and leaves the translations out), and the
    # security audit of the GitHub workflows: template injection, credentials left in the
    # workspace, caches in jobs that publish. Offline, so it needs no token.
    typos
    zizmor --offline .github
    # Markdown (`.rumdl.toml`: line length and the links to sibling projects are not checked).
    rumdl check .
    ;;
  mutants)
    # CRM-T94: change each pure rule a little and check that a unit test notices. A mutant that
    # survives is a rule no test pins. Scheduled weekly, not on every push: it takes a while.
    # `.cargo/mutants.toml` limits it to src/rules and the unit tests.
    (cd "${SVC}" && cargo mutants -j 2 --no-shuffle --output "${PWD}/mutants-out" "${@:2}")
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
    # The migration crate is a path dependency, already covered by the
    # service SBOM; remove the duplicate file cargo-cyclonedx leaves there.
    rm -f "${SVC}/migration/service.cdx.json"
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
  *)
    echo "unknown stage: ${stage}" >&2
    exit 2
    ;;
esac
