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
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"
codex_ensure_runtime_ready() { return 0; }
codex_prepare_runtime_env() { return 0; }
codex_wrapper_doctor() { printf 'wrapper\n'; }

human="$(codex_public_doctor)"
[ "$human" = $'upstream:doctor\n\nTermux Wrapper\nwrapper' ] \
    || fail "default doctor did not compose upstream and wrapper output"
json="$(codex_public_doctor --json)"
[ "$json" = "upstream:doctor --json" ] \
    || fail "doctor arguments were not passed directly to upstream"

printf 'doctor-contract: ok\n'
