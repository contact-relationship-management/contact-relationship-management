#!/usr/bin/env bash
# Chaos test (CRM-R25): break the service's dependencies and the service itself
# while it works, then check that it recovers and that no data is left half
# changed.   Usage: scripts/chaos.sh [path/to/crm-service-binary]
#
# Self-contained: starts its own PostgreSQL and Redis containers (ports 5439
# and 6389, names crm-chaos-*), its own database and the service on port 5180,
# and removes them on exit. Needs podman, curl and the debug or release binary
# (default: the service's target/debug build). Not part of `ci-check.sh`: it is
# slow and needs a container runtime, so run it on purpose.
set -uo pipefail
cd "$(dirname "$0")/.."

SVC_DIR="$(pwd)/contact-relationship-management-service-with-rust"
BIN="$(cd "$(dirname "${1:-${SVC_DIR}/target/debug/contact-relationship-management}")" && pwd)/$(basename "${1:-${SVC_DIR}/target/debug/contact-relationship-management}")"
PG=crm-chaos-pg
RD=crm-chaos-redis
PG_PORT=15439
RD_PORT=16379
PORT=5180
API="http://localhost:${PORT}"
DB_URL="postgres://loco:loco@localhost:${PG_PORT}/chaos"
LOG="$(mktemp -t crm-chaos.XXXXXX)"
SVC_PID=""
FAILS=0
PASSES=0

say()  { printf '\n== %s\n' "$*"; }
pass() { PASSES=$((PASSES + 1)); printf '  ok    %s\n' "$*"; }
fail() { FAILS=$((FAILS + 1)); printf '  FAIL  %s\n' "$*"; }
check() { local what="$1"; shift; if "$@"; then pass "${what}"; else fail "${what}"; fi; }

sql() { podman exec "${PG}" psql -U loco -d chaos -Atc "$1"; }

cleanup() {
  [[ -n "${SVC_PID}" ]] && kill -9 "${SVC_PID}" 2>/dev/null
  podman rm -f "${PG}" "${RD}" >/dev/null 2>&1
  rm -f "${LOG}"
}
trap cleanup EXIT

# status code and seconds, e.g. "200 0.012"
req() { # METHOD PATH [JSON]
  local method="$1" path="$2" body="${3:-}"
  if [[ -n "${body}" ]]; then
    curl -s -o /dev/null --max-time 20 -w '%{http_code} %{time_total}' -X "${method}" \
      -H 'content-type: application/json' -d "${body}" "${API}${path}"
  else
    curl -s -o /dev/null --max-time 20 -w '%{http_code} %{time_total}' -X "${method}" "${API}${path}"
  fi
}
code() { req "$@" | cut -d' ' -f1; }
json() { curl -s --max-time 20 -X POST -H 'content-type: application/json' -d "$2" "${API}$1"; }
pid_of() { sed -n 's/.*"pid":"\([^"]*\)".*/\1/p' | head -1; }
person() { printf 'person:%s' "$(uuidgen | tr 'A-Z' 'a-z')"; }
new_contact() { json /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"$1\"}" | pid_of; }

start_service() {
  DATABASE_URL="${DB_URL}" CRM_REDIS_URL="redis://localhost:${RD_PORT}" \
    nohup bash -c "cd '${SVC_DIR}' && exec '${BIN}' start --server-and-worker --port ${PORT}" >>"${LOG}" 2>&1 &
  SVC_PID=$!
}
wait_for() { # seconds, then a command
  local n="$1"; shift
  for _ in $(seq 1 "$((n * 2))"); do "$@" >/dev/null 2>&1 && return 0; sleep 0.5; done
  return 1
}
up()        { [[ "$(code GET /_ping)" == 200 ]]; }
serves()    { [[ "$(code GET /api/contacts)" == 200 ]]; }
refused()   { local c; c="$(code "$@")"; [[ "${c}" -ge 500 || "${c}" == 000 ]]; }

# Merge invariants: nothing left half merged, whatever happened.
invariants() {
  local bad
  bad="$(sql "SELECT count(*) FROM contacts WHERE deleted_at IS NULL AND merged_into_pid IS NOT NULL")"
  check "no live contact is marked merged" test "${bad}" = 0
  bad="$(sql "SELECT count(*) FROM contact_merges m JOIN contacts c ON c.pid = m.merged_pid
              WHERE c.deleted_at IS NULL OR c.merged_into_pid IS DISTINCT FROM m.survivor_pid")"
  check "every ledger row has a closed merged contact pointing at its survivor" test "${bad}" = 0
  bad="$(sql "SELECT (SELECT count(*) FROM activities a JOIN contact_merges m ON a.subject_pid = m.merged_pid WHERE a.subject_kind = 'contact')
                   + (SELECT count(*) FROM deals d JOIN contact_merges m ON d.primary_contact_pid = m.merged_pid)
                   + (SELECT count(*) FROM consent_events e JOIN contact_merges m ON e.contact_pid = m.merged_pid)")"
  check "nothing is still keyed to a merged-away contact" test "${bad}" = 0
  bad="$(sql "SELECT count(*) FROM contacts c WHERE c.deleted_at IS NOT NULL AND c.merged_into_pid IS NOT NULL
              AND NOT EXISTS (SELECT 1 FROM contact_merges m WHERE m.merged_pid = c.pid)")"
  check "every merged-away contact has a ledger row" test "${bad}" = 0
  check "the audit chain verifies" bash -c "curl -s --max-time 20 ${API}/api/audits/verify | grep -q '\"ok\":true'"
}

say "setup"
command -v podman >/dev/null || { echo "podman is required"; exit 2; }
[[ -x "${BIN}" ]] || { echo "no binary at ${BIN} (cargo build first)"; exit 2; }
podman rm -f "${PG}" "${RD}" >/dev/null 2>&1
podman run -d --name "${PG}" -p "${PG_PORT}:5432" -e POSTGRES_USER=loco -e POSTGRES_PASSWORD=loco \
  -e POSTGRES_DB=chaos docker.io/library/postgres:18-alpine >/dev/null
podman run -d --name "${RD}" -p "${RD_PORT}:6379" docker.io/library/redis:8-alpine >/dev/null
wait_for 40 podman exec "${PG}" pg_isready -U loco -d chaos || { echo "postgres did not start"; exit 2; }
sleep 2
( cd "${SVC_DIR}" && DATABASE_URL="${DB_URL}" CRM_REDIS_URL="redis://localhost:${RD_PORT}" "${BIN}" db migrate ) >>"${LOG}" 2>&1 \
  || { echo "migration failed"; tail -20 "${LOG}"; exit 2; }
start_service
wait_for 40 up || { echo "service did not start"; tail -20 "${LOG}"; exit 2; }
check "baseline: the service serves requests" serves

say "Redis stopped"
podman stop -t 0 "${RD}" >/dev/null
read -r c t <<<"$(req GET /api/contacts)"
printf '  (with Redis down, GET /api/contacts answered %s in %ss)\n' "${c}" "${t}"
check "answers within 15 seconds" awk -v t="${t}" 'BEGIN { exit !(t < 15) }'
read -r c t <<<"$(req POST /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"During Redis outage\"}")"
printf '  (a write answered %s in %ss)\n' "${c}" "${t}"
check "a write answers within 15 seconds" awk -v t="${t}" 'BEGIN { exit !(t < 15) }'
read -r c t <<<"$(req GET /api/insights/dpo)"
printf '  (an insights read answered %s in %ss)\n' "${c}" "${t}"
check "an insights read answers within 15 seconds" awk -v t="${t}" 'BEGIN { exit !(t < 15) }'
podman start "${RD}" >/dev/null
check "recovers after Redis returns" wait_for 40 serves

say "Redis frozen (paused, not stopped)"
podman pause "${RD}" >/dev/null
read -r c t <<<"$(req POST /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"During Redis freeze\"}")"
printf '  (a write answered %s in %ss)\n' "${c}" "${t}"
check "a write answers within 15 seconds" awk -v t="${t}" 'BEGIN { exit !(t < 15) }'
check "and the write succeeded (Redis is an optimisation, not a dependency of writes)" test "${c}" = 200
podman unpause "${RD}" >/dev/null
check "recovers after Redis thaws" wait_for 40 serves

say "PostgreSQL stopped"
podman stop -t 0 "${PG}" >/dev/null
read -r c t <<<"$(req GET /api/contacts)"
printf '  (with PostgreSQL down, a read answered %s in %ss)\n' "${c}" "${t}"
check "a read fails fast, as an error (not a success, not a hang)" bash -c "[[ ${c} -ge 500 || ${c} == 000 ]] && awk -v t=${t} 'BEGIN { exit !(t < 15) }'"
read -r c t <<<"$(req POST /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"During outage\"}")"
check "a write is refused, not pretended" bash -c "[[ ${c} -ge 500 || ${c} == 000 ]]"
check "the process is still running" kill -0 "${SVC_PID}"
podman start "${PG}" >/dev/null
wait_for 40 podman exec "${PG}" pg_isready -U loco -d chaos
check "recovers after PostgreSQL returns, without a restart" wait_for 60 serves
check "and accepts writes again" test -n "$(new_contact 'After outage')"

say "PostgreSQL frozen (paused, not stopped)"
podman pause "${PG}" >/dev/null
read -r c t <<<"$(req GET /api/contacts)"
printf '  (with PostgreSQL frozen, a read answered %s in %ss)\n' "${c}" "${t}"
check "a read gives up in bounded time" awk -v t="${t}" 'BEGIN { exit !(t < 20) }'
podman unpause "${PG}" >/dev/null
check "recovers after PostgreSQL thaws" wait_for 60 serves

say "two merges race for the same contact"
a="$(new_contact 'Race A')"; b="$(new_contact 'Race B')"; c2="$(new_contact 'Race C')"
codes="$(mktemp)"
racers=()
for i in 1 2 3 4; do
  ( code POST "/api/contacts/${a}/merge" "{\"into_pid\":\"${b}\",\"reason\":\"race ${i}\"}" >>"${codes}" ) &
  racers+=($!)
  ( code POST "/api/contacts/${a}/merge" "{\"into_pid\":\"${c2}\",\"reason\":\"race ${i}\"}" >>"${codes}" ) &
  racers+=($!)
done
wait "${racers[@]}"
ok="$(grep -c '^200$' "${codes}")"
printf '  (statuses: %s)\n' "$(sort "${codes}" | uniq -c | tr -s ' ' | tr '\n' ' ')"
check "exactly one of the eight merges succeeded" test "${ok}" = 1
rm -f "${codes}"
invariants

say "the service is killed (SIGKILL) in the middle of a burst of merges"
pairs="$(mktemp)"
for i in $(seq 1 150); do
  s="$(new_contact "Kill S${i}")"; m="$(new_contact "Kill M${i}")"
  json /api/activities "{\"subject_kind\":\"contact\",\"subject_pid\":\"${m}\",\"kind\":\"note\",\"summary\":\"n${i}\"}" >/dev/null
  echo "${m} ${s}" >>"${pairs}"
done
( while read -r m s; do code POST "/api/contacts/${m}/merge" "{\"into_pid\":\"${s}\",\"reason\":\"burst\"}" >/dev/null; done <"${pairs}" ) &
BURST=$!
sleep 0.5
kill -9 "${SVC_PID}"; SVC_PID=""
wait "${BURST}" 2>/dev/null
start_service
check "restarts and serves again" wait_for 60 serves
merged="$(sql "SELECT count(*) FROM contact_merges")"
printf '  (%s of the 150 burst merges had committed when the process died)\n' "$((merged - 1))"
if [[ "$((merged - 1))" -ge 150 || "$((merged - 1))" -le 0 ]]; then
  printf '  note: the kill did not land inside the burst; rerun for a meaningful result\n'
fi
invariants
rm -f "${pairs}"

say "result"
printf '  %s passed, %s failed\n' "${PASSES}" "${FAILS}"
if [[ "${FAILS}" -gt 0 ]]; then echo "service log tail:"; tail -15 "${LOG}"; exit 1; fi
