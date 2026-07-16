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

HOME="$TMP_DIR/home" \
PREFIX="$TMP_DIR/prefix" \
CODEX_TERMUX_AUTO_UPDATE=0 \
bash -c '. "$1"; [ "$(wrapper_package_root)" = "$2/src" ]; wrapper_cmd termux-version-help' \
    _ "$ROOT_DIR/lib/codex-termux.sh" "$ROOT_DIR" >"$TMP_DIR/shell.out"
grep -F 'Usage: codex termux version' "$TMP_DIR/shell.out" >/dev/null

printf 'python-package-compat: ok\n'
