#!/usr/bin/env python3
"""Checked arithmetic, the greps CI can run (spec/rust-checked-arithmetic, RCA-ENF-5 and RCA-ENF-7).

Fails on, in production code of the service and domain crates:

- `saturating_*`, `wrapping_*`, `overflowing_*` and `.clamp(` (RCA-INT-4, RCA-INT-5, RCA-INT-6);
- a `checked_*` call whose result is unwrapped (`unwrap`, `expect`, `unwrap_unchecked`,
  `unwrap_or_default`, RCA-NONE-1);
- a `parse()` or an environment or query lookup whose error is dropped and replaced (RCA-IN-2).

Production code is a file under `src/` up to its first `#[cfg(test)]`, comments left out. A line that
must keep one of these says why with `// RCA-IN-3: absent only` and the declaration it cites.
Run from the repository root; exit status 1 when there is a hit.
"""

import pathlib
import re
import sys

ROOTS = [
    "contact-relationship-management-service-with-rust/src",
    "contact-relationship-management-service-with-rust/contact-relationship-management-domain/src",
]
# (label, pattern, fails). RCA-ENF-5 is a MUST and fails the build. RCA-ENF-7 is a SHOULD: its hits are
# listed as warnings until the inputs they name are converted (spec/arithmetic/index.md, "Where it stands").
CHECKS = [
    ("RCA-INT-4/5/6: saturating, wrapping, overflowing or clamp",
     re.compile(r"\b(saturating|wrapping|overflowing)_\w+|\.clamp\("), True),
    ("RCA-NONE-1: a checked_* result is unwrapped",
     re.compile(r"checked_\w+\([^;]*?\)\s*\.(unwrap|expect|unwrap_unchecked|unwrap_or_default)\b", re.S), True),
    ("RCA-IN-2: a parse error is dropped and replaced",
     re.compile(r"\.parse(::<[^>]+>)?\(\)[^;]*?\.(ok\(\)|unwrap_or|unwrap_or_default|unwrap_or_else)\b", re.S), False),
    ("RCA-IN-2: an environment or query lookup is dropped and replaced",
     re.compile(r"(env::var|var_os|get_param|query)\([^;]*?\.ok\(\)[^;]*?\.(unwrap_or|unwrap_or_default|unwrap_or_else)\b", re.S), False),
]
# A file whose whole content is test support: operators for tests only (RCA-EXC-5).
SKIP_BLOCK = re.compile(r"(#\[[^\]]*\]\s*)*pub mod test_operators \{.*?\n\}\n", re.S)


def production(text: str) -> str:
    cut = text.find("#[cfg(test)]")
    text = text if cut < 0 else text[:cut]
    text = SKIP_BLOCK.sub("", text)
    return "\n".join(
        "" if line.lstrip().startswith("//") else re.sub(r"\s//.*$", "", line)
        for line in text.split("\n")
    )


def main() -> int:
    hits = 0
    warnings = 0
    for root in ROOTS:
        for path in sorted(pathlib.Path(root).rglob("*.rs")):
            code = production(path.read_text())
            raw = path.read_text().split("\n")
            for label, pattern, fails in CHECKS:
                for found in pattern.finditer(code):
                    line = code.count("\n", 0, found.start()) + 1
                    if "RCA-IN-3: absent only" in raw[line - 1]:
                        continue
                    if fails:
                        hits += 1
                    else:
                        warnings += 1
                    print(f"{path}:{line}: {'error' if fails else 'warning'}: {label}")
    if hits:
        print(f"check-checked-arithmetic: {hits} hit(s); see spec/rust-checked-arithmetic", file=sys.stderr)
        return 1
    print(f"check-checked-arithmetic: no errors, {warnings} warning(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
