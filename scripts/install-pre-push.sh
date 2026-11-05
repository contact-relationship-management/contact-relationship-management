#!/usr/bin/env bash
# Install the pre-push gate in this clone (CRM-T187). To remove it, delete .git/hooks/pre-push.
set -euo pipefail
cd "$(dirname "$0")/.."

marker="Installed by scripts/install-pre-push.sh"
hook="$(git rev-parse --git-path hooks)/pre-push"

if [[ -e "${hook}" ]] && ! grep -q "${marker}" "${hook}"; then
  echo "refusing to replace ${hook}: it is not ours" >&2
  exit 1
fi

mkdir -p "$(dirname "${hook}")"
cat > "${hook}" <<HOOK
#!/bin/sh
# ${marker}.
gate="\$(git rev-parse --show-toplevel)/scripts/pre-push-gate.sh"
# A branch from before the gate has no script to run.
[ -x "\${gate}" ] || exit 0
exec "\${gate}" "\$@"
HOOK
chmod +x "${hook}"
echo "installed ${hook}"
