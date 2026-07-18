#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$TMP_PARENT/codex-wrapper-contracts-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"
fail() { printf 'wrapper-contracts: FAIL: %s\n' "$*" >&2; exit 1; }
LIB_SH="$ROOT_DIR/lib/codex-termux.sh"
INSTALL_RUNTIME="$ROOT_DIR/bin/install-runtime.sh"
output="$(CODEX_TERMUX_HOME="$TMP_DIR/home" CODEX_TERMUX_PREFIX="$TMP_DIR/prefix" bash -lc '. "$1"; codex_ensure_runtime_ready() { return 0; }; codex_auto_update_if_needed() { return 0; }; codex_runtime_exec_with_context() { printf "%s\n" "$*"; }; codex_main run --flag' _ "$LIB_SH")"
[ "$output" = "run --flag" ] || fail "codex public entrypoint changed: $output"
CODEX_TERMUX_HOME="$TMP_DIR/home" CODEX_HOME="$TMP_DIR/ocdx-home" bash -lc '. "$1"; codex_termux_cmd profile-write-recent --profile work; [ "$(cat "$CODEX_TERMUX_LAST_PROFILE_FILE")" = work ]; case "$CODEX_TERMUX_LAST_PROFILE_FILE" in "$CODEX_HOME"/*) exit 9 ;; esac' _ "$LIB_SH" || fail 'parallel CODEX_HOME contaminated codex recent-profile state'
MANAGER_DIR="$TMP_DIR/manager"
mkdir -p "$MANAGER_DIR"
cp "$LIB_SH" "$MANAGER_DIR/lib.sh"
mkdir -p "$MANAGER_DIR/source"
cp -R "$ROOT_DIR/shell" "$MANAGER_DIR/source/shell"
CODEX_TERMUX_HOME="$TMP_DIR/home" CODEX_TERMUX_PREFIX="$TMP_DIR/prefix" CODEX_TERMUX_MANAGER_DIR="$MANAGER_DIR" bash -lc '
. "$1"
[ "$CODEX_TERMUX_SHELL_LIB" = "$2" ] || {
    printf "CODEX_TERMUX_SHELL_LIB=%s, expected=%s\n" "$CODEX_TERMUX_SHELL_LIB" "$2" >&2
    exit 1
}
' _ "$MANAGER_DIR/lib.sh" "$MANAGER_DIR/source/shell/loader.sh" || fail 'installed manager lib resolves wrong CODEX_TERMUX_SHELL_LIB'
# shellcheck disable=SC1090
. "$INSTALL_RUNTIME"
USAGE_CALLED=0; FAILED_MESSAGE=""; SUPPORT_COUNT=0; LAUNCHER_COUNT=0; CACHED_COUNT=0; UPSTREAM_ARG="__unset__"; DOCTOR_ARGS=""
usage() { USAGE_CALLED=1; }
codex_fail() { FAILED_MESSAGE="$*"; return 1; }
codex_with_lock() { local cmd="$1"; shift; "$cmd" "$@"; }
codex_prepare_fresh_wrapper_source() { return 0; }
codex_cleanup_fresh_wrapper_source() { return 0; }
codex_validate_runtime_retention() { return 0; }
codex_install_support_files() { SUPPORT_COUNT=$((SUPPORT_COUNT + 1)); }
codex_install_launchers() { LAUNCHER_COUNT=$((LAUNCHER_COUNT + 1)); }
codex_runtime_install_cached() { CACHED_COUNT=$((CACHED_COUNT + 1)); }
codex_runtime_install_upstream() { UPSTREAM_ARG="${1:-}"; }
codex_refresh_runtime_metadata() { return 0; }
codex_version() { return 0; }
codex_status() { :; }; codex_status_clear() { :; }; codex_say() { :; }
codex_termux_doctor() { DOCTOR_ARGS="$*"; }
codex_install_run_plan install support
[ "$SUPPORT_COUNT" -eq 1 ] && [ "$LAUNCHER_COUNT" -eq 1 ] && [ "$CACHED_COUNT" -eq 0 ] || fail 'install support contract failed'
SUPPORT_COUNT=0; LAUNCHER_COUNT=0; CACHED_COUNT=0
codex_install_run_plan install rebuild
[ "$SUPPORT_COUNT" -eq 1 ] && [ "$LAUNCHER_COUNT" -eq 1 ] && [ "$CACHED_COUNT" -eq 1 ] || fail 'install rebuild contract failed'
codex_install_run_plan install upstream
[ "$UPSTREAM_ARG" = "" ] || fail 'install upstream empty version contract failed'
codex_install_run_plan install upstream 0.142.0
[ "$UPSTREAM_ARG" = "0.142.0" ] || fail 'install upstream optional version contract failed'
if codex_install_run_plan install upstream --bad-option; then fail 'install upstream accepted option-like version'; fi
main doctor --json
[ "$DOCTOR_ARGS" = "--json" ] || fail "doctor did not dispatch to codex_termux_doctor: $DOCTOR_ARGS"
grep -F 'codex_rm_rf_managed "$CODEX_TERMUX_MANAGER_DIR/codex-termux"' "$INSTALL_RUNTIME" >/dev/null || fail 'domain support directory is not managed-protected'
grep -F 'cp -R "$source_dir/lib/codex-termux" "$CODEX_TERMUX_MANAGER_DIR/codex-termux"' "$INSTALL_RUNTIME" >/dev/null || fail 'managed support install does not copy domain directory'
printf 'wrapper-contracts: ok\n'
