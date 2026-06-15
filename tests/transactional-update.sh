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
trap 'chmod -R u+w "$FIXTURE_ROOT" 2>/dev/null || true; rm -rf "$FIXTURE_ROOT"' EXIT

# shellcheck disable=SC1091
. "$ROOT_DIR/tests/fixtures/activation-fixture.sh"

prepare_case() {
    local root="$1"
    activation_fixture_export_env "$root"
    # shellcheck disable=SC1091
    . "$ROOT_DIR/lib/codex-termux-lib.sh"
    activation_fixture_make_candidate "$root" old healthy
    activation_fixture_activate_candidate old >/dev/null || fail "initial activation failed"
    activation_fixture_capture "$root"
}

run_metadata_failure() (
    local name="$1" root blocked_parent
    root="$FIXTURE_ROOT/$name"
    prepare_case "$root"
    activation_fixture_make_candidate "$root" new healthy
    if [ "$name" = "state-write" ]; then
        blocked_parent="$(dirname "$CODEX_NATIVE_STATE_FILE")"
    else
        blocked_parent="$(dirname "$CODEX_NATIVE_REGISTRY_FILE")"
    fi
    chmod a-w "$blocked_parent"
    if activation_fixture_activate_candidate new >"$root/failure.log" 2>&1; then
        fail "$name failure injection unexpectedly activated candidate"
    fi
    chmod u+w "$blocked_parent"
    activation_fixture_assert_unchanged "$root" || fail "$name did not preserve old metadata and pointers"
    if grep -q 'rollback failed' "$root/failure.log"; then
        fail "$name caused a secondary rollback failure"
    fi
)

run_cleanup_failure() (
    local root="$FIXTURE_ROOT/cleanup"
    prepare_case "$root"
    activation_fixture_make_candidate "$root" new fail-cleanup
    if activation_fixture_activate_candidate new >"$root/failure.log" 2>&1; then
        fail "rollback cleanup failure injection unexpectedly succeeded"
    fi
    chmod u+w "$CODEX_NATIVE_RUNTIME_STORE_DIR"
    activation_fixture_assert_unchanged "$root" || fail "cleanup failure did not otherwise restore old transaction"
    grep -q 'rollback failed: transaction cleanup:' "$root/failure.log" \
        || fail "rollback cleanup failure was not aggregated"
)

run_snapshot_restore_failure() (
    local root="$FIXTURE_ROOT/snapshot-restore" expected="$FIXTURE_ROOT/snapshot-restore/expected"
    prepare_case "$root"
    activation_fixture_make_candidate "$root" new fail-state-restore
    if activation_fixture_activate_candidate new >"$root/failure.log" 2>&1; then
        fail "snapshot restore failure injection unexpectedly succeeded"
    fi
    chmod u+w "$CODEX_NATIVE_STATE_DIR"
    [ "$(readlink "$CODEX_NATIVE_CURRENT_LINK")" = "$(cat "$expected/current")" ] \
        || fail "snapshot restore failure did not restore current pointer"
    [ "$(readlink "$CODEX_NATIVE_VERIFIED_LINK")" = "$(cat "$expected/verified")" ] \
        || fail "snapshot restore failure did not restore verified pointer"
    [ "$(readlink "$CODEX_NATIVE_RAW_DIR")" = "$(cat "$expected/raw")" ] \
        || fail "snapshot restore failure did not restore raw pointer"
    cmp -s "$CODEX_NATIVE_REGISTRY_FILE" "$expected/registry.json" \
        || fail "snapshot restore failure did not restore registry"
    if cmp -s "$CODEX_NATIVE_STATE_FILE" "$expected/state.json"; then
        fail "snapshot restore failure injection did not leave state unrestored"
    fi
    grep -q 'rollback failed: state:' "$root/failure.log" \
        || fail "snapshot restore failure was not aggregated"
)

run_metadata_failure state-write
run_metadata_failure registry-write
run_cleanup_failure
run_snapshot_restore_failure

printf 'transactional-update: ok\n'
