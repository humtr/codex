#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${PREFIX:-/data/data/com.termux/files/usr}/tmp}"
TMP_DIR="$TMP_PARENT/codex-cli-surface-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"

fail() {
    printf 'cli-surface: FAIL: %s\n' "$*" >&2
    exit 1
}

output="$(
    CODEX_TERMUX_HOME="$TMP_DIR/home" \
    CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
    CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
    bash -lc '. /data/data/com.termux/files/home/prj/codex/lib/codex-termux.sh; codex_wrapper_help' 2>&1
)"
case "$output" in
    *"session   Resume previous Codex sessions across profiles."*) ;;
    *) fail 'session help still describes a reserved surface' ;;
esac

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. /data/data/com.termux/files/home/prj/codex/lib/codex-termux.sh; VERSION_CALLED=0; codex_version() { VERSION_CALLED=1; }; codex_main version >/dev/null 2>&1; [ "$VERSION_CALLED" -eq 1 ]; VERSION_CALLED=0; ! codex_main version junk >/dev/null 2>&1; [ "$VERSION_CALLED" -eq 0 ]'

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. /data/data/com.termux/files/home/prj/codex/lib/codex-termux.sh; codex_install_source_command() { return 1; }; REPAIR_CALLED=0; codex_repair_public() { REPAIR_CALLED=1; }; codex_repair_surface_public >/dev/null 2>&1; [ "$REPAIR_CALLED" -eq 1 ]; REPAIR_CALLED=0; ! codex_repair_surface_public junk >/dev/null 2>&1; [ "$REPAIR_CALLED" -eq 0 ]'

printf 'cli-surface: ok\n'
