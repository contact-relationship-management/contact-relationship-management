#!/usr/bin/env bash
# Tests for scripts/attest.sh and its call from `all` (CRM-T189). curl and security are stubs on
# PATH, so nothing reaches GitHub and the owner's Keychain is not read.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# Run from inside a hook, git's variables would point the fixture at the real repository.
# shellcheck disable=SC2046 # one variable name per word
unset $(git rev-parse --local-env-vars)
unset CRM_LOCAL_CI_TOKEN CRM_ATTEST_REMOTES

base="$(mktemp -d)"
trap 'rm -rf "${base}"' EXIT
failures=0

pass() { echo "ok - $1"; }
fail() {
  echo "not ok - $1" >&2
  failures=$((failures + 1))
}
check() { # <description> <command...>
  local description="$1"
  shift
  if "$@"; then pass "${description}"; else fail "${description}"; fi
}

mkdir "${base}/stubs"
PATH="${base}/stubs:${PATH}"

# curl records its arguments and its standard input, and answers CURL_CODE.
cat >"${base}/stubs/curl" <<STUB
#!/bin/sh
printf '%s\n' "\$*" >>"${base}/curl-args"
cat >>"${base}/curl-stdin"
[ "\${CURL_CODE:-201}" = down ] && exit 7
printf '%s' "\${CURL_CODE:-201}"
STUB
cat >"${base}/stubs/security" <<STUB
#!/bin/sh
[ -n "\${KEYCHAIN_TOKEN:-}" ] || exit 44
printf '%s' "\${KEYCHAIN_TOKEN}"
STUB
# fmt in the fixture's `all`: cargo fails when the marker exists.
cat >"${base}/stubs/cargo" <<STUB
#!/bin/sh
[ ! -e "${base}/cargo-fails" ]
STUB
chmod +x "${base}/stubs/"*

new_repo() {
  repo="${base}/repo-$((++repos))"
  git init -q "${repo}"
  git -C "${repo}" config user.name test
  git -C "${repo}" config user.email test@example.com
  mkdir -p "${repo}/scripts"
  cp "${here}/attest.sh" "${repo}/scripts/"
  sed 's/^ALL_STAGES=.*/ALL_STAGES=(fmt)/' "${here}/ci-check.sh" >"${repo}/scripts/ci-check.sh"
  chmod +x "${repo}/scripts/ci-check.sh"
  for dir in contact-relationship-management-service-with-rust/migration contact-relationship-management-matcher-with-rust; do
    mkdir -p "${repo}/${dir}"
    touch "${repo}/${dir}/.keep"
  done
  git -C "${repo}" remote add origin https://github.com/example-owner/example-repo.git
  git -C "${repo}" remote add site git@github.com:example-owner/example-site.git
  git -C "${repo}" remote add other https://codeberg.org/example-owner/example-repo.git
  git -C "${repo}" add .
  git -C "${repo}" commit -qm first
  commit="$(git -C "${repo}" rev-parse HEAD)"
  rm -f "${base}/curl-args" "${base}/curl-stdin" "${base}/cargo-fails"
  printf 'fmt 0 3\ntest 0 9\n' >"${base}/stages"
}
attest() { "${repo}/scripts/attest.sh" "$@" 2>"${base}/stderr"; }
note() { git -C "${repo}" notes --ref=ci show "${commit}" 2>/dev/null; }
no_note() { ! note >/dev/null; }
posted() { [[ -s "${base}/curl-args" ]]; }
not_posted() { ! posted; }
said() { grep -q "$1" "${base}/stderr" "${base}/stdout" 2>/dev/null; }
repos=0

# start
new_repo
check "start prints HEAD on a clean tree" test "$(attest start)" = "${commit}"
touch "${repo}/untracked"
check "start prints nothing with an untracked file" test -z "$(attest start)"
rm "${repo}/untracked"
echo changed >>"${repo}/scripts/attest.sh"
check "start prints nothing with a modified file" test -z "$(attest start)"

# write refuses
new_repo
printf 'fmt 0 3\ntest 1 9\n' >"${base}/stages"
if attest write "${commit}" "${base}/stages" >/dev/null; then fail "a failed stage is refused"; else pass "a failed stage is refused"; fi
check "a failed stage writes no note" no_note
: >"${base}/stages"
if attest write "${commit}" "${base}/stages" >/dev/null; then fail "no stages is refused"; else pass "no stages is refused"; fi

new_repo
touch "${repo}/appeared"
if attest write "${commit}" "${base}/stages" >/dev/null; then fail "a tree dirtied by the run is refused"; else pass "a tree dirtied by the run is refused"; fi
check "a dirtied tree writes no note" no_note
rm "${repo}/appeared"
git -C "${repo}" commit -qm second --allow-empty
if attest write "${commit}" "${base}/stages" >/dev/null; then fail "a moved HEAD is refused"; else pass "a moved HEAD is refused"; fi
check "a moved HEAD writes no note" no_note

# no token
new_repo
attest write "${commit}" "${base}/stages" >"${base}/stdout"
check "the note names the commit" grep -q "^commit: ${commit}$" <(note)
check "the note names the tree" grep -q "^tree: $(git -C "${repo}" rev-parse 'HEAD^{tree}')$" <(note)
check "the note lists each stage" grep -q '^  test 0 9$' <(note)
check "no token posts nothing" not_posted
check "no token says so" said 'no token'
attest write "${commit}" "${base}/stages" >/dev/null
check "a second run replaces the note" test "$(note | grep -c '^commit:')" = 1

# token from the environment
new_repo
CRM_LOCAL_CI_TOKEN=env-token attest write "${commit}" "${base}/stages" >"${base}/stdout"
check "posts to the commit on origin" grep -q "repos/example-owner/example-repo/statuses/${commit}" "${base}/curl-args"
check "the status is local-ci success" grep -q '"state":"success","context":"local-ci"' "${base}/curl-args"
check "the status calls itself self-reported" grep -q '2 stages passed on a clean tree; self-reported' "${base}/curl-args"
check "the token reaches curl" grep -q 'Bearer env-token' "${base}/curl-stdin"
check "the token is not in curl's arguments" test "$(grep -c env-token "${base}/curl-args")" = 0
check "only origin by default" test "$(wc -l <"${base}/curl-args" | tr -d ' ')" = 1

# token from the Keychain
new_repo
KEYCHAIN_TOKEN=keychain-token attest write "${commit}" "${base}/stages" >/dev/null
check "the Keychain supplies the token" grep -q 'Bearer keychain-token' "${base}/curl-stdin"

# remotes
new_repo
CRM_LOCAL_CI_TOKEN=t CRM_ATTEST_REMOTES='site other' attest write "${commit}" "${base}/stages" >/dev/null
check "an ssh GitHub remote is parsed" grep -q 'repos/example-owner/example-site/statuses' "${base}/curl-args"
check "a non-GitHub remote is not posted to" test "$(grep -c codeberg "${base}/curl-args")" = 0
check "a non-GitHub remote is reported" said 'other is not a GitHub remote'

# a refused status does not undo the note
new_repo
if CURL_CODE=422 CRM_LOCAL_CI_TOKEN=t attest write "${commit}" "${base}/stages" >/dev/null; then pass "a refused status does not fail the run"; else fail "a refused status does not fail the run"; fi
check "a refused status is reported" said 'answered 422'
check "a refused status keeps the note" grep -q '^commit: ' <(note)
new_repo
if CURL_CODE=down CRM_LOCAL_CI_TOKEN=t attest write "${commit}" "${base}/stages" >/dev/null; then pass "an unreachable host does not fail the run"; else fail "an unreachable host does not fail the run"; fi
check "an unreachable host is reported" said 'could not reach GitHub'

# through `all`
new_repo
if "${repo}/scripts/ci-check.sh" all >/dev/null 2>&1; then pass "all passes in the fixture"; else fail "all passes in the fixture"; fi
check "a passing all writes the note" grep -q '^  fmt 0 ' <(note)

new_repo
touch "${base}/cargo-fails"
if "${repo}/scripts/ci-check.sh" all >/dev/null 2>&1; then fail "a failing all exits non-zero"; else pass "a failing all exits non-zero"; fi
check "a failing all writes no note" no_note

new_repo
touch "${repo}/untracked"
if "${repo}/scripts/ci-check.sh" all >"${base}/stdout" 2>"${base}/stderr"; then pass "all on a dirty tree still reports its stages"; else fail "all on a dirty tree still reports its stages"; fi
check "all on a dirty tree writes no note" no_note
check "all on a dirty tree says so" said 'will not be recorded'

if ((failures > 0)); then
  echo "test-attest: ${failures} failed" >&2
  exit 1
fi
echo "test-attest: all passed"
