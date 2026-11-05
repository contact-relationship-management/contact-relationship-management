#!/usr/bin/env bash
# PostgreSQL 18 and Redis for `scripts/ci-check.sh test-db` and `all` on this machine (CRM-T186).
#
#   scripts/ci-services.sh run all            # start them, run the stages, always stop them
#   scripts/ci-services.sh run test-db
#   CRM_CI_PLATFORM=linux/amd64 scripts/ci-services.sh run test-db   # as CI's architecture
#
# The images and health checks are the ones in compose.test.yaml, pinned by digest. Podman is used
# when it answers, and docker otherwise. Ports come from the system, so this cannot land on another
# project's database. Keycloak is not started: the tests use an in-process identity provider.
set -euo pipefail
cd "$(dirname "$0")/.."

compose_file=contact-relationship-management-service-with-rust/compose.test.yaml
project="crm-ci-$$"

free_port() {
  python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}

find_engine() {
  if podman info >/dev/null 2>&1; then
    engine=(podman compose)
  elif docker info >/dev/null 2>&1; then
    engine=(docker compose)
  else
    echo "ci-services: neither podman nor docker answers; start one (podman machine start, or colima start)" >&2
    exit 1
  fi
}

compose() { "${engine[@]}" -p "${project}" -f "${compose_file}" "$@"; }

stop_services() {
  compose down --volumes --remove-orphans >/dev/null 2>&1 || echo "ci-services: could not stop ${project}" >&2
}

usage() {
  echo "usage: $0 run <stage>..." >&2
  exit 2
}

[[ "${1:-}" == run && $# -ge 2 ]] || usage
shift

find_engine
TEST_DB_PORT="$(free_port)"
TEST_REDIS_PORT="$(free_port)"
export TEST_DB_PORT TEST_REDIS_PORT
if [[ -n "${CRM_CI_PLATFORM:-}" ]]; then
  export DOCKER_DEFAULT_PLATFORM="${CRM_CI_PLATFORM}"
fi

trap stop_services EXIT
compose up --detach --wait >&2

export PG_HOST=localhost PG_PORT="${TEST_DB_PORT}" PG_USER=loco PG_PASSWORD=loco
export REDIS_HOST=localhost REDIS_PORT="${TEST_REDIS_PORT}"

status=0
for stage in "$@"; do
  scripts/ci-check.sh "${stage}" || status=$?
done
exit "${status}"
