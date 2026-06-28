#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
RUN_TMP="$(mktemp -d "$TMP_PARENT/codex-tests.XXXXXX")"
trap 'rm -rf "$RUN_TMP"' EXIT
TEST_TIMEOUT_SECONDS="${CODEX_TERMUX_TEST_TIMEOUT_SECONDS:-120}"

run_test() {
    local test_script="$1" name log status=0
    name="${test_script##*/}"
    log="$RUN_TMP/$name.log"
    printf '== %s ==\n' "$name"
    if command -v timeout >/dev/null 2>&1; then
        timeout "$TEST_TIMEOUT_SECONDS" bash "$test_script" >"$log" 2>&1 || status=$?
    else
        bash "$test_script" >"$log" 2>&1 || status=$?
    fi
    cat "$log"
    if [ "$status" -ne 0 ]; then
        printf 'tests: FAIL: %s exited with status %s\n' "$name" "$status" >&2
        return "$status"
    fi
}

for test_script in \
    "$ROOT_DIR/tests/invariants.sh" \
    "$ROOT_DIR/tests/runtime-build.sh" \
    "$ROOT_DIR/tests/package-safety.sh" \
    "$ROOT_DIR/tests/tmp-paths.sh" \
    "$ROOT_DIR/tests/lock.sh" \
    "$ROOT_DIR/tests/install-dispatch.sh" \
    "$ROOT_DIR/tests/doctor.sh" \
    "$ROOT_DIR/tests/wrapper-source-config.sh" \
    "$ROOT_DIR/tests/cli-surface.sh" \
    "$ROOT_DIR/tests/notify.sh" \
    "$ROOT_DIR/tests/store-rollback.sh" \
    "$ROOT_DIR/tests/session.sh"
do
    run_test "$test_script"
done

printf 'tests: ok\n'
