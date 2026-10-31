#!/usr/bin/env python3
"""Fail if a GitHub Actions workflow uses an action, or a file names a container image, that is not pinned.

A tag such as `@v4` can be moved to different code after it is reviewed; a 40-character commit
SHA cannot (WPM-R119, WPM-T194). Write `uses: owner/repo@<sha> # v4` so a person can still read
the version; Dependabot updates both. Local actions (`./...`) and `docker://` references with a
digest are fine.

Container images (service containers in workflows, `image:` in compose files, `FROM` and `COPY --from`
in a Containerfile, a registry name in a script) must carry `@sha256:<digest>` (WPM-T231): a tag moves.

    python3 scripts/check-actions-pinned.py                  # actions and images
    python3 scripts/check-actions-pinned.py --actions-only   # actions only
"""
import subprocess
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
USES = re.compile(r"^\s*(?:-\s+)?uses:\s*(\S+)")
PINNED = re.compile(r"^[\w.-]+/[\w./-]+@[0-9a-f]{40}$")


IMAGE_KEY = re.compile(r"^\s*(?:image:|-\s+&\w+)\s*[\"']?([^\s\"']+)")
FROM = re.compile(r"^FROM\s+(?:--platform=\S+\s+)?(\S+)(?:\s+AS\s+(\S+))?", re.IGNORECASE)
COPY_FROM = re.compile(r"--from=(\S+)")
REGISTRY = re.compile(r"\b((?:docker\.io|quay\.io|ghcr\.io)/[\w./:@-]+)")
DIGEST = "@sha256:"


def unpinned_images() -> list[str]:
    """Every container image reference in a tracked file that has no digest."""
    files = subprocess.run(
        ["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout.split()
    bad = []
    for name in files:
        path = ROOT / name
        if not path.is_file() or path.suffix in {".md", ".json", ".lock", ".svg", ".png"}:
            continue
        try:
            lines = path.read_text().splitlines()
        except UnicodeDecodeError:
            continue
        stages: set[str] = set()
        for number, line in enumerate(lines, 1):
            refs = []
            if path.suffix in {".yml", ".yaml"} and (m := IMAGE_KEY.match(line)):
                refs.append(m.group(1))
            if path.name.startswith("Containerfile") or path.name.startswith("Dockerfile"):
                if m := FROM.match(line):
                    if m.group(1) not in stages and m.group(1).lower() != "scratch":
                        refs.append(m.group(1))
                    if m.group(2):
                        stages.add(m.group(2))
                if m := COPY_FROM.search(line):
                    if "/" in m.group(1) and m.group(1) not in stages:
                        refs.append(m.group(1))
            elif path.suffix == ".sh":
                refs += REGISTRY.findall(line)
            for ref in refs:
                # `$VAR` is resolved elsewhere, `*alias` points at an anchor checked where it is defined, and
                # `name:local` is an image the same compose file builds itself.
                if "$" in ref or ref.startswith("*") or ref.endswith(":local") or DIGEST in ref:
                    continue
                bad.append(f"{name}:{number}: image {ref} has no @sha256 digest")
    return bad


def main() -> int:
    bad = []
    for path in sorted((ROOT / ".github").rglob("*.y*ml")):
        for number, line in enumerate(path.read_text().splitlines(), 1):
            m = USES.match(line)
            if not m:
                continue
            ref = m.group(1)
            if ref.startswith("./") or re.search(r"^docker://.+@sha256:[0-9a-f]{64}$", ref):
                continue
            if not PINNED.match(ref):
                bad.append(f"{path.relative_to(ROOT)}:{number}: {ref} is not pinned to a commit SHA")
    actions_only = "--actions-only" in sys.argv[1:]
    if not actions_only:
        bad += unpinned_images()
    for line in bad:
        print(line)
    print(("all actions are pinned" if actions_only else "all actions and images are pinned") if not bad else f"{len(bad)} unpinned reference(s)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
