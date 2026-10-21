#!/usr/bin/env bash
# Run a released service image the way a deployment would, and use it (CRM-T72).
#   scripts/image-smoke.sh [IMAGE]       default: the v0.1.5 multi-arch image by digest
#
# Phase 1, production mode: hardened flags (read-only root, no capabilities, no new
# privileges, non-root), authentication on with real signed tokens (PASETO v4.public,
# minted by `cargo run --example mint_token`), field encryption required, the event outbox,
# the image's own `db migrate`, then a workflow through the API: persona checks (no token,
# bad token, reader, writer, admin), create, merge, unmerge, a link, the audit chain.
# Phase 2, development mode with a mail server (Mailpit): a campaign run is queued, and no
# mail leaves until a second container started with `--worker` picks the job up.
#
# Production refuses unverified SMTP certificates, so phase 2 runs the image with
# LOCO_ENV=development; the mail path and the worker are the same code. Self-contained:
# containers crm-smoke-*, ports 15150 and 18025, removed on exit. Needs podman, curl,
# python3 and a cargo toolchain (for the token). Not part of ci-check.sh.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

IMAGE="${1:-ghcr.io/contact-relationship-management/contact-relationship-management/service@sha256:8579252e8a2702199cd9f12b768f75a1b2ba510ad22c53cd9041255b2ce2d5a5}"
NET=crm-smoke-net PG=crm-smoke-pg RD=crm-smoke-redis MAIL=crm-smoke-mail SVC=crm-smoke-svc WRK=crm-smoke-worker
PORT=15150 MAIL_API=18025
API="http://localhost:${PORT}"
KEYS="$(mktemp -t crm-smoke-keys.XXXXXX)"
PASSES=0 FAILS=0

say()  { printf '\n== %s\n' "$*"; }
pass() { PASSES=$((PASSES + 1)); printf '  ok    %s\n' "$*"; }
fail() { FAILS=$((FAILS + 1)); printf '  FAIL  %s\n' "$*"; }
check() { local what="$1"; shift; if "$@"; then pass "${what}"; else fail "${what}"; fi; }
cleanup() { [[ -n "${KEEP:-}" ]] && return; podman rm -f "${SVC}" "${WRK}" "${PG}" "${RD}" "${MAIL}" >/dev/null 2>&1; podman network rm "${NET}" >/dev/null 2>&1; rm -f "${KEYS}"; }
trap cleanup EXIT

command -v podman >/dev/null && command -v cargo >/dev/null || { echo "podman and cargo are required"; exit 2; }
sql() { podman exec "${PG}" psql -U loco -d crm -Atc "$1"; }
wait_for() { local n="$1"; shift; for _ in $(seq 1 "$((n * 2))"); do "$@" >/dev/null 2>&1 && return 0; sleep 0.5; done; return 1; }

# Tokens: admin may do everything; rep may write but not destructive actions; reader is masked.
mint() { (cd contact-relationship-management-service-with-rust && cargo run -q --example mint_token -- "${KEYS}" "$@"); }
ADMIN="$(mint admin-user access=admin)"; REP="$(mint rep-user rep=true)"; READER="$(mint reader-user)"
KEYS_JSON="$(cat "${KEYS}")"
[[ -n "${ADMIN}" && -n "${KEYS_JSON}" ]] || { echo "could not mint tokens"; exit 2; }

call() { # TOKEN METHOD PATH [JSON]  -> "status body"
  local token="$1" method="$2" path="$3" body="${4:-}"
  local args=(-s --max-time 30 -o /tmp/crm-smoke-body.$$ -w '%{http_code}' -X "${method}")
  [[ -n "${token}" ]] && args+=(-H "authorization: Bearer ${token}")
  [[ -n "${body}" ]] && args+=(-H 'content-type: application/json' -d "${body}")
  local code; code="$(curl "${args[@]}" "${API}${path}")"
  printf '%s %s' "${code}" "$(cat /tmp/crm-smoke-body.$$)"
}
status() { call "$@" | cut -d' ' -f1; }
body()   { call "$@" | cut -d' ' -f2-; }
pid_of() { sed -n 's/.*"pid":"\([^"]*\)".*/\1/p' | head -1; }
person() { printf 'person:%s' "$(uuidgen | tr 'A-Z' 'a-z')"; }
contact() { body "${ADMIN}" POST /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"$1\"}" | pid_of; }

common_env=(
  -e "DATABASE_URL=postgres://loco:loco@${PG}:5432/crm" -e JWT_SECRET=smoke-test-secret-not-a-secret
  -e "CRM_REDIS_URL=redis://${RD}:6379" -e CRM_REQUIRE_AUTH=1 -e "CRM_PASETO_KEYS=${KEYS_JSON}"
  -e CRM_ABAC_POLICY_FILE=/app/config/abac-policy.reference.json -e CRM_EVENT_TRANSPORT=outbox
  -e "CRM_FIELD_KEYS=k1:$(head -c 32 /dev/zero | tr '\0' 'a' | base64)" -e CRM_REQUIRE_FIELD_ENCRYPTION=1
)
hardened=(--read-only --tmpfs /tmp --cap-drop all --security-opt no-new-privileges --memory 768m --cpus 2 --network "${NET}")
run_image() { # NAME EXTRA-RUN-ARGS... -- IMAGE-ARGS...
  local name="$1"; shift; local extra=(); while [[ "$1" != -- ]]; do extra+=("$1"); shift; done; shift
  podman rm -f "${name}" >/dev/null 2>&1
  podman run -d --name "${name}" "${hardened[@]}" "${common_env[@]}" "${extra[@]}" "${IMAGE}" "$@" >/dev/null
}
serves() { [[ "$(status '' GET /_ping)" == 200 ]]; }

say "setup"
podman rm -f "${SVC}" "${WRK}" "${PG}" "${RD}" "${MAIL}" >/dev/null 2>&1; podman network rm "${NET}" >/dev/null 2>&1
podman network create "${NET}" >/dev/null
podman run -d --name "${PG}" --network "${NET}" -e POSTGRES_USER=loco -e POSTGRES_PASSWORD=loco -e POSTGRES_DB=crm docker.io/library/postgres:18-alpine >/dev/null
podman run -d --name "${RD}" --network "${NET}" docker.io/library/redis:8-alpine >/dev/null
podman run -d --name "${MAIL}" --network "${NET}" -p "${MAIL_API}:8025" docker.io/axllent/mailpit >/dev/null
wait_for 60 podman exec "${PG}" pg_isready -U loco -d crm || { echo "postgres did not start"; exit 2; }
sleep 2
printf '  image: %s\n' "${IMAGE}"
podman image exists "${IMAGE}" || podman pull -q "${IMAGE}" >/dev/null \
  || { echo "cannot pull the image (a private package needs: gh auth token | podman login ghcr.io -u <user> --password-stdin)"; exit 2; }
out="$(podman run --rm --network "${NET}" --entrypoint /app/crm-service "${common_env[@]}" "${IMAGE}" db migrate 2>&1)"
check "the image's own 'db migrate' applies every migration" bash -c "[[ \"\$(podman exec ${PG} psql -U loco -d crm -Atc 'select count(*) from seaql_migrations')\" -ge 19 ]]"
[[ $? -ne 0 ]] && echo "${out}" | tail -5

say "phase 1: production mode, hardened, authentication on"
run_image "${SVC}" -p "${PORT}:5150" --
check "the service starts and answers /_ping" wait_for 60 serves
check "it runs as a non-root user" bash -c "[[ \"\$(podman exec ${PG} true; podman inspect ${SVC} --format '{{.Config.User}}')\" == nonroot* ]]"
check "no token: 401" test "$(status '' GET /api/contacts)" = 401
check "a forged token: 401" test "$(status 'v4.public.AAAA.bbb' GET /api/contacts)" = 401
check "a signed reader token reads: 200" test "$(status "${READER}" GET /api/contacts)" = 200
check "a reader cannot write: 403" test "$(status "${READER}" POST /api/contacts "{\"person_ref\":\"$(person)\",\"display_name\":\"No\"}")" = 403
A="$(contact 'Smoke A')"; B="$(contact 'Smoke B')"
check "an admin creates contacts" test -n "${A}" -a -n "${B}"
check "a rep (writer) records an activity" test "$(status "${REP}" POST /api/activities "{\"subject_kind\":\"contact\",\"subject_pid\":\"${A}\",\"kind\":\"note\",\"summary\":\"smoke\"}")" = 200
check "a rep cannot merge (destructive): 403" test "$(status "${REP}" POST "/api/contacts/${A}/merge" "{\"into_pid\":\"${B}\",\"reason\":\"x\"}")" = 403
check "an admin links the contact to HubSpot" test "$(status "${ADMIN}" POST "/api/contacts/${A}/external-links" '{"system":"hubspot","url":"https://app.hubspot.com/contacts/1/record/0-1/9"}')" = 200
check "an admin merges A into B" test "$(status "${ADMIN}" POST "/api/contacts/${A}/merge" '{"into_pid":"'"${B}"'","reason":"image smoke"}')" = 200
check "the activity and the link moved to B" bash -c "[[ \"\$(curl -s -H 'authorization: Bearer ${ADMIN}' ${API}/api/contacts/${B}/external-links)\" == *hubspot* ]]"
check "a rep cannot unmerge: 403" test "$(status "${REP}" POST "/api/contacts/${A}/unmerge" '{"reason":"x"}')" = 403
check "an admin unmerges A" test "$(status "${ADMIN}" POST "/api/contacts/${A}/unmerge" '{"reason":"image smoke"}')" = 200
check "A is back and its link with it" bash -c "[[ \"\$(curl -s -H 'authorization: Bearer ${ADMIN}' ${API}/api/contacts/${A}/external-links)\" == *hubspot* ]]"
check "the audit chain verifies" bash -c "curl -s -H 'authorization: Bearer ${ADMIN}' ${API}/api/audits/verify | grep -q '\"ok\":true'"
check "the contact's channel text is stored encrypted" test "$(sql "select count(*) from activities where summary = 'smoke'")" = 0
run_image "${WRK}" -- --worker
sleep 5
check "a --worker container stays up and says it is the worker" bash -c "[[ \"\$(podman inspect ${WRK} --format '{{.State.Running}}')\" == true ]] && podman logs ${WRK} 2>&1 | grep -qi 'modes: worker'"
check "and the service still answers" serves
podman rm -f "${SVC}" "${WRK}" >/dev/null 2>&1

say "phase 2: a campaign send needs the worker (development mode, Mailpit)"
mail_env=(-e LOCO_ENV=development -e CRM_MAIL_MODE=smtp -e "CRM_SMTP_HOST=${MAIL}" -e CRM_SMTP_PORT=1025 -e CRM_SMTP_TLS=none
  -e CRM_MAIL_FROM=crm@example.test -e CRM_MAIL_DEV_DOMAIN=example.test
  -e CRM_UNSUBSCRIBE_SECRET=smoke-unsubscribe-secret-0123456789abcdef -e CRM_PUBLIC_URL=https://crm.example.test)
# development binds to localhost; the container needs every interface
run_image "${SVC}" -p "${PORT}:5150" "${mail_env[@]}" -- --binding 0.0.0.0
check "the service starts in development mode with SMTP" wait_for 60 serves
C1="$(contact 'Mail One')"; C2="$(contact 'Mail Two')"; contact 'Mail Three, never consented' >/dev/null  # created and never given consent
for c in "${C1}" "${C2}"; do status "${ADMIN}" POST "/api/contacts/${c}/consent" '{"action":"granted","source":"image smoke"}' >/dev/null; done
SEG="$(body "${ADMIN}" POST /api/segments '{"name":"Everyone","filter":{}}' | pid_of)"
CAMP="$(body "${ADMIN}" POST /api/campaigns "{\"name\":\"Smoke send\",\"segment_pid\":\"${SEG}\",\"subject\":\"Smoke\",\"body\":\"Hello from the image.\"}" | pid_of)"
status "${ADMIN}" POST "/api/campaigns/${CAMP}/status" '{"to":"scheduled"}' >/dev/null
code="$(status "${ADMIN}" POST "/api/campaigns/${CAMP}/run")"
check "running the campaign answers 202 (queued, not done)" test "${code}" = 202
mails() { curl -s "http://localhost:${MAIL_API}/api/v1/messages" | sed -n 's/.*"total":\([0-9]*\).*/\1/p'; }
sleep 6
check "with no worker, no mail has left" test "$(mails)" = 0
run_image "${WRK}" "${mail_env[@]}" -- --worker
check "a worker container picks the job up and the mail arrives (two consented contacts)" wait_for 60 bash -c "[[ \"\$(curl -s http://localhost:${MAIL_API}/api/v1/messages | sed -n 's/.*\"total\":\([0-9]*\).*/\1/p')\" == 2 ]]"
check "the campaign completes" wait_for 30 bash -c "curl -s -H 'authorization: Bearer ${ADMIN}' ${API}/api/campaigns | grep -q '\"status\":\"completed\"'"
check "each message carries a signed unsubscribe link" bash -c "curl -s http://localhost:${MAIL_API}/api/v1/messages | grep -q 'crm.example.test/unsubscribe/'"
sleep 3
check "no contact is mailed twice" test "$(mails)" = 2

say "result"
printf '  %s passed, %s failed\n' "${PASSES}" "${FAILS}"
if [[ "${FAILS}" -gt 0 ]]; then echo "service log tail:"; podman logs --tail 20 "${SVC}" 2>&1; echo "worker:"; podman logs --tail 20 "${WRK}" 2>&1; exit 1; fi
