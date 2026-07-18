#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SLICE_NAME="${1:-}"
BASE_REF="${2:-}"
HEAD_REF="${3:-HEAD}"

if [ -z "$SLICE_NAME" ]; then
    printf 'usage: %s <slice> [base] [head]\n' "$0" >&2
    exit 64
fi

PYTHONDONTWRITEBYTECODE=1 python3 -B - \
    "$ROOT_DIR" "$SLICE_NAME" "$BASE_REF" "$HEAD_REF" <<'PYTHON'
from __future__ import annotations

import fnmatch
import json
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
slice_name = sys.argv[2]
base_ref_arg = sys.argv[3]
head_ref = sys.argv[4]

contract = json.loads(
    (root / "config/refactor-boundaries.json").read_text(encoding="utf-8")
)
slices = contract["slices"]
if slice_name not in slices:
    available = ", ".join(contract["dependency_order"])
    raise SystemExit(f"unknown refactor slice {slice_name!r}; expected one of: {available}")

base_ref = base_ref_arg or contract["baseline_commit"]
item = slices[slice_name]
allowed_patterns = item["allowed_paths"]
required_groups = item["required_change_groups"]


def git(*args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(root), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise SystemExit(f"git {' '.join(args)} failed: {detail}")
    return completed.stdout


git("rev-parse", "--verify", f"{base_ref}^{{commit}}")
git("rev-parse", "--verify", f"{head_ref}^{{commit}}")
changed = [
    line.strip()
    for line in git(
        "diff",
        "--name-only",
        "--diff-filter=ACDMRTUXB",
        f"{base_ref}...{head_ref}",
    ).splitlines()
    if line.strip()
]

if not changed:
    raise SystemExit(
        f"refactor-scope: no changed paths between {base_ref} and {head_ref}"
    )


def matches(path: str, patterns: list[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


outside = [path for path in changed if not matches(path, allowed_patterns)]
if outside:
    print(f"refactor-scope: slice={slice_name}", file=sys.stderr)
    print("refactor-scope: paths outside the declared boundary:", file=sys.stderr)
    for path in outside:
        print(f"  - {path}", file=sys.stderr)
    print("refactor-scope: allowed patterns:", file=sys.stderr)
    for pattern in allowed_patterns:
        print(f"  - {pattern}", file=sys.stderr)
    raise SystemExit(1)

missing_groups: list[str] = []
for group_name, patterns in required_groups.items():
    if not any(matches(path, patterns) for path in changed):
        missing_groups.append(group_name)

if missing_groups:
    print(f"refactor-scope: slice={slice_name}", file=sys.stderr)
    print(
        "refactor-scope: required change groups missing: "
        + ", ".join(missing_groups),
        file=sys.stderr,
    )
    for group_name in missing_groups:
        print(f"  {group_name}:", file=sys.stderr)
        for pattern in required_groups[group_name]:
            print(f"    - {pattern}", file=sys.stderr)
    raise SystemExit(1)

print(f"refactor-scope: slice={slice_name}")
print(f"refactor-scope: base={base_ref}")
print(f"refactor-scope: head={head_ref}")
for path in changed:
    print(f"refactor-scope: path={path}")
print("refactor-scope: ok")
PYTHON
