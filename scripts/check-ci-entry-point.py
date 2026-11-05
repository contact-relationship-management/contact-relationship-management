#!/usr/bin/env python3
"""Fail if a CI step does its checking anywhere but `scripts/ci-check.sh` (CRM-T185).

The local and hosted runs mean the same thing only while both call the same stages, so a step in
`ci.yml`, `mutants.yml` or `.woodpecker.yml` must be a call to `scripts/ci-check.sh <stage>`, or an
exception listed in EXCEPTIONS below. An exception installs a tool; it never checks anything.

It also fails if a call names a stage that does not exist, if a stage is called by no hosted
workflow, or if `all` leaves out a stage that NOT_IN_ALL does not list.

    python3 scripts/check-ci-entry-point.py

Standard library only: Woodpecker's images and the hosted runners do not all carry PyYAML.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENTRY = ROOT / "scripts" / "ci-check.sh"
HOSTED = [ROOT / ".github/workflows/ci.yml", ROOT / ".github/workflows/mutants.yml"]
OTHER = [ROOT / ".woodpecker.yml"]

CALL = re.compile(r"^scripts/ci-check\.sh\s+(\S+.*)$")
MATRIX_CALL = "${{ matrix.stage }}"
MATRIX = re.compile(r"^\s*stage:\s*\[([^\]]*)\]")
RUN = re.compile(r"^(\s*)(?:-\s+)?run:\s*(.*)$")
COMMANDS = re.compile(r"^(\s*)commands:\s*$")
ITEM = re.compile(r"^\s*-\s+(.*)$")
STAGE = re.compile(r"^  ([a-z][a-z0-9-]*)\)\s*$", re.MULTILINE)
ALL_STAGES = re.compile(r"^ALL_STAGES=\(([^)]*)\)", re.MULTILINE)

# Each pattern, and why it is allowed. These put a tool on the runner; none of them checks the code.
EXCEPTIONS = {
    r"^pipx install rumdl==\S+$": "rumdl is not in install-action; the version is pinned",
    r"^curl -sSL -o gitleaks\.tgz https://github\.com/gitleaks/": "the scanner, by version",
    r"^echo \"[0-9a-f]{64}  gitleaks\.tgz\" \| sha256sum -c -$": "the scanner's published SHA-256",
    r"^sudo tar -xzf gitleaks\.tgz -C /usr/local/bin gitleaks$": "puts the scanner on the PATH",
    r"^rustup component add \S+$": "a toolchain component the image lacks",
    r"^apt-get update && apt-get install -y --no-install-recommends \S+$": "a system package",
    r"^npm install --global pnpm@\S+$": "pnpm, which Node no longer ships",
    r"^cargo install --locked [\w -]+$": "a cargo tool the image lacks",
    r"^curl -fsSL https://deb\.nodesource\.com/setup_\d+\.x \| bash - && apt-get install -y nodejs$": "Node in a Rust image",
}

# Stages that `all` may leave out, and why.
NOT_IN_ALL = {
    "all": "it is the list",
    "mutants": "it takes hours (spec/local-ci-cd.md), so it is a scheduled job",
    "pre-push": "it repeats stages `all` runs",
}

# Stages for this machine only, which no hosted workflow calls, and why.
LOCAL_ONLY = {
    "all": "the hosted workflow runs the same stages as separate jobs",
    "pre-push": "the hook's subset of stages the hosted workflow already runs",
}


def run_commands(text: str) -> list[tuple[int, str]]:
    """Every shell line under a GitHub Actions `run:` key, with its line number."""
    lines = text.splitlines()
    found = []
    for number, line in enumerate(lines, 1):
        m = RUN.match(line)
        if not m:
            continue
        value = m.group(2).strip()
        if value and value[0] not in "|>":
            found.append((number, value))
            continue
        key = line.index("run:")
        for offset, body in enumerate(lines[number:], number + 1):
            if body.strip() and len(body) - len(body.lstrip()) <= key:
                break
            if body.strip() and not body.strip().startswith("#"):
                found.append((offset, body.strip()))
    return found


def woodpecker_commands(text: str) -> list[tuple[int, str]]:
    """Every item under a Woodpecker `commands:` key, with its line number."""
    lines = text.splitlines()
    found = []
    for number, line in enumerate(lines, 1):
        m = COMMANDS.match(line)
        if not m:
            continue
        for offset, body in enumerate(lines[number:], number + 1):
            if not body.strip() or body.strip().startswith("#"):
                continue
            if len(body) - len(body.lstrip()) <= len(m.group(1)):
                break
            if item := ITEM.match(body):
                found.append((offset, item.group(1).strip()))
            else:
                found.append((offset, body.strip()))
    return found


def declared_stages(script: str) -> set[str]:
    """The stages `ci-check.sh` has a case label for."""
    return set(STAGE.findall(script))


def all_stages(script: str) -> set[str]:
    """The stages `all` runs."""
    m = ALL_STAGES.search(script)
    return set(m.group(1).split()) if m else set()


def matrix_stages(text: str) -> set[str]:
    """The stages named in a workflow's `stage: [...]` matrix."""
    named: set[str] = set()
    for line in text.splitlines():
        if m := MATRIX.match(line):
            named |= {s.strip() for s in m.group(1).split(",") if s.strip()}
    return named


def check_commands(
    name: str, commands: list[tuple[int, str]], stages: set[str], matrix: set[str]
) -> tuple[list[str], set[str]]:
    """The offending commands in one file, and the stages it calls."""
    bad = []
    called: set[str] = set()
    for number, command in commands:
        if m := CALL.match(command):
            stage = m.group(1).strip()
            if stage == MATRIX_CALL:
                called |= matrix
            else:
                called.add(stage)
        elif not any(re.search(pattern, command) for pattern in EXCEPTIONS):
            bad.append(f"{name}:{number}: not a ci-check.sh call or a listed exception: {command}")
    bad += [f"{name}: calls a stage ci-check.sh does not have: {s}" for s in sorted(called - stages)]
    return bad, called


def check_coverage(stages: set[str], called: set[str], in_all: set[str]) -> list[str]:
    """A stage no hosted workflow calls, and a stage `all` leaves out or invents."""
    bad = [f"ci-check.sh: no hosted workflow calls stage {s}" for s in sorted(stages - called - set(LOCAL_ONLY))]
    expected = stages - set(NOT_IN_ALL)
    bad += [f"ci-check.sh: `all` leaves out stage {s}" for s in sorted(expected - in_all)]
    bad += [f"ci-check.sh: `all` runs what is not a stage: {s}" for s in sorted(in_all - expected)]
    return bad


def main() -> int:
    script = ENTRY.read_text()
    stages = declared_stages(script)
    bad = []
    hosted_calls: set[str] = set()
    for path in HOSTED:
        text = path.read_text()
        found, called = check_commands(path.name, run_commands(text), stages, matrix_stages(text))
        bad += found
        hosted_calls |= called
    for path in OTHER:
        found, _ = check_commands(path.name, woodpecker_commands(path.read_text()), stages, set())
        bad += found
    bad += check_coverage(stages, hosted_calls, all_stages(script))
    for line in bad:
        print(line, file=sys.stderr)
    if bad:
        return 1
    print(f"every CI step goes through ci-check.sh ({len(stages)} stages)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
