#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

fail() {
    printf 'lock: FAIL: %s\n' "$*" >&2
    exit 1
}

CODEX_TERMUX_HOME="$TMP_ROOT/home" \
CODEX_TERMUX_PREFIX="$TMP_ROOT/prefix" \
CODEX_TERMUX_STATE_DIR="$TMP_ROOT/state" \
CODEX_TERMUX_LOCK_HELD=1 \
bash -lc '
    . "$1"
    check_lock_file() {
        [ -e "$CODEX_TERMUX_LOCK_FILE" ] || return 42
    }
    codex_with_lock check_lock_file
' _ "$ROOT_DIR/lib/codex-termux.sh" || fail 'external CODEX_TERMUX_LOCK_HELD bypassed locking'

CODEX_TERMUX_HOME="$TMP_ROOT/home" \
CODEX_TERMUX_PREFIX="$TMP_ROOT/prefix" \
CODEX_TERMUX_STATE_DIR="$TMP_ROOT/state" \
bash -lc '
    . "$1"
    inner() { :; }
    outer() { codex_with_lock inner; }
    codex_with_lock outer
' _ "$ROOT_DIR/lib/codex-termux.sh" || fail 'nested lock did not complete'

printf 'lock: ok\n'
