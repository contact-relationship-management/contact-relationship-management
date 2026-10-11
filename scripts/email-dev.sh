#!/usr/bin/env bash
# Start, configure, check and stop the local Stalwart mail server (CRM-T39).
# See contact-relationship-management-service-with-rust/compose.email.yaml.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dir="$root/contact-relationship-management-service-with-rust"
compose=(podman compose -f "$dir/compose.email.yaml")
cli_image="docker.io/stalwartlabs/cli:1.0.13@sha256:9a1e07307d6f890a193322c4b737fc0868405f0f8486a19922cbf362f43884fd"

admin_password="${MAIL_ADMIN_PASSWORD:-dev-admin-change-me}"
http_port="${MAIL_HTTP_PORT:-8081}"
submission_port="${MAIL_SUBMISSION_PORT:-2587}"

# True when the port answers with an SMTP greeting. A bare TCP connect is not
# enough: the published port accepts connections even when nothing listens.
smtp_ready() {
  local line
  exec 3<>"/dev/tcp/127.0.0.1/$1" 2>/dev/null || return 1
  read -r -t 3 -u 3 line || { exec 3>&-; return 1; }
  exec 3>&-
  [[ "$line" == 220* ]]
}

wait_for_smtp() { # <port>
  for _ in $(seq 1 60); do
    smtp_ready "$1" && return 0
    sleep 1
  done
  echo "timed out waiting for SMTP on port $1" >&2
  return 1
}

# The published port accepts connections before the server is ready, so wait
# for an actual HTTP answer.
wait_for_http() {
  for _ in $(seq 1 90); do
    [ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$http_port/" || true)" != 000 ] && return 0
    sleep 1
  done
  echo "timed out waiting for the management API on port $http_port" >&2
  return 1
}

apply_plan() {
  podman run --rm \
    -e STALWART_URL="http://host.containers.internal:$http_port" \
    -e STALWART_USER=admin -e STALWART_PASSWORD="$admin_password" \
    -v "$dir/dev/stalwart/plan.ndjson:/plan.ndjson:ro,z" \
    "$cli_image" apply --file /plan.ndjson
}

case "${1:-}" in
  up)
    "${compose[@]}" up -d
    wait_for_http
    apply_plan
    # A new listener binds on start, so restart once if submission is not up.
    if ! smtp_ready "$submission_port"; then
      podman restart mxi-contact-relationship-management-mail >/dev/null
      wait_for_smtp "$submission_port"
    fi
    echo "mail server ready: submission 127.0.0.1:$submission_port (STARTTLS), crm@crm.test / crm-dev-password"
    ;;
  check)
    python3 - "$submission_port" "$http_port" <<'PY'
import json, smtplib, ssl, sys, time, urllib.request, base64, uuid
port, http = int(sys.argv[1]), int(sys.argv[2])
user, password = "crm@crm.test", "crm-dev-password"
subject = f"check {uuid.uuid4()}"
from email.message import EmailMessage
m = EmailMessage()
m["From"], m["To"], m["Subject"] = user, user, subject
m.set_content("sent by scripts/email-dev.sh check")
ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE   # the dev certificate is self-signed
s = smtplib.SMTP("127.0.0.1", port, timeout=20)
s.starttls(context=ctx)
s.login(user, password)
s.send_message(m)
s.quit()
import re
auth = "Basic " + base64.b64encode(f"{user}:{password}".encode()).decode()
# The session document is not strictly valid JSON (Stalwart 0.16), so read the
# mail account id with a pattern instead of a parser.
session = urllib.request.urlopen(urllib.request.Request(
    f"http://127.0.0.1:{http}/jmap/session", headers={"Authorization": auth})).read().decode()
account = re.search(r'"urn:ietf:params:jmap:mail":"([^"]+)"', session).group(1)
body = json.dumps({
    "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
    "methodCalls": [["Email/query", {"accountId": account, "filter": {"subject": subject}}, "q"]],
}).encode()
for _ in range(20):
    req = urllib.request.Request(f"http://127.0.0.1:{http}/jmap", body,
                                 {"Authorization": auth, "Content-Type": "application/json"})
    ids = json.load(urllib.request.urlopen(req))["methodResponses"][0][1].get("ids", [])
    if ids:
        print(f"ok: sent and received '{subject}'")
        sys.exit(0)
    time.sleep(0.5)
print("FAILED: the message was accepted but did not arrive", file=sys.stderr)
sys.exit(1)
PY
    ;;
  down)
    "${compose[@]}" down
    ;;
  reset)
    "${compose[@]}" down -v
    ;;
  *)
    echo "usage: $0 up|check|down|reset" >&2
    exit 2
    ;;
esac
