#!/usr/bin/env bash
# Fetch the Lily Design System's Swift and Kotlin libraries at the commit pinned in lily-design-system.lock, into
# vendor/lily-design-system/ (spec/mobile-apps, CRM-D28). A shallow, sparse clone: only the two subdirectories.
# Idempotent: when vendor/ already holds the pinned commit, it does nothing.
#   CONTACT_RELATIONSHIP_MANAGEMENT_LILY_REMOTE=<url or path>   fetch from another clone of the same history
set -euo pipefail
cd "$(dirname "$0")/.."

lock=lily-design-system.lock
remote="$(sed -n 's/^remote=//p' "${lock}")"
commit="$(sed -n 's/^commit=//p' "${lock}")"
if [[ ! "${commit}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "fetch-lily: ${lock} must pin a 40-character commit, not '${commit}'" >&2
  exit 2
fi
remote="${CONTACT_RELATIONSHIP_MANAGEMENT_LILY_REMOTE:-${remote}}"
dest=vendor/lily-design-system

if [[ -d "${dest}/.git" ]] && [[ "$(git -C "${dest}" rev-parse HEAD 2>/dev/null)" == "${commit}" ]]; then
  exit 0
fi

rm -rf "${dest}"
mkdir -p "${dest}"
git -C "${dest}" init --quiet
git -C "${dest}" remote add origin "${remote}"
git -C "${dest}" sparse-checkout set --no-cone /lily-design-system-swift/ /lily-design-system-kotlin/
git -C "${dest}" fetch --quiet --depth 1 --filter=blob:none origin "${commit}"
git -C "${dest}" checkout --quiet FETCH_HEAD

got="$(git -C "${dest}" rev-parse HEAD)"
if [[ "${got}" != "${commit}" ]]; then
  echo "fetch-lily: expected ${commit}, got ${got}" >&2
  exit 1
fi
echo "fetch-lily: ${dest} at ${commit}"
