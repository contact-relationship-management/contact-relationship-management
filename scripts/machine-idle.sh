#!/usr/bin/env bash
# May the scheduled jobs start now (CRM-T191)? Exit 0 if this Mac is on wall power and its 5-minute
# load average is under half its cores (or CRM_NIGHTLY_MAX_LOAD). Otherwise print why and exit 1.
# A value that cannot be read counts as no: these jobs take all the cores for hours.
set -uo pipefail

power="$(pmset -g batt 2>/dev/null | head -1)"
if [[ "${power}" != *"'AC Power'"* ]]; then
  echo "not on wall power (${power:-pmset gave no answer})"
  exit 1
fi

cores="$(sysctl -n hw.ncpu 2>/dev/null)"
read -r _ _ load _ <<<"$(sysctl -n vm.loadavg 2>/dev/null)"
limit="${CRM_NIGHTLY_MAX_LOAD:-$((${cores:-0} / 2))}"
number='^[0-9]+([.][0-9]+)?$'
if ! [[ "${load:-}" =~ ${number} && "${limit}" =~ ${number} ]] || [[ "${limit}" == 0 ]]; then
  echo "could not read the load average (load '${load:-}', limit '${limit}')"
  exit 1
fi
if ! awk -v now="${load}" -v most="${limit}" 'BEGIN { exit !(now < most) }'; then
  echo "load average ${load} is not under ${limit}"
  exit 1
fi
