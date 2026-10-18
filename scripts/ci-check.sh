#!/usr/bin/env bash
# One entry point for every CI check, so GitHub Actions and Woodpecker run
# byte-identical commands.   Usage: scripts/ci-check.sh <stage>
#   fmt | clippy | test | test-db | front-end | supply-chain
set -euo pipefail
cd "$(dirname "$0")/.."

SVC=contact-relationship-management-service-with-rust
FE=contact-relationship-management-ui-with-svelte
# `keycloak` gates the optional Keycloak bearer path and the `test-idp`
# dev-dependency feature; without it CI never compiles that code.
FEATURES=(--features keycloak)

stage="${1:?usage: $0 <fmt|clippy|test|test-db|front-end|supply-chain>}"

case "${stage}" in
  fmt)
    (cd "${SVC}" && cargo fmt --check)
    (cd "${SVC}/migration" && cargo fmt --check)
    ;;
  clippy)
    (cd "${SVC}" && cargo clippy --all-targets "${FEATURES[@]}" -- -D warnings)
    (cd "${SVC}/migration" && cargo clippy --all-targets -- -D warnings)
    ;;
  test)
    (cd "${SVC}" && cargo test "${FEATURES[@]}")
    (cd "${SVC}/migration" && cargo test)
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
    ;;
  front-end)
    cd "${FE}"
    pnpm install --frozen-lockfile
    pnpm run check
    pnpm exec vitest run
    pnpm run build
    pnpm run lint
    ;;
  supply-chain)
    # CRM-T34: known-vulnerability, licence, and source gates, plus SBOMs
    # (CycloneDX) written to sbom/ for the release record.
    mkdir -p sbom
    (cd "${SVC}" && cargo audit && cargo deny check advisories bans licenses sources)
    (cd "${SVC}" && cargo cyclonedx --format json --spec-version 1.5 --override-filename service.cdx \
      && mv service.cdx.json ../sbom/service.cdx.json)
    # The migration crate is a path dependency, already covered by the
    # service SBOM; remove the duplicate file cargo-cyclonedx leaves there.
    rm -f "${SVC}/migration/service.cdx.json"
    (cd "${FE}" && pnpm install --frozen-lockfile && pnpm audit --prod \
      && pnpm sbom --sbom-format cyclonedx --prod > ../sbom/ui.cdx.json)
    ;;
  *)
    echo "unknown stage: ${stage}" >&2
    exit 2
    ;;
esac
