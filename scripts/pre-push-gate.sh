#!/usr/bin/env bash
# The pre-push gate (CRM-T187): `scripts/ci-check.sh pre-push` on the commit being pushed.
# Run by the hook that scripts/install-pre-push.sh installs; git passes the refs on stdin.
#
# It checks the commit, not the working tree, so uncommitted work neither blocks a push nor passes
# for what was pushed. The checkout lives in <git dir>/ci-gate and keeps its build output.
#
#   CRM_SKIP_PRE_PUSH="why" git push    # skip it; the reason goes in <git dir>/ci-gate-skips.log
set -euo pipefail

common="$(cd "$(git rev-parse --git-common-dir)" && pwd)"
checkout="${common}/ci-gate"
lock="${common}/ci-gate.lock"
none=0000000000000000000000000000000000000000

commits=()
while read -r _ local_sha _ _; do
  [[ -z "${local_sha}" || "${local_sha}" == "${none}" ]] && continue
  # A branch deleted above; here, a tree with no gate: `make github-pages` pushes only the site.
  git cat-file -e "${local_sha}:scripts/ci-check.sh" 2>/dev/null || continue
  commits+=("${local_sha}")
done

((${#commits[@]} > 0)) || exit 0

if [[ -n "${CRM_SKIP_PRE_PUSH:-}" ]]; then
  for commit in "${commits[@]}"; do
    printf '%s %s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${commit}" "${CRM_SKIP_PRE_PUSH}" \
      >> "${common}/ci-gate-skips.log"
  done
  echo "pre-push gate skipped: ${CRM_SKIP_PRE_PUSH}" >&2
  exit 0
fi

if ! mkdir "${lock}" 2>/dev/null; then
  echo "pre-push gate: another push is being checked; if none is, remove ${lock}" >&2
  exit 1
fi
trap 'rmdir "${lock}"' EXIT

# Git sets these for a hook; a command inside the checkout must see its own repository.
# shellcheck disable=SC2046 # one variable name per word
unset $(git rev-parse --local-env-vars)

git --git-dir="${common}" worktree prune
for commit in "${commits[@]}"; do
  if [[ -d "${checkout}" ]]; then
    git -C "${checkout}" checkout --quiet --force --detach "${commit}"
    git -C "${checkout}" clean -fdq
  else
    git --git-dir="${common}" worktree add --quiet --detach "${checkout}" "${commit}"
  fi
  echo "pre-push gate: checking ${commit:0:9}" >&2
  (cd "${checkout}" && scripts/ci-check.sh pre-push </dev/null) >&2
done
