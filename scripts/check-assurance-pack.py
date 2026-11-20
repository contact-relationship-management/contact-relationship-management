#!/usr/bin/env python3
"""Hold the supplier assurance pack's matrix to the spec (CRM-R99, CRM-T276).

Each row of spec/compliance/supplier-assurance/shared-responsibility.md names a requirement and a
row of the control mapping. This fails when a row names a requirement that is not a heading in
spec/requirements.md or a control that is not the first cell of a row in control-mapping.md, or
when a requirement from CRM-R100 to CRM-R109 has no row. Run by `scripts/ci-check.sh lint`.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "spec"
MATRIX = SPEC / "compliance" / "supplier-assurance" / "shared-responsibility.md"
COVERED = [f"CRM-R{n}" for n in range(100, 110)]


def table_rows(text: str) -> list[list[str]]:
    """The cells of every Markdown table row, header and divider rows left out."""
    rows = []
    for line in text.splitlines():
        if not line.startswith("|") or re.match(r"^\|\s*-", line):
            continue
        rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return rows


def main() -> int:
    requirements = set(re.findall(r"^## (CRM-R\d+) ", (SPEC / "requirements.md").read_text(), re.M))
    controls = {row[0] for row in table_rows((SPEC / "compliance" / "control-mapping.md").read_text())}
    rows = table_rows(MATRIX.read_text())
    if not rows or rows[0][:3] != ["Area", "Requirement", "Control mapping row"]:
        print(f"{MATRIX.relative_to(ROOT)}: the matrix table is missing or its header changed")
        return 1
    problems = []
    named = set()
    for row in rows[1:]:
        if len(row) != len(rows[0]):
            problems.append(f"row {row[0]!r} has {len(row)} cells, not {len(rows[0])}")
            continue
        area, requirement, control = row[0], row[1], row[2]
        named.add(requirement)
        if requirement not in requirements:
            problems.append(f"row {area!r} names {requirement}, which is not a requirement")
        if control not in controls:
            problems.append(f"row {area!r} names {control!r}, which is not a control-mapping row")
        if any(not cell for cell in row):
            problems.append(f"row {area!r} has an empty cell")
    for requirement in COVERED:
        if requirement not in named:
            problems.append(f"{requirement} has no row")
    for problem in problems:
        print(f"{MATRIX.relative_to(ROOT)}: {problem}")
    if problems:
        return 1
    print(f"assurance pack: {len(rows) - 1} rows, each naming a requirement and a control")
    return 0


if __name__ == "__main__":
    sys.exit(main())
