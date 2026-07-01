#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
RUN_TMP="$(mktemp -d "$TMP_PARENT/codex-tests.XXXXXX")"
trap 'rm -rf "$RUN_TMP"' EXIT
TEST_TIMEOUT_SECONDS="${CODEX_TERMUX_TEST_TIMEOUT_SECONDS:-120}"
TEST_LOG_DIR="${CODEX_TERMUX_TEST_LOG_DIR:-}"
TEST_STATUS_FILE="${CODEX_TERMUX_TEST_STATUS_FILE:-}"
RERUN_FAILED_XTRACE="${CODEX_TERMUX_TEST_RERUN_FAILED_XTRACE:-0}"

clean_bytecode_noise() {
    find "$ROOT_DIR" \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -exec rm -rf {} + 2>/dev/null || true
}

if [ -n "$TEST_LOG_DIR" ]; then
    mkdir -p "$TEST_LOG_DIR"
fi
if [ -n "$TEST_STATUS_FILE" ]; then
    mkdir -p "$(dirname "$TEST_STATUS_FILE")"
    : >"$TEST_STATUS_FILE"
fi

test_log_path() {
    local name="$1"
    if [ -n "$TEST_LOG_DIR" ]; then
        printf '%s/%s.log\n' "$TEST_LOG_DIR" "$name"
    else
        printf '%s/%s.log\n' "$RUN_TMP" "$name"
    fi
}

test_meta_path() {
    local name="$1"
    if [ -n "$TEST_LOG_DIR" ]; then
        printf '%s/%s.meta\n' "$TEST_LOG_DIR" "$name"
    else
        printf '%s/%s.meta\n' "$RUN_TMP" "$name"
    fi
}

run_test_script() {
    local test_script="$1" name="$2" xtrace="${3:-0}"
    local bash_args=()
    if [ "$xtrace" = "1" ]; then
        bash_args=(-x)
    fi
    case "$name" in
        package-safety.sh)
            CODEX_TERMUX_TARBALL_VALIDATOR_TIMEOUT_SECONDS="${CODEX_TERMUX_TARBALL_VALIDATOR_TIMEOUT_SECONDS:-10}" \
                PS4='+ ${BASH_SOURCE##*/}:${LINENO}: ' \
                bash "${bash_args[@]}" "$test_script"
            ;;
        wrapper-archive-safety.sh|release-package.sh)
            if command -v timeout >/dev/null 2>&1; then
                PS4='+ ${BASH_SOURCE##*/}:${LINENO}: ' \
                    timeout -k 5s "$TEST_TIMEOUT_SECONDS" bash "${bash_args[@]}" "$test_script"
            else
                PS4='+ ${BASH_SOURCE##*/}:${LINENO}: ' \
                    bash "${bash_args[@]}" "$test_script"
            fi
            ;;
        *)
            if command -v timeout >/dev/null 2>&1; then
                PS4='+ ${BASH_SOURCE##*/}:${LINENO}: ' \
                    timeout -k 5s "$TEST_TIMEOUT_SECONDS" bash "${bash_args[@]}" "$test_script"
            else
                PS4='+ ${BASH_SOURCE##*/}:${LINENO}: ' \
                    bash "${bash_args[@]}" "$test_script"
            fi
            ;;
    esac
}

write_test_meta() {
    local test_script="$1" name="$2" status="$3" log="$4" meta
    meta="$(test_meta_path "$name")"
    {
        printf 'name=%s\n' "$name"
        printf 'script=%s\n' "$test_script"
        printf 'status=%s\n' "$status"
        printf 'log=%s\n' "$log"
        printf 'root=%s\n' "$ROOT_DIR"
        printf 'tmp_parent=%s\n' "$TMP_PARENT"
        printf 'run_tmp=%s\n' "$RUN_TMP"
        printf 'timeout_seconds=%s\n' "$TEST_TIMEOUT_SECONDS"
        printf 'pwd=%s\n' "$(pwd)"
        printf 'uname=%s\n' "$(uname -a 2>/dev/null || true)"
        printf 'bash=%s\n' "${BASH_VERSION:-unknown}"
    } >"$meta"
}

append_test_status() {
    local name="$1" status="$2"
    if [ -n "$TEST_STATUS_FILE" ]; then
        printf '%s %s\n' "$name" "$status" >>"$TEST_STATUS_FILE"
    fi
}

run_test() {
    local test_script="$1" name log status=0 xtrace_log xtrace_status=0
    name="${test_script##*/}"
    log="$(test_log_path "$name")"
    printf '== %s ==\n' "$name"
    run_test_script "$test_script" "$name" 0 >"$log" 2>&1 || status=$?
    append_test_status "$name" "$status"
    write_test_meta "$test_script" "$name" "$status" "$log"
    cat "$log"
    if [ "$status" -ne 0 ]; then
        printf 'tests: FAIL: %s exited with status %s\n' "$name" "$status" >&2
        printf 'tests: log: %s\n' "$log" >&2
        printf 'tests: meta: %s\n' "$(test_meta_path "$name")" >&2
        if [ "$RERUN_FAILED_XTRACE" = "1" ]; then
            xtrace_log="${log%.log}.xtrace.log"
            printf 'tests: rerunning %s with bash -x for diagnostics\n' "$name" >&2
            run_test_script "$test_script" "$name" 1 >"$xtrace_log" 2>&1 || xtrace_status=$?
            printf 'tests: xtrace status: %s\n' "$xtrace_status" >&2
            printf 'tests: xtrace log: %s\n' "$xtrace_log" >&2
            tail -n 200 "$xtrace_log" >&2 || true
        fi
        return "$status"
    fi
}

clean_bytecode_noise

for test_script in \
    "$ROOT_DIR/tests/invariants.sh" \
    "$ROOT_DIR/tests/runtime-build.sh" \
    "$ROOT_DIR/tests/package-safety.sh" \
    "$ROOT_DIR/tests/wrapper-archive-safety.sh" \
    "$ROOT_DIR/tests/release-package.sh" \
    "$ROOT_DIR/tests/tmp-paths.sh" \
    "$ROOT_DIR/tests/lock.sh" \
    "$ROOT_DIR/tests/install-plan.sh" \
    "$ROOT_DIR/tests/install-dispatch.sh" \
    "$ROOT_DIR/tests/wrapper-contracts.sh" \
    "$ROOT_DIR/tests/repair-diagnosis.sh" \
    "$ROOT_DIR/tests/doctor.sh" \
    "$ROOT_DIR/tests/wrapper-source-config.sh" \
    "$ROOT_DIR/tests/cli-surface.sh" \
    "$ROOT_DIR/tests/notify.sh" \
    "$ROOT_DIR/tests/store-rollback.sh" \
    "$ROOT_DIR/tests/session.sh" \
    "$ROOT_DIR/tools/smoke-termux-wrapper.sh"
do
    run_test "$test_script"
done

printf 'portable tests: ok\n'
