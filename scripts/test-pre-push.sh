#!/usr/bin/env bash
# Tests for the pre-push gate: real pushes to a throwaway bare repository (CRM-T187). The fixture's
# ci-check.sh fails if a file called FAIL is in the commit, so nothing in the gate is stubbed.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# Run from inside a hook, git's variables would point these fixtures at the real repository.
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
pushes() { if git push -q origin "$@" >/dev/null 2>&1; then return 0; else return 1; fi; }
remote_has() { [[ "$(git rev-parse HEAD)" == "$(git -C ../remote.git rev-parse "$1" 2>/dev/null)" ]]; }

git init -q --bare "${base}/remote.git"
git init -q -b main "${base}/clone"
cd "${base}/clone"
git config user.name test
git config user.email test@example.invalid
git remote add origin ../remote.git
mkdir scripts
cp "${here}/pre-push-gate.sh" "${here}/install-pre-push.sh" scripts/
cat > scripts/ci-check.sh <<'FIXTURE'
#!/usr/bin/env bash
[[ "$1" == pre-push && ! -e FAIL ]]
FIXTURE
chmod +x scripts/*.sh
git add scripts
git commit -qm one

echo '#!/bin/sh' > .git/hooks/pre-push
if scripts/install-pre-push.sh >/dev/null 2>&1; then
  fail "the installer replaces a hook that is not ours"
else
  pass "the installer leaves a hook that is not ours"
fi
rm .git/hooks/pre-push
scripts/install-pre-push.sh >/dev/null
scripts/install-pre-push.sh >/dev/null && pass "the installer can run twice"

if pushes main && remote_has main; then pass "a good commit is pushed"; else fail "a good commit is pushed"; fi

touch FAIL
git commit -q --allow-empty -m two
if pushes main && remote_has main; then
  pass "an uncommitted file does not block a push"
else
  fail "an uncommitted file does not block a push"
fi

git add FAIL
git commit -qm three
rm FAIL
if pushes main || remote_has main; then
  fail "a bad commit is refused even when the working tree is clean of it"
else
  pass "a bad commit is refused even when the working tree is clean of it"
fi
[[ ! -d .git/ci-gate.lock ]] && pass "a refused push releases the lock" || fail "a refused push releases the lock"

if CRM_SKIP_PRE_PUSH="" git push -q origin main >/dev/null 2>&1; then
  fail "an empty skip variable does not skip"
else
  pass "an empty skip variable does not skip"
fi

if CRM_SKIP_PRE_PUSH="test reason" git push -q origin main >/dev/null 2>&1 && remote_has main; then
  pass "a skip with a reason pushes"
else
  fail "a skip with a reason pushes"
fi
grep -q "$(git rev-parse HEAD) test reason" .git/ci-gate-skips.log \
  && pass "a skip is recorded with its commit and reason" \
  || fail "a skip is recorded with its commit and reason"

git rm -q FAIL
git commit -qm four
if pushes main && remote_has main; then pass "a fixed commit is pushed"; else fail "a fixed commit is pushed"; fi

git checkout -q --orphan old
git rm -rqf scripts
echo page > index.html
git add index.html
git commit -qm old
if pushes old && remote_has old; then
  pass "a checkout with no gate script pushes"
else
  fail "a checkout with no gate script pushes"
fi

# As `git subtree push` does: from a full checkout, a commit whose tree has only the site.
site="$(git rev-parse HEAD)"
git checkout -q main
if pushes "${site}:refs/heads/site" && [[ "${site}" == "$(git -C ../remote.git rev-parse site)" ]]; then
  pass "a commit with no ci-check.sh is not gated"
else
  fail "a commit with no ci-check.sh is not gated"
fi

if pushes --delete site; then pass "deleting a branch is not gated"; else fail "deleting a branch is not gated"; fi

((failures == 0)) || exit 1
echo "pre-push gate: all tests passed"
