#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'termux rebuild smoke: FAIL: %s\n' "$*" >&2
    exit 1
}

case "${PREFIX:-}" in
    /data/data/com.termux/files/usr)
        ;;
    *)
        fail 'this test must run in a real Termux environment'
        ;;
esac

command -v codex >/dev/null 2>&1 || fail 'codex command is not installed on PATH'

printf '== install-local-support ==\n'
bash "$ROOT_DIR/bin/install-local.sh" support

printf '== rebuild-smoke ==\n'
CODEX_TERMUX_AUTO_UPDATE=0 \
CODEX_TERMUX_REQUIRE_CHECKOUT_MATCH=1 \
CODEX_TERMUX_RUN_REBUILD_SMOKE=1 \
    bash "$ROOT_DIR/tests/run-termux.sh"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
report="$tmp_dir/doctor.json"

printf '== runtime-compat-tools ==\n'
codex termux doctor --json >"$report"

PYTHONDONTWRITEBYTECODE=1 python3 -B - "$report" <<'PYTHON'
import filecmp
import json
import os
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
data = json.loads(report_path.read_text(encoding="utf-8"))
paths = data.get("paths")
if not isinstance(paths, dict):
    raise SystemExit("doctor report missing paths object")

runtime_root_raw = paths.get("current_target") or paths.get("currentTarget")
manager_root_raw = paths.get("manager_target") or paths.get("managerTarget")
if not runtime_root_raw:
    raise SystemExit("doctor report missing paths.current_target")
if not manager_root_raw:
    raise SystemExit("doctor report missing paths.manager_target")

runtime_root = Path(runtime_root_raw)
manager_root = Path(manager_root_raw)
checks = (
    (runtime_root / "codex-path" / "bwrap", manager_root / "bwrap-termux-compat.py"),
    (runtime_root / "codex-path" / "rg", manager_root / "rg-termux-shim.sh"),
)

for runtime_file, manager_file in checks:
    if runtime_file.is_symlink():
        raise SystemExit(f"runtime compatibility tool is a symlink: {runtime_file}")
    if not runtime_file.is_file():
        raise SystemExit(f"runtime compatibility tool is not a regular file: {runtime_file}")
    if not os.access(runtime_file, os.X_OK):
        raise SystemExit(f"runtime compatibility tool is not executable: {runtime_file}")
    if not manager_file.is_file():
        raise SystemExit(f"manager compatibility source is not readable: {manager_file}")
    if not filecmp.cmp(runtime_file, manager_file, shallow=False):
        raise SystemExit(
            f"runtime compatibility tool differs from manager source: "
            f"{runtime_file} != {manager_file}"
        )
PYTHON

printf 'termux rebuild smoke: ok\n'
