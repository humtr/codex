#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'network-boundary: FAIL: %s\n' "$*" >&2
    exit 1
}

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"
[ -x "$CODEX_NATIVE_RUNTIME" ] || fail "live runtime is required"
codex_prepare_runtime_env
report="$(codex_network_boundary_json)"
status="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["overallStatus"])' <<<"$report")"
case "$status" in
    ok|inconclusive)
        ;;
    *)
        printf '%s\n' "$report" >&2
        fail "network boundary contract failed"
        ;;
esac
printf 'network-boundary: %s\n' "$status"
