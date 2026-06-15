#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'pointer-rollback: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/pointer-rollback-test.XXXXXX")"
trap 'chmod -R u+w "$FIXTURE_ROOT" 2>/dev/null || true; rm -rf "$FIXTURE_ROOT"' EXIT

# shellcheck disable=SC1091
. "$ROOT_DIR/tests/fixtures/activation-fixture.sh"

run_failure_case() (
    local name="$1" mode="${2:-healthy}" root blocked_parent=""
    root="$FIXTURE_ROOT/$name"
    activation_fixture_export_env "$root"
    # shellcheck disable=SC1091
    . "$ROOT_DIR/lib/codex-termux-lib.sh"
    activation_fixture_make_candidate "$root" old healthy
    activation_fixture_activate_candidate old >/dev/null || fail "$name initial activation failed"
    activation_fixture_capture "$root"
    activation_fixture_make_candidate "$root" new "$mode"

    case "$name" in
        current) blocked_parent="$(dirname "$CODEX_NATIVE_CURRENT_LINK")" ;;
        verified) blocked_parent="$(dirname "$CODEX_NATIVE_VERIFIED_LINK")" ;;
        raw) blocked_parent="$(dirname "$CODEX_NATIVE_RAW_DIR")" ;;
    esac
    [ -z "$blocked_parent" ] || chmod a-w "$blocked_parent"
    if activation_fixture_activate_candidate new >"$root/failure.log" 2>&1; then
        fail "$name failure injection unexpectedly activated candidate"
    fi
    [ -z "$blocked_parent" ] || chmod u+w "$blocked_parent"
    activation_fixture_assert_unchanged "$root" || fail "$name did not restore the old transaction"
    if grep -q 'rollback failed' "$root/failure.log"; then
        fail "$name failure caused a secondary rollback failure"
    fi
)

run_failure_case current
run_failure_case verified
run_failure_case raw
run_failure_case readiness fail-readiness

printf 'pointer-rollback: ok\n'
