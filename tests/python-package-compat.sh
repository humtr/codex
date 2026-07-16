#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-python-compat.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
    python3 -B -m wrapper.cli --help >"$TMP_DIR/wrapper.help"
grep -F 'Internal helper interface' "$TMP_DIR/wrapper.help" >/dev/null

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli --help >"$TMP_DIR/legacy.help"
grep -F 'Internal helper interface' "$TMP_DIR/legacy.help" >/dev/null

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - "$ROOT_DIR" <<'PYTHON'
from pathlib import Path
import sys

root = Path(sys.argv[1])
from wrapper import canon as wrapper_canon
wrapper_report = wrapper_canon.audit(root)
assert wrapper_report["status"] == "ok", wrapper_report

for name in list(sys.modules):
    if name == "codex_termux" or name.startswith("codex_termux."):
        sys.modules.pop(name)
from codex_termux import canon as legacy_canon
legacy_report = legacy_canon.audit(root)
assert legacy_report["status"] == "ok", legacy_report
assert legacy_report["findings"] == wrapper_report["findings"]
PYTHON

HOME="$TMP_DIR/home" \
PREFIX="$TMP_DIR/prefix" \
CODEX_TERMUX_AUTO_UPDATE=0 \
bash -c '. "$1"; [ "$(wrapper_package_root)" = "$2/src" ]; wrapper_cmd termux-version-help' \
    _ "$ROOT_DIR/lib/codex-termux.sh" "$ROOT_DIR" >"$TMP_DIR/shell.out"
grep -F 'Usage: codex termux version' "$TMP_DIR/shell.out" >/dev/null

printf 'python-package-compat: ok\n'
