#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'doctor-contract: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/doctor-contract-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

fake_runtime="$FIXTURE_ROOT/codex"
printf '#!%s\nprintf '\''upstream:%%s\\n'\'' "$*"\n' "$(command -v sh)" >"$fake_runtime"
chmod 755 "$fake_runtime"

export CODEX_NATIVE_RUNTIME="$fake_runtime"
fixture_json=""

# shellcheck disable=SC1091
. "$ROOT_DIR/tests/fixtures/doctor-fixtures.sh"

run_wrapper_case() {
    local name="$1" expected_exit="$2" expected_summary="$3" summary_snippet="$4" extra_snippet="${5:-}"
    local output status
    set +e
    output="$(printf '%s\n' "$fixture_json" | PYTHONPATH="$ROOT_DIR/tools" python3 -m codex_native.cli doctor-render --mode human 2>&1)"
    status=$?
    set -e
    [ "$status" -eq "$expected_exit" ] || fail "$name exit code mismatch: got $status"
    [[ "$output" == "Termux Wrapper Doctor"* ]] || fail "$name did not render title"
    [[ "$output" == *$'\nStorage\n'* ]] || fail "$name did not render storage section"
    [[ "$output" == *$'\nMigration\n'* ]] || fail "$name did not render migration section"
    [[ "$output" == *"$expected_summary"* ]] || fail "$name summary mismatch"
    [[ "$output" == *"$summary_snippet"* ]] || fail "$name missing expected snippet"
    if [ -n "$extra_snippet" ]; then
        [[ "$output" == *"$extra_snippet"* ]] || fail "$name missing extra snippet"
    fi
    case "$output" in
        "{"*) fail "$name default output is still raw JSON" ;;
    esac
}

fixture_json="$(doctor_fixture_healthy_not_needed)"
run_wrapper_case \
    "healthy-not-needed" \
    0 \
    "23 ok · 1 idle · 0 warn · 0 fail ok" \
    "legacy store cache migration not-needed" \
    "manager                  /manager"

fixture_json="$(doctor_fixture_healthy_issues)"
run_wrapper_case \
    "healthy-issues" \
    0 \
    "23 ok · 0 idle · 1 warn · 0 fail degraded" \
    "legacy store cache migration issues" \
    "migration    legacy store migration needs attention; active runtime may still be healthy."

fixture_json="$(doctor_fixture_broken_current)"
run_wrapper_case \
    "broken-current" \
    1 \
    "23 ok · 0 idle · 0 warn · 1 fail fail" \
    "current      active runtime pointer is aligned with registry" \
    "wrapper      one or more wrapper checks failed."

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"
codex_ensure_runtime_ready() { return 0; }
codex_prepare_runtime_env() { return 0; }
codex_wrapper_doctor() { printf 'wrapper\n'; }

human="$(codex_public_doctor)"
[ "$human" = $'upstream:doctor\n\n─────────────────────────────────────────────────────────────\n\nwrapper' ] \
    || fail "default doctor did not compose upstream and wrapper output"
json="$(codex_public_doctor --json)"
[ "$json" = "upstream:doctor --json" ] \
    || fail "doctor arguments were not passed directly to upstream"

printf 'doctor-contract: ok\n'
