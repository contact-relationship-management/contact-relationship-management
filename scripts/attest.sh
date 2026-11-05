#!/usr/bin/env bash
# Records a passing `scripts/ci-check.sh all` (CRM-T189): a note on refs/notes/ci, and a `local-ci`
# commit status on each remote in CRM_ATTEST_REMOTES (default origin). Called by `all`.
#
#   scripts/attest.sh start                      # print HEAD when the tree is clean; nothing if not
#   scripts/attest.sh write <commit> <stages>    # <stages> holds "<stage> <exit code> <seconds>" lines
#
# Read one with: git notes --ref=ci show <commit>
#
# This says that this machine ran the stages. It is not independent evidence (spec/local-ci-cd.md).
# A note stays in this clone until refs/notes/ci is pushed, which this script does not do.
#
# The token comes from CRM_LOCAL_CI_TOKEN, or from the macOS Keychain item crm-local-ci-status:
#   security add-generic-password -a "${USER}" -s crm-local-ci-status -w
# Make it a fine-grained token for this repository with "Commit statuses: read and write" and
# nothing else. With no token the note is still written and the run says no status was posted.
set -euo pipefail
cd "$(dirname "$0")/.."

context=local-ci
keychain_item=crm-local-ci-status
compose_file=contact-relationship-management-service-with-rust/compose.test.yaml

tree_changes() { git status --porcelain; }

status_token() {
  if [[ -n "${CRM_LOCAL_CI_TOKEN:-}" ]]; then
    printf '%s' "${CRM_LOCAL_CI_TOKEN}"
  elif command -v security >/dev/null; then
    security find-generic-password -s "${keychain_item}" -w 2>/dev/null || true
  fi
}

# owner/name of a github.com remote; empty for anything else.
github_repo() {
  local repo
  repo="$(git remote get-url "$1" 2>/dev/null \
    | sed -E 's#^(https://github\.com/|git@github\.com:|ssh://git@github\.com/)##; s#\.git$##')"
  if [[ "${repo}" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then printf '%s' "${repo}"; fi
}

# Goes to curl on standard input, so the token is never in a process's arguments.
post_status() {
  local repo="$1" commit="$2" description="$3" token="$4" code
  if ! code="$(printf 'header = "Authorization: Bearer %s"\n' "${token}" \
    | curl --silent --show-error --max-time 30 --config - --output /dev/null --write-out '%{http_code}' \
      --request POST \
      --header 'Accept: application/vnd.github+json' \
      --header 'X-GitHub-Api-Version: 2022-11-28' \
      --data "{\"state\":\"success\",\"context\":\"${context}\",\"description\":\"${description}\"}" \
      "https://api.github.com/repos/${repo}/statuses/${commit}")"; then
    echo "attest: could not reach GitHub for ${repo}; no status posted" >&2
    return
  fi
  if [[ "${code}" == 201 ]]; then
    echo "attest: posted ${context} to ${repo}"
  else
    # 422 is a commit the remote does not have yet: push it, then run again.
    echo "attest: GitHub answered ${code} for ${repo}; no status posted" >&2
  fi
}

post_statuses() {
  local commit="$1" description="$2" token remote repo
  token="$(status_token)"
  if [[ -z "${token}" ]]; then
    echo "attest: no token, so no ${context} status posted (see the top of scripts/attest.sh)"
    return
  fi
  for remote in ${CRM_ATTEST_REMOTES:-origin}; do
    repo="$(github_repo "${remote}")"
    if [[ -z "${repo}" ]]; then
      echo "attest: ${remote} is not a GitHub remote; no status posted" >&2
      continue
    fi
    post_status "${repo}" "${commit}" "${description}" "${token}"
  done
}

write_note() {
  local commit="$1" stages="$2"
  {
    echo "commit: ${commit}"
    echo "tree: $(git rev-parse "${commit}^{tree}")"
    echo "finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "host: $(hostname -s 2>/dev/null || hostname), $(uname -sm)"
    echo "rustc: $(rustc --version 2>/dev/null || echo unavailable)"
    echo "node: $(node --version 2>/dev/null || echo unavailable)"
    echo "pnpm: $(pnpm --version 2>/dev/null || echo unavailable)"
    grep -E '^[[:space:]]*image:' "${compose_file}" 2>/dev/null \
      | sed -E 's/^[[:space:]]*image:[[:space:]]*/image: /' || true
    echo "stages:"
    sed 's/^/  /' "${stages}"
  } | git notes --ref=ci add --force --file=- "${commit}" 2>/dev/null
  echo "attest: wrote refs/notes/ci for ${commit}"
}

case "${1:-}" in
  start)
    if [[ -z "$(tree_changes)" ]]; then
      git rev-parse HEAD
    else
      echo "attest: the tree has changes, so this run will not be recorded" >&2
    fi
    ;;
  write)
    commit="${2:?usage: $0 write <commit> <stages file>}"
    stages="${3:?usage: $0 write <commit> <stages file>}"
    if [[ ! -s "${stages}" ]]; then
      echo "attest: no stages recorded; nothing attested" >&2
      exit 1
    fi
    if awk '$2 != 0 { bad = 1 } END { exit !bad }' "${stages}"; then
      echo "attest: a stage failed; nothing attested" >&2
      exit 1
    fi
    if [[ "$(git rev-parse HEAD)" != "${commit}" ]]; then
      echo "attest: HEAD moved during the run; nothing attested" >&2
      exit 1
    fi
    if [[ -n "$(tree_changes)" ]]; then
      echo "attest: the tree changed during the run; nothing attested" >&2
      tree_changes >&2
      exit 1
    fi
    write_note "${commit}" "${stages}"
    post_statuses "${commit}" "$(wc -l <"${stages}" | tr -d ' ') stages passed on a clean tree; self-reported by a local run"
    ;;
  *)
    echo "usage: $0 start | write <commit> <stages file>" >&2
    exit 2
    ;;
esac
