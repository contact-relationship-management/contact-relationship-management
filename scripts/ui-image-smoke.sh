#!/usr/bin/env bash
# Run a UI image the way a deployment would, and use it (CRM-T74 follow-up).
#   scripts/ui-image-smoke.sh [IMAGE]       default: the v0.1.5 multi-arch image by digest
#
# Phase 1, hardened (read-only root, no capabilities, no new privileges, non-root) with the
# guided tour OFF (CRM_DEMO_TOUR=off): the page and its static files are served, every security header is set,
# an unknown path is a 404, and the demo sign-in does not create a demo session (so the demo
# data is not reachable). Phase 2, the shipped default, which has the tour ON: signing in with the demo address
# gives a session that the same-origin proxy answers from fictitious data, a write sticks,
# and a record page renders. The UI never talks to the service here (CRM-T72 covers that).
#
# The server derives its own origin as https://<host>, so the sign-in posts that Origin
# (SvelteKit's cross-site form check). Containers crm-ui-smoke-*, ports 13100 and 13101,
# removed on exit. Needs podman and curl. Not part of ci-check.sh.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

IMAGE="${1:-ghcr.io/contact-relationship-management/contact-relationship-management/ui@sha256:6d20ba20daf38a126102ac915b7fa5d3084c1c2a23ff65727a24de340a03ec9b}"
A=crm-ui-smoke-plain B=crm-ui-smoke-tour
PA=13100 PB=13101
JAR="$(mktemp -t crm-ui-smoke-jar.XXXXXX)"
PASSES=0 FAILS=0

say()  { printf '\n== %s\n' "$*"; }
pass() { PASSES=$((PASSES + 1)); printf '  ok    %s\n' "$*"; }
fail() { FAILS=$((FAILS + 1)); printf '  FAIL  %s\n' "$*"; }
check() { local what="$1"; shift; if "$@"; then pass "${what}"; else fail "${what}"; fi; }
cleanup() { [[ -n "${KEEP:-}" ]] && return; podman rm -f "${A}" "${B}" >/dev/null 2>&1; rm -f "${JAR}"; }
trap cleanup EXIT
command -v podman >/dev/null && command -v curl >/dev/null || { echo "podman and curl are required"; exit 2; }

hardened=(--read-only --cap-drop all --security-opt no-new-privileges)
wait_up() { for _ in $(seq 1 40); do curl -sf -o /dev/null "http://localhost:$1/signin" && return 0; sleep 0.5; done; return 1; }
code() { curl -s -o /dev/null --max-time 20 -w '%{http_code}' "$@"; }
signin() { # PORT -> posts the demo address with the origin the server expects
  curl -s -o /dev/null -c "${JAR}" -X POST -H "Origin: https://localhost:$1" \
    --data-urlencode 'email=example@example.com' "http://localhost:$1/signin"
}

say "phase 1: hardened, tour off"
podman rm -f "${A}" "${B}" >/dev/null 2>&1
podman run -d --name "${A}" "${hardened[@]}" -p "${PA}:3000" -e CRM_DEMO_TOUR=off "${IMAGE}" >/dev/null
check "the container starts and /signin answers" wait_up "${PA}"
version="$(podman inspect --format '{{ index .Config.Labels "org.opencontainers.image.version" }}' "${IMAGE}")"
check "the image carries a version label (${version})" test -n "${version}"
check "it runs as a non-root user" test "$(podman inspect --format '{{.Config.User}}' "${IMAGE}")" = "nonroot:nonroot"
check "/signin is 200 and / is 200" test "$(code "http://localhost:${PA}/signin")$(code "http://localhost:${PA}/")" = "200200"
check "an unknown path is a 404" test "$(code "http://localhost:${PA}/does-not-exist")" = "404"
headers="$(curl -sI "http://localhost:${PA}/signin" | tr -d '\r' | tr 'A-Z' 'a-z')"
for h in content-security-policy strict-transport-security x-content-type-options x-frame-options referrer-policy permissions-policy cross-origin-opener-policy cross-origin-resource-policy; do
  check "header ${h}" grep -q "^${h}:" <<<"${headers}"
done
asset="$(curl -s "http://localhost:${PA}/signin" | grep -o '/_app/immutable/[^"]*\.js' | head -1)"
check "a static file from the page is served (${asset:-none})" test -n "${asset}" -a "$(code "http://localhost:${PA}${asset}")" = "200"
signin "${PA}"
check "with the tour off the demo address gives no demo data" bash -c "! curl -s -b '${JAR}' --max-time 20 http://localhost:${PA}/api/proxy/contacts | grep -q person-demo"
check "the server is still running" test "$(podman inspect --format '{{.State.Running}}' "${A}")" = true

say "phase 2: hardened, the shipped default (tour on)"
: >"${JAR}"
podman run -d --name "${B}" "${hardened[@]}" -p "${PB}:3000" "${IMAGE}" >/dev/null
check "the container starts" wait_up "${PB}"
signin "${PB}"
check "no session, no demo data" bash -c "! curl -s --max-time 20 http://localhost:${PB}/api/proxy/contacts | grep -q person-demo"
check "after the demo sign-in the proxy answers from demo data" bash -c "curl -s -b '${JAR}' --max-time 20 http://localhost:${PB}/api/proxy/contacts | grep -q person-demo"
edit="$(curl -s -b "${JAR}" --max-time 20 -X PUT -H 'content-type: application/json' -d '{"name":"Renamed by the smoke test"}' "http://localhost:${PB}/api/proxy/deals/deal-1")"
check "a deal edit goes through" grep -q 'Renamed by the smoke test' <<<"${edit}"
check "and the list shows it" bash -c "curl -s -b '${JAR}' --max-time 20 http://localhost:${PB}/api/proxy/deals | grep -q 'Renamed by the smoke test'"
check "a record page renders" test "$(code -b "${JAR}" "http://localhost:${PB}/deals/deal-1")" = 200
check "the server is still running" test "$(podman inspect --format '{{.State.Running}}' "${B}")" = true

say "result"
printf '  %d passed, %d failed\n' "${PASSES}" "${FAILS}"
[[ "${FAILS}" = 0 ]]
