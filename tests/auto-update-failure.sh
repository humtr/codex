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
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_AUTO_UPDATE_PENDING="$CODEX_NATIVE_STATE_DIR/pending"
export CODEX_NATIVE_AUTO_UPDATE_FAILED="$CODEX_NATIVE_STATE_DIR/failed"
export CODEX_NATIVE_AUTO_UPDATE_STAMP="$CODEX_NATIVE_STATE_DIR/stamp"
export CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS=3600
export CODEX_NATIVE_AUTO_UPDATE_MODE=force
mkdir -p "$CODEX_NATIVE_STATE_DIR"
printf 'stale-test-version\n' >"$CODEX_NATIVE_AUTO_UPDATE_PENDING"
date +%s >"$CODEX_NATIVE_AUTO_UPDATE_STAMP"
cat >"$CODEX_NATIVE_STATE_FILE" <<'EOF'
{"schema":3,"version":"1.0.0","raw_sha256":"raw","runtime_sha256":"runtime","package_spec":"@openai/codex@1.0.0","active_tuple_id":"tuple","wrapper_version":"test","wrapper_commit":"test","updated_at":"2026-06-15T00:00:00Z","verified_tuple_id":"tuple","verified_at":"2026-06-15T00:00:00Z"}
EOF

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

update_calls=0
codex_runtime_ok() { return 0; }
codex_install_auto_update() {
    update_calls=$((update_calls + 1))
    codex_write_failed_auto_update "$2"
    return 1
}

codex_auto_update_if_needed >/dev/null 2>&1 || fail "auto-update failure leaked into runtime path"
codex_auto_update_if_needed >/dev/null 2>&1 || fail "repeated auto-update failure leaked into runtime path"
[ "$update_calls" -eq 1 ] || fail "failed version was retried before backoff elapsed"

printf 'stale-test-version\n' >"$CODEX_NATIVE_AUTO_UPDATE_PENDING"
printf '0\n' >"$CODEX_NATIVE_AUTO_UPDATE_STAMP"
export CODEX_NATIVE_AUTO_UPDATE_TIMEOUT_SECONDS=0
codex_auto_update_if_needed >/dev/null 2>&1 || fail "stale pending validation failure leaked into runtime path"
[ "$(codex_read_pending_auto_update)" != "stale-test-version" ] || fail "unverified stale pending update was retained"

update_called=0
codex_update() { update_called=1; return 1; }

if codex_update_public >/dev/null 2>&1; then
    fail "explicit update failure was hidden"
fi
[ "$update_called" -eq 1 ] || fail "explicit update path did not reach codex_update"

printf 'auto-update-failure: ok\n'
