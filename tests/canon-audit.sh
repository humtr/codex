#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-canon.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
REPORT="$TMP_DIR/canon-audit.json"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
    python3 -B -m wrapper.cli canon-audit --root "$ROOT_DIR" --strict >"$REPORT"

python3 -B - "$ROOT_DIR" "$REPORT" <<'PYTHON'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
report = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
manifest = json.loads((root / "codex-wrapper.manifest.json").read_text(encoding="utf-8"))
metrics = report["metrics"]

assert report["status"] == "ok", report
blockers = [item for item in report["findings"] if item.get("severity") == "blocker"]
assert blockers == [], blockers
assert not (root / "GOAL.md").exists()

required_metrics = {
    "cli_py_lines",
    "build_shell_lines",
    "ui_shell_lines",
    "fs_shell_lines",
    "runtime_shell_lines",
    "state_shell_lines",
    "prompt_shell_lines",
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
for domain in ("build", "ui", "fs", "runtime", "state", "prompt", "notify", "profile"):
    key = f"{domain}_shell_lines"
    assert metrics[key] == shell_lines[f"shell/{domain}.sh"], (key, metrics[key], shell_lines)

assert metrics["unregistered_helper_commands"] == []
for command in metrics["shell_helper_commands"]:
    assert command in metrics["cli_registered_commands"], command

target_gaps = metrics["target_budget_gaps"]
targets = manifest["target_shell_budgets_95"]
for key in (
    "cli_py_lines",
    "install_runtime_lines",
    "domain_shell_lines",
    "runtime_shell_lines",
    "state_shell_lines",
    "notify_shell_lines",
    "profile_shell_lines",
):
    target = targets[key]
    if metrics[key] > target:
        assert key in target_gaps, key
        assert target_gaps[key]["target"] == target, target_gaps[key]
        assert target_gaps[key]["value"] == metrics[key], (key, target_gaps[key], metrics[key])
        assert target_gaps[key]["gap"] == metrics[key] - target, target_gaps[key]
    else:
        assert key not in target_gaps, (key, target_gaps.get(key))
PYTHON

printf 'canon-audit: ok\n'
