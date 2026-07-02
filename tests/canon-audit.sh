#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

REPORT="$TMP_DIR/canon-audit.json"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli canon-audit --root "$ROOT_DIR" --strict >"$REPORT"

python3 - "$ROOT_DIR" "$REPORT" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
report = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
metrics = report["metrics"]

assert report["status"] == "ok", report
assert report["findings"] == [], report["findings"]
assert not (root / "GOAL.md").exists()

required_metrics = {
    "cli_py_lines",
    "runtime_shell_lines",
    "state_shell_lines",
    "notify_shell_lines",
    "profile_shell_lines",
    "shell_file_lines",
    "shell_file_functions",
    "cli_registered_commands",
    "shell_helper_commands",
    "unregistered_helper_commands",
    "target_budget_gaps",
}
missing = sorted(required_metrics - set(metrics))
assert not missing, missing

shell_lines = metrics["shell_file_lines"]
assert metrics["runtime_shell_lines"] == shell_lines["lib/codex-termux/runtime.sh"]
assert metrics["state_shell_lines"] == shell_lines["lib/codex-termux/state.sh"]
assert metrics["notify_shell_lines"] == shell_lines["lib/codex-termux/notify.sh"]
assert metrics["profile_shell_lines"] == shell_lines["lib/codex-termux/profile.sh"]

assert metrics["unregistered_helper_commands"] == []
for command in metrics["shell_helper_commands"]:
    assert command in metrics["cli_registered_commands"], command

target_gaps = metrics["target_budget_gaps"]
expected_targets = {
    "cli_py_lines": 250,
    "install_runtime_lines": 375,
    "domain_shell_lines": 1450,
    "runtime_shell_lines": 450,
    "state_shell_lines": 220,
    "notify_shell_lines": 180,
    "profile_shell_lines": 220,
}
for key, target in expected_targets.items():
    if metrics[key] > target:
        assert key in target_gaps, key
        assert target_gaps[key]["target"] == target, target_gaps[key]
        assert target_gaps[key]["value"] == metrics[key], (key, target_gaps[key], metrics[key])
        assert target_gaps[key]["gap"] == metrics[key] - target, target_gaps[key]
    else:
        assert key not in target_gaps, (key, target_gaps.get(key))
PY

printf 'canon-audit: ok\n'
