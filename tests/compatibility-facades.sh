#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PYTHONDONTWRITEBYTECODE=1 python3 -B - "$ROOT_DIR" <<'PYTHON'
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
contract = json.loads((root / "config/layout-contracts.json").read_text(encoding="utf-8"))
retirement = contract["compatibility_retirement"]
rules = retirement["rules"]
forbidden = [re.compile(pattern) for pattern in retirement["forbidden_patterns"]]

expected_targets = {
    "lib/codex-termux/*.sh": "shell/",
    "tools/codex_termux/*.py": "wrapper",
    "tools/build-runtime.py": "libexec/build-runtime.py",
    "tools/bwrap-termux-compat.py": "libexec/bwrap-termux-compat.py",
    "tools/rg-termux-shim.sh": "libexec/rg-termux-shim.sh",
    "tools/codex-launcher.c": "../native/codex-launcher.c",
    "tools/codex-turn-notify.sh": "libexec/notify",
    "tools/termux-notify.sh": "libexec/notify",
    "tools/smoke-termux-wrapper.sh": "smoke-wrapper.sh",
}

checked: set[str] = set()
for rule in rules:
    pattern = rule["glob"]
    matches = sorted(root.glob(pattern))
    assert matches, f"compatibility facade glob is empty: {pattern}"
    for path in matches:
        assert path.is_file(), path
        relative = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8")
        line_count = len(text.splitlines())
        assert line_count <= int(rule["max_lines"]), (
            relative,
            line_count,
            rule["max_lines"],
        )
        for regex in forbidden:
            assert regex.search(text) is None, (relative, regex.pattern)
        target = expected_targets[pattern]
        assert target in text, (relative, target)
        if path.suffix == ".py":
            assert re.search(r"^\s*(def|class)\s+", text, re.MULTILINE) is None, relative
        if path.suffix == ".sh":
            assert re.search(
                r"^\s*(?:function\s+)?[A-Za-z_][A-Za-z0-9_]*\s*\(\)",
                text,
                re.MULTILINE,
            ) is None, relative
        checked.add(relative)

assert "tools/codex-launcher.c" in checked
assert "tools/codex-turn-notify.sh" in checked
assert "tools/termux-notify.sh" in checked
assert contract["target_paths"]["support_recovery_journal"].endswith(
    "/support-recovery.json"
)
assert retirement["window"]
PYTHON

printf 'compatibility-facades: ok\n'
