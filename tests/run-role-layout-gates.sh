#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="${CODEX_TERMUX_VALIDATION_LOG_DIR:-$ROOT_DIR/.validation/role-layout}"
mkdir -p "$LOG_DIR"

run_logged() {
    local name="$1"
    shift
    printf '== %s ==\n' "$name"
    "$@" >"$LOG_DIR/$name.log" 2>&1
    cat "$LOG_DIR/$name.log"
}

run_logged portable bash "$ROOT_DIR/tests/run-portable.sh"

if [ -d /data/data/com.termux/files/usr ] && command -v termux-info >/dev/null 2>&1; then
    run_logged termux env CODEX_TERMUX_AUTO_UPDATE=0 bash "$ROOT_DIR/tests/run-termux.sh"
    run_logged doctor codex termux doctor --json
    PYTHONDONTWRITEBYTECODE=1 python3 -B - "$LOG_DIR/doctor.log" <<'PYTHON'
import json
import sys
from pathlib import Path

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if report.get("overallStatus") != "ok":
    raise SystemExit("doctor overallStatus is not ok")
checks = report.get("checks")
if not isinstance(checks, dict) or not checks or not all(checks.values()):
    raise SystemExit("doctor contains a failed or missing check")
PYTHON
else
    printf 'Termux device gates skipped: this host is not a Termux environment.\n' | tee "$LOG_DIR/termux.skipped.log"
fi

{
    printf 'branch=%s\n' "$(git -C "$ROOT_DIR" branch --show-current 2>/dev/null || true)"
    printf 'commit=%s\n' "$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || true)"
    printf 'date=%s\n' "$(date -Is)"
    printf 'uname=%s\n' "$(uname -a)"
    printf 'prefix=%s\n' "${PREFIX:-}"
} >"$LOG_DIR/environment.txt"

printf 'Role-layout validation logs: %s\n' "$LOG_DIR"
