#!/usr/bin/env bash
# Schedule scripts/nightly.sh with launchd (CRM-T191): `nightly` at 02:17 every day and `weekly` at
# 08:17 on Sunday, because a nightly run took 2h54 and the two share a lock. launchd runs a missed job when the machine wakes.
#
#   scripts/install-nightly.sh            # install and load
#   scripts/install-nightly.sh --remove   # unload and delete
#   scripts/install-nightly.sh --print    # show the nightly agent; change nothing
#
# launchd starts with a bare PATH, so the PATH this runs under is written into each agent.
set -euo pipefail
cd "$(dirname "$0")/.."

repo="${PWD}"
agents="${HOME}/Library/LaunchAgents"
domain="gui/$(id -u)"

plist() {
  local kind="$1" schedule="$2"
  cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.contact-relationship-management.${kind}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${repo}/scripts/nightly.sh</string>
    <string>${kind}</string>
  </array>
  <key>WorkingDirectory</key><string>${repo}</string>
  <key>EnvironmentVariables</key>
  <dict><key>PATH</key><string>${PATH}</string></dict>
  <key>StartCalendarInterval</key>
  <dict>${schedule}</dict>
  <key>StandardOutPath</key><string>/tmp/crm-${kind}.out</string>
  <key>StandardErrorPath</key><string>/tmp/crm-${kind}.err</string>
</dict>
</plist>
PLIST
}

nightly_schedule='<key>Hour</key><integer>2</integer><key>Minute</key><integer>17</integer>'
weekly_schedule='<key>Weekday</key><integer>0</integer><key>Hour</key><integer>8</integer><key>Minute</key><integer>17</integer>'

case "${1:-}" in
  --print)
    plist nightly "${nightly_schedule}"
    ;;
  --remove)
    for kind in nightly weekly; do
      launchctl bootout "${domain}/com.contact-relationship-management.${kind}" 2>/dev/null || true
      rm -f "${agents}/com.contact-relationship-management.${kind}.plist"
    done
    echo "removed"
    ;;
  "")
    mkdir -p "${agents}"
    plist nightly "${nightly_schedule}" >"${agents}/com.contact-relationship-management.nightly.plist"
    plist weekly "${weekly_schedule}" >"${agents}/com.contact-relationship-management.weekly.plist"
    for kind in nightly weekly; do
      file="${agents}/com.contact-relationship-management.${kind}.plist"
      plutil -lint -s "${file}"
      launchctl bootout "${domain}/com.contact-relationship-management.${kind}" 2>/dev/null || true
      launchctl bootstrap "${domain}" "${file}"
    done
    echo "installed; results go to $(git rev-parse --git-common-dir)/ci-nightly-last-run.md and ci-weekly-last-run.md"
    ;;
  *)
    echo "usage: $0 [--print|--remove]" >&2
    exit 2
    ;;
esac
