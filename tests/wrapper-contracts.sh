#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$(mktemp -d "$TMP_PARENT/codex-wrapper-contracts.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
fail() { printf 'wrapper-contracts: FAIL: %s\n' "$*" >&2; exit 1; }

LIB_SH="$ROOT_DIR/lib/codex-termux.sh"
INSTALL_RUNTIME="$ROOT_DIR/bin/install-runtime.sh"
output="$(
    CODEX_TERMUX_HOME="$TMP_DIR/home" CODEX_TERMUX_PREFIX="$TMP_DIR/prefix" \
    bash -lc '. "$1"; codex_ensure_runtime_ready() { return 0; }; codex_auto_update_if_needed() { return 0; }; codex_runtime_exec_with_context() { printf "%s\n" "$*"; }; codex_main run --flag' \
        _ "$LIB_SH"
)"
[ "$output" = "run --flag" ] || fail "codex public entrypoint changed: $output"

CODEX_TERMUX_HOME="$TMP_DIR/home" CODEX_HOME="$TMP_DIR/ocdx-home" \
    bash -lc '. "$1"; wrapper_cmd profile-write-recent --profile work; [ "$(cat "$CODEX_TERMUX_LAST_PROFILE_FILE")" = work ]; case "$CODEX_TERMUX_LAST_PROFILE_FILE" in "$CODEX_HOME"/*) exit 9 ;; esac' \
        _ "$LIB_SH" \
    || fail 'parallel CODEX_HOME contaminated codex recent-profile state'

MANAGER_DIR="$TMP_DIR/manager"
mkdir -p "$MANAGER_DIR"
cp "$LIB_SH" "$MANAGER_DIR/lib.sh"
cp -R "$ROOT_DIR/shell" "$MANAGER_DIR/shell"
cp -R "$ROOT_DIR/src" "$MANAGER_DIR/src"
cp -R "$ROOT_DIR/libexec" "$MANAGER_DIR/libexec"
cp -R "$ROOT_DIR/lib/codex-termux" "$MANAGER_DIR/codex-termux"
cp -R "$ROOT_DIR/tools/codex_termux" "$MANAGER_DIR/codex_termux"
CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PREFIX="$TMP_DIR/prefix" \
CODEX_TERMUX_MANAGER_DIR="$MANAGER_DIR" \
    bash -lc '
. "$1"
[ "$CODEX_TERMUX_SHELL_LIB" = "$2/shell/loader.sh" ] || {
    printf "CODEX_TERMUX_SHELL_LIB=%s, expected=%s\n" "$CODEX_TERMUX_SHELL_LIB" "$2/shell/loader.sh" >&2
    exit 1
}
[ "$(wrapper_package_root)" = "$2/src" ]
' _ "$MANAGER_DIR/lib.sh" "$MANAGER_DIR" \
    || fail 'installed manager facade resolves wrong role-oriented paths'

# shellcheck disable=SC1090
. "$INSTALL_RUNTIME"
USAGE_CALLED=0
FAILED_MESSAGE=""
SUPPORT_COUNT=0
LAUNCHER_COUNT=0
CACHED_COUNT=0
UPSTREAM_ARG="__unset__"
DOCTOR_ARGS=""
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
codex_status() { :; }
codex_status_clear() { :; }
codex_say() { :; }
codex_termux_doctor() { DOCTOR_ARGS="$*"; }

codex_install_run_plan install support
[ "$SUPPORT_COUNT" -eq 1 ] && [ "$LAUNCHER_COUNT" -eq 1 ] && [ "$CACHED_COUNT" -eq 0 ] \
    || fail 'install support contract failed'
SUPPORT_COUNT=0; LAUNCHER_COUNT=0; CACHED_COUNT=0
codex_install_run_plan install rebuild
[ "$SUPPORT_COUNT" -eq 1 ] && [ "$LAUNCHER_COUNT" -eq 1 ] && [ "$CACHED_COUNT" -eq 1 ] \
    || fail 'install rebuild contract failed'
codex_install_run_plan install upstream
[ "$UPSTREAM_ARG" = "" ] || fail 'install upstream empty version contract failed'
codex_install_run_plan install upstream 0.142.0
[ "$UPSTREAM_ARG" = "0.142.0" ] || fail 'install upstream optional version contract failed'
if codex_install_run_plan install upstream --bad-option; then
    fail 'install upstream accepted option-like version'
fi
main doctor --json
[ "$DOCTOR_ARGS" = "--json" ] || fail "doctor did not dispatch to codex_termux_doctor: $DOCTOR_ARGS"

grep -F 'prepare_support_install' "$INSTALL_RUNTIME" >/dev/null \
    || fail 'support install is not transactional'
grep -F 'rollback_support_install' "$INSTALL_RUNTIME" >/dev/null \
    || fail 'support rollback contract missing'
grep -F 'commit_support_install' "$INSTALL_RUNTIME" >/dev/null \
    || fail 'support commit contract missing'

printf 'wrapper-contracts: ok\n'
