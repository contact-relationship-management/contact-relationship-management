#!/usr/bin/env bash
# Chaos test (CRM-R25): break the service's dependencies and the service itself
# while it works, then check that it recovers and that no data is left half
# changed.   Usage: scripts/chaos.sh [path/to/crm-service-binary]
#
# Self-contained: starts its own PostgreSQL and Redis containers (ports 15439
# and 16379, names crm-chaos-*), a TCP proxy between the service and PostgreSQL
# (scripts/chaos-proxy.py, port 15440), its own database and the service on port
# 5180, and removes them on exit. It breaks: Redis and PostgreSQL (stopped and
# frozen), the network between the service and PostgreSQL (dropped, reset,
# slow), the service itself (killed mid-burst, and killed just after a COMMIT),
# racing merges, and a primary-to-replica failover. Needs podman, curl and the debug or release binary
# (default: the service's target/debug build). Not part of `ci-check.sh`: it is
# slow and needs a container runtime, so run it on purpose.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

SVC_DIR="$(pwd)/contact-relationship-management-service-with-rust"
BIN="$(cd "$(dirname "${1:-${SVC_DIR}/target/debug/contact-relationship-management}")" && pwd)/$(basename "${1:-${SVC_DIR}/target/debug/contact-relationship-management}")"
PG=crm-chaos-pg
RD=crm-chaos-redis
PG_PORT=15439
RD_PORT=16379
PORT=5180
API="http://localhost:${PORT}"
PROXY_PORT=15440
REPLICA=crm-chaos-replica
REPLICA_PORT=15441
NET=crm-chaos-net
PRIMARY="${PG}"                       # the container `sql` talks to; failover changes it
DIRECT_URL="postgres://loco:loco@localhost:${PG_PORT}/chaos"
DB_URL="postgres://loco:loco@localhost:${PROXY_PORT}/chaos"   # the service goes through the proxy
CTL="$(mktemp -d -t crm-chaos-ctl.XXXXXX)"
PROXY_PID=""
LOG="$(mktemp -t crm-chaos.XXXXXX)"
SVC_PID=""
FAILS=0
PASSES=0

say()  { printf '\n== %s\n' "$*"; }
pass() { PASSES=$((PASSES + 1)); printf '  ok    %s\n' "$*"; }
fail() { FAILS=$((FAILS + 1)); printf '  FAIL  %s\n' "$*"; }
check() { local what="$1"; shift; if "$@"; then pass "${what}"; else fail "${what}"; fi; }

sql() { podman exec "${PRIMARY}" psql -U loco -d chaos -Atc "$1"; }

cleanup() {
  [[ -n "${SVC_PID}" ]] && kill -9 "${SVC_PID}" 2>/dev/null
  [[ -n "${PROXY_PID}" ]] && kill -9 "${PROXY_PID}" 2>/dev/null
  podman rm -f "${PG}" "${RD}" "${REPLICA}" >/dev/null 2>&1
  podman network rm "${NET}" >/dev/null 2>&1
  rm -rf "${LOG}" "${CTL}"
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
  # The outbox transport is the one that writes events in the mutation's own transaction.
  CRM_EVENT_TRANSPORT=outbox DATABASE_URL="${DB_URL}" CRM_REDIS_URL="redis://localhost:${RD_PORT}" \
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
command -v podman >/dev/null && command -v python3 >/dev/null || { echo "podman and python3 are required"; exit 2; }
[[ -x "${BIN}" ]] || { echo "no binary at ${BIN} (cargo build first)"; exit 2; }
podman rm -f "${PG}" "${RD}" "${REPLICA}" >/dev/null 2>&1
podman network rm "${NET}" >/dev/null 2>&1
podman network create "${NET}" >/dev/null
podman run -d --name "${PG}" --network "${NET}" -p "${PG_PORT}:5432" -e POSTGRES_USER=loco -e POSTGRES_PASSWORD=loco \
  -e POSTGRES_DB=chaos docker.io/library/postgres:18-alpine >/dev/null
podman run -d --name "${RD}" -p "${RD_PORT}:6379" docker.io/library/redis:8-alpine >/dev/null
wait_for 40 podman exec "${PG}" pg_isready -U loco -d chaos || { echo "postgres did not start"; exit 2; }
sleep 2
echo "${PG_PORT}" >"${CTL}/target"; echo forward >"${CTL}/mode"
python3 "$(dirname "$0")/chaos-proxy.py" "${PROXY_PORT}" "${CTL}" >>"${LOG}" 2>&1 &
PROXY_PID=$!
sleep 1
( cd "${SVC_DIR}" && DATABASE_URL="${DIRECT_URL}" CRM_REDIS_URL="redis://localhost:${RD_PORT}" "${BIN}" db migrate ) >>"${LOG}" 2>&1 \
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

say "network partition: packets dropped (the service and PostgreSQL stop hearing each other)"
echo blackhole >"${CTL}/mode"
read -r c t <<<"$(req GET /api/contacts)"
printf '  (a read answered %s in %ss)\n' "${c}" "${t}"
check "a read gives up in bounded time" awk -v t="${t}" 'BEGIN { exit !(t < 20) }'
check "and does not report success" bash -c "[[ ${c} -ge 500 || ${c} == 000 ]]"
check "the process is still running" kill -0 "${SVC_PID}"
echo forward >"${CTL}/mode"
check "recovers when the partition heals, without a restart" wait_for 90 serves
check "and accepts writes again" test -n "$(new_contact 'After drop partition')"

say "network partition: connections reset"
echo reset >"${CTL}/mode"
read -r c t <<<"$(req GET /api/contacts)"
printf '  (a read answered %s in %ss)\n' "${c}" "${t}"
check "a read fails fast, as an error" bash -c "[[ ${c} -ge 500 || ${c} == 000 ]] && awk -v t=${t} 'BEGIN { exit !(t < 15) }'"
read -r c t <<<"$(req POST /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"During reset\"}")"
check "a write is refused, not pretended" bash -c "[[ ${c} -ge 500 || ${c} == 000 ]]"
echo forward >"${CTL}/mode"
check "recovers when connections are allowed again" wait_for 90 serves
check "and accepts writes again" test -n "$(new_contact 'After reset partition')"

say "slow database"
echo delay:100 >"${CTL}/mode"
read -r c t <<<"$(req GET /api/contacts)"
printf '  (100 ms per chunk: a read answered %s in %ss)\n' "${c}" "${t}"
check "a slow database still gives correct answers" test "${c}" = 200
read -r c t <<<"$(req POST /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"During slowness\"}")"
printf '  (a write answered %s in %ss)\n' "${c}" "${t}"
check "a write succeeds, only slower" test "${c}" = 200
echo delay:4000 >"${CTL}/mode"
read -r c t <<<"$(req GET /api/contacts)"
printf '  (4 s per chunk: a read answered %s in %ss)\n' "${c}" "${t}"
check "a very slow database gives an answer or an error in bounded time, never a hang" awk -v t="${t}" 'BEGIN { exit !(t < 20) }'
echo forward >"${CTL}/mode"
check "recovers when the database speeds up" wait_for 120 serves

say "the service is killed just after a transaction COMMITs, before it can answer"
a="$(new_contact 'Crash A')"; b="$(new_contact 'Crash B')"
json /api/activities "{\"subject_kind\":\"contact\",\"subject_pid\":\"${a}\",\"kind\":\"note\",\"summary\":\"before the crash\"}" >/dev/null
before="$(sql "SELECT count(*) FROM contact_merges")"
echo "${SVC_PID}" >"${CTL}/killpid"
c="$(code POST "/api/contacts/${a}/merge" "{\"into_pid\":\"${b}\",\"reason\":\"crash after commit\"}")"
printf '  (the client saw status %s)\n' "${c}"
check "the client did not get a success it could rely on" bash -c "[[ ${c} != 200 ]]"
wait_for 10 bash -c "! kill -0 ${SVC_PID} 2>/dev/null"
SVC_PID=""
check "the proxy killed the service" test ! -e "${CTL}/killpid"
start_service
check "restarts and serves again" wait_for 60 serves
check "the merge committed: one new ledger row" test "$(( $(sql "SELECT count(*) FROM contact_merges") - before ))" = 1
check "and exactly one audit row and one outbox event for it" test "$(sql "SELECT (SELECT count(*) FROM audit_logs WHERE action='contacts_merged' AND snapshot->>'merged_pid'='${a}') || '/' || (SELECT count(*) FROM event_outbox WHERE kind='contacts_merged' AND payload::text LIKE '%${a}%')")" = "1/1"
c="$(code POST "/api/contacts/${a}/merge" "{\"into_pid\":\"${b}\",\"reason\":\"client retries\"}")"
printf '  (the client retried the merge: status %s)\n' "${c}"
check "a retry is refused as already merged, not applied twice" bash -c "[[ ${c} == 409 || ${c} == 404 ]]"
check "still one ledger row for it" test "$(sql "SELECT count(*) FROM contact_merges WHERE merged_pid='${a}'")" = 1
invariants

say "the service is killed just after a create COMMITs, and the client retries with an Idempotency-Key"
key="chaos-$(uuidgen | tr 'A-Z' 'a-z')"
ref="$(person)"
body="{\"person_ref\":\"${ref}\",\"display_name\":\"Crash create\"}"
echo "${SVC_PID}" >"${CTL}/killpid"
c="$(curl -s -o /dev/null --max-time 20 -w '%{http_code}' -X POST -H 'content-type: application/json' -H "Idempotency-Key: ${key}" -d "${body}" "${API}/api/contacts")"
printf '  (the client saw status %s)\n' "${c}"
wait_for 10 bash -c "! kill -0 ${SVC_PID} 2>/dev/null"
SVC_PID=""
start_service
wait_for 60 serves
check "the contact was created exactly once before the crash" test "$(sql "SELECT count(*) FROM contacts WHERE person_ref='${ref}'")" = 1
r="$(curl -s -o /dev/null --max-time 20 -w '%{http_code}' -X POST -H 'content-type: application/json' -H "Idempotency-Key: ${key}" -d "${body}" "${API}/api/contacts")"
printf '  (the retry with the same key answered %s)\n' "${r}"
n="$(sql "SELECT count(*) FROM contacts WHERE person_ref='${ref}' AND deleted_at IS NULL")"
check "the retry did not create a second contact" test "${n}" = 1
if [[ "${r}" == 409 ]]; then
  printf '  note: the retry answered 409 (the first attempt looks still running); the service takes\n'
  printf '        such a key over after two minutes, and a retry then would run again.\n'
fi
invariants

say "failover: the primary dies and a streaming replica is promoted"
podman exec "${PG}" sh -c 'echo "host replication all all scram-sha-256" >>"$PGDATA/pg_hba.conf"' >/dev/null
sql "SELECT pg_reload_conf()" >/dev/null
podman run -d --name "${REPLICA}" --network "${NET}" -p "${REPLICA_PORT}:5432" -e PGPASSWORD=loco --user postgres \
  --entrypoint sh docker.io/library/postgres:18-alpine \
  -c 'pg_basebackup -h crm-chaos-pg -U loco -D "$PGDATA" -R -X stream && exec postgres' >/dev/null
rsql() { podman exec "${REPLICA}" psql -U loco -d chaos -Atc "$1"; }
check "a streaming replica starts and is in recovery" wait_for 60 bash -c "[[ \"\$(podman exec ${REPLICA} psql -U loco -d chaos -Atc 'select pg_is_in_recovery()' 2>/dev/null)\" == t ]]"
synced() { [[ "$(sql 'SELECT count(*) FROM contacts')" == "$(rsql 'SELECT count(*) FROM contacts')" ]]; }
safe="$(mktemp)"
for i in $(seq 1 20); do new_contact "Failover safe ${i}" >>"${safe}"; done
check "the replica catches up with 20 acknowledged writes" wait_for 30 synced
burst="$(mktemp)"
for i in $(seq 1 20); do new_contact "Failover burst ${i}" >>"${burst}"; done
podman kill "${PG}" >/dev/null 2>&1                                   # the primary dies at once, no wait
rsql "SELECT pg_promote()" >/dev/null
echo "${REPLICA_PORT}" >"${CTL}/target"
PRIMARY="${REPLICA}"
check "the replica is promoted" wait_for 30 bash -c "[[ \"\$(podman exec ${REPLICA} psql -U loco -d chaos -Atc 'select pg_is_in_recovery()')\" == f ]]"
check "the service recovers through the proxy, without a restart" wait_for 90 serves
lost=0; while read -r p; do [[ "$(code GET "/api/contacts/${p}")" == 200 ]] || lost=$((lost + 1)); done <"${safe}"
check "every write acknowledged before the replica caught up survived" test "${lost}" = 0
lost=0; while read -r p; do [[ "$(code GET "/api/contacts/${p}")" == 200 ]] || lost=$((lost + 1)); done <"${burst}"
printf '  (writes acknowledged just before the primary died and lost to async replication: %s of 20; informational)\n' "${lost}"
check "the promoted database accepts writes" test -n "$(new_contact 'After failover')"
check "a merge works on the promoted database" bash -c "[[ \"\$(curl -s -o /dev/null -w '%{http_code}' --max-time 20 -X POST -H 'content-type: application/json' -d \"{\\\"into_pid\\\":\\\"$(new_contact 'Failover S')\\\",\\\"reason\\\":\\\"after failover\\\"}\" ${API}/api/contacts/$(new_contact 'Failover M')/merge)\" == 200 ]]"
invariants
rm -f "${safe}" "${burst}"

say "result"
printf '  %s passed, %s failed\n' "${PASSES}" "${FAILS}"
if [[ "${FAILS}" -gt 0 ]]; then echo "service log tail:"; tail -15 "${LOG}"; exit 1; fi
