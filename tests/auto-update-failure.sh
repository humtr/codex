#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

fail() {
    printf 'auto-update-failure: FAIL: %s\n' "$*" >&2
    exit 1
}

export CODEX_NATIVE_STATE_DIR="$fixture_root/state"
export CODEX_NATIVE_AUTO_UPDATE_PENDING="$CODEX_NATIVE_STATE_DIR/pending"
export CODEX_NATIVE_AUTO_UPDATE_FAILED="$CODEX_NATIVE_STATE_DIR/failed"
export CODEX_NATIVE_AUTO_UPDATE_STAMP="$CODEX_NATIVE_STATE_DIR/stamp"
export CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS=3600

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

update_calls=0
codex_runtime_ok() { return 0; }
codex_read_state_field() {
    case "$1" in
        version) printf '1.0.0\n' ;;
        *) printf '\n' ;;
    esac
}
codex_read_pending_auto_update() { printf '\n'; }
codex_auto_update_due() { return 0; }
codex_mark_auto_update_checked() { :; }
codex_latest_linux_arm64_version() { printf '2.0.0\n'; }
codex_auto_update_mode() { printf 'force\n'; }
codex_install_auto_update() {
    update_calls=$((update_calls + 1))
    codex_write_failed_auto_update "$2"
    return 1
}

codex_auto_update_if_needed >/dev/null 2>&1 || fail "auto-update failure leaked into runtime path"
codex_auto_update_if_needed >/dev/null 2>&1 || fail "repeated auto-update failure leaked into runtime path"
[ "$update_calls" -eq 1 ] || fail "failed version was retried before backoff elapsed"

update_called=0
codex_update() { update_called=1; return 1; }
codex_version() { printf 'version-called\n'; }

if codex_update_public >/dev/null 2>&1; then
    fail "explicit update failure was hidden"
fi
[ "$update_called" -eq 1 ] || fail "explicit update path did not reach codex_update"

printf 'auto-update-failure: ok\n'
