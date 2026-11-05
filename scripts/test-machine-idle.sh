#!/usr/bin/env bash
# Tests for the scheduled jobs' start gate (CRM-T191). pmset and sysctl are stubs on PATH, so every
# case runs on any machine. The last cases run the real scripts/nightly.sh against a fixture.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# Run from inside a hook, git's variables would point the fixture at the real repository.
# shellcheck disable=SC2046 # one variable name per word
unset $(git rev-parse --local-env-vars)

base="$(mktemp -d)"
trap 'rm -rf "${base}"' EXIT
failures=0

pass() { echo "ok - $1"; }
fail() {
  echo "not ok - $1" >&2
  failures=$((failures + 1))
}

mkdir "${base}/stubs"
PATH="${base}/stubs:${PATH}"

# stub <pmset first line> <vm.loadavg line> [hw.ncpu]
stub() {
  printf '#!/bin/sh\n[ -n "%s" ] && echo "%s"\n' "$1" "$1" >"${base}/stubs/pmset"
  cat >"${base}/stubs/sysctl" <<STUB
#!/bin/sh
case "\$2" in
  vm.loadavg) echo "$2" ;;
  hw.ncpu) echo "${3-16}" ;;
esac
STUB
  chmod +x "${base}/stubs/pmset" "${base}/stubs/sysctl"
}

allows() {
  if "${here}/machine-idle.sh" >/dev/null; then pass "$1"; else fail "$1"; fi
}
refuses() {
  local why
  if why="$("${here}/machine-idle.sh")"; then
    fail "$1"
  elif [[ "${why}" == *"$2"* ]]; then
    pass "$1"
  else
    fail "$1 (said: ${why})"
  fi
}

ac="Now drawing from 'AC Power'"
stub "${ac}" "{ 90.00 1.50 90.00 }"
allows "wall power and a low 5-minute load, whatever the 1- and 15-minute loads are"
stub "Now drawing from 'Battery Power'" "{ 1.00 1.00 1.00 }"
refuses "battery" "not on wall power"
stub "" "{ 1.00 1.00 1.00 }"
refuses "pmset gives no answer" "not on wall power"
stub "${ac}" "{ 1.00 9.00 1.00 }"
refuses "load over half the cores" "load average 9.00"
stub "${ac}" "{ 1.00 8.00 1.00 }"
refuses "load exactly at the limit" "load average 8.00"
stub "${ac}" "{ 1.00 7.99 1.00 }"
allows "load just under the limit"
stub "${ac}" "unreadable"
refuses "a load that cannot be read" "could not read"
stub "${ac}" "{ 1.00 1.00 1.00 }" ""
refuses "a core count that cannot be read" "could not read"
stub "${ac}" "{ 1.00 1.00 1.00 }" 1
refuses "one core gives a limit of nought" "could not read"
stub "${ac}" "{ 1.00 9.00 1.00 }"
if CRM_NIGHTLY_MAX_LOAD=10 "${here}/machine-idle.sh" >/dev/null; then
  pass "CRM_NIGHTLY_MAX_LOAD raises the limit"
else
  fail "CRM_NIGHTLY_MAX_LOAD raises the limit"
fi

git init -q -b main "${base}/clone"
cd "${base}/clone"
git config user.name test
git config user.email test@example.invalid
mkdir scripts
cp "${here}/nightly.sh" "${here}/machine-idle.sh" scripts/
git add scripts
git commit -qm fixture

stub "Now drawing from 'Battery Power'" "{ 1.00 1.00 1.00 }"
for kind in nightly weekly; do
  if scripts/nightly.sh "${kind}" >/dev/null 2>&1; then
    pass "${kind} skipped is not a failure"
  else
    fail "${kind} skipped is not a failure"
  fi
  if grep -q 'skipped, nothing ran: not on wall power' ".git/ci-${kind}-last-run.md"; then
    pass "${kind} page says why it skipped"
  else
    fail "${kind} page says why it skipped"
  fi
  if grep -q "${kind}: not on wall power" .git/ci-nightly-logs/skips.log; then
    pass "${kind} skip is kept in the log"
  else
    fail "${kind} skip is kept in the log"
  fi
done
if [[ ! -e .git/ci-nightly && ! -e .git/ci-nightly.lock ]]; then
  pass "a skip makes no worktree and leaves no lock"
else
  fail "a skip makes no worktree and leaves no lock"
fi

if ((failures > 0)); then
  echo "${failures} failed" >&2
  exit 1
fi
