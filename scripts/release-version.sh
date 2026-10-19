#!/usr/bin/env bash
# The release version of an image, taken from its manifest, and a check that
# it agrees with the git tag.   Usage: scripts/release-version.sh <service|ui> <tag>
# Prints the image version (the tag without its leading "v") on success.
# A tag may carry a pre-release suffix (v0.1.2-rc.1); the part before the
# hyphen must equal the manifest version, so a release cannot be tagged
# without bumping Cargo.toml / package.json first.
set -euo pipefail
cd "$(dirname "$0")/.."

kind="${1:?usage: $0 <service|ui> <tag>}"
tag="${2:?usage: $0 <service|ui> <tag>}"

case "${kind}" in
  service)
    manifest=contact-relationship-management-service-with-rust/Cargo.toml
    declared="$(sed -n 's/^version = "\(.*\)"$/\1/p' "${manifest}" | head -1)"
    ;;
  ui)
    manifest=contact-relationship-management-ui-with-svelte/package.json
    declared="$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "${manifest}" | head -1)"
    ;;
  *) echo "unknown image: ${kind}" >&2; exit 2 ;;
esac

version="${tag#v}"
if [[ "${tag}" != v* || -z "${declared}" || "${version%%-*}" != "${declared}" ]]; then
  echo "tag ${tag} does not match ${manifest} version '${declared}'" >&2
  echo "bump the version in the manifest before tagging" >&2
  exit 1
fi
echo "${version}"
