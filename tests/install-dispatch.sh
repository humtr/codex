#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

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
codex_version() { return 0; }
codex_repair_public() { REPAIR_COUNT=$((REPAIR_COUNT + 1)); }
codex_status_clear() { return 0; }

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
codex_install_dispatch rebuild
[ "$SUPPORT_COUNT" -eq 1 ] || fail 'install rebuild did not refresh support'
[ "$LAUNCHER_COUNT" -eq 1 ] || fail 'install rebuild did not refresh launcher'
[ "$CACHED_COUNT" -eq 1 ] || fail 'install rebuild did not rebuild cached runtime'
[ "$REPAIR_COUNT" -eq 0 ] || fail 'install rebuild called repair'

printf 'install-dispatch: ok\n'
