#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$TMP_PARENT/codex-cli-surface-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"
LIB_SH="$ROOT_DIR/lib/codex-termux.sh"
INSTALL_RUNTIME="$ROOT_DIR/bin/install-runtime.sh"

fail() {
    printf 'cli-surface: FAIL: %s\n' "$*" >&2
    exit 1
}

output="$(
    CODEX_TERMUX_HOME="$TMP_DIR/home" \
    CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
    CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
    bash -lc '. "$1"; codex_termux_help' _ "$LIB_SH" 2>&1
)"
case "$output" in
    *"codex termux <command> [args...]"*) ;;
    *) fail 'termux help did not describe the wrapper namespace' ;;
esac
case "$output" in
    *"Top-level codex arguments are reserved for upstream Codex."*) ;;
    *) fail 'termux help did not reserve top-level codex args for upstream' ;;
esac

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; VERSION_CALLED=0; codex_version() { VERSION_CALLED=1; }; codex_termux_main version >/dev/null 2>&1; [ "$VERSION_CALLED" -eq 1 ]; VERSION_CALLED=0; ! codex_termux_main version junk >/dev/null 2>&1; [ "$VERSION_CALLED" -eq 0 ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; codex_ensure_runtime_ready() { return 0; }; codex_auto_update_if_needed() { return 0; }; codex_runtime_exec_with_context() { printf "%s\n" "$*"; }; [ "$(codex_main version)" = "version" ]; [ "$(codex_main doctor --json)" = "doctor --json" ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; codex_install_source_command() { return 1; }; REPAIR_CALLED=0; codex_repair_public() { REPAIR_CALLED=1; }; codex_termux_main repair >/dev/null 2>&1; [ "$REPAIR_CALLED" -eq 1 ]; REPAIR_CALLED=0; ! codex_termux_main repair junk >/dev/null 2>&1; [ "$REPAIR_CALLED" -eq 0 ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; output="$(codex_termux_main repair --help 2>&1)"; case "$output" in *"Codex Termux wrapper commands"*) ;; *) exit 1 ;; esac' _ "$LIB_SH" "$INSTALL_RUNTIME"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; stable="$CODEX_TERMUX_RUNTIME_STORE_DIR/stable-runtime"; codex_prepare_system_config() { return 0; }; codex_resolve_path() { [ "$1" = "$CODEX_TERMUX_RUNTIME_DIR" ] && printf "%s\n" "$stable"; }; CODEX_SELF_EXE="$CODEX_TERMUX_RUNTIME"; codex_prepare_runtime_env; [ "$CODEX_SELF_EXE" = "$stable/codex" ]; case "$PATH" in "$stable/codex-path:$stable/codex-resources:"*) ;; *) exit 1 ;; esac' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
TEST_TMP_DIR="$TMP_DIR" \
bash -lc '. "$1"; source_file="$TEST_TMP_DIR/source/bin/install-runtime.sh"; mkdir -p "$(dirname "$source_file")"; printf "#!/bin/sh\nexit 0\n" >"$source_file"; chmod 755 "$source_file"; ran=""; bash() { ran="$1"; [ "$1" != "$source_file" ]; }; codex_mktemp_dir() { d="$TEST_TMP_DIR/snapshot"; rm -rf "$d"; mkdir -p "$d"; printf "%s\n" "$d"; }; codex_run_install_source_command "$source_file" install; case "$ran" in "$TEST_TMP_DIR/snapshot/source/bin/install-runtime.sh") ;; *) exit 1 ;; esac' _ "$LIB_SH"

printf 'cli-surface: ok\n'
