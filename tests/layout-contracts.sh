#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PYTHONDONTWRITEBYTECODE=1 python3 -B - "$ROOT_DIR" <<'PYTHON'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
contract_path = root / "config/layout-contracts.json"
contract = json.loads(contract_path.read_text(encoding="utf-8"))

assert contract["schema"] == 1
assert contract["baseline_commit"] == "803fef15fe17960bdf8e9f7d5d199b813d08224b"
assert contract["source_layers"] == {
    "bin": "public execution and installation entrypoints",
    "shell": "shell orchestration domains",
    "src/wrapper": "Python policy and domain implementation",
    "libexec": "installed internal executables",
    "native": "native launcher sources",
    "lib": "public compatibility facades",
    "tools": "developer and release tooling",
}

preserved = set(contract["preserved_contracts"])
for required in (
    "public codex argument passthrough",
    "codex termux command compatibility",
    "state and registry schema 3",
    "FD 33 resolver inheritance",
    "FD 34 system config inheritance",
    "runtime and verified tuple semantics",
    "notification ID and dedupe behavior",
):
    assert required in preserved, required

notify = contract["notification_baseline"]
assert notify["fallback_title"] == "Codex: General"
assert notify["stop_suffix"] == ""
assert notify["max_lines"] == 10
assert notify["default_content_chars"] == 0
assert notify["preserve_newlines"] is True
assert notify["single_line_trailing_newline"] is True
assert notify["click_actions"] == ["none", "open_termux", "open_tmux"]

non_goals = set(contract["non_goals"])
assert "Rust rewrite" in non_goals
assert "user .gitignore changes" in non_goals

manifest = json.loads((root / "codex-wrapper.manifest.json").read_text(encoding="utf-8"))
assert ".gitignore" not in manifest.get("protected_paths", [])

release_source = (root / "tools/codex_termux/release.py").read_text(encoding="utf-8")
assert '".gitignore"' in release_source
PYTHON

printf 'layout-contracts: ok\n'
