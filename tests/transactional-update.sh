#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'transactional-update: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/transactional-update-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

active_raw="$FIXTURE_ROOT/native/raw/vendor/aarch64-unknown-linux-musl"
active_runtime="$FIXTURE_ROOT/native/runtime"
candidate_raw_root="$FIXTURE_ROOT/candidate.raw"
candidate_raw="$candidate_raw_root/vendor/aarch64-unknown-linux-musl"
candidate_runtime="$FIXTURE_ROOT/candidate.runtime"
mkdir -p "$active_raw/bin" "$active_runtime" "$candidate_raw/bin" "$candidate_runtime"
printf 'old raw\n' >"$active_raw/bin/codex"
printf 'old runtime\n' >"$active_runtime/codex"
printf 'new raw\n' >"$candidate_raw/bin/codex"
printf 'new runtime\n' >"$candidate_runtime/codex"
chmod 755 "$active_raw/bin/codex" "$candidate_raw/bin/codex" "$candidate_runtime/codex"

export CODEX_NATIVE_RAW_DIR="$FIXTURE_ROOT/native/raw"
export CODEX_NATIVE_RUNTIME_DIR="$active_runtime"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$CODEX_NATIVE_STATE_DIR/store"
mkdir -p "$CODEX_NATIVE_STATE_DIR"
printf 'old state\n' >"$CODEX_NATIVE_STATE_FILE"
printf 'old registry\n' >"$CODEX_NATIVE_REGISTRY_FILE"
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

active_raw_before="$(cat "$active_raw/bin/codex")"
active_runtime_before="$(cat "$active_runtime/codex")"

codex_store_runtime_payload() {
    printf '%s\n' "$FIXTURE_ROOT/store/runtime"
}
codex_store_raw_payload() {
    printf '%s\n' "$FIXTURE_ROOT/store/raw"
}
codex_smoke_test_runtime() { return 0; }
codex_record_registry() {
    printf 'new registry\n' >"$CODEX_NATIVE_REGISTRY_FILE"
    printf 'new-tuple\n'
}
codex_write_json_state() {
    printf 'new state\n' >"$CODEX_NATIVE_STATE_FILE"
    return 1
}

if codex_commit_runtime_candidate "$candidate_runtime" "2.0.0" "raw-sha" "runtime-sha" "package" "$candidate_raw_root"; then
    fail "candidate commit unexpectedly succeeded after state write failure"
fi

[ "$(cat "$active_raw/bin/codex")" = "$active_raw_before" ] || fail "active raw changed during rollback"
[ "$(cat "$active_runtime/codex")" = "$active_runtime_before" ] || fail "active runtime changed during rollback"
[ "$(cat "$CODEX_NATIVE_STATE_FILE")" = "old state" ] || fail "state was not restored"
[ "$(cat "$CODEX_NATIVE_REGISTRY_FILE")" = "old registry" ] || fail "registry was not restored"

printf 'transactional-update: ok\n'
