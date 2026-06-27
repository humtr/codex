#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${PREFIX:-/data/data/com.termux/files/usr}/tmp}"
TMP_DIR="$TMP_PARENT/codex-install-dispatch-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"

fail() {
    printf 'install-dispatch: FAIL: %s\n' "$*" >&2
    exit 1
}

# shellcheck disable=SC1090
. "$ROOT_DIR/bin/install-runtime.sh"

USAGE_CALLED=0
FAILED_MESSAGE=""
UPSTREAM_ARG="__unset__"
SUPPORT_COUNT=0
LAUNCHER_COUNT=0
CACHED_COUNT=0
REPAIR_COUNT=0
VERSION_COUNT=0
STATUS_LOG=""
SAY_LOG=""

usage() { USAGE_CALLED=1; }
codex_fail() { FAILED_MESSAGE="$*"; return 1; }
codex_with_lock() { local cmd="$1"; shift; "$cmd" "$@"; }
codex_prepare_fresh_wrapper_source() { return 0; }
codex_cleanup_fresh_wrapper_source() { return 0; }
codex_validate_runtime_retention() { return 0; }
codex_install_support_files() { SUPPORT_COUNT=$((SUPPORT_COUNT + 1)); }
codex_install_launchers() { LAUNCHER_COUNT=$((LAUNCHER_COUNT + 1)); }
codex_runtime_install_upstream() { UPSTREAM_ARG="${1:-}"; }
codex_runtime_install_cached() { CACHED_COUNT=$((CACHED_COUNT + 1)); }
codex_refresh_runtime_metadata() { return 0; }
codex_version() { VERSION_COUNT=$((VERSION_COUNT + 1)); }
codex_repair_core_unlocked() { REPAIR_COUNT=$((REPAIR_COUNT + 1)); }
codex_status() { STATUS_LOG="${STATUS_LOG}${STATUS_LOG:+|}$*"; }
codex_status_clear() { STATUS_LOG="${STATUS_LOG}${STATUS_LOG:+|}<clear>"; }
codex_say() { SAY_LOG="${SAY_LOG}${SAY_LOG:+|}$*"; }

USAGE_CALLED=0
codex_install_dispatch upstream --help
[ "$USAGE_CALLED" -eq 1 ] || fail 'install upstream --help did not show usage'
[ "$UPSTREAM_ARG" = "__unset__" ] || fail 'install upstream --help reached upstream install'

if codex_install_dispatch upstream --bad-option; then
    fail 'install upstream accepted an option-looking version'
fi
case "$FAILED_MESSAGE" in
    *"must not start with '-'"*) ;;
    *) fail "unexpected upstream option error: $FAILED_MESSAGE" ;;
esac

if codex_install_dispatch upstream 0.1.0 extra; then
    fail 'install upstream accepted extra arguments'
fi
case "$FAILED_MESSAGE" in
    *"at most one version"*) ;;
    *) fail "unexpected upstream arity error: $FAILED_MESSAGE" ;;
esac

SUPPORT_COUNT=0
LAUNCHER_COUNT=0
CACHED_COUNT=0
REPAIR_COUNT=0
VERSION_COUNT=0
STATUS_LOG=""
SAY_LOG=""
codex_install_dispatch rebuild
[ "$SUPPORT_COUNT" -eq 1 ] || fail 'install rebuild did not refresh support'
[ "$LAUNCHER_COUNT" -eq 1 ] || fail 'install rebuild did not refresh launcher'
[ "$CACHED_COUNT" -eq 1 ] || fail 'install rebuild did not rebuild cached runtime'
[ "$REPAIR_COUNT" -eq 0 ] || fail 'install rebuild called repair'
[ "$VERSION_COUNT" -eq 1 ] || fail 'install rebuild did not render version from surface'
case "$STATUS_LOG" in
    "Rebuilding runtime from cached raw package"*) ;;
    *) fail "install rebuild did not use rebuild surface: $STATUS_LOG" ;;
esac

SUPPORT_COUNT=0
LAUNCHER_COUNT=0
VERSION_COUNT=0
STATUS_LOG=""
SAY_LOG=""
codex_install_dispatch support
[ "$SUPPORT_COUNT" -eq 1 ] || fail 'install support did not refresh support'
[ "$LAUNCHER_COUNT" -eq 1 ] || fail 'install support did not refresh launcher'
[ "$VERSION_COUNT" -eq 0 ] || fail 'install support should not render version'
case "$STATUS_LOG" in
    "Installing wrapper support and launcher"*) ;;
    *) fail "install support did not use support surface: $STATUS_LOG" ;;
esac
case "$SAY_LOG" in
    "Support files and launcher are ready") ;;
    *) fail "install support did not render support completion: $SAY_LOG" ;;
esac

REPAIR_COUNT=0
VERSION_COUNT=0
STATUS_LOG=""
SAY_LOG=""
main repair
[ "$REPAIR_COUNT" -eq 1 ] || fail 'repair did not call repair core'
[ "$VERSION_COUNT" -eq 1 ] || fail 'repair did not render version from surface'
case "$STATUS_LOG" in
    "Repairing managed installation"*) ;;
    *) fail "repair did not use repair surface: $STATUS_LOG" ;;
esac

VERSION_COUNT=0
STATUS_LOG=""
SAY_LOG=""
codex_version() { VERSION_COUNT=$((VERSION_COUNT + 1)); return 7; }
if codex_install_surface_run install "" codex_validate_runtime_retention; then
    fail 'surface finish failure did not propagate'
fi
codex_version() { VERSION_COUNT=$((VERSION_COUNT + 1)); }

SUPPORT_COUNT=0
LAUNCHER_COUNT=0
VERSION_COUNT=0
STATUS_LOG=""
SAY_LOG=""
CODEX_TERMUX_INSTALL_SURFACE=0 codex_install_dispatch support
[ "$SUPPORT_COUNT" -eq 1 ] || fail 'quiet install support did not refresh support'
[ "$LAUNCHER_COUNT" -eq 1 ] || fail 'quiet install support did not refresh launcher'
[ -z "$STATUS_LOG" ] || fail "quiet install support emitted status: $STATUS_LOG"
[ -z "$SAY_LOG" ] || fail "quiet install support emitted completion: $SAY_LOG"
unset CODEX_TERMUX_INSTALL_SURFACE

curl() {
    local out="" arg
    printf '%s\n' "$*" >"$TMP_DIR/curl-args"
    while [ "$#" -gt 0 ]; do
        arg="$1"
        shift
        if [ "$arg" = "-o" ]; then
            out="${1:-}"
            shift || true
        fi
    done
    [ -n "$out" ] || fail 'mock curl did not receive output path'
    printf 'archive\n' >"$out"
}

CODEX_TERMUX_WRAPPER_RELEASE_TOKEN="test-token" \
codex_download_wrapper_archive \
    "https://api.github.com/repos/example/private/releases/assets/123" \
    "$TMP_DIR/release.tgz"
grep -F "Authorization: Bearer test-token" "$TMP_DIR/curl-args" >/dev/null \
    || fail 'release token was not passed to curl'
grep -F "Accept: application/octet-stream" "$TMP_DIR/curl-args" >/dev/null \
    || fail 'GitHub release asset accept header was not passed to curl'
grep -Fx "archive" "$TMP_DIR/release.tgz" >/dev/null \
    || fail 'mock release archive was not written'

printf 'install-dispatch: ok\n'
