#!/usr/bin/env bash
# The scheduled local jobs (CRM-T191), on the latest origin/main in a checkout of their own, so
# uncommitted work is never tested. Installed by scripts/install-nightly.sh.
#
#   scripts/nightly.sh nightly   # gitleaks, advisories and licences, one seventh of the mutants
#   scripts/nightly.sh weekly    # the image smokes; needs Podman
#   CRM_NIGHTLY_REF=HEAD scripts/nightly.sh nightly   # a commit not yet pushed
#
# A job that fails does not stop the next. The result of each is written to
# <git dir>/ci-<kind>-last-run.md, with its log in <git dir>/ci-nightly-logs. A mutant lost to a
# kache daemon failure is reported as lost: cargo-mutants calls it unviable, and it was never tested.
# Exits 1 if a job failed. Missed mutants are a finding, not a failure, and do not change that.
#
# Nothing starts unless scripts/machine-idle.sh agrees: on wall power, load average low. A skip is
# not a failure. It is written to the page and kept in ci-nightly-logs/skips.log, so a job that
# never runs can be seen. A skipped night's seventh of the mutants waits a week for its turn.
set -uo pipefail

kind="${1:-}"
[[ "${kind}" == nightly || "${kind}" == weekly ]] || {
  echo "usage: $0 <nightly|weekly>" >&2
  exit 2
}

cd "$(dirname "$0")/.." || exit 1
common="$(cd "$(git rev-parse --git-common-dir)" && pwd)"
checkout="${common}/ci-nightly"
lock="${common}/ci-nightly.lock"
logs="${common}/ci-nightly-logs"
page="${common}/ci-${kind}-last-run.md"
service=contact-relationship-management-service-with-rust

if ! mkdir "${lock}" 2>/dev/null; then
  echo "nightly: another run holds ${lock}" >&2
  exit 1
fi
trap 'rmdir "${lock}"' EXIT
mkdir -p "${logs}"

if ! why="$(scripts/machine-idle.sh)"; then
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "${now} ${kind}: ${why}" >>"${logs}/skips.log"
  {
    echo "# Last ${kind} run"
    echo
    echo "${now}: skipped, nothing ran: ${why}."
  } >"${page}"
  cat "${page}"
  exit 0
fi

rows=()
failed=0

# record <job> <outcome> <seconds> <log name>
record() { rows+=("| $1 | $2 | $3 | \`$4\` |"); }

# run_job <job>: run scripts/ci-check.sh <job> and any arguments in the checkout.
run_job() {
  local job="$1" log="${kind}-$1.log" started="${SECONDS}" status=0
  shift
  (cd "${checkout}" && scripts/ci-check.sh "${job}" "$@") >"${logs}/${log}" 2>&1 || status=$?
  outcome=passed
  if ((status != 0)); then
    outcome="FAILED (exit ${status})"
    failed=1
  fi
}

git fetch --quiet origin main || {
  echo "nightly: could not fetch origin main" >&2
  exit 1
}
commit="$(git rev-parse "${CRM_NIGHTLY_REF:-FETCH_HEAD}")"
git worktree prune
if [[ -d "${checkout}" ]]; then
  git -C "${checkout}" checkout --quiet --force --detach "${commit}"
  git -C "${checkout}" clean -fdq
else
  git worktree add --quiet --detach "${checkout}" "${commit}"
fi

if [[ "${kind}" == nightly ]]; then
  for job in secrets supply-chain; do
    started="${SECONDS}"
    run_job "${job}"
    record "${job}" "${outcome}" "$((SECONDS - started))" "${kind}-${job}.log"
  done

  # A different seventh each night, so every rule's mutants are tried once a week. The checkout
  # keeps its build output, so a mutants-out directory from last night has to go first.
  shard="$(($(date +%u) % 7))"
  rm -rf "${checkout:?}/${service}/mutants-out"
  started="${SECONDS}"
  export CRM_MUTANTS_JOBS="${CRM_MUTANTS_JOBS:-8}"
  run_job mutants --shard "${shard}/7"
  mutants_dir="${checkout}/${service}/mutants-out/mutants.out"
  missed=0
  if [[ -f "${mutants_dir}/missed.txt" ]]; then
    missed="$(wc -l <"${mutants_dir}/missed.txt" | tr -d ' ')"
    cp "${mutants_dir}/missed.txt" "${logs}/${kind}-mutants-missed.txt"
  fi
  # cargo-mutants exits 2 for a missed mutant and 3 for a timeout: findings, not a broken run.
  case "${outcome}" in
    "FAILED (exit 2)" | "FAILED (exit 3)")
      outcome="ran, ${missed} missed"
      failed=0
      for row in "${rows[@]}"; do
        [[ "${row}" == *FAILED* ]] && failed=1
      done
      ;;
    passed) outcome="passed, ${missed} missed" ;;
  esac
  lost="$(grep -rl 'daemon compilation did not complete' "${mutants_dir}/log" 2>/dev/null | wc -l | tr -d ' ')"
  if ((lost > 0)); then
    outcome="${outcome}, ${lost} lost to kache and not tested"
  fi
  record "mutants shard ${shard}/7 at -j ${CRM_MUTANTS_JOBS}" "${outcome}" "$((SECONDS - started))" "${kind}-mutants.log"
else
  for smoke in image-smoke ui-image-smoke; do
    if ! podman info >/dev/null 2>&1; then
      record "${smoke}" "skipped: Podman does not answer" 0 "-"
      continue
    fi
    started="${SECONDS}"
    outcome=passed
    if ! (cd "${checkout}" && "scripts/${smoke}.sh") >"${logs}/${kind}-${smoke}.log" 2>&1; then
      outcome=FAILED
      failed=1
    fi
    record "${smoke}" "${outcome}" "$((SECONDS - started))" "${kind}-${smoke}.log"
  done
fi

{
  echo "# Last ${kind} run"
  echo
  echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) on commit \`${commit:0:9}\`, rustc wrapper: ${RUSTC_WRAPPER:-cargo config}."
  echo
  echo "| Job | Result | Seconds | Log |"
  echo "|---|---|---|---|"
  printf '%s\n' "${rows[@]}"
} >"${page}"
cat "${page}"
exit "${failed}"
